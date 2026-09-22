//! Lifecycle policy. Successful transitions require backend acknowledgement.
use std::time::Duration;

pub const ENTER_TIMEOUT: Duration = Duration::from_secs(3);
pub const ACTIVE_DURATION: Duration = Duration::from_secs(15);
pub const RESTORE_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Normal,
    Entering {
        since: Duration,
    },
    Active {
        since: Duration,
    },
    Restoring {
        since: Duration,
    },
    /// Cleanup remains owned by the backend; no new entry is permitted.
    Fault,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub enter: bool,
    pub revert: bool,
}

/// Implementations retain ownership of all pending work until `restore` returns
/// true, including late spawn results. Errors never mean cleanup succeeded.
pub trait Backend {
    fn begin(&mut self) -> Result<(), String>;
    fn poll_ready(&mut self) -> Result<bool, String>;
    fn maintain(&mut self) -> Result<(), String>;
    fn restore(&mut self) -> Result<bool, String>;
}

pub struct Session {
    phase: Phase,
    last_time: Duration,
    last_error: Option<String>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            phase: Phase::Normal,
            last_time: Duration::ZERO,
            last_error: None,
        }
    }
}

impl Session {
    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn tick(&mut self, time: Duration, input: Input, backend: &mut impl Backend) {
        // Never extend deadlines if a caller supplies a regressing timestamp.
        let now = time.max(self.last_time);
        self.last_time = now;
        match self.phase {
            Phase::Normal => {
                // Escape/revert takes priority over simultaneous activation.
                if input.enter && !input.revert {
                    self.last_error = None;
                    match backend.begin() {
                        Ok(()) => self.phase = Phase::Entering { since: now },
                        Err(error) => self.fail(now, error),
                    }
                }
            }
            Phase::Entering { since } => {
                if input.revert {
                    self.phase = Phase::Restoring { since: now };
                } else if now.saturating_sub(since) >= ENTER_TIMEOUT {
                    // A late ready result must not activate after the deadline.
                    self.fail(now, "enter_timeout".into());
                } else {
                    match backend.poll_ready() {
                        Ok(true) => self.phase = Phase::Active { since: now },
                        Ok(false) => {}
                        Err(error) => self.fail(now, error),
                    }
                }
            }
            Phase::Active { since } => {
                if input.revert || now.saturating_sub(since) >= ACTIVE_DURATION {
                    self.phase = Phase::Restoring { since: now };
                } else if let Err(error) = backend.maintain() {
                    self.fail(now, error);
                }
            }
            Phase::Restoring { .. } | Phase::Fault => {}
        }

        // Begin cleanup in the same tick that requested it. A failed begin can
        // still have partial resources, so it always goes through this path.
        if matches!(self.phase, Phase::Restoring { .. } | Phase::Fault) {
            match backend.restore() {
                Ok(true) => self.phase = Phase::Normal,
                Ok(false) => {}
                Err(error) => self.last_error = Some(error),
            }
            if let Phase::Restoring { since } = self.phase
                && now.saturating_sub(since) >= RESTORE_TIMEOUT
            {
                self.last_error = Some("restore_timeout; cleanup still pending".into());
                self.phase = Phase::Fault;
            }
        }
    }

    fn fail(&mut self, now: Duration, error: String) {
        self.last_error = Some(error);
        self.phase = Phase::Restoring { since: now };
    }
}

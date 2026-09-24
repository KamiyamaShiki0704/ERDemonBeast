//! Temporary single-c0010 test bridge. Uses bounded text messages, never Lua code.
//! No new engine pointers/hooks; the HKS acknowledges real animation completion.
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
static SERIAL: AtomicU64 = AtomicU64::new(0);

pub(super) struct BehaviorBridge {
    root: PathBuf,
    token: String,
    exiting: bool,
    cached: String,
    next_poll: Instant,
    options: String,
}
impl BehaviorBridge {
    pub(super) fn start(root: PathBuf, form: &crate::forms::Form) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let token = format!(
            "{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_millis(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        );
        let result = Self {
            root,
            token,
            exiting: false,
            cached: String::new(),
            next_poll: Instant::now(),
            options: form.bridge_options(),
        };
        result.command("enter")?;
        Ok(result)
    }
    fn command(&self, command: &str) -> Result<(), String> {
        fs::write(
            self.root.join("behavior-command.txt"),
            format!("{}\n{}\n{}", self.token, command, self.options),
        )
        .map_err(|e| format!("behavior command: {e}"))
    }
    fn poll(&mut self) -> Result<&str, String> {
        if Instant::now() >= self.next_poll {
            self.next_poll = Instant::now() + Duration::from_millis(50);
            let path = self.root.join("behavior-status.txt");
            if let Ok(meta) = fs::metadata(&path) {
                if meta.len() > 256 {
                    return Err("oversized behavior status".into());
                }
                let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
                let mut lines = text.lines();
                if lines.next() == Some(self.token.as_str())
                    && let Some(state @ ("starting" | "ready" | "finishing" | "done")) =
                        lines.next()
                    && lines.next().is_none()
                {
                    self.cached = state.into();
                }
            }
        }
        Ok(&self.cached)
    }
    pub(super) fn ready(&mut self) -> Result<bool, String> {
        Ok(self.poll()? == "ready")
    }
    pub(super) fn completed(&mut self) -> Result<bool, String> {
        Ok(self.poll()? == "done")
    }
    pub(super) fn finish(&mut self) -> Result<bool, String> {
        if !self.exiting {
            self.command("exit")?;
            self.exiting = true;
            self.cached.clear();
            self.next_poll = Instant::now();
        }
        Ok(self.poll()? == "done")
    }
    pub(super) fn close(&self) {
        let _ = self.command("closed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_stale_partial_and_ready_as_exit_ack() {
        let root = std::env::temp_dir().join(format!(
            "erdb-bridge-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        let c = crate::forms::Config::parse(include_str!("../../er-demon-beast.toml")).unwrap();
        let mut b = BehaviorBridge::start(root.clone(), &c.forms[0]).unwrap();
        assert!(
            fs::read_to_string(root.join("behavior-command.txt"))
                .unwrap()
                .contains(
                    "enter\noriginal\nv2\ninputs:0,1,2,3,5\nactions:3000,3005,3030,3035,6000\n"
                )
        );
        let status = root.join("behavior-status.txt");
        fs::write(&status, "old-token\ndone\n").unwrap();
        assert!(!b.ready().unwrap());
        fs::write(&status, format!("{}\nready\n", b.token)).unwrap();
        b.next_poll = Instant::now();
        assert!(b.ready().unwrap());
        assert!(!b.completed().unwrap());
        assert!(!b.finish().unwrap());
        fs::write(&status, format!("{}\ndo", b.token)).unwrap();
        b.next_poll = Instant::now();
        assert!(!b.finish().unwrap());
        fs::write(&status, format!("{}\ndone\n", b.token)).unwrap();
        b.next_poll = Instant::now();
        assert!(b.finish().unwrap());
        assert!(b.completed().unwrap());
        b.close();
        assert!(
            fs::read_to_string(root.join("behavior-command.txt"))
                .unwrap()
                .contains("closed")
        );
        fs::remove_file(status).unwrap();
        fs::remove_file(root.join("behavior-command.txt")).unwrap();
        fs::remove_dir(root).unwrap();
    }
}

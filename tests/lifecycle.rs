use er_demon_beast::session::{Backend, Input, Phase, Session};
use std::time::Duration;

#[derive(Default)]
struct FakeGame {
    begin_count: usize,
    ready: bool,
    body_hidden: bool,
    fail_begin: bool,
    fail_active: bool,
    cleanup_pending: bool,
    fail_restore: bool,
    restore_count: usize,
    finishing_pending: bool,
    animation_completed: bool,
}

impl Backend for FakeGame {
    fn begin(&mut self) -> Result<(), String> {
        self.begin_count += 1;
        if self.fail_begin {
            Err("spawn rejected".into())
        } else {
            Ok(())
        }
    }
    fn poll_ready(&mut self) -> Result<bool, String> {
        self.body_hidden = self.ready;
        Ok(self.ready)
    }
    fn maintain(&mut self) -> Result<(), String> {
        if self.fail_active {
            Err("target lost".into())
        } else {
            Ok(())
        }
    }
    fn restore(&mut self) -> Result<bool, String> {
        self.restore_count += 1;
        if self.fail_restore {
            return Err("restore failed".into());
        }
        self.body_hidden = false;
        Ok(!self.cleanup_pending)
    }
    fn finish(&mut self) -> Result<bool, String> {
        Ok(!self.finishing_pending)
    }
    fn completed(&mut self) -> Result<bool, String> {
        Ok(self.animation_completed)
    }
}

#[test]
fn finisher_ack_restores_before_expiry_without_manual_exit() {
    let mut s = Session::default();
    let mut g = FakeGame {
        ready: true,
        ..Default::default()
    };
    tick(&mut s, &mut g, 0, true, false);
    tick(&mut s, &mut g, 1, false, false);
    tick(&mut s, &mut g, 1000, false, false);
    assert!(matches!(s.phase(), Phase::Active { .. }));
    g.animation_completed = true;
    tick(&mut s, &mut g, 4000, false, false);
    assert_eq!(s.phase(), Phase::Normal);
    assert_eq!(g.restore_count, 1);
    assert!(s.last_error().is_none());
}

#[test]
fn manual_and_expiry_wait_for_animation_ack_before_cleanup() {
    for manual in [false, true] {
        let (mut s, mut g) = (
            Session::default(),
            FakeGame {
                ready: true,
                finishing_pending: true,
                ..Default::default()
            },
        );
        tick(&mut s, &mut g, 0, true, false);
        tick(&mut s, &mut g, 1, false, false);
        let end = if manual { 500 } else { 15001 };
        tick(&mut s, &mut g, end, false, manual);
        assert!(matches!(s.phase(), Phase::Finishing { .. }));
        assert_eq!(g.restore_count, 0);
        assert!(g.body_hidden);
        tick(&mut s, &mut g, end + 1000, true, true);
        assert_eq!(g.begin_count, 1);
        assert_eq!(g.restore_count, 0);
        g.finishing_pending = false;
        tick(&mut s, &mut g, end + 2000, false, false);
        assert_eq!(s.phase(), Phase::Normal);
        assert!(!g.body_hidden);
    }
}

#[test]
fn missing_animation_ack_uses_bounded_emergency_recovery() {
    let (mut s, mut g) = (
        Session::default(),
        FakeGame {
            ready: true,
            finishing_pending: true,
            ..Default::default()
        },
    );
    tick(&mut s, &mut g, 0, true, false);
    tick(&mut s, &mut g, 1, false, false);
    tick(&mut s, &mut g, 100, false, true);
    tick(&mut s, &mut g, 8100, false, false);
    assert_eq!(s.phase(), Phase::Normal);
    assert!(s.last_error().unwrap().contains("behavior_finish_timeout"));
}

fn tick(session: &mut Session, game: &mut FakeGame, ms: u64, enter: bool, revert: bool) {
    session.tick(Duration::from_millis(ms), Input { enter, revert }, game);
}

#[test]
fn slow_spawn_does_not_hide_body_or_consume_active_duration() {
    let (mut s, mut g) = (Session::default(), FakeGame::default());
    tick(&mut s, &mut g, 0, true, false);
    tick(&mut s, &mut g, 2500, false, false);
    assert!(!g.body_hidden);
    g.ready = true;
    tick(&mut s, &mut g, 2900, false, false);
    tick(&mut s, &mut g, 17899, false, false);
    assert!(matches!(s.phase(), Phase::Active { .. }));
    tick(&mut s, &mut g, 17900, false, false);
    assert_eq!(s.phase(), Phase::Normal);
    assert!(!g.body_hidden);
}

#[test]
fn late_spawn_at_deadline_is_cleaned_instead_of_activated() {
    let (mut s, mut g) = (Session::default(), FakeGame::default());
    tick(&mut s, &mut g, 0, true, false);
    g.ready = true;
    g.cleanup_pending = true;
    tick(&mut s, &mut g, 3000, false, false);
    assert!(!g.body_hidden);
    tick(&mut s, &mut g, 3100, true, false);
    assert_eq!(g.begin_count, 1);
    g.cleanup_pending = false;
    tick(&mut s, &mut g, 3200, false, false);
    assert_eq!(s.phase(), Phase::Normal);
    assert_eq!(s.last_error(), Some("enter_timeout"));
}

#[test]
fn ten_cycles_ignore_reentry_and_restore_each_time() {
    let (mut s, mut g) = (
        Session::default(),
        FakeGame {
            ready: true,
            ..Default::default()
        },
    );
    for cycle in 0..10 {
        let base = cycle * 100;
        tick(&mut s, &mut g, base, true, false);
        tick(&mut s, &mut g, base + 1, true, false);
        for ms in 2..5 {
            tick(&mut s, &mut g, base + ms, true, false);
        }
        assert_eq!(g.begin_count, cycle as usize + 1);
        tick(&mut s, &mut g, base + 5, false, true);
        assert_eq!(s.phase(), Phase::Normal);
        assert!(!g.body_hidden);
    }
    assert_eq!(g.restore_count, 10);
}

#[test]
fn begin_failure_and_active_failure_both_run_cleanup() {
    let (mut s, mut g) = (
        Session::default(),
        FakeGame {
            fail_begin: true,
            ..Default::default()
        },
    );
    tick(&mut s, &mut g, 0, true, false);
    assert_eq!(s.phase(), Phase::Normal);
    assert_eq!(g.restore_count, 1);
    g.fail_begin = false;
    g.ready = true;
    tick(&mut s, &mut g, 1, true, false);
    tick(&mut s, &mut g, 2, false, false);
    g.fail_active = true;
    tick(&mut s, &mut g, 3, false, false);
    assert_eq!(s.phase(), Phase::Normal);
    assert!(!g.body_hidden);
    assert_eq!(s.last_error(), Some("target lost"));
}

#[test]
fn restore_failure_stays_owned_and_blocks_new_entries_until_recovered() {
    let (mut s, mut g) = (
        Session::default(),
        FakeGame {
            ready: true,
            ..Default::default()
        },
    );
    tick(&mut s, &mut g, 0, true, false);
    tick(&mut s, &mut g, 1, false, false);
    g.fail_restore = true;
    tick(&mut s, &mut g, 2, false, true);
    tick(&mut s, &mut g, 3002, true, false);
    assert_eq!(s.phase(), Phase::Fault);
    tick(&mut s, &mut g, 4000, true, false);
    assert_eq!(g.begin_count, 1);
    g.fail_restore = false;
    tick(&mut s, &mut g, 4001, true, false);
    assert_eq!(s.phase(), Phase::Normal);
    assert_eq!(g.begin_count, 1);
    tick(&mut s, &mut g, 4002, true, false);
    assert_eq!(g.begin_count, 2);
}

#[test]
fn cancel_while_loading_and_simultaneous_keys_never_activate() {
    let (mut s, mut g) = (
        Session::default(),
        FakeGame {
            ready: true,
            ..Default::default()
        },
    );
    tick(&mut s, &mut g, 0, true, true);
    assert_eq!(g.begin_count, 0);
    tick(&mut s, &mut g, 1, true, false);
    tick(&mut s, &mut g, 2, false, true);
    assert_eq!(s.phase(), Phase::Normal);
    assert!(!g.body_hidden);
}

#![cfg(feature = "native")]
use er_demon_beast::{
    forms::{ActionMode, Config, Selector, Trigger},
    session::{Backend, Input, Phase, Session},
};
use std::{collections::BTreeSet, time::Duration};
fn config() -> Config {
    let mut c = Config::parse(include_str!("../er-demon-beast.toml")).unwrap();
    let mut b = c.forms[0].clone();
    b.key = "second".into();
    b.priority = 20;
    b.sp_effect_id = Some(200);
    b.trigger = Trigger::Automatic;
    b.duration_seconds = 5.0;
    c.forms.push(b);
    c
}
#[test]
fn config_validation_and_infinite_representation() {
    let c = config();
    c.validate().unwrap();
    let mut invalid = c.clone();
    invalid.forms[1].priority = 0;
    assert!(invalid.validate().is_err());
    for d in [0.0, -2.0, f64::INFINITY, f64::NAN] {
        let mut c = c.clone();
        c.forms[0].duration_seconds = d;
        assert!(c.validate().is_err());
    }
    let infinite =
        Config::parse(&include_str!("../er-demon-beast.toml").replace("15.0", "-1")).unwrap();
    assert_eq!(infinite.forms[0].duration().unwrap(), None);
    let mut invalid = c.clone();
    invalid.forms[0].inputs.heavy = 0;
    assert!(invalid.validate().is_err());
    let mut invalid = c.clone();
    invalid.forms[0].actions.light = 9999;
    assert!(invalid.validate().is_err());
    let mut invalid = c.clone();
    invalid.forms[0].action_mode = ActionMode::DirectC0000;
    assert!(invalid.validate().is_err());
    let mut invalid = c.clone();
    invalid.forms[0].chr_id = 4070;
    assert!(invalid.validate().is_err());
    invalid.forms[0].action_mode = ActionMode::Native;
    invalid.validate().unwrap();
    invalid.forms[0].inputs.dodge = -1;
    assert!(invalid.validate().is_err());
    assert!(Config::parse("enabled = false\nchr_id = 10").is_err());
}
#[test]
fn priority_manual_default_and_persistent_effect_rearm() {
    let mut c = config();
    let mut s = Selector::default();
    let idle = Input::default();
    let press = Input {
        enter: true,
        revert: false,
    };
    let ids = BTreeSet::from([200]);
    let empty = BTreeSet::new();
    assert_eq!(
        s.route(&c, 0, Phase::Normal, idle, Some(&ids)).enter,
        Some(1)
    );
    // Expiry/manual/finisher or failed entry can't retrigger the same aura.
    assert_eq!(s.route(&c, 1, Phase::Normal, idle, Some(&ids)).enter, None);
    assert_eq!(
        s.route(&c, 1, Phase::Normal, press, Some(&ids)).enter,
        Some(1)
    );
    s.route(&c, 1, Phase::Normal, idle, None); // read failure isn't effect removal
    assert_eq!(s.route(&c, 1, Phase::Normal, idle, Some(&ids)).enter, None);
    s.route(&c, 1, Phase::Normal, idle, Some(&empty));
    assert_eq!(
        s.route(&c, 1, Phase::Normal, idle, Some(&ids)).enter,
        Some(1)
    );
    let mut top = c.forms[1].clone();
    top.key = "top".into();
    top.priority = 30;
    top.sp_effect_id = Some(300);
    top.trigger = Trigger::Manual;
    c.forms.push(top);
    let both = BTreeSet::from([200, 300]);
    assert_eq!(s.route(&c, 1, Phase::Normal, idle, Some(&both)).enter, None);
    assert_eq!(
        s.route(&c, 1, Phase::Normal, press, Some(&both)).enter,
        Some(2)
    );
    assert_eq!(
        s.route(&c, 1, Phase::Normal, press, Some(&empty)).enter,
        Some(0)
    );
    let result = s.route(
        &c,
        1,
        Phase::Normal,
        Input {
            enter: true,
            revert: true,
        },
        Some(&both),
    );
    assert!(result.revert && result.enter.is_none());
}

#[derive(Default)]
struct Game {
    selected: usize,
    owned: bool,
    finishing: bool,
    cleanup_blocked: bool,
    fail_second: bool,
    begins: Vec<usize>,
}
impl Backend for Game {
    fn begin(&mut self) -> Result<(), String> {
        assert!(!self.owned, "created a second beast before cleanup");
        self.begins.push(self.selected);
        if self.fail_second && self.selected == 1 {
            return Err("missing target".into());
        }
        self.owned = true;
        Ok(())
    }
    fn poll_ready(&mut self) -> Result<bool, String> {
        Ok(true)
    }
    fn maintain(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn finish(&mut self) -> Result<bool, String> {
        Ok(!self.finishing)
    }
    fn restore(&mut self) -> Result<bool, String> {
        if self.cleanup_blocked {
            Ok(false)
        } else {
            self.owned = false;
            Ok(true)
        }
    }
}
fn step(
    c: &Config,
    r: &mut Selector,
    s: &mut Session,
    g: &mut Game,
    ms: u64,
    input: Input,
    ids: &BTreeSet<i32>,
) {
    let intent = r.route(c, g.selected, s.phase(), input, Some(ids));
    if let Some(i) = intent.enter {
        assert_eq!(s.phase(), Phase::Normal);
        g.selected = i;
        s.configure_duration(c.forms[i].duration().unwrap())
            .unwrap();
    }
    s.tick(
        Duration::from_millis(ms),
        Input {
            enter: intent.enter.is_some(),
            revert: intent.revert,
        },
        g,
    );
    if s.last_error().is_some() {
        r.cancel();
    }
}
#[test]
fn switch_waits_for_action_and_destructor_then_failure_does_not_loop() {
    let c = config();
    let (mut r, mut s, mut g) = (Selector::default(), Session::default(), Game::default());
    let e = BTreeSet::new();
    let b = BTreeSet::from([200]);
    step(
        &c,
        &mut r,
        &mut s,
        &mut g,
        0,
        Input {
            enter: true,
            revert: false,
        },
        &e,
    );
    step(&c, &mut r, &mut s, &mut g, 1, Input::default(), &e);
    g.finishing = true;
    g.cleanup_blocked = true;
    step(&c, &mut r, &mut s, &mut g, 2, Input::default(), &b);
    assert!(matches!(s.phase(), Phase::Finishing { .. }));
    step(&c, &mut r, &mut s, &mut g, 500, Input::default(), &b);
    assert_eq!(g.begins, vec![0]);
    g.finishing = false;
    step(&c, &mut r, &mut s, &mut g, 1000, Input::default(), &b);
    assert!(matches!(s.phase(), Phase::Restoring { .. }));
    assert_eq!(g.begins, vec![0]);
    g.cleanup_blocked = false;
    g.fail_second = true;
    step(&c, &mut r, &mut s, &mut g, 1100, Input::default(), &b);
    assert_eq!(s.phase(), Phase::Normal);
    step(&c, &mut r, &mut s, &mut g, 1101, Input::default(), &b);
    assert_eq!(g.begins, vec![0, 1]);
    assert_eq!(s.phase(), Phase::Normal);
    assert!(!g.owned);
    step(&c, &mut r, &mut s, &mut g, 5000, Input::default(), &b);
    assert_eq!(g.begins, vec![0, 1]);
}
#[test]
fn f7_cancels_queued_form_and_effect_removal_does_not_revert_infinite() {
    let mut c = config();
    c.forms[0].duration_seconds = -1.0;
    c.forms[0].sp_effect_id = Some(100);
    let (mut r, mut s, mut g) = (Selector::default(), Session::default(), Game::default());
    let e = BTreeSet::new();
    let b = BTreeSet::from([200]);
    let selected_effect = BTreeSet::from([100]);
    step(
        &c,
        &mut r,
        &mut s,
        &mut g,
        0,
        Input {
            enter: true,
            revert: false,
        },
        &selected_effect,
    );
    step(
        &c,
        &mut r,
        &mut s,
        &mut g,
        1,
        Input::default(),
        &selected_effect,
    );
    step(&c, &mut r, &mut s, &mut g, 10000, Input::default(), &e);
    step(&c, &mut r, &mut s, &mut g, 60001, Input::default(), &e);
    assert!(matches!(s.phase(), Phase::Active { .. }));
    g.finishing = true;
    step(&c, &mut r, &mut s, &mut g, 60002, Input::default(), &b);
    step(
        &c,
        &mut r,
        &mut s,
        &mut g,
        60003,
        Input {
            enter: false,
            revert: true,
        },
        &b,
    );
    g.finishing = false;
    step(&c, &mut r, &mut s, &mut g, 60004, Input::default(), &b);
    step(&c, &mut r, &mut s, &mut g, 60005, Input::default(), &b);
    assert_eq!(g.begins, vec![0]);
}
#[test]
fn custom_timer_starts_after_ready_and_cannot_mutate_active() {
    let mut s = Session::default();
    let mut g = Game::default();
    s.configure_duration(Some(Duration::from_secs(5))).unwrap();
    s.tick(
        Duration::ZERO,
        Input {
            enter: true,
            revert: false,
        },
        &mut g,
    );
    s.tick(Duration::from_secs(2), Input::default(), &mut g);
    assert!(s.configure_duration(None).is_err());
    s.tick(Duration::from_millis(6999), Input::default(), &mut g);
    assert!(matches!(s.phase(), Phase::Active { .. }));
    s.tick(Duration::from_secs(7), Input::default(), &mut g);
    assert_eq!(s.phase(), Phase::Normal);
}

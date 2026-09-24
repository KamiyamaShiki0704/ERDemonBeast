#![cfg(feature = "native")]
use er_demon_beast::{
    forms::{Config, Effects, Selector, Trigger},
    session::{Backend, Input, Phase, Session},
};
use std::{collections::BTreeSet, time::Duration};

fn config() -> Config {
    let mut c = Config::parse(include_str!("../er-demon-beast.toml")).unwrap();
    c.forms[0].trigger = Trigger::Automatic;
    c.forms[0].sp_effect_id = Some(100);
    c.forms[0].revert_sp_effect_id = Some(900);
    c.forms[0].duration_seconds = -1.0;
    c
}
fn active() -> Phase {
    Phase::Active {
        since: Duration::ZERO,
    }
}

#[test]
fn automatic_default_accepts_configurable_ids_and_rejects_conflicts() {
    let mut c = config();
    c.validate().unwrap();
    assert!(c.has_effect_routes());
    assert!(!c.debug_hotkeys_enabled);
    for id in [0, -1, 100] {
        c.forms[0].revert_sp_effect_id = Some(id);
        assert!(c.validate().is_err());
    }
    c.forms[0].revert_sp_effect_id = Some(900);
    let mut b = c.forms[0].clone();
    b.key = "b".into();
    b.priority = 10;
    b.sp_effect_id = Some(900);
    b.revert_sp_effect_id = Some(901);
    c.forms.push(b);
    assert!(c.validate().is_err(), "cross-form enter/exit collision");
    c.forms[1].sp_effect_id = Some(200);
    c.forms[1].revert_sp_effect_id = Some(900); // shared exit is supported
    c.validate().unwrap();
    c.forms[0].sp_effect_id = None;
    assert!(c.validate().is_err(), "automatic default needs enter ID");
}

#[test]
fn disabled_hotkeys_do_not_block_effect_controls_or_repeat_when_held() {
    let mut c = config();
    let mut r = Selector::default();
    let effects = BTreeSet::from([100]);
    let disabled = c.debug_input(true, [true, true], [false, false]);
    assert!(!disabled.enter && !disabled.revert);
    assert_eq!(
        r.route(&c, 0, Phase::Normal, disabled, Some(&effects))
            .enter,
        Some(0)
    );
    c.debug_hotkeys_enabled = true;
    assert!(c.debug_input(true, [true, false], [false, false]).enter);
    assert!(c.debug_input(true, [false, true], [false, false]).revert);
    let held = c.debug_input(true, [true, true], [true, true]);
    assert!(!held.enter && !held.revert);
    let unfocused = c.debug_input(false, [true, true], [false, false]);
    assert!(!unfocused.enter && !unfocused.revert);
}

#[test]
fn each_source_has_independent_edges_and_unknown_is_not_removal() {
    let c = config();
    let mut r = Selector::default();
    let empty = BTreeSet::new();
    let exit = BTreeSet::from([900]);
    let beast = Effects {
        player: Some(&empty),
        beast: Some(&exit),
    };
    assert!(r.route(&c, 0, active(), Input::default(), beast).revert);
    assert!(
        !r.route(&c, 0, active(), Input::default(), Effects::default())
            .revert
    );
    assert!(!r.route(&c, 0, active(), Input::default(), beast).revert);
    let both = Effects {
        player: Some(&exit),
        beast: Some(&exit),
    };
    assert!(
        r.route(&c, 0, active(), Input::default(), both).revert,
        "new player signal remains visible while beast signal persists"
    );
    assert!(!r.route(&c, 0, active(), Input::default(), both).revert);
    r.route(
        &c,
        0,
        active(),
        Input::default(),
        Effects {
            player: None,
            beast: Some(&empty),
        },
    );
    assert!(r.route(&c, 0, active(), Input::default(), both).revert);
    r.reset_beast_effects();
    assert!(r.route(&c, 0, active(), Input::default(), both).revert);
}

#[test]
fn exit_wins_simultaneous_enter_and_cancels_a_queued_switch() {
    let mut c = config();
    let mut r = Selector::default();
    let both = BTreeSet::from([100, 900]);
    let intent = r.route(&c, 0, Phase::Normal, Input::default(), Some(&both));
    assert!(intent.revert && intent.enter.is_none());
    assert_eq!(
        r.route(&c, 0, Phase::Normal, Input::default(), Some(&both))
            .enter,
        None
    );
    let mut b = c.forms[0].clone();
    b.key = "b".into();
    b.priority = 10;
    b.sp_effect_id = Some(200);
    b.revert_sp_effect_id = Some(901);
    c.forms.push(b);
    let enter_b = BTreeSet::from([200]);
    assert!(
        r.route(&c, 0, active(), Input::default(), Some(&enter_b))
            .revert
    );
    let cancel = BTreeSet::from([200, 900]);
    assert!(
        r.route(
            &c,
            0,
            Phase::Finishing {
                since: Duration::ZERO
            },
            Input::default(),
            Some(&cancel)
        )
        .revert
    );
    assert_eq!(
        r.route(&c, 0, Phase::Normal, Input::default(), Some(&enter_b))
            .enter,
        None
    );
}

#[test]
fn beast_enter_effect_never_selects_a_form_and_inactive_exit_is_ignored() {
    let mut c = config();
    let mut r = Selector::default();
    let empty = BTreeSet::new();
    let ids = BTreeSet::from([100, 901]);
    let mut b = c.forms[0].clone();
    b.key = "b".into();
    b.priority = 10;
    b.sp_effect_id = Some(200);
    b.revert_sp_effect_id = Some(901);
    c.forms.push(b);
    let effects = Effects {
        player: Some(&empty),
        beast: Some(&ids),
    };
    assert_eq!(
        r.route(&c, 0, Phase::Normal, Input::default(), effects)
            .enter,
        None
    );
    assert!(!r.route(&c, 0, active(), Input::default(), effects).revert);
    assert!(
        !r.route(
            &c,
            0,
            active(),
            Input::default(),
            Some(&BTreeSet::from([901]))
        )
        .revert
    );
}

#[test]
fn outgoing_exit_can_cancel_a_switch_after_cleanup_before_next_entry() {
    let mut c = config();
    let mut b = c.forms[0].clone();
    b.key = "b".into();
    b.priority = 10;
    b.sp_effect_id = Some(200);
    b.revert_sp_effect_id = Some(901);
    c.forms.push(b);
    let mut r = Selector::default();
    let enter_b = BTreeSet::from([200]);
    assert!(
        r.route(&c, 0, active(), Input::default(), Some(&enter_b))
            .revert
    );
    let cancel = BTreeSet::from([200, 900]);
    let intent = r.route(&c, 0, Phase::Normal, Input::default(), Some(&cancel));
    assert!(intent.revert && intent.enter.is_none());
    assert_eq!(
        r.route(&c, 0, Phase::Normal, Input::default(), Some(&enter_b))
            .enter,
        None
    );
}

#[derive(Default)]
struct Game {
    owned: bool,
    action_running: bool,
    cleanup_blocked: bool,
    begins: usize,
}
impl Backend for Game {
    fn begin(&mut self) -> Result<(), String> {
        assert!(!self.owned);
        self.owned = true;
        self.begins += 1;
        Ok(())
    }
    fn poll_ready(&mut self) -> Result<bool, String> {
        Ok(true)
    }
    fn maintain(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn finish(&mut self) -> Result<bool, String> {
        Ok(!self.action_running)
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
#[test]
fn body_and_beast_exit_wait_for_action_then_cleanup_without_keyboard() {
    for on_beast in [false, true] {
        let c = config();
        let mut r = Selector::default();
        let mut s = Session::default();
        let mut g = Game::default();
        s.configure_duration(c.forms[0].duration().unwrap())
            .unwrap();
        let enter = BTreeSet::from([100]);
        let both = BTreeSet::from([100, 900]);
        let exit = BTreeSet::from([900]);
        let mut tick = |ms, effects: Effects<'_>, s: &mut Session, g: &mut Game| {
            let intent = r.route(&c, 0, s.phase(), Input::default(), effects);
            s.tick(
                Duration::from_millis(ms),
                Input {
                    enter: intent.enter.is_some(),
                    revert: intent.revert,
                },
                g,
            );
        };
        tick(0, Some(&enter).into(), &mut s, &mut g);
        tick(1, Some(&enter).into(), &mut s, &mut g);
        tick(60001, Some(&enter).into(), &mut s, &mut g);
        assert!(matches!(s.phase(), Phase::Active { .. }));
        g.action_running = true;
        g.cleanup_blocked = true;
        let effects = if on_beast {
            Effects {
                player: Some(&enter),
                beast: Some(&exit),
            }
        } else {
            Some(&both).into()
        };
        tick(60002, effects, &mut s, &mut g);
        assert!(matches!(s.phase(), Phase::Finishing { .. }));
        tick(61000, effects, &mut s, &mut g);
        assert!(g.owned);
        g.action_running = false;
        tick(61500, effects, &mut s, &mut g);
        assert!(matches!(s.phase(), Phase::Restoring { .. }));
        g.cleanup_blocked = false;
        tick(61501, effects, &mut s, &mut g);
        assert_eq!(s.phase(), Phase::Normal);
        assert!(!g.owned);
        tick(66501, effects, &mut s, &mut g);
        assert_eq!(s.phase(), Phase::Normal);
        assert_eq!(g.begins, 1);
    }
}

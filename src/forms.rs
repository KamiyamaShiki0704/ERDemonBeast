//! Validated form definitions and deterministic intent routing. No engine access.
use crate::session::{Input, Phase};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, time::Duration};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    #[serde(default)]
    pub control_enabled: bool,
    #[serde(default)]
    pub action_trace: bool,
    pub ownership_tag: i32,
    pub default_form: String,
    pub forms: Vec<Form>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Manual,
    Automatic,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionMode {
    Mapped,
    Native,
    DirectC0000,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    pub key: String,
    pub chr_id: i32,
    pub npc_param_id: i32,
    pub npc_think_param_id: i32,
    pub trigger: Trigger,
    pub priority: i32,
    pub sp_effect_id: Option<i32>,
    #[serde(default = "yes")]
    pub enabled: bool,
    pub duration_seconds: f64,
    pub action_mode: ActionMode,
    #[serde(default)]
    pub locked_four_way_movement: bool,
    #[serde(default)]
    pub camera_distance: f32,
    #[serde(default)]
    pub camera_height: f32,
    #[serde(default)]
    pub inputs: Inputs,
    #[serde(default)]
    pub actions: Actions,
    #[serde(default = "default_combos")]
    pub combos: Vec<Combo>,
}
fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Inputs {
    pub light: i32,
    pub heavy: i32,
    pub guard: i32,
    pub skill: i32,
    pub dodge: i32,
}
impl Default for Inputs {
    fn default() -> Self {
        Self {
            light: 0,
            heavy: 1,
            guard: 2,
            skill: 3,
            dodge: 5,
        }
    }
}
impl Inputs {
    pub fn values(&self) -> [i32; 5] {
        [self.light, self.heavy, self.guard, self.skill, self.dodge]
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Actions {
    pub light: i32,
    pub heavy: i32,
    pub guard: i32,
    pub skill: i32,
    pub dodge: i32,
}
impl Default for Actions {
    fn default() -> Self {
        Self {
            light: 3000,
            heavy: 3005,
            guard: 3030,
            skill: 3035,
            dodge: 6000,
        }
    }
}
impl Actions {
    pub fn values(&self) -> [i32; 5] {
        [self.light, self.heavy, self.guard, self.skill, self.dodge]
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Combo {
    pub from: i32,
    pub light: i32,
    pub heavy: i32,
}
fn default_combos() -> Vec<Combo> {
    [
        (3000, 3001, 3005),
        (3001, 3002, 3005),
        (3002, 3003, 3005),
        (3003, 3000, 3005),
        (3005, 3001, 3007),
        (3007, 3000, 3005),
        (3010, 3001, 3007),
        (3030, 3000, 3005),
        (6000, 3010, 3005),
    ]
    .into_iter()
    .map(|(from, light, heavy)| Combo { from, light, heavy })
    .collect()
}
const ACTIONS: [i32; 10] = [3000, 3001, 3002, 3003, 3005, 3007, 3010, 3030, 3035, 6000];

impl Form {
    pub fn bridge_options(&self) -> String {
        let csv = |values: [i32; 5]| values.map(|v| v.to_string()).join(",");
        let mut text = format!(
            "{}\nv2\ninputs:{}\nactions:{}\n",
            if self.locked_four_way_movement {
                "fourway"
            } else {
                "original"
            },
            csv(self.inputs.values()),
            csv(self.actions.values())
        );
        for c in &self.combos {
            text.push_str(&format!("combo:{},{},{}\n", c.from, c.light, c.heavy));
        }
        text
    }

    pub fn duration(&self) -> Result<Option<Duration>, String> {
        if self.duration_seconds == -1.0 {
            return Ok(None);
        }
        if !self.duration_seconds.is_finite() || self.duration_seconds <= 0.0 {
            return Err(format!(
                "{}: duration_seconds must be positive or -1",
                self.key
            ));
        }
        let d = Duration::try_from_secs_f64(self.duration_seconds).map_err(|e| e.to_string())?;
        if d.is_zero() {
            return Err("duration below timer resolution".into());
        }
        Ok(Some(d))
    }
    pub fn behavior_lifecycle(&self) -> bool {
        self.action_mode == ActionMode::Mapped
    }
    fn validate(&self) -> Result<(), String> {
        if self.key.is_empty()
            || self.key.len() > 48
            || !self
                .key
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err("form key must be 1..48 ASCII letters, digits, '_' or '-'".into());
        }
        if !(1..=9999).contains(&self.chr_id)
            || self.npc_param_id <= 0
            || self.npc_think_param_id <= 0
        {
            return Err(format!("{}: invalid character/param ID", self.key));
        }
        if self.sp_effect_id.is_some_and(|id| id <= 0)
            || (self.trigger == Trigger::Automatic && self.sp_effect_id.is_none())
        {
            return Err(format!(
                "{}: automatic trigger requires a positive sp_effect_id",
                self.key
            ));
        }
        self.duration()?;
        if !self.camera_distance.is_finite()
            || !self.camera_height.is_finite()
            || !(0.0..=20.0).contains(&self.camera_distance)
            || !(0.0..=10.0).contains(&self.camera_height)
        {
            return Err(format!("{}: invalid camera profile", self.key));
        }
        match self.action_mode {
            ActionMode::DirectC0000 => return Err("direct_c0000 is reserved, not implemented".into()),
            ActionMode::Mapped if self.chr_id!=10 => return Err("mapped currently requires the private c0010 HKS/Behbnd adapter; another ID needs its own adapter".into()),
            ActionMode::Native if self.inputs!=Inputs::default() || self.actions!=Actions::default() || self.combos!=default_combos() || self.locked_four_way_movement => {
                return Err("native mode has no private input/movement adapter; mapping overrides are unsupported".into());
            }
            _ => {}
        }
        let mut inputs = BTreeSet::new();
        for id in self.inputs.values() {
            if ![-1, 0, 1, 2, 3, 5].contains(&id) || (id >= 0 && !inputs.insert(id)) {
                return Err(
                    "input sources must be unique ER actions 0,1,2,3,5 or -1 (disabled)".into(),
                );
            }
        }
        for id in self.actions.values() {
            if id != 0 && !ACTIONS.contains(&id) {
                return Err(format!("unsupported mapped animation {id}"));
            }
        }
        let mut owners = BTreeSet::new();
        if self.combos.len() > 10 {
            return Err("at most ten combo rows".into());
        }
        for c in &self.combos {
            if !ACTIONS.contains(&c.from)
                || c.from == 3035
                || !owners.insert(c.from)
                || [c.light, c.heavy]
                    .into_iter()
                    .any(|id| id != 0 && !ACTIONS.contains(&id))
            {
                return Err("invalid, duplicate or unsupported combo mapping".into());
            }
            // Other action self-transitions have not been validated. Step has
            // its own safe same-node deferral in the shipped adapter.
            if c.light == c.from || c.heavy == c.from {
                return Err("self-transition combo targets are unsupported".into());
            }
        }
        Ok(())
    }
}
impl Config {
    pub fn parse(text: &str) -> Result<Self, String> {
        let c: Self = toml::from_str(text)
            .map_err(|e| format!("form config: {e}; legacy single-form files must be migrated"))?;
        c.validate()?;
        Ok(c)
    }
    pub fn default_index(&self) -> usize {
        self.forms
            .iter()
            .position(|f| f.key == self.default_form)
            .expect("validated default")
    }
    pub fn has_effect_routes(&self) -> bool {
        self.forms
            .iter()
            .any(|f| f.enabled && f.sp_effect_id.is_some())
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.ownership_tag <= 0 || self.forms.is_empty() || self.forms.len() > 32 {
            return Err("positive ownership_tag and 1..32 forms required".into());
        }
        let mut keys = BTreeSet::new();
        let mut priorities = BTreeSet::new();
        for f in &self.forms {
            f.validate()?;
            if !keys.insert(&f.key) || (f.enabled && !priorities.insert(f.priority)) {
                return Err("duplicate form key or enabled priority".into());
            }
            if f.enabled && f.key != self.default_form && f.sp_effect_id.is_none() {
                return Err("non-default enabled forms require sp_effect_id".into());
            }
        }
        if !self
            .forms
            .iter()
            .any(|f| f.key == self.default_form && f.enabled && f.trigger == Trigger::Manual)
        {
            return Err("default_form must name an enabled manual form".into());
        }
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct Selector {
    seen: BTreeSet<i32>,
    queued: Option<usize>,
}
#[derive(Default, Debug, PartialEq, Eq)]
pub struct Intent {
    pub enter: Option<usize>,
    pub revert: bool,
}
impl Selector {
    pub fn cancel(&mut self) {
        self.queued = None;
    }
    pub fn reset_effects(&mut self) {
        self.seen.clear();
    }
    pub fn route(
        &mut self,
        c: &Config,
        current: usize,
        phase: Phase,
        input: Input,
        effects: Option<&BTreeSet<i32>>,
    ) -> Intent {
        let winner = effects.and_then(|ids| {
            c.forms
                .iter()
                .enumerate()
                .filter(|(_, f)| f.enabled && f.sp_effect_id.is_some_and(|id| ids.contains(&id)))
                .max_by_key(|(_, f)| f.priority)
                .map(|(i, _)| i)
        });
        let automatic = winner.filter(|&i| {
            c.forms[i].trigger == Trigger::Automatic
                && !self.seen.contains(&c.forms[i].sp_effect_id.unwrap())
        });
        if let Some(ids) = effects {
            self.seen = ids.clone();
        }
        if input.revert {
            self.queued = None;
            return Intent {
                enter: None,
                revert: true,
            };
        }
        if phase == Phase::Normal
            && let Some(i) = self.queued.take()
        {
            return Intent {
                enter: Some(i),
                revert: false,
            };
        }
        if effects.is_none() {
            return Intent::default();
        } // unknown != no effects
        let request = if input.enter {
            Some(winner.unwrap_or_else(|| c.default_index()))
        } else {
            automatic
        };
        match (phase, request) {
            (Phase::Normal, Some(i)) => Intent {
                enter: Some(i),
                revert: false,
            },
            (Phase::Active { .. }, Some(i)) if i != current => {
                self.queued = Some(i);
                Intent {
                    enter: None,
                    revert: true,
                }
            }
            _ => Intent::default(),
        }
    }
}

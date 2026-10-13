//! Project-specific evidence stream. No game mutation is performed by sampling.
use super::*;
use crate::session::Phase;
use serde_json::{Value, json};
use std::{io::BufWriter, sync::mpsc, time::SystemTime};

pub(super) struct Capture {
    tx: mpsc::SyncSender<Value>,
    run: String,
    case: Option<u8>,
    last_command: u64,
    next_poll: Duration,
    next_frame: Duration,
    dropped: u64,
    action_trace: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    run: String,
    id: u64,
    action: String,
    case: Option<u8>,
    note: Option<String>,
}

impl Capture {
    pub(super) fn is_recording(&self) -> bool {
        self.case.is_some()
    }

    pub(super) fn new(config: &Config) -> Result<Self, String> {
        let root = DIRECTORY.get().ok_or("no DLL directory")?;
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();
        let run = format!("{stamp}-{}", unsafe { GetCurrentProcessId() });
        let dir = root.join("Saved/er-demon-beast").join(&run);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(dir.join("frames.jsonl"))
            .map_err(|e| e.to_string())?;
        let dll_hash = format!(
            "{:x}",
            Sha256::digest(fs::read(root.join("er_demon_beast.dll")).map_err(|e| e.to_string())?)
        );
        let meta = json!({"schema":1,"run":run,"exe_sha256":EXE_HASH,
            "dll_sha256":dll_hash,"version":env!("CARGO_PKG_VERSION"),
            "config":config,"spec":"spec-custom-forms.md@1.0 + spec-effect-controls.md@1.0","sample_period_s":0.05,
            "active_action_trace":config.action_trace,"sampling_stage":"FrameEnd",
            "report_path":dir.join("report.md"),"frames_path":dir.join("frames.jsonl"),
            "build_manifest":fs::read_to_string(root.join("build-result.json")).ok()});
        fs::write(
            dir.join("meta.json"),
            serde_json::to_vec_pretty(&meta).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        fs::write(
            root.join("capture-latest.json"),
            serde_json::to_vec_pretty(&meta).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::sync_channel::<Value>(1024);
        std::thread::spawn(move || {
            let mut writer = BufWriter::new(file);
            for row in rx {
                if serde_json::to_writer(&mut writer, &row).is_err()
                    || writer.write_all(b"\n").is_err()
                    || writer.flush().is_err()
                {
                    log("capture writer failed; runtime evidence incomplete");
                    break;
                }
            }
        });
        Ok(Self {
            tx,
            run,
            case: None,
            last_command: 0,
            next_poll: Duration::ZERO,
            next_frame: Duration::ZERO,
            dropped: 0,
            action_trace: config.action_trace,
        })
    }

    fn emit(&mut self, mut row: Value) {
        row["dropped"] = json!(self.dropped);
        if self.tx.try_send(row).is_err() {
            self.dropped += 1;
        }
    }

    pub(super) fn commands(
        &mut self,
        now: Duration,
        focused: bool,
        control_allowed: bool,
        input: &mut Input,
    ) {
        if now < self.next_poll {
            return;
        }
        self.next_poll = now + Duration::from_millis(100);
        let path = DIRECTORY.get().unwrap().join("capture-command.json");
        // Atomic replacement by the CLI; stale requests from other runs are ignored.
        let Ok(raw) = fs::read(path) else {
            return;
        };
        if raw.len() > 4096 {
            return;
        }
        let Ok(cmd) = serde_json::from_slice::<Command>(&raw) else {
            return;
        };
        if cmd.run != self.run || cmd.id <= self.last_command {
            return;
        }
        self.last_command = cmd.id;
        let accepted = match cmd.action.as_str() {
            "start" if self.case.is_none() && cmd.case.is_some_and(|n| (1..=6).contains(&n)) => {
                self.case = cmd.case;
                true
            }
            "end" | "mark" if self.case.is_some() => true,
            "enter" if focused && control_allowed && self.case.is_some() => {
                input.enter = true;
                true
            }
            "revert" if focused && control_allowed && self.case.is_some() => {
                input.revert = true;
                true
            }
            _ => false,
        };
        self.emit(json!({"kind":"command","t":now.as_secs_f64(),"id":cmd.id,
            "action":cmd.action,"case":self.case,"note":cmd.note,"accepted":accepted}));
        if accepted && cmd.action == "end" {
            self.case = None;
        }
        self.next_frame = Duration::ZERO;
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn frame(
        &mut self,
        now: Duration,
        before: Phase,
        after: Phase,
        input: Input,
        focused: bool,
        pre: Value,
        post: Value,
        error: Option<String>,
    ) {
        let trace = self.action_trace
            && (matches!(before, Phase::Active { .. } | Phase::Finishing { .. })
                || matches!(after, Phase::Active { .. } | Phase::Finishing { .. }));
        if !trace && now < self.next_frame && before == after && !input.enter && !input.revert {
            return;
        }
        self.next_frame = now + Duration::from_millis(if self.case.is_some() { 50 } else { 1000 });
        self.emit(
            json!({"kind":"frame","t":now.as_secs_f64(),"case":self.case,
            "before":phase(before),"phase":phase(after),"active_since":match after {
                Phase::Active{since} => Some(since.as_secs_f64()), _ => None },
            "enter":input.enter,"revert":input.revert,"focused":focused,
            "action_trace":trace,"physical_input":if trace && focused { Some(physical_input()) } else { None },
            "pre":pre,"post":post,"error":error}),
        );
    }
}

// These are physical keys, not inferred player actions or configurable bindings.
// Only queried while a traced transformation is active and the game has focus.
fn physical_input() -> Value {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        VK_CONTROL, VK_LBUTTON, VK_RBUTTON, VK_SHIFT, VK_SPACE,
    };
    let down = |key: i32| unsafe { GetAsyncKeyState(key) < 0 };
    json!({"mouse_left":down(VK_LBUTTON.0 as i32),
        "mouse_right":down(VK_RBUTTON.0 as i32),
        "shift":down(VK_SHIFT.0 as i32),"ctrl":down(VK_CONTROL.0 as i32),
        "space":down(VK_SPACE.0 as i32)})
}

fn phase(phase: Phase) -> &'static str {
    match phase {
        Phase::Normal => "normal",
        Phase::Entering { .. } => "entering",
        Phase::Active { .. } => "active",
        Phase::Finishing { .. } => "finishing",
        Phase::Restoring { .. } => "restoring",
        Phase::Fault => "fault",
    }
}

fn actor<M: shikifsrs_runtime::memory::AddressSpace>(
    objects: &shikifsrs_elden_ring::objects::GameObjectCatalog<'_, '_, M>,
    identity: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) -> shikifsrs_runtime::Result<Value> {
    use shikifsrs_elden_ring::character_flags::CharacterFlagByte as F;
    use shikifsrs_runtime::Error;
    let absent = || Error::NativeOperationRejected("capture component absent");
    let vitals = objects.character_vitals(identity)?.ok_or_else(absent)?;
    let spatial = objects.character_spatial(identity)?.ok_or_else(absent)?;
    let flags = objects.character_flags(identity)?.ok_or_else(absent)?;
    let opacity = objects.character_opacity(identity)?.ok_or_else(absent)?;
    let policy = objects.controller_policy(identity)?.ok_or_else(absent)?;
    let action = objects
        .character_actions(identity)?
        .ok_or_else(absent)?
        .words;
    let channels = objects
        .character_control_channels(identity)?
        .ok_or_else(absent)?;
    if objects.character_identity(identity.address, identity.epoch)? != Some(identity) {
        return Err(Error::NativeOperationRejected("capture character changed"));
    }
    let factors = opacity.factors();
    Ok(
        json!({"handle":format!("{:016x}",identity.native_identifier),"address":identity.address,
        "hp":vitals.hp.current,"position":spatial.finite_position()?,
        "render":flags.model_enable_requested(),"alpha":factors[0],"tint_alpha":factors[2],
        "debug_flags":u16::from(flags.value(F::DebugLow)) | (u16::from(flags.value(F::DebugHigh))<<8),
        "disable_hit":policy.low_byte & 2 != 0,
        "actions":action.action_requests,"new_actions":action.new_action_presses,
        "readback_new_actions":action.readback_new_presses,"readback_npc_action":action.readback_identifier,
        "disabled_actions":action.disabled_action_inputs,"possible_inputs":action.possible_action_inputs,
        "possible_cancels":action.possible_action_cancels,"movement_flags":action.movement_request_flags,
        "tae_cancels":action.tae_cancel_flags,"npc_action":action.requested_identifier,"tae":action.queue_selector,
        "override":channels.override_input.map(|v|v.object.address)}),
    )
}

pub(super) fn snapshot(backend: &NativeBackend) -> Value {
    match collect(backend) {
        Ok(value) => value,
        Err(error) => json!({"capture_error":error}),
    }
}

fn collect(backend: &NativeBackend) -> Result<Value, String> {
    backend.objects.observe(|objects,epoch| {
        use shikifsrs_runtime::Error;
        let player=objects.primary_player_identity(epoch)?.ok_or(Error::NativeOperationRejected("no local player"))?;
        let body=actor(objects,player)?;
        let target=match objects.primary_player_target(epoch)? {
            Some(target)=>backend::resolve_handle(objects,target.handle,epoch)?.map(|id|actor(objects,id)).transpose()?,
            None=>None,
        };
        let beast=backend.beast.map(|id|actor(objects,id)).transpose()?;
        let mut owned_count=0;
        if let Some(set)=objects.world_creation_set(65536)? {
            for entry in set.set.characters {
                let id=objects.character_identity(entry.character.address,epoch)?.ok_or(Error::NativeOperationRejected("capture member disappeared"))?;
                if objects.character_event_id(id)?.is_some_and(|id|id.stored_id==backend.config.ownership_tag as u32) {owned_count+=1;}
            }
        }
        let debug=objects.debug_world_links(epoch)?.ok_or(Error::NativeOperationRejected("no debug links"))?;
        Ok(json!({"form_key":backend.form().key,"duration_seconds":backend.form().duration_seconds,
            "body":body,"beast":beast,"target":target,"owned_count":owned_count,
            "camera":debug.character_override.map_or(0,|id|id.address),
            "manipulator":debug.input.map_or(0,|input|input.object.address),
            "manip_owner":debug.input.map(|input|input.native_character_address),
            "linked":backend.linked,"pending_spawn":backend.pending,
            "patches":backend.policy_pending(),"retirement_pending":backend.retirement_pending(),
            "diagnostic_semantics":"raw action/request fields; experimental native integration; not completed gameplay predicates"}))
    }).map_err(|e|e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_trace_preserves_one_frame_action_pulses_without_changing_idle_sampling() {
        for (enabled, active, expected) in [(true, true, 2), (false, true, 1), (true, false, 1)] {
            let (tx, rx) = mpsc::sync_channel(8);
            let mut capture = Capture {
                tx,
                run: "offline".into(),
                case: None,
                last_command: 0,
                next_poll: Duration::ZERO,
                next_frame: Duration::ZERO,
                dropped: 0,
                action_trace: enabled,
            };
            let phase = if active {
                Phase::Active {
                    since: Duration::ZERO,
                }
            } else {
                Phase::Normal
            };
            for (ms, action) in [(1000, 0), (1016, 1)] {
                let state = json!({"beast":{"actions":action}});
                capture.frame(
                    Duration::from_millis(ms),
                    phase,
                    phase,
                    Input::default(),
                    false,
                    state.clone(),
                    state,
                    None,
                );
            }
            let rows: Vec<_> = rx.try_iter().collect();
            assert_eq!(rows.len(), expected);
            if enabled && active {
                assert_eq!(rows[1]["post"]["beast"]["actions"], 1);
                assert!(rows.iter().all(|r| r["physical_input"].is_null()));
            }
        }
    }
}

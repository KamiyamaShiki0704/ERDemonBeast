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
            "config":config,"spec":"spec.md@1.0","sample_period_s":0.05,
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
        })
    }

    fn emit(&mut self, mut row: Value) {
        row["dropped"] = json!(self.dropped);
        if self.tx.try_send(row).is_err() {
            self.dropped += 1;
        }
    }

    pub(super) fn commands(&mut self, now: Duration, focused: bool, input: &mut Input) {
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
            "enter" if focused && self.case.is_some() => {
                input.enter = true;
                true
            }
            "revert" if focused && self.case.is_some() => {
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
        if now < self.next_frame && before == after && !input.enter && !input.revert {
            return;
        }
        self.next_frame = now + Duration::from_millis(if self.case.is_some() { 50 } else { 1000 });
        self.emit(
            json!({"kind":"frame","t":now.as_secs_f64(),"case":self.case,
            "before":phase(before),"phase":phase(after),"active_since":match after {
                Phase::Active{since} => Some(since.as_secs_f64()), _ => None },
            "enter":input.enter,"revert":input.revert,"focused":focused,
            "pre":pre,"post":post,"error":error}),
        );
    }
}

fn phase(phase: Phase) -> &'static str {
    match phase {
        Phase::Normal => "normal",
        Phase::Entering { .. } => "entering",
        Phase::Active { .. } => "active",
        Phase::Restoring { .. } => "restoring",
        Phase::Fault => "fault",
    }
}

fn actor(chr: &ChrIns) -> Value {
    let modules = chr.modules.as_ref();
    let pos = modules.physics.as_ref().position;
    let action = modules.action_request.as_ref();
    json!({"handle":chr.field_ins_handle.to_string(),"address":chr as *const _ as usize,
        "hp":modules.data.as_ref().hp,"position":[pos.0,pos.1,pos.2],
        "render":chr.chr_flags1c5.enable_render(),"alpha":chr.base_transparency,
        "tint_alpha":chr.tint_alpha_multiplier,"debug_flags":chr.debug_flags.0,
        "disable_hit":chr.chr_ctrl.as_ref().flags.disable_hit(),
        "actions":action.action_requests.0,"new_actions":action.new_action_presses.0,
        "npc_action":action.npc_action_id,"tae":action.action_request_queue.current_tae_id,
        "override":pointer(chr.chr_ctrl.as_ref() as *const _ as usize + CTRL_OVERRIDE).ok()})
}

pub(super) fn snapshot(backend: &NativeBackend) -> Value {
    match collect(backend) {
        Ok(value) => value,
        Err(error) => json!({"capture_error":error}),
    }
}

fn collect(backend: &NativeBackend) -> Result<Value, String> {
    let world = backend.world()?;
    let player = world.main_player.as_ref().ok_or("no local player")?;
    let target = player.locked_on_enemy;
    let body = actor(&player.chr_ins);
    let target = world.chr_ins_by_handle(&target).map(actor);
    let beast = backend
        .beast
        .map(|id| id.resolve(world).map(|chr| actor(chr)))
        .transpose()?;
    let owned_count = world
        .debug_chr_set
        .characters()
        .filter(|chr| chr.event_entity_id == backend.config.ownership_tag as u32)
        .count();
    let dbg = unsafe { WorldChrManDbg::instance() }.map_err(|e| e.to_string())?;
    let camera = dbg
        .cam_override_chr_ins
        .map_or(0, |ptr| ptr.as_ptr() as usize);
    let owner = if dbg.debug_manipulator == 0 {
        None
    } else {
        pointer(dbg.debug_manipulator + MANIP_OWNER).ok()
    };
    Ok(
        json!({"body":body,"beast":beast,"target":target,"owned_count":owned_count,
        "camera":camera,"manipulator":dbg.debug_manipulator,"manip_owner":owner,
        "linked":backend.linked,"pending_spawn":backend.pending,
        "patches":backend.patches.len()}),
    )
}

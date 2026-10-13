//! Independent ShikiFSRS backend; native game execution remains experimental.
use crate::session::{Backend, Input, Session};
mod backend;
mod behavior_bridge;
mod camera_profile;
mod capture;
mod creation;
mod resident;
mod retirement;
use backend::NativeBackend;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    ffi::c_void,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{HINSTANCE, HMODULE},
    System::{LibraryLoader::GetModuleFileNameW, Threading::GetCurrentProcessId},
    UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F6, VK_F7},
        WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    },
};
const EXE_HASH: &str = "1a3547101327f65d0c76da2f9190ac0aa66871ea42bae2aecc61e11a8b597891";
static DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
use crate::forms::{Config, Effects, Form, Selector};

type EffectIdentity = (u64, u64, u64);
struct EffectSnapshot {
    identity: EffectIdentity,
    ids: std::collections::BTreeSet<i32>,
}

struct Runtime {
    session: Session,
    backend: NativeBackend,
    epoch: Instant,
    keys: [bool; 2],
    last_error: Option<String>,
    capture: capture::Capture,
    selector: Selector,
    effect_ids: Option<std::collections::BTreeSet<i32>>,
    effect_identity: Option<EffectIdentity>,
    beast_effect_ids: Option<std::collections::BTreeSet<i32>>,
    beast_effect_identity: Option<EffectIdentity>,
    beast_effect_error: Option<String>,
    next_effect_poll: Duration,
    effect_error: Option<String>,
}

pub(crate) fn log(message: impl std::fmt::Display) {
    if let Some(dir) = DIRECTORY.get()
        && let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("er-demon-beast.log"))
    {
        let _ = writeln!(file, "{message}");
    }
}

#[unsafe(no_mangle)]
unsafe extern "system" fn DllMain(module: HINSTANCE, reason: u32, _: *mut c_void) -> i32 {
    if reason == 1 {
        // Game threads can reserve only 64KiB. A stack array here makes even
        // DLL_THREAD_ATTACH overflow before the reason check executes.
        let mut path = vec![0_u16; 32768];
        let len = unsafe { GetModuleFileNameW(Some(HMODULE(module.0)), &mut path) } as usize;
        if len > 0 && len < path.len() {
            let dll = PathBuf::from(String::from_utf16_lossy(&path[..len]));
            if let Some(parent) = dll.parent() {
                let _ = DIRECTORY.set(parent.to_path_buf());
                std::thread::spawn(|| {
                    if let Err(error) = initialize() {
                        log(format!("initialization rejected: {error}"));
                    }
                });
            }
        }
    }
    1
}

fn initialize() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let bytes = fs::read(&exe).map_err(|e| e.to_string())?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if hash != EXE_HASH {
        return Err(format!("unsupported executable SHA256={hash}"));
    }
    shikifsrs_elden_ring::Ww171File::verify(&bytes).map_err(|e| e.to_string())?;
    let config = Config::parse(
        &fs::read_to_string(DIRECTORY.get().unwrap().join("er-demon-beast.toml"))
            .map_err(|e| e.to_string())?,
    )?;
    log(format!(
        "prototype {}; exe={hash}; ABI verified; enabled={} control_enabled={} debug_hotkeys_enabled={}",
        env!("CARGO_PKG_VERSION"),
        config.enabled,
        config.control_enabled,
        config.debug_hotkeys_enabled
    ));
    if !config.enabled {
        return Ok(());
    }
    resident::install(bytes.into(), config)
}

impl Runtime {
    unsafe fn bind(
        config: Config,
        bytes: Arc<[u8]>,
        base: u64,
        epoch: u64,
    ) -> Result<Self, String> {
        let capture_config = config.clone();
        // SAFETY: Actual FrameEnd thread retains its unique native readers and journals.
        let backend = unsafe { NativeBackend::bind(config, bytes, base, epoch) }?;
        let capture = capture::Capture::new(&capture_config)?;
        Ok(Self {
            session: Session::default(),
            backend,
            epoch: Instant::now(),
            keys: [false; 2],
            last_error: None,
            capture,
            selector: Selector::default(),
            effect_ids: None,
            effect_identity: None,
            beast_effect_ids: None,
            beast_effect_identity: None,
            beast_effect_error: None,
            next_effect_poll: Duration::ZERO,
            effect_error: None,
        })
    }
}
fn update(runtime: &mut Runtime, data: shikifsrs_elden_ring::TaskInvocation) {
    static CALLBACK: std::sync::Once = std::sync::Once::new();
    CALLBACK.call_once(|| {
        log(format!(
            "first callback group={:#x}; frame_end_accepted={}",
            data.group_id,
            is_frame_end(data.group_id)
        ))
    });
    if !is_frame_end(data.group_id) {
        return;
    }
    runtime.backend.poll_retirement();
    let mut foreground_pid = 0;
    unsafe {
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut foreground_pid));
    }
    let focused = foreground_pid == unsafe { GetCurrentProcessId() };
    let pressed = unsafe {
        [
            GetAsyncKeyState(VK_F6.0 as i32) < 0,
            GetAsyncKeyState(VK_F7.0 as i32) < 0,
        ]
    };
    let mut input = runtime
        .backend
        .config
        .debug_input(focused, pressed, runtime.keys);
    runtime.keys = pressed;
    let before = runtime.session.phase();
    let now = runtime.epoch.elapsed();
    let control_allowed = runtime.backend.config.control_enabled;
    runtime
        .capture
        .commands(now, focused, control_allowed, &mut input);
    if !runtime.backend.config.control_enabled {
        if input.enter || input.revert {
            log("control input rejected: observe-only mode");
        }
        input = Input::default();
    }
    if (input.enter || input.revert)
        && matches!(
            before,
            crate::session::Phase::Restoring { .. } | crate::session::Phase::Fault
        )
    {
        log(format!(
            "control input deferred: phase={before:?}; cleanup still owned; see retirement read_error"
        ));
    }
    // Enter effects belong to the protected player. Revert effects may also
    // belong to our owned beast; never resolve it once cleanup owns retirement.
    let scan_beast = matches!(
        before,
        crate::session::Phase::Entering { .. }
            | crate::session::Phase::Active { .. }
            | crate::session::Phase::Finishing { .. }
    ) && runtime.backend.form().revert_sp_effect_id.is_some();
    if !scan_beast {
        runtime.beast_effect_ids = None;
        runtime.beast_effect_identity = None;
        runtime.selector.reset_beast_effects();
    }
    // Both sources are sampled at 10 Hz. Failed reads are unknown, not empty.
    if control_allowed && focused {
        if !runtime.backend.config.has_effect_routes() {
            runtime.effect_ids = Some(Default::default());
        } else if now >= runtime.next_effect_poll || input.enter {
            runtime.next_effect_poll = now + Duration::from_millis(100);
            match runtime.backend.sample_effects() {
                Ok(EffectSnapshot { identity, ids }) => {
                    if runtime.effect_identity != Some(identity)
                        && before == crate::session::Phase::Normal
                    {
                        runtime.selector.reset_effects();
                    }
                    runtime.effect_identity = Some(identity);
                    runtime.effect_ids = Some(ids);
                    runtime.effect_error = None;
                }
                Err(error) => {
                    runtime.effect_ids = None;
                    if runtime.effect_error.as_ref() != Some(&error) {
                        log(format!("form effect sampling unavailable: {error}"));
                        runtime.effect_error = Some(error);
                    }
                }
            }
            if scan_beast {
                match runtime.backend.sample_beast_effects() {
                    Ok(snapshot) => {
                        let identity = snapshot.as_ref().map(|s| s.identity);
                        if runtime.beast_effect_identity != identity {
                            runtime.selector.reset_beast_effects();
                        }
                        runtime.beast_effect_identity = identity;
                        runtime.beast_effect_ids = snapshot.map(|s| s.ids);
                        runtime.beast_effect_error = None;
                    }
                    Err(error) => {
                        runtime.beast_effect_ids = None;
                        if runtime.beast_effect_error.as_ref() != Some(&error) {
                            log(format!("owned beast effect sampling unavailable: {error}"));
                            runtime.beast_effect_error = Some(error);
                        }
                    }
                }
            }
        }
        let Runtime {
            selector,
            backend,
            effect_ids,
            beast_effect_ids,
            session,
            ..
        } = &mut *runtime;
        let intent = selector.route(
            &backend.config,
            backend.form_index,
            before,
            input,
            Effects {
                player: effect_ids.as_ref(),
                beast: beast_effect_ids.as_ref(),
            },
        );
        if intent.revert && !input.revert {
            log(format!(
                "form route requested exit or switch: form={} phase={before:?}",
                backend.form().key
            ));
        }
        input = Input {
            enter: intent.enter.is_some(),
            revert: intent.revert,
        };
        if let Some(index) = intent.enter {
            let result = backend
                .select_form(index)
                .and_then(|()| session.configure_duration(backend.form().duration()?));
            if let Err(error) = result {
                log(format!("form selection rejected: {error}"));
                return;
            }
            log(format!(
                "selected form={} chr=c{:04} duration={}",
                backend.form().key,
                backend.form().chr_id,
                backend.form().duration_seconds
            ));
        }
    } else {
        // No auto requests or queued switches while unfocused / observe-only.
        input = Input::default();
    }
    // Loading/title screens may expose a temporary player with partially
    // initialized modules. Idle callbacks have no reason to traverse actors.
    if !needs_engine_access(before, input, runtime.capture.is_recording()) {
        return;
    }
    let pre = capture::snapshot(&runtime.backend);
    let Runtime {
        session, backend, ..
    } = &mut *runtime;
    session.tick(now, input, backend);
    let after = runtime.session.phase();
    if runtime.session.last_error().is_some() {
        runtime.selector.cancel();
    }
    let post = capture::snapshot(&runtime.backend);
    let failure = runtime.session.last_error().map(str::to_owned);
    runtime
        .capture
        .frame(now, before, after, input, focused, pre, post, failure);
    if before != after {
        log(format!(
            "t={:.3} {before:?} -> {after:?}",
            now.as_secs_f64()
        ));
    }
    let error = runtime.session.last_error().map(str::to_owned);
    if error != runtime.last_error {
        if let Some(ref error) = error {
            log(format!("t={:.3} failure={error}", now.as_secs_f64()));
        }
        runtime.last_error = error;
    }
}

fn is_frame_end(group: u32) -> bool {
    // Current native dispatch ID and its independently verified registration index.
    group == shikifsrs_elden_ring::TaskPhase::FRAME_END.registration_index() as u32
        || group == 0x9000_00a8
}

fn needs_engine_access(phase: crate::session::Phase, input: Input, recording: bool) -> bool {
    phase != crate::session::Phase::Normal || input.enter || input.revert || recording
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_end_encoding_and_idle_engine_access_remain_exact() {
        assert!(is_frame_end(168));
        assert!(is_frame_end(0x9000_00a8));
        for value in [167, 169, 0x8000_00a8, 0x9000_00a7, u32::MAX] {
            assert!(!is_frame_end(value));
        }
        let normal = crate::session::Phase::Normal;
        assert!(!needs_engine_access(normal, Input::default(), false));
        assert!(needs_engine_access(
            normal,
            Input {
                enter: true,
                revert: false
            },
            false
        ));
        assert!(needs_engine_access(normal, Input::default(), true));
        assert!(needs_engine_access(
            crate::session::Phase::Fault,
            Input::default(),
            false
        ));
    }
}

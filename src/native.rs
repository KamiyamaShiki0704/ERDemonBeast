//! Experimental WW 2.7.1 backend. All engine work runs on the game task thread.
//! Native control and recovery for the supported Elden Ring executable.
use crate::session::{Backend, Input, Session};
mod behavior_bridge;
mod camera_profile;
mod capture;
mod effect_scan;
use eldenring::{
    cs::{
        CSTaskGroupIndex, CSTaskImp, ChrIns, FieldInsHandle, NpcParam, NpcThinkParam,
        SoloParamRepository, WorldChrMan, WorldChrManDbg,
    },
    fd4::FD4TaskData,
    position::HavokPosition,
    util::system::wait_for_system_init,
};
use fromsoftware_shared::{F32Vector4, FromStatic, SharedTaskImpExt, program::Program};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    ffi::c_void,
    fs::{self, OpenOptions},
    io::Write,
    mem::{offset_of, size_of},
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{HINSTANCE, HMODULE},
    System::{
        Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory},
        LibraryLoader::GetModuleFileNameW,
        Threading::{GetCurrentProcess, GetCurrentProcessId},
    },
    UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F6, VK_F7},
        WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    },
};

const EXE_HASH: &str = "1a3547101327f65d0c76da2f9190ac0aa66871ea42bae2aecc61e11a8b597891";
const CTRL_OVERRIDE: usize = 0x3b0;
const MANIP_OWNER: usize = 0xa8;
static DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();

use crate::forms::{Config, Effects, Form, Selector};

type EffectIdentity = (usize, usize, FieldInsHandle);
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
    let hash = format!("{:x}", Sha256::digest(bytes));
    if hash != EXE_HASH {
        return Err(format!("unsupported executable SHA256={hash}"));
    }
    validate_abi()?;
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
    // Same startup separation used by Summon; never initialize engine state
    // inside the loader callback. This delay is not a proof of scene readiness.
    std::thread::sleep(Duration::from_secs(5));
    wait_for_system_init(&Program::current(), Duration::from_secs(180))
        .map_err(|e| e.to_string())?;
    let task_imp =
        CSTaskImp::wait_for_instance(Duration::from_secs(30)).map_err(|e| e.to_string())?;
    let capture = capture::Capture::new(&config)?;
    RUNTIME
        .set(Mutex::new(Runtime {
            session: Session::default(),
            backend: NativeBackend::new(config),
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
        }))
        .map_err(|_| "already initialized")?;
    let task = task_imp.run_recurring(update as fn(&FD4TaskData), CSTaskGroupIndex::FrameEnd);
    // Game task owns this callback until process exit. Hot unloading unsupported.
    std::mem::forget(task);
    log(
        "game task installed at FrameEnd; configured SpEffects drive controls; optional debug F6/F7 require debug_hotkeys_enabled=true",
    );
    Ok(())
}

fn validate_abi() -> Result<(), String> {
    use eldenring::cs::{CSChrDataModule, CSChrPhysicsModule, ChrCtrl, ChrInsModuleContainer};
    let checks = [
        ("ChrIns.size", size_of::<ChrIns>(), 0x580),
        (
            "SpEffect.owner",
            offset_of!(eldenring::cs::SpecialEffect, owner),
            0x10,
        ),
        (
            "SpEffectEntry.id",
            offset_of!(eldenring::cs::SpecialEffectEntry, param_id),
            8,
        ),
        (
            "SpEffectEntry.removal",
            offset_of!(eldenring::cs::SpecialEffectEntry, removal_timer),
            0x40,
        ),
        ("ChrIns.ctrl", offset_of!(ChrIns, chr_ctrl), 0x58),
        ("ChrIns.modules", offset_of!(ChrIns, modules), 0x190),
        ("ChrIns.debug", offset_of!(ChrIns, debug_flags), 0x538),
        ("ChrCtrl.size", size_of::<ChrCtrl>(), 0x3d0),
        (
            "ChrCtrl.manipulator",
            offset_of!(ChrCtrl, manipulator),
            0x18,
        ),
        (
            "Dbg.manipulator",
            offset_of!(WorldChrManDbg, debug_manipulator),
            0xa8,
        ),
        (
            "Dbg.camera",
            offset_of!(WorldChrManDbg, cam_override_chr_ins),
            0xb8,
        ),
        (
            "Modules.physics",
            offset_of!(ChrInsModuleContainer, physics),
            0x68,
        ),
        ("Data.hp", offset_of!(CSChrDataModule, hp), 0x138),
        (
            "Physics.fade",
            offset_of!(CSChrPhysicsModule, fade_out_gravity_disabled),
            0x1d3,
        ),
    ];
    for (name, actual, expected) in checks {
        if actual != expected {
            return Err(format!("ABI mismatch {name}: {actual:#x} != {expected:#x}"));
        }
    }
    Ok(())
}

fn update(data: &FD4TaskData) {
    static CALLBACK: std::sync::Once = std::sync::Once::new();
    CALLBACK.call_once(|| {
        log(format!(
            "first callback group={:#x}; frame_end_accepted={}",
            data.task_group_id,
            is_frame_end(data.task_group_id)
        ))
    });
    if !is_frame_end(data.task_group_id) {
        return;
    }
    let Some(runtime) = RUNTIME.get() else {
        return;
    };
    let Ok(mut runtime) = runtime.try_lock() else {
        return;
    };
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
    // Exact dispatch encoding observed by pinned Summon on this EXE.
    group == CSTaskGroupIndex::FrameEnd as u32 || group == 0x9000_00a8
}

fn needs_engine_access(phase: crate::session::Phase, input: Input, recording: bool) -> bool {
    phase != crate::session::Phase::Normal || input.enter || input.revert || recording
}

// Process-memory copies reject unreadable/unwritable pages instead of blindly
// dereferencing private layout fields. Object identity is checked separately.
fn read(address: usize, size: usize) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0; size];
    unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            address as *const c_void,
            bytes.as_mut_ptr().cast(),
            size,
            None,
        )
    }
    .map_err(|e| format!("read {address:#x}: {e}"))?;
    Ok(bytes)
}
fn pointer(address: usize) -> Result<usize, String> {
    Ok(usize::from_ne_bytes(read(address, 8)?.try_into().unwrap()))
}
fn write(address: usize, bytes: &[u8]) -> Result<(), String> {
    unsafe {
        WriteProcessMemory(
            GetCurrentProcess(),
            address as *const c_void,
            bytes.as_ptr().cast(),
            bytes.len(),
            None,
        )
    }
    .map_err(|e| format!("write {address:#x}: {e}"))
}

struct Patch {
    address: usize,
    before: Vec<u8>,
    after: Vec<u8>,
    mask: Vec<u8>,
    applied: bool,
    exclusive: bool,
}
impl Patch {
    fn new(address: usize, after: &[u8], mask: &[u8]) -> Result<Self, String> {
        let before = read(address, after.len())?;
        Ok(Self {
            address,
            before,
            after: after.to_vec(),
            mask: mask.to_vec(),
            applied: false,
            exclusive: false,
        })
    }
    fn apply(&mut self, restore: bool) -> Result<(), String> {
        if restore && !self.applied {
            return Ok(());
        }
        let mut current = read(self.address, self.before.len())?;
        // Another controller may have taken over. Never clear its association.
        if restore && self.exclusive && current != self.after {
            return Ok(());
        }
        let value = if restore { &self.before } else { &self.after };
        for ((byte, val), mask) in current.iter_mut().zip(value).zip(&self.mask) {
            *byte = (*byte & !mask) | (*val & mask);
        }
        // Retain cleanup ownership even if a write fails part-way through.
        if !restore {
            self.applied = true;
        }
        write(self.address, &current)?;
        self.applied = !restore;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Identity {
    handle: FieldInsHandle,
    address: usize,
    ctrl: usize,
    modules: usize,
    data: usize,
    physics: usize,
    modifier: usize,
}
impl Identity {
    fn of(chr: &ChrIns) -> Result<Self, String> {
        Self::at(chr as *const _ as usize)
    }
    fn at(address: usize) -> Result<Self, String> {
        use eldenring::cs::{
            CSChrDataModule, CSChrPhysicsModule, ChrCtrl, ChrCtrlModifier, ChrInsModuleContainer,
        };
        // Read pointers as integers before constructing references. OwnedPtr's
        // Rust type does not prove that an engine module is initialized/live.
        read(address, size_of::<ChrIns>())?;
        let ctrl = checked_object(
            pointer(address + offset_of!(ChrIns, chr_ctrl))?,
            size_of::<ChrCtrl>(),
        )?;
        let modules = checked_object(
            pointer(address + offset_of!(ChrIns, modules))?,
            size_of::<ChrInsModuleContainer>(),
        )?;
        let data = checked_object(
            pointer(modules + offset_of!(ChrInsModuleContainer, data))?,
            size_of::<CSChrDataModule>(),
        )?;
        let physics = checked_object(
            pointer(modules + offset_of!(ChrInsModuleContainer, physics))?,
            size_of::<CSChrPhysicsModule>(),
        )?;
        let modifier = checked_object(
            pointer(ctrl + offset_of!(ChrCtrl, modifier))?,
            size_of::<ChrCtrlModifier>(),
        )?;
        for owner in [
            ctrl + offset_of!(ChrCtrl, owner),
            data + offset_of!(CSChrDataModule, owner),
            physics + offset_of!(CSChrPhysicsModule, owner),
        ] {
            if pointer(owner)? != address {
                return Err("module owner mismatch".into());
            }
        }
        let bytes = read(
            address + offset_of!(ChrIns, field_ins_handle),
            size_of::<FieldInsHandle>(),
        )?;
        // FieldInsHandle is two integer bitfields; every bit pattern is valid.
        let handle = unsafe { bytes.as_ptr().cast::<FieldInsHandle>().read_unaligned() };
        Ok(Self {
            handle,
            address,
            ctrl,
            modules,
            data,
            physics,
            modifier,
        })
    }
    fn resolve<'a>(&self, world: &'a mut WorldChrMan) -> Result<&'a mut ChrIns, String> {
        let chr = world
            .chr_ins_by_handle_mut(&self.handle)
            .ok_or("character handle no longer resolves")?;
        let current = Self::of(chr)?;
        if current.address != self.address
            || current.ctrl != self.ctrl
            || current.modules != self.modules
            || current.data != self.data
            || current.physics != self.physics
            || current.modifier != self.modifier
            || chr.chr_ctrl.as_ref().owner.as_ptr() as usize != self.address
            || chr.modules.as_ref().data.as_ref().owner.as_ptr() as usize != self.address
            || chr.modules.as_ref().physics.as_ref().owner.as_ptr() as usize != self.address
        {
            return Err("character identity changed; refusing stale writes".into());
        }
        Ok(chr)
    }
}

fn checked_object(address: usize, size: usize) -> Result<usize, String> {
    if address == 0 || !address.is_multiple_of(8) {
        return Err(format!("invalid module address {address:#x}"));
    }
    read(address, size)?;
    Ok(address)
}

struct NativeBackend {
    config: Config,
    form_index: usize,
    body: Option<Identity>,
    beast: Option<Identity>,
    world_address: usize,
    pending: bool,
    patches: Vec<Patch>,
    linked: bool,
    debug_address: usize,
    debug_manipulator: usize,
    body_patch_count: usize,
    retirement: Retirement,
    spawn_existing: std::collections::HashSet<FieldInsHandle>,
    behavior: Option<behavior_bridge::BehaviorBridge>,
    camera: camera_profile::CameraProfile,
}
#[derive(Default)]
enum Retirement {
    #[default]
    None,
    Requested,
    Draining,
    Complete,
    Quarantined(String),
}
impl NativeBackend {
    fn form(&self) -> &Form {
        &self.config.forms[self.form_index]
    }
    fn select_form(&mut self, index: usize) -> Result<(), String> {
        if self.body.is_some()
            || self.beast.is_some()
            || self.pending
            || !self.patches.is_empty()
            || !matches!(self.retirement, Retirement::None)
            || crate::retirement::pending()
        {
            return Err("old form still owns resources".into());
        }
        if !self.config.forms.get(index).is_some_and(|f| f.enabled) {
            return Err("invalid form index".into());
        }
        self.form_index = index;
        Ok(())
    }
    fn sample_effects(&self) -> Result<EffectSnapshot, String> {
        let world = unsafe { WorldChrMan::instance() }.map_err(|e| e.to_string())?;
        let world_address = world as *const _ as usize;
        if self.world_address != 0 && self.world_address != world_address {
            return Err("world changed".into());
        }
        let player = world.main_player.as_ref().ok_or("no local player")?;
        let id = Identity::of(&player.chr_ins)?;
        if self
            .body
            .is_some_and(|old| old.address != id.address || old.handle != id.handle)
        {
            return Err("main player identity changed".into());
        }
        let ids = Self::read_effect_ids(id)?;
        Ok(EffectSnapshot {
            identity: (world_address, id.address, id.handle),
            ids,
        })
    }
    fn sample_beast_effects(&self) -> Result<Option<EffectSnapshot>, String> {
        // The retirement queue may own a stale handle even before self.beast is
        // cleared. This guard precedes every resolve of the beast for sampling.
        if !matches!(self.retirement, Retirement::None) {
            return Ok(None);
        }
        let Some(id) = self.beast else {
            return Ok(None);
        };
        let world = self.world()?;
        self.validate_session(world, true)?;
        let ids = Self::read_effect_ids(id)?;
        Ok(Some(EffectSnapshot {
            identity: (self.world_address, id.address, id.handle),
            ids,
        }))
    }
    fn read_effect_ids(id: Identity) -> Result<std::collections::BTreeSet<i32>, String> {
        let manager = checked_object(
            pointer(id.address + offset_of!(ChrIns, special_effect))?,
            0x18,
        )?;
        if pointer(manager + 0x10)? != id.address {
            return Err("SpEffect owner mismatch".into());
        }
        let head = pointer(manager + 8)?;
        let ids = effect_scan::collect(head, |address| {
            let data = read(checked_object(address, 0x40)?, 0x40)?;
            Ok((
                i32::from_le_bytes(data[8..12].try_into().unwrap()),
                usize::from_le_bytes(data[0x30..0x38].try_into().unwrap()),
            ))
        })?;
        if pointer(id.address + offset_of!(ChrIns, special_effect))? != manager
            || pointer(manager + 8)? != head
            || pointer(manager + 0x10)? != id.address
        {
            return Err("SpEffect list changed during read".into());
        }
        Ok(ids)
    }
    fn finish_restore(&mut self) {
        debug_assert!(!self.pending && self.patches.is_empty() && self.beast.is_none());
        self.body = None;
        self.world_address = 0;
        self.debug_address = 0;
        self.debug_manipulator = 0;
        self.body_patch_count = 0;
        self.linked = false;
        self.spawn_existing.clear();
        self.retirement = Retirement::None;
        if let Some(bridge) = self.behavior.take() {
            bridge.close();
        }
    }
    fn new(config: Config) -> Self {
        let form_index = config.default_index();
        Self {
            config,
            form_index,
            body: None,
            beast: None,
            world_address: 0,
            pending: false,
            patches: Vec::new(),
            linked: false,
            debug_address: 0,
            debug_manipulator: 0,
            body_patch_count: 0,
            retirement: Retirement::None,
            spawn_existing: Default::default(),
            behavior: None,
            camera: Default::default(),
        }
    }
    fn world(&self) -> Result<&'static mut WorldChrMan, String> {
        let world = unsafe { WorldChrMan::instance_mut() }.map_err(|e| e.to_string())?;
        if self.world_address != 0 && world as *mut _ as usize != self.world_address {
            return Err("world changed; stale session quarantined until restart".into());
        }
        Ok(world)
    }
    fn poll_retirement(&mut self) {
        if !matches!(
            self.retirement,
            Retirement::Requested | Retirement::Draining
        ) {
            return;
        }
        if let Err(error) = self.retire_at_frame_end() {
            crate::retirement::quarantine_all(&error);
            log(format!("retirement quarantined: {error}; restart required"));
            self.retirement = Retirement::Quarantined(error);
        }
    }
    fn retire_at_frame_end(&mut self) -> Result<(), String> {
        let world = self.world()?;
        self.validate_session(world, false)?;
        if matches!(self.retirement, Retirement::Draining) {
            crate::retirement::poll(world);
            if !crate::retirement::pending() {
                self.retirement = Retirement::Complete;
            }
            return Ok(());
        }
        let id = self.beast.ok_or("retirement missing owned beast")?;
        id.resolve(world)?;
        let dbg = unsafe { WorldChrManDbg::instance() }.map_err(|e| e.to_string())?;
        if dbg
            .cam_override_chr_ins
            .is_some_and(|p| p.as_ptr() as usize == id.address)
            || (dbg.debug_manipulator != 0
                && pointer(dbg.debug_manipulator + MANIP_OWNER)? == id.address)
            || pointer(id.ctrl + CTRL_OVERRIDE)? != 0
            || !self.patches.is_empty()
            || self.linked
        {
            return Err("beast still referenced by control/camera; refusing destruction".into());
        }
        match crate::chr_lifecycle::destroy_summon(world, id.handle) {
            crate::chr_lifecycle::DestroyOutcome::RetirementStarted => {
                // Ownership moved to Summon's queue; never resolve this old handle again.
                self.beast = None;
                self.retirement = Retirement::Draining;
                Ok(())
            }
            outcome => Err(format!("Summon detach refused: {outcome}")),
        }
    }
    fn capture_pending(&mut self, world: &mut WorldChrMan) -> Result<bool, String> {
        if !self.pending {
            return Ok(self.beast.is_some());
        }
        // The creator does not guarantee event-id registration (Summon fixes
        // this explicitly). Bind by our unchanged request, pre-spawn set and
        // last-created pointer together, then register through Summon's code.
        let init = &world.debug_chr_creator.init_data;
        if init.event_entity_id != self.config.ownership_tag
            || init.npc_param_id != self.form().npc_param_id
            || init.npc_think_param_id != self.form().npc_think_param_id
        {
            return Err(
                "debug creator request overwritten; ownership uncertain; restart required".into(),
            );
        }
        let Some(last_created) = world.debug_chr_creator.last_created_chr else {
            return Ok(false);
        };
        let matches: Vec<_> = world
            .debug_chr_set
            .characters()
            .filter(|chr| {
                !self.spawn_existing.contains(&chr.field_ins_handle)
                    && chr.npc_id == self.form().chr_id
                    && chr.npc_param_id == self.form().npc_param_id
            })
            .map(|chr| Identity::of(chr))
            .collect::<Result<Vec<_>, _>>()?;
        if matches.len() > 1 {
            return Err("ambiguous owned beast; manual cleanup required".into());
        }
        if let Some(identity) = matches.first() {
            if identity.address != last_created.as_ptr() as usize {
                return Err("last-created pointer disagrees with newly spawned beast".into());
            }
            self.beast = Some(*identity);
            self.pending = false;
            self.spawn_existing.clear();
            world.debug_chr_creator.last_created_chr = None;
            let registered = crate::chr_lifecycle::register_event_entity_id(
                world,
                identity.handle,
                self.config.ownership_tag,
            );
            if !matches!(
                registered,
                crate::chr_lifecycle::EntityIdOutcome::RegisteredByDll
                    | crate::chr_lifecycle::EntityIdOutcome::AlreadyRegisteredByEngine
            ) {
                return Err(format!("owned beast registration refused: {registered}"));
            }
            log(format!(
                "owned beast ready handle={} addr={:#x}",
                identity.handle, identity.address
            ));
            return Ok(true);
        }
        Ok(false)
    }
    fn add_patch(&mut self, address: usize, bytes: &[u8], mask: &[u8]) -> Result<(), String> {
        self.patches.push(Patch::new(address, bytes, mask)?);
        Ok(())
    }
    fn link(&mut self, world: &mut WorldChrMan) -> Result<(), String> {
        let body_id = self.body.ok_or("missing body")?;
        let beast_id = self.beast.ok_or("missing beast")?;
        let dbg = unsafe { WorldChrManDbg::instance() }.map_err(|e| e.to_string())?;
        self.debug_address = dbg as *const _ as usize;
        self.debug_manipulator = dbg.debug_manipulator;
        if self.debug_manipulator == 0
            || dbg.cam_override_chr_ins.is_some()
            || ![0, body_id.address].contains(&pointer(self.debug_manipulator + MANIP_OWNER)?)
            || pointer(beast_id.ctrl + CTRL_OVERRIDE)? != 0
        {
            return Err("native debug control channel unavailable or occupied".into());
        }
        let body = body_id.resolve(world)?;
        let body_team = body.team_type;
        let body_address = body_id.address;
        // Prepare every snapshot before applying any write.
        self.add_patch(
            body_address + offset_of!(ChrIns, debug_flags),
            &[0x38],
            &[0x38],
        )?;
        self.add_patch(body_address + offset_of!(ChrIns, chr_flags1c5), &[0], &[8])?;
        // Use documented hit protection rather than the unknown data+19B bit
        // or rewriting the player's ChrType to NPC as the reference mod does.
        self.add_patch(
            body_id.ctrl + offset_of!(eldenring::cs::ChrCtrl, flags),
            &[2],
            &[2],
        )?;
        let physics = body.modules.as_ref().physics.as_ref() as *const _ as usize;
        self.add_patch(
            physics + offset_of!(eldenring::cs::CSChrPhysicsModule, gravity_disabled),
            &[1],
            &[255],
        )?;
        let modifier = body.chr_ctrl.as_ref().modifier.as_ref() as *const _ as usize;
        let action_offset = offset_of!(eldenring::cs::ChrCtrlModifier, data)
            + offset_of!(eldenring::cs::ChrCtrlModifierData, action_flags);
        self.add_patch(modifier + action_offset, &[4], &[4])?;
        for offset in [
            offset_of!(ChrIns, base_transparency),
            offset_of!(ChrIns, tint_alpha_multiplier),
        ] {
            self.add_patch(body_address + offset, &0_f32.to_ne_bytes(), &[255; 4])?;
        }
        self.body_patch_count = self.patches.len();
        let beast = beast_id.resolve(world)?;
        activate(beast);
        self.add_patch(
            beast_id.address + offset_of!(ChrIns, team_type),
            &[body_team],
            &[255],
        )?;
        self.add_patch(
            self.debug_manipulator + MANIP_OWNER,
            &beast_id.address.to_ne_bytes(),
            &[255; 8],
        )?;
        self.add_patch(
            beast_id.ctrl + CTRL_OVERRIDE,
            &self.debug_manipulator.to_ne_bytes(),
            &[255; 8],
        )?;
        // Publish camera last; main_player always remains the original PlayerIns.
        self.add_patch(
            self.debug_address + offset_of!(WorldChrManDbg, cam_override_chr_ins),
            &beast_id.address.to_ne_bytes(),
            &[255; 8],
        )?;
        let control_start = self.patches.len() - 3;
        for patch in &mut self.patches[control_start..] {
            patch.exclusive = true;
        }
        for patch in &mut self.patches {
            patch.apply(false)?;
        }
        self.linked = true;
        log(format!(
            "control linked: body={} beast={} camera={:#x}; runtime behavior unverified",
            body_id.handle, beast_id.handle, beast_id.address
        ));
        Ok(())
    }
    fn validate_session(&self, world: &mut WorldChrMan, require_beast: bool) -> Result<(), String> {
        let body = self.body.ok_or("missing body identity")?;
        let player = world
            .main_player
            .as_ref()
            .ok_or("main player unavailable")?;
        if &player.chr_ins as *const _ as usize != body.address {
            return Err("main player identity changed".into());
        }
        body.resolve(world)?;
        if require_beast && let Some(beast) = self.beast {
            beast.resolve(world)?;
        }
        if !self.patches.is_empty() {
            let dbg = unsafe { WorldChrManDbg::instance() }.map_err(|e| e.to_string())?;
            if dbg as *const _ as usize != self.debug_address
                || dbg.debug_manipulator != self.debug_manipulator
            {
                return Err("debug control identity changed".into());
            }
        }
        Ok(())
    }
}

impl Backend for NativeBackend {
    fn begin(&mut self) -> Result<(), String> {
        if self.pending
            || !self.patches.is_empty()
            || !matches!(self.retirement, Retirement::None)
            || crate::retirement::pending()
        {
            return Err("previous cleanup still pending".into());
        }
        let world = self.world()?;
        let player = world.main_player.as_ref().ok_or("no local player")?;
        let body_id = Identity::of(&player.chr_ins)?;
        checked_object(
            pointer(body_id.modules + offset_of!(eldenring::cs::ChrInsModuleContainer, ride))?,
            size_of::<eldenring::cs::CSChrRideModule>(),
        )?;
        if player.chr_ins.modules.as_ref().data.as_ref().hp <= 0
            || player.chr_ins.modules.as_ref().ride.as_ref().is_mounted
            || player.chr_ins.modules.as_ref().ride.as_ref().is_mounting
        {
            return Err("player must be alive and unmounted".into());
        }
        let physics = player.chr_ins.modules.as_ref().physics.as_ref();
        let position = physics.position;
        let yaw = physics.orientation_euler.1;
        if ![position.0, position.1, position.2, yaw]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err("invalid player position".into());
        }
        let repository = unsafe { SoloParamRepository::instance() }.map_err(|e| e.to_string())?;
        if repository
            .get::<NpcParam>(self.form().npc_param_id as u32)
            .is_none()
            || repository
                .get::<NpcThinkParam>(self.form().npc_think_param_id as u32)
                .is_none()
        {
            return Err("missing configured NpcParam/NpcThinkParam".into());
        }
        self.body = Some(body_id);
        self.world_address = world as *mut _ as usize;
        if world.debug_chr_creator.spawn
            || world.debug_chr_creator.last_created_chr.is_some()
            || world
                .debug_chr_set
                .characters()
                .any(|c| c.event_entity_id == self.config.ownership_tag as u32)
        {
            return Err("debug creator busy or ownership tag occupied".into());
        }
        self.spawn_existing = world
            .debug_chr_set
            .characters()
            .map(|chr| chr.field_ins_handle)
            .collect();
        let init = &mut world.debug_chr_creator.init_data;
        if size_of_val(init) != 0x100 {
            return Err("creator layout mismatch".into());
        }
        init.name.fill(0);
        let name: Vec<_> = format!("c{:04}", self.form().chr_id)
            .encode_utf16()
            .collect();
        init.name[..name.len()].copy_from_slice(&name);
        init.chara_init_param_id = -1;
        init.npc_param_id = self.form().npc_param_id;
        init.npc_think_param_id = self.form().npc_think_param_id;
        init.event_entity_id = self.config.ownership_tag;
        init.talk_id = 0;
        init.spawn_position = F32Vector4(position.0, position.1, position.2, 0.0);
        let init_address = init as *mut _ as usize;
        let rotation: Vec<_> = [0_f32, yaw, 0.0, 0.0]
            .iter()
            .flat_map(|v| v.to_ne_bytes())
            .collect();
        write(init_address + 0x10, &rotation)?;
        write(init_address + 0xc8, &[0])?;
        if self.form().behavior_lifecycle() {
            // Shared with the private HKS, including when the DLL is copied into
            // a manual-runs directory. Only one owned c0010 is supported here.
            self.behavior = Some(behavior_bridge::BehaviorBridge::start(
                PathBuf::from("F:/GoldenAge/_GoldenAge/dll/demon_beast/Saved/er-demon-beast"),
                self.form(),
            )?);
        }
        self.pending = true;
        world.debug_chr_creator.spawn = true;
        Ok(())
    }
    fn poll_ready(&mut self) -> Result<bool, String> {
        let world = self.world()?;
        self.validate_session(world, true)?;
        if !self.capture_pending(world)? {
            return Ok(false);
        }
        if !self.linked {
            self.link(world)?;
        }
        self.maintain()?;
        if let Some(bridge) = &mut self.behavior {
            bridge.ready()
        } else {
            Ok(true)
        }
    }
    fn maintain(&mut self) -> Result<(), String> {
        let world = self.world()?;
        self.validate_session(world, true)?;
        let mut camera_ids = vec![0, 1, 2];
        if let Some(player) = world.main_player.as_ref()
            && let Some(target) = world.chr_ins_by_handle(&player.locked_on_enemy)
        {
            Identity::of(target)?;
            let repo = unsafe { SoloParamRepository::instance() }.map_err(|e| e.to_string())?;
            if let Some(npc) = repo.get::<NpcParam>(target.npc_param_id as u32)
                && npc.lock_camera_param_id() >= 0
            {
                camera_ids.push(npc.lock_camera_param_id() as u32);
            }
        }
        self.camera.apply(
            &camera_ids,
            self.form().camera_distance,
            self.form().camera_height,
        )?;
        let beast = self.beast.ok_or("no beast")?.resolve(world)?;
        if beast.modules.as_ref().data.as_ref().hp <= 0 {
            return Err("beast died; recovery requested".into());
        }
        let physics = beast.modules.as_ref().physics.as_ref();
        let position = physics.position;
        let yaw = physics.orientation_euler.1;
        // Keep the hidden body within the loaded area. Final recovery uses this
        // same position; no fixed vertical offset is applied.
        let body = self.body.ok_or("no body")?.resolve(world)?;
        move_to(body, position, yaw);
        // Reapply only body protection; control pointers are never stolen back.
        for patch in self.patches.iter_mut().take(self.body_patch_count) {
            patch.apply(false)?;
        }
        if pointer(self.debug_address + 0xb8)? != self.beast.unwrap().address
            || pointer(self.debug_manipulator + MANIP_OWNER)? != self.beast.unwrap().address
            || pointer(self.beast.unwrap().ctrl + CTRL_OVERRIDE)? != self.debug_manipulator
        {
            return Err("control ownership lost".into());
        }
        Ok(())
    }
    fn completed(&mut self) -> Result<bool, String> {
        match &mut self.behavior {
            Some(bridge) => bridge.completed(),
            None => Ok(false),
        }
    }
    fn finish(&mut self) -> Result<bool, String> {
        self.maintain()?;
        if let Some(bridge) = &mut self.behavior {
            bridge.finish()
        } else {
            Ok(true)
        }
    }
    fn restore(&mut self) -> Result<bool, String> {
        self.camera.restore()?;
        match &self.retirement {
            Retirement::Requested | Retirement::Draining => return Ok(false),
            Retirement::Quarantined(error) => return Err(error.clone()),
            Retirement::Complete => {
                self.finish_restore();
                return Ok(true);
            }
            Retirement::None => {}
        }
        if self.body.is_none() && self.beast.is_none() && self.patches.is_empty() && !self.pending {
            self.finish_restore();
            return Ok(true);
        }
        let world = self.world()?;
        self.validate_session(world, false)?;
        if self.pending && !self.capture_pending(world)? {
            // A dispatched spawn cannot be declared cancelled without proof.
            // Keep owning it so a late result is parked, never activated.
            return Ok(false);
        }
        let beast_valid = self.beast.is_some_and(|beast| beast.resolve(world).is_ok());
        if self.linked && beast_valid {
            let beast = self
                .beast
                .ok_or("missing beast on restore")?
                .resolve(world)?;
            let position = beast.modules.as_ref().physics.as_ref().position;
            let yaw = beast.modules.as_ref().physics.as_ref().orientation_euler.1;
            move_to(
                self.body.ok_or("missing body on restore")?.resolve(world)?,
                position,
                yaw,
            );
        }
        // Reverse order: stop camera override and input first, restore body last.
        // Pop only after successful restoration so failures remain retryable.
        while let Some(patch) = self.patches.last_mut() {
            let target_patch = self.beast.is_some_and(|beast| {
                patch.address == beast.ctrl + CTRL_OVERRIDE
                    || patch.address == beast.address + offset_of!(ChrIns, team_type)
            });
            if beast_valid || !target_patch {
                patch.apply(true)?;
            }
            self.patches.pop();
        }
        self.linked = false;
        self.body_patch_count = 0;
        if beast_valid {
            park(self.beast.unwrap().resolve(world)?);
            // At least one further FrameEnd before detach; the shared Summon
            // queue then requires two quiet frame ends before freeing memory.
            self.retirement = Retirement::Requested;
            return Ok(false);
        } else {
            if self.beast.is_some() {
                self.retirement =
                    Retirement::Quarantined("owned beast identity lost; no free attempted".into());
                return Err("owned beast identity lost; no free attempted".into());
            }
            self.beast = None;
        }
        self.finish_restore();
        Ok(true)
    }
}

fn move_to(chr: &mut ChrIns, position: HavokPosition, yaw: f32) {
    let half = yaw * 0.5;
    let rotation = eldenring::rotation::Quaternion(0.0, half.sin(), 0.0, half.cos());
    let physics = chr.modules.as_mut().physics.as_mut();
    physics.position = position;
    physics.last_update_position = position;
    physics.chr_proxy_pos_update_requested = true;
    physics.orientation_euler = F32Vector4(0.0, yaw, 0.0, 0.0);
    physics.orientation = rotation;
    physics.interpolated_orientation = rotation;
    chr.chr_ctrl
        .as_mut()
        .chr_proxy_flags
        .set_rotation_sync_requested(true);
    chr.chunk_position = F32Vector4(position.0, position.1, position.2, 0.0);
}
fn activate(chr: &mut ChrIns) {
    chr.chr_flags1c5.set_enable_render(true);
    chr.debug_flags.set_disabled_updates(false);
    chr.debug_flags.set_disabled_hit(false);
    chr.debug_flags.set_disabled_movement(false);
    chr.debug_flags.set_disabled_secondary_actions(false);
    chr.base_transparency = 1.0;
    chr.tint_alpha_multiplier = 1.0;
}
fn park(chr: &mut ChrIns) {
    chr.chr_flags1c5.set_enable_render(false);
    chr.debug_flags.set_disabled_updates(true);
    chr.debug_flags.set_disabled_hit(true);
    chr.debug_flags.set_disabled_movement(true);
    chr.debug_flags.set_disabled_secondary_actions(true);
    chr.base_transparency = 0.0;
    chr.tint_alpha_multiplier = 0.0;
}

#[cfg(test)]
mod tests {
    #[test]
    fn missing_modules_are_rejected_before_owned_pointer_dereference() {
        let storage = vec![0_usize; std::mem::size_of::<eldenring::cs::ChrIns>() / 8];
        assert!(super::Identity::at(storage.as_ptr() as usize).is_err());
        assert!(super::Identity::at(1).is_err());
        assert!(super::checked_object(0, 8).is_err());
        assert!(super::checked_object(8, 8).is_err());
    }

    #[test]
    fn observed_frame_end_encoding_is_accepted_without_accepting_other_stages() {
        assert!(super::is_frame_end(0x9000_00a8));
        assert!(super::is_frame_end(168));
        for stage in [167, 169, 0x8000_00a8, 0x9000_00a7, u32::MAX] {
            assert!(!super::is_frame_end(stage));
        }
    }

    #[test]
    fn pinned_native_layout_matches_target_contract() {
        super::validate_abi().unwrap();
    }

    #[test]
    fn field_restore_preserves_unrelated_bits_and_original_state() {
        let mut value = 0b1000_0001_u8;
        let address = std::ptr::from_mut(&mut value) as usize;
        let mut patch = super::Patch::new(address, &[0b0011_1000], &[0b0011_1000]).unwrap();
        patch.apply(false).unwrap();
        assert_eq!(value, 0b1011_1001);
        // An independent owner changes bit 6 during the session.
        value |= 0b0100_0000;
        patch.apply(true).unwrap();
        assert_eq!(value, 0b1100_0001);
    }

    #[test]
    fn rollback_does_not_overwrite_a_new_controller_or_an_unapplied_patch() {
        let mut value = 0_usize;
        let address = std::ptr::from_mut(&mut value) as usize;
        let mut patch = super::Patch::new(address, &123_usize.to_ne_bytes(), &[255; 8]).unwrap();
        patch.exclusive = true;
        value = 456;
        patch.apply(true).unwrap();
        assert_eq!(value, 456);
        value = 0;
        patch.apply(false).unwrap();
        assert_eq!(value, 123);
        value = 789;
        patch.apply(true).unwrap();
        assert_eq!(value, 789);
    }

    #[test]
    fn completed_retirement_releases_world_binding_for_next_session() {
        use crate::session::Backend;
        let config = toml::from_str(include_str!("../er-demon-beast.toml")).unwrap();
        let mut backend = super::NativeBackend::new(config);
        backend.world_address = 123;
        backend.debug_address = 456;
        backend.debug_manipulator = 789;
        backend.retirement = super::Retirement::Complete;
        assert!(backend.restore().unwrap());
        assert_eq!(
            backend.world_address, 0,
            "finished session must not pin the next world"
        );
        assert_eq!(backend.debug_address, 0);
        assert_eq!(backend.debug_manipulator, 0);
    }
}

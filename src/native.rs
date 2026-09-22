//! Experimental WW 2.7.1 backend. All engine work runs on the game task thread.
//! The debug control fields are documented in research/control-audit.md.
use crate::session::{Backend, Input, Session};
mod capture;
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

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    enabled: bool,
    chr_id: i32,
    npc_param_id: i32,
    npc_think_param_id: i32,
    ownership_tag: i32,
}

struct Runtime {
    session: Session,
    backend: NativeBackend,
    epoch: Instant,
    keys: [bool; 2],
    last_error: Option<String>,
    capture: capture::Capture,
}

fn log(message: impl std::fmt::Display) {
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
    let config: Config = toml::from_str(
        &fs::read_to_string(DIRECTORY.get().unwrap().join("er-demon-beast.toml"))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if !(1..=9999).contains(&config.chr_id)
        || config.npc_param_id <= 0
        || config.npc_think_param_id <= 0
        || config.ownership_tag <= 0
    {
        return Err("invalid beast configuration".into());
    }
    log(format!(
        "prototype {}; exe={hash}; ABI verified; enabled={}",
        env!("CARGO_PKG_VERSION"),
        config.enabled
    ));
    if !config.enabled {
        return Ok(());
    }
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
        }))
        .map_err(|_| "already initialized")?;
    let task = task_imp.run_recurring(
        update as fn(&FD4TaskData),
        CSTaskGroupIndex::ChrIns_PostPhysics,
    );
    // Game task owns this callback until process exit. Hot unloading unsupported.
    std::mem::forget(task);
    log("game task installed; F6 enter / F7 restore; runtime acceptance NOT performed");
    Ok(())
}

fn validate_abi() -> Result<(), String> {
    use eldenring::cs::{CSChrDataModule, CSChrPhysicsModule, ChrCtrl, ChrInsModuleContainer};
    let checks = [
        ("ChrIns.size", size_of::<ChrIns>(), 0x580),
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

fn update(_: &FD4TaskData) {
    let Some(runtime) = RUNTIME.get() else {
        return;
    };
    let Ok(mut runtime) = runtime.try_lock() else {
        return;
    };
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
    let mut input = Input {
        enter: focused && pressed[0] && !runtime.keys[0],
        revert: focused && pressed[1] && !runtime.keys[1],
    };
    runtime.keys = pressed;
    let before = runtime.session.phase();
    let now = runtime.epoch.elapsed();
    runtime.capture.commands(now, focused, &mut input);
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
    fn of(chr: &ChrIns) -> Self {
        Self {
            handle: chr.field_ins_handle,
            address: chr as *const _ as usize,
            ctrl: chr.chr_ctrl.as_ref() as *const _ as usize,
            modules: chr.modules.as_ref() as *const _ as usize,
            data: chr.modules.as_ref().data.as_ref() as *const _ as usize,
            physics: chr.modules.as_ref().physics.as_ref() as *const _ as usize,
            modifier: chr.chr_ctrl.as_ref().modifier.as_ref() as *const _ as usize,
        }
    }
    fn resolve<'a>(&self, world: &'a mut WorldChrMan) -> Result<&'a mut ChrIns, String> {
        let chr = world
            .chr_ins_by_handle_mut(&self.handle)
            .ok_or("character handle no longer resolves")?;
        let current = Self::of(chr);
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

struct NativeBackend {
    config: Config,
    body: Option<Identity>,
    beast: Option<Identity>,
    world_address: usize,
    pending: bool,
    patches: Vec<Patch>,
    linked: bool,
    debug_address: usize,
    debug_manipulator: usize,
    body_patch_count: usize,
}
impl NativeBackend {
    fn new(config: Config) -> Self {
        Self {
            config,
            body: None,
            beast: None,
            world_address: 0,
            pending: false,
            patches: Vec::new(),
            linked: false,
            debug_address: 0,
            debug_manipulator: 0,
            body_patch_count: 0,
        }
    }
    fn world(&self) -> Result<&'static mut WorldChrMan, String> {
        let world = unsafe { WorldChrMan::instance_mut() }.map_err(|e| e.to_string())?;
        if self.world_address != 0 && world as *mut _ as usize != self.world_address {
            return Err("world changed; stale session quarantined until restart".into());
        }
        Ok(world)
    }
    fn capture_pending(&mut self, world: &mut WorldChrMan) -> Result<bool, String> {
        if !self.pending {
            return Ok(self.beast.is_some());
        }
        // Enumerate the owning set instead of trusting a shared last-created
        // pointer which another mod can consume or replace.
        let matches: Vec<_> = world
            .debug_chr_set
            .characters()
            .filter(|chr| {
                chr.event_entity_id == self.config.ownership_tag as u32
                    && chr.npc_id == self.config.chr_id
                    && chr.npc_param_id == self.config.npc_param_id
            })
            .map(|chr| Identity::of(chr))
            .collect();
        if matches.len() > 1 {
            return Err("ambiguous owned beast; manual cleanup required".into());
        }
        if let Some(identity) = matches.first() {
            self.beast = Some(*identity);
            self.pending = false;
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
        if self.pending || !self.patches.is_empty() {
            return Err("previous cleanup still pending".into());
        }
        let world = self.world()?;
        let player = world.main_player.as_ref().ok_or("no local player")?;
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
            .get::<NpcParam>(self.config.npc_param_id as u32)
            .is_none()
            || repository
                .get::<NpcThinkParam>(self.config.npc_think_param_id as u32)
                .is_none()
        {
            return Err("missing configured NpcParam/NpcThinkParam".into());
        }
        self.body = Some(Identity::of(&player.chr_ins));
        self.world_address = world as *mut _ as usize;
        if let Some(identity) = self.beast {
            let beast = identity.resolve(world)?;
            move_to(beast, position, yaw);
            return Ok(());
        }
        if world.debug_chr_creator.spawn
            || world
                .debug_chr_set
                .characters()
                .any(|c| c.event_entity_id == self.config.ownership_tag as u32)
        {
            return Err("debug creator busy or ownership tag occupied".into());
        }
        let init = &mut world.debug_chr_creator.init_data;
        if size_of_val(init) != 0x100 {
            return Err("creator layout mismatch".into());
        }
        init.name.fill(0);
        let name: Vec<_> = format!("c{:04}", self.config.chr_id)
            .encode_utf16()
            .collect();
        init.name[..name.len()].copy_from_slice(&name);
        init.chara_init_param_id = -1;
        init.npc_param_id = self.config.npc_param_id;
        init.npc_think_param_id = self.config.npc_think_param_id;
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
        self.link(world)?;
        Ok(true)
    }
    fn maintain(&mut self) -> Result<(), String> {
        let world = self.world()?;
        self.validate_session(world, true)?;
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
    fn restore(&mut self) -> Result<bool, String> {
        if self.body.is_none() && self.patches.is_empty() && !self.pending {
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
        } else {
            self.beast = None;
        }
        self.body = None;
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
}

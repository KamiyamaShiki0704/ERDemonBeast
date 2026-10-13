//! Two callback threads retain their own unique native tickets. Only copied admissions cross phases.
use super::creation::{Attempt, Completed, NativeDriver, Queue, Request};
use super::*;
use shikifsrs_elden_ring::{
    TaskCallback, TaskInvocation, TaskPhase, Ww171File,
    character_sets::{CharacterIdentity, CharacterRegistrationObservation},
    creation_input_preparation::OrdinaryCreationInput,
    current_debug_creator::CurrentDebugCreator,
    current_objects::CurrentObjects,
    current_tasks::CurrentTaskEngine,
    task_resident::ResidentCallbackGroupSlot,
};
use shikifsrs_runtime::{Result as SdkResult, memory::CurrentProcess};
use std::{
    cell::RefCell,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
};
use windows::Win32::System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId};

const LIMIT: usize = 65536;
static TASKS: ResidentCallbackGroupSlot<2> = ResidentCallbackGroupSlot::new();
static EPOCH: AtomicU64 = AtomicU64::new(1);
static CREATE_THREAD: AtomicU32 = AtomicU32::new(0);
static FRAME_THREAD: AtomicU32 = AtomicU32::new(0);
static SERIAL: AtomicU64 = AtomicU64::new(1);
#[derive(Clone)]
struct SpawnRequest {
    serial: u64,
    body: CharacterIdentity,
    input: OrdinaryCreationInput,
}
#[derive(Clone)]
pub(super) enum Admission {
    Created(CharacterRegistrationObservation),
    Cancelled,
    Failed(String),
}
#[derive(Default)]
struct Mailbox {
    request: Option<SpawnRequest>,
    admission: Option<(u64, Admission)>,
    cancel: Option<u64>,
    release: Option<u64>,
    blocked: Option<String>,
}
impl Mailbox {
    fn publish(&mut self, serial: u64, value: Admission) -> Result<(), String> {
        if self.request.as_ref().map(|r| r.serial) != Some(serial) || self.admission.is_some() {
            return Err("admission publication owner changed".into());
        }
        self.admission = Some((serial, value));
        Ok(())
    }
    fn request_release(&mut self, serial: u64) -> Result<bool, String> {
        if self.request.is_none() && self.admission.is_none() {
            return Ok(true);
        }
        if self.request.as_ref().map(|r| r.serial) != Some(serial)
            || self.admission.as_ref().map(|(id, _)| *id) != Some(serial)
        {
            return Err("creation release lacks matching publication".into());
        }
        self.release = Some(serial);
        Ok(false)
    }
    fn acknowledge_release(&mut self, owner: Option<u64>) -> bool {
        if owner.is_none() || self.release != owner {
            return false;
        }
        if self.request.as_ref().map(|r| r.serial) != owner
            || self.admission.as_ref().map(|(id, _)| *id) != owner
        {
            return false;
        }
        self.request = None;
        self.admission = None;
        self.cancel = None;
        self.release = None;
        true
    }
    fn revoke_old_request(&mut self, epoch: u64) {
        if self.request.as_ref().is_some_and(|r| r.body.epoch != epoch) {
            self.request = None;
            self.admission = None;
            self.cancel = None;
            self.release = None;
        }
    }
}
static MAILBOX: LazyLock<Mutex<Mailbox>> = LazyLock::new(|| Mutex::new(Mailbox::default()));
thread_local! {
    static CREATION: RefCell<CreationSlot> = RefCell::new(CreationSlot::default());
    static FRAME: RefCell<FrameSlot> = RefCell::new(FrameSlot::default());
}
#[derive(Default)]
struct CreationSlot {
    current: Option<Worker>,
    quarantined: Vec<Worker>,
    error: Option<String>,
}
#[derive(Default)]
struct FrameSlot {
    current: Option<Runtime>,
    quarantined: Vec<Runtime>,
    error: Option<String>,
}
struct Worker {
    epoch: u64,
    objects: CurrentObjects,
    driver: NativeDriver,
    queue: Queue<Attempt>,
    active: Option<SpawnRequest>,
    completed: Option<Completed<Attempt>>,
    error: Option<String>,
}
fn thread_matches(owner: &AtomicU32) -> bool {
    // SAFETY: Only queries the current calling thread's integer ID.
    let current = unsafe { GetCurrentThreadId() };
    match owner.compare_exchange(0, current, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => true,
        Err(original) => original == current,
    }
}
pub(super) fn epoch() -> u64 {
    EPOCH.load(Ordering::Acquire)
}
/// Explicit native lifetime notification. Does not clean up old graphs or infer callback drain.
#[unsafe(no_mangle)]
pub extern "system" fn ShikiDemonBeastNotifyEpoch(value: u64) {
    if value != 0 {
        let _ = EPOCH.fetch_update(Ordering::AcqRel, Ordering::Acquire, |old| {
            (value > old).then_some(value)
        });
    }
}
pub(super) fn enqueue(
    body: CharacterIdentity,
    input: OrdinaryCreationInput,
) -> Result<u64, String> {
    let mut mailbox = MAILBOX.lock().map_err(|_| "creation mailbox poisoned")?;
    if let Some(error) = &mailbox.blocked {
        return Err(error.clone());
    }
    if mailbox.request.is_some() || mailbox.admission.is_some() {
        return Err("previous creation publication still owned".into());
    }
    let serial = SERIAL
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
        .map_err(|_| "creation serial exhausted")?;
    mailbox.request = Some(SpawnRequest {
        serial,
        body,
        input,
    });
    mailbox.cancel = None;
    mailbox.release = None;
    Ok(serial)
}
pub(super) fn admission(serial: u64) -> Result<Option<Admission>, String> {
    let mailbox = MAILBOX.lock().map_err(|_| "creation mailbox poisoned")?;
    if let Some(error) = &mailbox.blocked {
        return Err(error.clone());
    }
    Ok(mailbox
        .admission
        .as_ref()
        .filter(|(id, _)| *id == serial)
        .map(|(_, value)| value.clone()))
}
pub(super) fn cancel_unsubmitted(serial: u64) {
    if let Ok(mut mailbox) = MAILBOX.lock() {
        mailbox.cancel = Some(serial);
    }
}
pub(super) fn release(serial: u64) -> Result<bool, String> {
    MAILBOX
        .lock()
        .map_err(|_| "creation mailbox poisoned")?
        .request_release(serial)
}
impl Worker {
    unsafe fn bind(bytes: Arc<[u8]>, base: u64, epoch: u64) -> SdkResult<Self> {
        // SAFETY: Exclusive PostPhysics callback host; graph/residency contracts remain experimental.
        unsafe {
            Ok(Self {
                epoch,
                objects: CurrentObjects::bind(bytes.clone(), base, epoch, None)?,
                driver: NativeDriver {
                    creator: CurrentDebugCreator::bind(bytes, base, epoch, None)?,
                    set_limit: LIMIT,
                },
                queue: Queue::new(1)?,
                active: None,
                completed: None,
                error: None,
            })
        }
    }
    fn publish(&self, value: Admission) -> Result<(), String> {
        let serial = self
            .active
            .as_ref()
            .ok_or("creation has no publication owner")?
            .serial;
        MAILBOX
            .lock()
            .map_err(|_| "creation mailbox poisoned")?
            .publish(serial, value)
    }
    fn tick(&mut self) -> Result<(), String> {
        let (request, cancel, published) = {
            let mut mailbox = MAILBOX.lock().map_err(|_| "creation mailbox poisoned")?;
            if mailbox.acknowledge_release(self.active.as_ref().map(|r| r.serial)) {
                self.completed = None;
                self.active = None;
            }
            (
                mailbox.request.clone(),
                mailbox.cancel,
                mailbox.admission.is_some(),
            )
        };
        if published {
            return Ok(());
        }
        if self.active.is_none()
            && let Some(request) = request
        {
            if request.body.epoch != self.epoch {
                return Err("creation request has a revoked epoch".into());
            }
            if self.queue.has_configuration(0) {
                return Err("duplicate configuration request".into());
            }
            self.active = Some(request.clone());
            self.queue
                .enqueue(Request {
                    config_index: 0,
                    requested_at: Instant::now(),
                    expires_at: None,
                    input: request.input,
                })
                .map_err(|e| e.to_string())?;
        }
        if let Some(error) = self.queue.blocked() {
            return Err(error.to_string());
        }
        let Some(active) = &self.active else {
            return Ok(());
        };
        if self.completed.is_some() {
            return Ok(());
        }
        if cancel == Some(active.serial) && self.queue.active_ticket().is_none() {
            self.queue.quarantine_epoch();
            self.publish(Admission::Cancelled)?;
            return Ok(());
        }
        let primary = self
            .objects
            .observe(|objects, epoch| objects.primary_player_identity(epoch))
            .map_err(|e| e.to_string())?;
        if primary != Some(active.body) {
            return Err("creation primary scope changed; old native request retained".into());
        }
        // A dispatched request never expires into an assumed cancellation. Session owns late admission.
        if let Some(completed) = self
            .queue
            .poll(&mut self.driver, Instant::now(), Duration::MAX)
            .map_err(|e| e.to_string())?
        {
            let registration = completed.registration;
            log(format!(
                "creation admitted: serial={} config={} retained_input_operations={}",
                active.serial,
                completed.pending.request.config_index,
                completed.pending.ticket.operations.len()
            ));
            self.completed = Some(completed);
            self.publish(Admission::Created(registration))?;
        }
        Ok(())
    }
}
fn creation_callback(bytes: Arc<[u8]>, base: u64, data: TaskInvocation) {
    if !matches!(data.group_id, 113 | 0x9000_0071) {
        return;
    }
    if !thread_matches(&CREATE_THREAD) {
        if let Ok(mut mailbox) = MAILBOX.lock() {
            mailbox.blocked = Some(
                "PostPhysics callback moved to another thread; native tickets retained".into(),
            );
        }
        return;
    }
    CREATION.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.error.is_some() {
            return;
        }
        let current_epoch = epoch();
        if slot
            .current
            .as_ref()
            .is_some_and(|v| v.epoch != current_epoch)
        {
            let mut old = slot.current.take().unwrap();
            old.queue.quarantine_epoch();
            slot.quarantined.push(old);
            if let Ok(mut mailbox) = MAILBOX.lock() {
                mailbox.revoke_old_request(current_epoch);
            }
        }
        if slot.current.is_none() {
            if slot.quarantined.len() >= 64 {
                slot.error = Some("creation quarantine bound".into());
                return;
            }
            // SAFETY: Actual callback thread retains its exclusive unique creation binding.
            match unsafe { Worker::bind(bytes, base, current_epoch) } {
                Ok(worker) => slot.current = Some(worker),
                Err(e) => {
                    let error = e.to_string();
                    if let Ok(mut mailbox) = MAILBOX.lock() {
                        mailbox.blocked = Some(error.clone());
                    }
                    slot.error = Some(error);
                    return;
                }
            }
        }
        let worker = slot.current.as_mut().unwrap();
        if worker.error.is_some() {
            return;
        }
        if let Err(error) = worker.tick() {
            let _ = worker.publish(Admission::Failed(error.clone()));
            log(format!(
                "creation blocked: {error}; native attempt retained"
            ));
            worker.error = Some(error);
        }
    });
}
fn frame_callback(bytes: Arc<[u8]>, base: u64, config: Config, data: TaskInvocation) {
    if !retirement::accepts_frame_end(data.group_id) {
        return;
    }
    if !thread_matches(&FRAME_THREAD) {
        return;
    }
    FRAME.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.error.is_some() {
            return;
        }
        let current_epoch = epoch();
        if slot
            .current
            .as_ref()
            .is_some_and(|v| v.backend.epoch() != current_epoch)
        {
            let mut old = slot.current.take().unwrap();
            old.backend.quarantine();
            slot.quarantined.push(old);
        }
        if slot.current.is_none() {
            if slot.quarantined.len() >= 64 {
                slot.error = Some("frame quarantine bound".into());
                return;
            }
            // SAFETY: Actual FrameEnd host retains its unique bindings and restoration journals.
            match unsafe { Runtime::bind(config, bytes, base, current_epoch) } {
                Ok(runtime) => slot.current = Some(runtime),
                Err(error) => {
                    slot.error = Some(error);
                    return;
                }
            }
        }
        update(slot.current.as_mut().unwrap(), data);
    });
}
pub(super) fn install(bytes: Arc<[u8]>, config: Config) -> Result<(), String> {
    let file = Ww171File::verify(&bytes).map_err(|e| e.to_string())?;
    // SAFETY: Reads the host executable module handle; no game call.
    let base = unsafe { GetModuleHandleW(None) }
        .map_err(|e| e.to_string())?
        .0 as u64;
    let started = Instant::now();
    loop {
        let objects = file
            .bind(&CurrentProcess, base)
            .map_err(|e| e.to_string())?;
        let mut ready = true;
        for phase in [TaskPhase::CHR_POST_PHYSICS, TaskPhase::FRAME_END] {
            let registered = match objects
                .observe_phase(phase)
                .map_err(|e| e.to_string())?
                .and_then(|v| v.native_group_id())
            {
                Some(id) => objects
                    .registered_group(id, 4096)
                    .map_err(|e| e.to_string())?
                    .is_some(),
                None => false,
            };
            ready &= registered;
        }
        if ready {
            break;
        }
        if started.elapsed() >= Duration::from_secs(30) {
            return Err("DemonBeast task startup timeout".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    // SAFETY: Process-resident registration; actual native engine lifetime remains a host obligation.
    let engine =
        unsafe { CurrentTaskEngine::bind(bytes.clone(), base) }.map_err(|e| e.to_string())?;
    let read = bytes.clone();
    let creation = TaskCallback::new(move |data| creation_callback(read.clone(), base, data));
    let frame =
        TaskCallback::new(move |data| frame_callback(bytes.clone(), base, config.clone(), data));
    // SAFETY: Static owner pins the DLL before registration and retains both partial outcomes.
    let installed = unsafe {
        TASKS.install(
            engine,
            [TaskPhase::CHR_POST_PHYSICS, TaskPhase::FRAME_END],
            [creation, frame],
        )
    }
    .map_err(|e| e.to_string())?;
    let control = installed.registration.map_err(|e| e.to_string())?;
    if !control.registered_all() {
        return Err("partial task installation retained".into());
    }
    log("ShikiFSRS two-phase DemonBeast host registered; runtime contracts experimental");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use shikifsrs_elden_ring::debug_creation_fields::OrdinaryCreationRoute;
    fn request(serial: u64, epoch: u64) -> SpawnRequest {
        SpawnRequest {
            serial,
            body: CharacterIdentity {
                address: 8,
                world_address: 16,
                native_identifier: 32,
                epoch,
            },
            input: OrdinaryCreationInput {
                name_units: vec![99],
                position_bits: [0; 4],
                rotation_bits: [0; 4],
                npc_param_id: 1,
                npc_think_param_id: 2,
                event_entity_id: 3,
                talk_id: 0,
                chara_init_param_id: -1,
                animation_id: -1,
                route: OrdinaryCreationRoute::Enemy,
            },
        }
    }
    #[test]
    fn cancellation_is_published_once_and_requires_its_worker_release_acknowledgement() {
        let mut mailbox = Mailbox {
            request: Some(request(5, 1)),
            ..Default::default()
        };
        mailbox.publish(5, Admission::Cancelled).unwrap();
        assert!(mailbox.publish(5, Admission::Cancelled).is_err());
        assert!(!mailbox.request_release(5).unwrap());
        assert!(!mailbox.acknowledge_release(Some(4)));
        assert!(mailbox.request.is_some());
        assert!(mailbox.acknowledge_release(Some(5)));
        assert!(mailbox.request_release(5).unwrap());
        assert!(mailbox.request.is_none() && mailbox.admission.is_none());
    }
    #[test]
    fn release_cannot_cancel_an_unpublished_or_another_owned_creation() {
        let mut mailbox = Mailbox {
            request: Some(request(7, 1)),
            ..Default::default()
        };
        assert!(mailbox.request_release(7).is_err());
        assert!(!mailbox.acknowledge_release(Some(7)));
        assert!(mailbox.publish(6, Admission::Cancelled).is_err());
        mailbox.publish(7, Admission::Cancelled).unwrap();
        assert!(mailbox.request_release(6).is_err());
        assert_eq!(mailbox.release, None);
    }
    #[test]
    fn delayed_old_epoch_worker_preserves_the_new_frame_owners_request() {
        let mut mailbox = Mailbox {
            request: Some(request(9, 2)),
            ..Default::default()
        };
        mailbox.revoke_old_request(2);
        assert_eq!(mailbox.request.as_ref().unwrap().serial, 9);
        mailbox.publish(9, Admission::Cancelled).unwrap();
        assert!(mailbox.publish(8, Admission::Cancelled).is_err());
        mailbox.revoke_old_request(3);
        assert!(mailbox.request.is_none() && mailbox.admission.is_none());
    }
}

//! Actual independent SDK creation, control journals, presentation and FrameEnd retirement.
use super::*;
use shikifsrs_elden_ring::{
    Ww171File,
    character_flags::CharacterFlagByte,
    character_handles::NativeCharacterHandle,
    character_policy_session::{
        CharacterPolicyEdit, CharacterPolicyField, CharacterPolicyReport, CharacterPolicySession,
        CurrentCharacterPolicyAccess, ManualOpacityField,
    },
    character_resources::{CharacterBackreadRequest, CharacterBackreadWrite},
    character_sets::{
        CharacterIdentity, CharacterRegistrationObservation, WorldCollectionSelection,
    },
    character_spatial::{NativeCharacterPlacementCall, NativePoseInput},
    control_link_session::{ControlLinkSession, ControlLinkSessionReport},
    controller_policy::ControllerPolicyField,
    creation_input_preparation::OrdinaryCreationInput,
    current_character_events::{CurrentCharacterEvents, EventEntityInsertion},
    current_character_flags::CurrentCharacterFlags,
    current_character_resources::CurrentCharacterResources,
    current_character_scalars::CurrentCharacterScalarAccess,
    current_character_spatial::{CurrentCharacterSpatial, NativeEulerCall},
    current_character_vitals::CurrentCharacterVitals,
    current_control_links::CurrentControlLinks,
    current_controller_policy::CurrentControllerPolicy,
    current_objects::CurrentObjects,
    current_parameters::CurrentParameterAccess,
    current_physics_suppression::CurrentPhysicsSuppression,
    debug_creation_fields::OrdinaryCreationRoute,
    objects::GameObjectCatalog,
    parameter_fields::{ParameterField, ScalarValue},
    parameters::ParameterSlot,
};
use shikifsrs_runtime::{Error, memory::AddressSpace};
use std::collections::{BTreeSet, VecDeque};
const LIMIT: usize = 65536;
const ROW_LIMIT: usize = 65535;
// Only copied identities cross phases. Native reports/tickets remain with this callback owner.
#[allow(dead_code)]
enum NativeReceipt {
    Placement(Box<NativeCharacterPlacementCall>),
    Euler(NativeEulerCall),
    Backread(CharacterBackreadWrite),
    Policy(CharacterPolicyReport),
    Control(ControlLinkSessionReport),
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
pub(super) struct NativeBackend {
    pub(super) config: Config,
    pub(super) form_index: usize,
    pub(super) body: Option<CharacterIdentity>,
    pub(super) beast: Option<CharacterIdentity>,
    pub(super) pending: bool,
    pub(super) linked: bool,
    pub(super) objects: CurrentObjects,
    epoch: u64,
    npc_think_slot: ParameterSlot,
    flags: CurrentCharacterFlags,
    policy: CurrentControllerPolicy,
    physics: CurrentPhysicsSuppression,
    scalars: CurrentCharacterScalarAccess,
    vitals: CurrentCharacterVitals,
    spatial: CurrentCharacterSpatial,
    resources: CurrentCharacterResources,
    events: CurrentCharacterEvents,
    links: CurrentControlLinks,
    parameters: CurrentParameterAccess,
    retirement_access: retirement::NativeAccess,
    creation_serial: Option<u64>,
    actor: Option<CharacterRegistrationObservation>,
    body_policy: Option<CharacterPolicySession>,
    beast_policy: Option<CharacterPolicySession>,
    control: Option<ControlLinkSession>,
    parking: Option<CharacterPolicySession>,
    insertion: Option<EventEntityInsertion>,
    present_requested: bool,
    retirement: Retirement,
    retiring: Option<retirement::Pending>,
    behavior: Option<behavior_bridge::BehaviorBridge>,
    camera: camera_profile::CameraProfile,
    receipts: VecDeque<NativeReceipt>,
    retained_failures: Vec<NativeReceipt>,
    native_fault: Option<String>,
}
fn err(error: Error) -> String {
    error.to_string()
}
fn absent() -> String {
    "required current character component unavailable".into()
}
fn edit(field: CharacterPolicyField, mask: u32, bits: u32) -> CharacterPolicyEdit {
    CharacterPolicyEdit { field, mask, bits }
}
fn opacity(field: ManualOpacityField, value: f32) -> CharacterPolicyEdit {
    edit(
        CharacterPolicyField::ManualOpacity(field),
        u32::MAX,
        value.to_bits(),
    )
}
fn body_edits() -> Vec<CharacterPolicyEdit> {
    use CharacterPolicyField as F;
    vec![
        edit(F::Flag(CharacterFlagByte::DebugLow), 0x38, 0x38),
        edit(F::Flag(CharacterFlagByte::Status1), 8, 0),
        edit(F::Controller(ControllerPolicyField::ControllerLow), 2, 2),
        edit(F::PhysicsRequest, 255, 1),
        edit(F::Controller(ControllerPolicyField::ModifierWord10), 4, 4),
        opacity(ManualOpacityField::Base24c, 0.0),
        opacity(ManualOpacityField::Factor240, 0.0),
    ]
}
fn actor_edits(enabled: bool, team: Option<u8>) -> Vec<CharacterPolicyEdit> {
    use CharacterPolicyField as F;
    let mut edits = vec![
        edit(
            F::Flag(CharacterFlagByte::Status1),
            8,
            if enabled { 8 } else { 0 },
        ),
        edit(
            F::Flag(CharacterFlagByte::DebugHigh),
            1,
            u32::from(!enabled),
        ),
        edit(
            F::Flag(CharacterFlagByte::DebugLow),
            0x38,
            if enabled { 0 } else { 0x38 },
        ),
        opacity(ManualOpacityField::Base24c, f32::from(u8::from(enabled))),
        opacity(ManualOpacityField::Factor240, f32::from(u8::from(enabled))),
    ];
    if let Some(team) = team {
        edits.push(edit(
            F::Flag(CharacterFlagByte::StoredTeam),
            255,
            u32::from(team),
        ));
    }
    edits
}
fn access(backend: &NativeBackend) -> CurrentCharacterPolicyAccess<'_> {
    CurrentCharacterPolicyAccess {
        flags: &backend.flags,
        controller: &backend.policy,
        physics: &backend.physics,
        scalars: &backend.scalars,
    }
}
impl NativeBackend {
    pub(super) unsafe fn bind(
        config: Config,
        bytes: Arc<[u8]>,
        base: u64,
        epoch: u64,
    ) -> Result<Self, String> {
        let think = Ww171File::verify(&bytes)
            .map_err(err)?
            .parameter_slot("NpcThinkParam")
            .map_err(err)?
            .ok_or("independently named NpcThinkParam slot missing")?;
        let form_index = config.default_index();
        // SAFETY: Actual thread-confined FrameEnd host; complete graph/phase/lifetime contracts remain experimental.
        unsafe {
            Ok(Self {
                config,
                form_index,
                body: None,
                beast: None,
                pending: false,
                linked: false,
                epoch,
                objects: CurrentObjects::bind(bytes.clone(), base, epoch, None).map_err(err)?,
                npc_think_slot: think,
                flags: CurrentCharacterFlags::bind(bytes.clone(), base, epoch, None)
                    .map_err(err)?,
                policy: CurrentControllerPolicy::bind(bytes.clone(), base, epoch, None)
                    .map_err(err)?,
                physics: CurrentPhysicsSuppression::bind(bytes.clone(), base, epoch, None)
                    .map_err(err)?,
                scalars: CurrentCharacterScalarAccess::bind(bytes.clone(), base, epoch)
                    .map_err(err)?,
                vitals: CurrentCharacterVitals::bind(bytes.clone(), base, epoch, None)
                    .map_err(err)?,
                spatial: CurrentCharacterSpatial::bind(bytes.clone(), base, epoch, None)
                    .map_err(err)?,
                resources: CurrentCharacterResources::bind(bytes.clone(), base, epoch, None)
                    .map_err(err)?,
                events: CurrentCharacterEvents::bind(bytes.clone(), base, epoch, None)
                    .map_err(err)?,
                links: CurrentControlLinks::bind(bytes.clone(), base, epoch, None).map_err(err)?,
                parameters: CurrentParameterAccess::bind(bytes.clone(), base, epoch, ROW_LIMIT)
                    .map_err(err)?,
                retirement_access: retirement::NativeAccess::bind(bytes, base, epoch)
                    .map_err(err)?,
                creation_serial: None,
                actor: None,
                body_policy: None,
                beast_policy: None,
                control: None,
                parking: None,
                insertion: None,
                present_requested: false,
                retirement: Retirement::None,
                retiring: None,
                behavior: None,
                camera: Default::default(),
                receipts: VecDeque::new(),
                retained_failures: Vec::new(),
                native_fault: None,
            })
        }
    }
    pub(super) fn epoch(&self) -> u64 {
        self.epoch
    }
    pub(super) fn form(&self) -> &Form {
        &self.config.forms[self.form_index]
    }
    pub(super) fn policy_pending(&self) -> usize {
        self.body_policy.as_ref().map_or(0, |p| p.pending_restore())
            + self
                .beast_policy
                .as_ref()
                .map_or(0, |p| p.pending_restore())
            + self.control.as_ref().map_or(0, |p| p.pending_restore())
            + self.camera.pending()
    }
    pub(super) fn retirement_pending(&self) -> bool {
        matches!(
            self.retirement,
            Retirement::Requested | Retirement::Draining | Retirement::Quarantined(_)
        )
    }
    pub(super) fn quarantine(&mut self) {
        if let Some(pending) = &mut self.retiring {
            pending.record.quarantine();
        }
        self.retirement = Retirement::Quarantined(
            "epoch revoked; all old journals and native tickets retained".into(),
        );
    }
    fn remember(
        &mut self,
        receipt: NativeReceipt,
        completion: Result<(), String>,
    ) -> Result<(), String> {
        if let Err(error) = &completion {
            if matches!(
                &receipt,
                NativeReceipt::Placement(_) | NativeReceipt::Euler(_) | NativeReceipt::Backread(_)
            ) {
                self.native_fault = Some(error.clone());
            }
            self.retained_failures.push(receipt);
        } else {
            self.receipts.push_back(receipt);
            if self.receipts.len() > 64 {
                self.receipts.pop_front();
            }
        }
        completion
    }
    pub(super) fn select_form(&mut self, index: usize) -> Result<(), String> {
        if self.body.is_some()
            || self.beast.is_some()
            || self.pending
            || self.policy_pending() != 0
            || !matches!(self.retirement, Retirement::None)
            || self.creation_serial.is_some()
        {
            return Err("old form still owns resources".into());
        }
        if !self.config.forms.get(index).is_some_and(|f| f.enabled) {
            return Err("invalid form index".into());
        }
        self.form_index = index;
        Ok(())
    }
    fn validate_session(&self, beast: bool) -> Result<(), String> {
        if resident::epoch() != self.epoch {
            return Err("epoch revoked".into());
        }
        let body = self.body.ok_or("missing body identity")?;
        self.objects
            .observe(|objects, epoch| {
                if objects.primary_player_identity(epoch)? != Some(body) {
                    return Err(Error::NativeOperationRejected("primary identity changed"));
                }
                if beast
                    && let Some(target) = self.beast
                    && objects.character_identity(target.address, epoch)? != Some(target)
                {
                    return Err(Error::NativeOperationRejected("beast identity changed"));
                }
                Ok(())
            })
            .map_err(err)
    }
    pub(super) fn sample_effects(&self) -> Result<EffectSnapshot, String> {
        self.objects
            .observe(|objects, epoch| {
                let id = objects
                    .primary_player_identity(epoch)?
                    .ok_or(Error::NativeOperationRejected("no primary"))?;
                if self.body.is_some_and(|body| body != id) {
                    return Err(Error::NativeOperationRejected("primary changed"));
                }
                effect_snapshot(objects, id)
            })
            .map_err(err)
    }
    pub(super) fn sample_beast_effects(&self) -> Result<Option<EffectSnapshot>, String> {
        if !matches!(self.retirement, Retirement::None) {
            return Ok(None);
        }
        let Some(id) = self.beast else {
            return Ok(None);
        };
        self.validate_session(true)?;
        self.objects
            .observe(|objects, _| effect_snapshot(objects, id))
            .map(Some)
            .map_err(err)
    }
    fn pose(&mut self, identity: CharacterIdentity) -> Result<NativePoseInput, String> {
        let position = self
            .spatial
            .observe(identity)
            .map_err(err)?
            .ok_or_else(absent)?;
        position.finite_position().map_err(err)?;
        // SAFETY: Actual exclusive read phase retains character/physics/getter/math graph.
        let receipt = unsafe { self.spatial.euler(identity) }.map_err(err)?;
        let angles = receipt.value.finite_xyz().map_err(err);
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(|e| e.to_string());
        self.remember(NativeReceipt::Euler(receipt), completion)?;
        Ok(NativePoseInput {
            position_bits: position.position_bits,
            angle_bits: [0.0f32.to_bits(), angles?[1].to_bits(), 0.0f32.to_bits()],
        })
    }
    fn follow(&mut self) -> Result<(), String> {
        let input = self.pose(self.beast.ok_or("missing beast")?)?;
        let source = self
            .spatial
            .observe_placement(self.body.ok_or("missing body")?)
            .map_err(err)?
            .ok_or_else(absent)?;
        // SAFETY: Exact exclusive live body graph and native placement phase. No fixed vertical offset.
        let receipt = unsafe { self.spatial.place_character(source, input) }.map_err(err)?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(|e| e.to_string());
        self.remember(NativeReceipt::Placement(Box::new(receipt)), completion)
    }
    fn capture_pending(&mut self) -> Result<bool, String> {
        if !self.pending {
            if let Some(error) = &self.native_fault {
                return Err(error.clone());
            }
            return Ok(self.beast.is_some());
        }
        let serial = self
            .creation_serial
            .ok_or("pending creation has no serial")?;
        match resident::admission(serial)? {
            None => Ok(false),
            Some(resident::Admission::Failed(error)) => Err(error),
            Some(resident::Admission::Cancelled) => {
                self.pending = false;
                Ok(false)
            }
            Some(resident::Admission::Created(actor)) => {
                self.actor = Some(actor);
                self.beast = Some(actor.character);
                self.pending = false;
                let completion = (|| {
                    self.validate_session(true)?;
                    let source = self
                        .events
                        .observe(actor, LIMIT)
                        .map_err(err)?
                        .ok_or_else(absent)?;
                    if source.id.stored_id != self.config.ownership_tag as u32 {
                        return Err("admitted ownership tag disagrees".into());
                    }
                    // SAFETY: Exact attributed registration; insertion is attempted once and retained.
                    let receipt =
                        unsafe { self.events.insert_stored_id(source, LIMIT) }.map_err(err)?;
                    let completion = match &receipt {
                        EventEntityInsertion::AlreadyPresent(_) => Ok(()),
                        EventEntityInsertion::Submitted { after, .. } => {
                            after.as_ref().map(|_| ()).map_err(|e| e.to_string())
                        }
                    };
                    self.insertion = Some(receipt);
                    completion
                })();
                if let Err(error) = &completion {
                    self.native_fault = Some(error.clone());
                }
                completion.map(|_| true)
            }
        }
    }
    fn ready_marks(&mut self) -> Result<bool, String> {
        let actor = self.actor.ok_or("missing actor registration")?;
        let value = self
            .resources
            .observe(actor.character, WorldCollectionSelection::Creation, LIMIT)
            .map_err(err)?
            .ok_or_else(absent)?;
        if value.member.registration != actor {
            return Err("resource registration superseded".into());
        }
        if !self.present_requested {
            // SAFETY: Owned single actor request byte and retained live resources. Not a whole-graph ready barrier.
            let receipt = unsafe {
                self.resources
                    .request(value.member, CharacterBackreadRequest::Present, LIMIT)
            }
            .map_err(err)?;
            self.present_requested = true;
            let completion = receipt
                .postflight
                .as_ref()
                .map(|_| ())
                .map_err(|e| e.to_string());
            self.remember(NativeReceipt::Backread(receipt), completion)?;
            return Ok(false);
        }
        Ok(value.native_presentation_marks())
    }
    fn link(&mut self) -> Result<(), String> {
        let body = self.body.ok_or("missing body")?;
        let beast = self.beast.ok_or("missing beast")?;
        let team = self
            .flags
            .observe(body)
            .map_err(err)?
            .ok_or_else(absent)?
            .value(CharacterFlagByte::StoredTeam);
        let current = access(self);
        let body_policy = CharacterPolicySession::capture(&current, body, &body_edits())
            .map_err(err)?
            .ok_or_else(absent)?;
        let beast_policy =
            CharacterPolicySession::capture(&current, beast, &actor_edits(true, Some(team)))
                .map_err(err)?
                .ok_or_else(absent)?;
        let control = ControlLinkSession::capture(&self.links, body, beast)
            .map_err(err)?
            .ok_or_else(absent)?;
        self.body_policy = Some(body_policy);
        self.beast_policy = Some(beast_policy);
        self.control = Some(control);
        let mut beast_policy = self.beast_policy.take().unwrap();
        // SAFETY: All policy inputs captured before any write; actual-store journal survives each partial outcome.
        let report = unsafe { beast_policy.apply(&access(self)) };
        self.beast_policy = Some(beast_policy);
        self.policy_receipt(report)?;
        let mut body_policy = self.body_policy.take().unwrap();
        // SAFETY: Same complete exclusive graph, selected body bits reserved for the active interval.
        let report = unsafe { body_policy.apply(&access(self)) };
        self.body_policy = Some(body_policy);
        self.policy_receipt(report)?;
        let report = unsafe { self.control.as_mut().unwrap().apply(&self.links) };
        let completion = if report.stopped.is_some() {
            Err(format!("partial control link: {:?}", report.stopped))
        } else {
            Ok(())
        };
        self.remember(NativeReceipt::Control(report), completion)?;
        self.linked = true;
        Ok(())
    }
    fn policy_receipt(&mut self, report: CharacterPolicyReport) -> Result<(), String> {
        let completion = report
            .stopped
            .as_ref()
            .map_or(Ok(()), |stop| Err(stop.error.to_string()));
        self.remember(NativeReceipt::Policy(report), completion)
    }
    fn owned_links(&self) -> Result<(), String> {
        let baseline = self
            .control
            .as_ref()
            .ok_or("missing link journal")?
            .baseline();
        let beast = self.beast.ok_or("missing beast")?;
        let debug = self
            .links
            .observe_debug_links()
            .map_err(err)?
            .ok_or_else(absent)?;
        let channels = self
            .links
            .observe_channels(beast)
            .map_err(err)?
            .ok_or_else(absent)?;
        let input = self
            .links
            .observe_manipulator(baseline.input.object.address)
            .map_err(err)?
            .ok_or_else(absent)?;
        if debug.object != baseline.debug.object
            || debug.character_override != Some(beast)
            || input.object != baseline.input.object
            || input.character != Some(beast)
            || channels.controller != baseline.channels.controller
            || channels.override_input.map(|v| v.object) != Some(input.object)
            || debug.input.map(|v| v.object) != Some(input.object)
        {
            return Err("control ownership lost".into());
        }
        Ok(())
    }
    pub(super) fn poll_retirement(&mut self) {
        if !matches!(
            self.retirement,
            Retirement::Requested | Retirement::Draining
        ) {
            return;
        }
        if let Err(error) = self.retire_at_frame_end() {
            if let Some(pending) = &mut self.retiring {
                pending.record.quarantine();
            }
            self.retirement = Retirement::Quarantined(error.clone());
            log(format!("retirement quarantined: {error}"));
        }
    }
    fn retire_at_frame_end(&mut self) -> Result<(), String> {
        self.validate_session(false)?;
        if self.native_fault.is_some() {
            return Err("uncertain native operation; destruction prohibited".into());
        }
        if matches!(self.retirement, Retirement::Requested) {
            let actor = self.actor.ok_or("missing actor retirement")?;
            let debug = self
                .links
                .observe_debug_links()
                .map_err(err)?
                .ok_or_else(absent)?;
            let channels = self
                .links
                .observe_channels(actor.character)
                .map_err(err)?
                .ok_or_else(absent)?;
            if debug.character_override == Some(actor.character)
                || debug
                    .input
                    .is_some_and(|v| v.character == Some(actor.character))
                || channels.override_input.is_some()
                || self.linked
                || self.policy_pending() != 0
            {
                return Err("control/restore obligations still reference beast".into());
            }
            self.retiring = Some(retirement::Pending::new(retirement::Request {
                actor,
                primary: self.body.ok_or("missing retirement primary")?,
                not_before: Instant::now(),
            }));
            self.beast = None;
            self.retirement = Retirement::Draining;
        }
        let pending = self.retiring.as_mut().ok_or("missing retirement record")?;
        pending.advance(&self.retirement_access).map_err(err)?;
        if pending.protocol.complete {
            self.retirement = Retirement::Complete;
        }
        Ok(())
    }
    fn finish_restore(&mut self) -> Result<bool, String> {
        if let Some(serial) = self.creation_serial {
            if !resident::release(serial)? {
                return Ok(false);
            }
            self.creation_serial = None;
        }
        self.body = None;
        self.beast = None;
        self.actor = None;
        self.pending = false;
        self.linked = false;
        self.body_policy = None;
        self.beast_policy = None;
        self.control = None;
        self.parking = None;
        self.insertion = None;
        self.present_requested = false;
        self.retiring = None;
        self.retirement = Retirement::None;
        self.native_fault = None;
        self.receipts.clear();
        if let Some(bridge) = self.behavior.take() {
            bridge.close();
        }
        Ok(true)
    }
}
impl Backend for NativeBackend {
    fn begin(&mut self) -> Result<(), String> {
        if self.body.is_some()
            || self.pending
            || self.creation_serial.is_some()
            || self.policy_pending() != 0
            || !matches!(self.retirement, Retirement::None)
        {
            return Err("previous cleanup still pending".into());
        }
        let body = self
            .links
            .observe_primary_identity()
            .map_err(err)?
            .ok_or_else(absent)?;
        let vitals = self.vitals.observe(body).map_err(err)?.ok_or_else(absent)?;
        let ride = self
            .objects
            .observe(|objects, _| objects.character_ride_phase(body))
            .map_err(err)?
            .ok_or_else(absent)?;
        if vitals.hp.current <= 0 || ride.native_active || ride.native_endpoint_one_query {
            return Err("player must be alive and unmounted".into());
        }
        let pose = self.pose(body)?;
        self.objects
            .observe(|objects, _| {
                for (slot, id) in [
                    (
                        ParameterSlot::new(ParameterField::NpcHp.definition().slot)?,
                        self.form().npc_param_id,
                    ),
                    (self.npc_think_slot, self.form().npc_think_param_id),
                ] {
                    let rows = objects.parameter_rows(slot, 0, ROW_LIMIT)?.ok_or(
                        Error::NativeOperationRejected("configured parameter table absent"),
                    )?;
                    if !rows.rows.iter().any(|row| row.id == id as u32) {
                        return Err(Error::NativeOperationRejected(
                            "configured parameter row missing",
                        ));
                    }
                }
                let set = objects
                    .world_creation_set(LIMIT)?
                    .ok_or(Error::NativeOperationRejected("creation set absent"))?;
                for entry in &set.set.characters {
                    let identity = objects
                        .character_identity(entry.character.address, self.epoch)?
                        .ok_or(Error::NativeOperationRejected(
                            "creation member disappeared",
                        ))?;
                    if objects
                        .character_event_id(identity)?
                        .is_some_and(|id| id.stored_id == self.config.ownership_tag as u32)
                    {
                        return Err(Error::NativeOperationRejected("ownership tag occupied"));
                    }
                }
                Ok(())
            })
            .map_err(err)?;
        if self.form().behavior_lifecycle() {
            self.behavior = Some(behavior_bridge::BehaviorBridge::start(
                PathBuf::from("F:/GoldenAge/_GoldenAge/dll/demon_beast/Saved/er-demon-beast"),
                self.form(),
            )?);
        }
        let input = OrdinaryCreationInput {
            name_units: format!("c{:04}", self.form().chr_id)
                .encode_utf16()
                .collect(),
            position_bits: pose.position_bits,
            rotation_bits: [
                pose.angle_bits[0],
                pose.angle_bits[1],
                pose.angle_bits[2],
                0,
            ],
            npc_param_id: self.form().npc_param_id,
            npc_think_param_id: self.form().npc_think_param_id,
            event_entity_id: self.config.ownership_tag as u32,
            talk_id: 0,
            chara_init_param_id: -1,
            animation_id: -1,
            route: OrdinaryCreationRoute::Enemy,
        };
        self.body = Some(body);
        self.creation_serial = Some(resident::enqueue(body, input)?);
        self.pending = true;
        Ok(())
    }
    fn poll_ready(&mut self) -> Result<bool, String> {
        if let Some(error) = &self.native_fault {
            return Err(error.clone());
        }
        self.validate_session(true)?;
        if !self.capture_pending()? {
            return Ok(false);
        }
        if !self.ready_marks()? {
            return Ok(false);
        }
        if !self.linked {
            self.link()?;
        }
        self.maintain()?;
        match &mut self.behavior {
            Some(bridge) => bridge.ready(),
            None => Ok(true),
        }
    }
    fn maintain(&mut self) -> Result<(), String> {
        self.validate_session(true)?;
        self.owned_links()?;
        let beast = self.beast.ok_or("no beast")?;
        if self
            .vitals
            .observe(beast)
            .map_err(err)?
            .ok_or_else(absent)?
            .hp
            .current
            <= 0
        {
            return Err("beast died; recovery requested".into());
        }
        let mut camera_ids = vec![0, 1, 2];
        if let Some(id) = self
            .objects
            .observe(|objects, epoch| {
                let Some(target) = objects.primary_player_target(epoch)? else {
                    return Ok(None);
                };
                let Some(target) = resolve_handle(objects, target.handle, epoch)? else {
                    return Ok(None);
                };
                let Some(npc) = objects.character_npc_request_id(target)? else {
                    return Ok(None);
                };
                let value = objects.experimental_parameter_field(
                    ParameterField::NpcLockCamera,
                    0,
                    npc.stored_id as u32,
                    ROW_LIMIT,
                )?;
                match value.map(|v| v.value) {
                    Some(ScalarValue::I32(id)) if id >= 0 => Ok(Some(id as u32)),
                    _ => Ok(None),
                }
            })
            .map_err(err)?
        {
            camera_ids.push(id);
        }
        let distance = self.form().camera_distance;
        let height = self.form().camera_height;
        // SAFETY: Exclusive exact live camera parameter rows; conditional original restoration is retained.
        unsafe {
            self.camera
                .apply(&self.parameters, &camera_ids, distance, height)
        }?;
        self.follow()?;
        let mut policy = self.body_policy.take().ok_or("missing body policy")?;
        // SAFETY: Selected body bits remain reserved; never refresh the control-link pointers.
        let report = unsafe { policy.refresh(&access(self)) };
        self.body_policy = Some(policy);
        self.policy_receipt(report)?;
        self.owned_links()
    }
    fn completed(&mut self) -> Result<bool, String> {
        match &mut self.behavior {
            Some(bridge) => bridge.completed(),
            None => Ok(false),
        }
    }
    fn finish(&mut self) -> Result<bool, String> {
        self.maintain()?;
        match &mut self.behavior {
            Some(bridge) => bridge.finish(),
            None => Ok(true),
        }
    }
    fn restore(&mut self) -> Result<bool, String> {
        // SAFETY: Exact parameter overlay identities and exclusive restoration phase.
        unsafe { self.camera.restore(&self.parameters) }?;
        match &self.retirement {
            Retirement::Requested | Retirement::Draining => return Ok(false),
            Retirement::Quarantined(error) => return Err(error.clone()),
            Retirement::Complete => return self.finish_restore(),
            Retirement::None => (),
        }
        if self.body.is_none() {
            return self.finish_restore();
        }
        self.validate_session(false)?;
        if self.pending {
            resident::cancel_unsubmitted(self.creation_serial.ok_or("missing creation serial")?);
            if !self.capture_pending()? {
                if self.pending {
                    return Ok(false);
                }
                return self.finish_restore();
            }
        }
        if self.linked {
            self.validate_session(true)?;
            self.follow()?;
        }
        if let Some(control) = &mut self.control {
            // SAFETY: Conditional reverse debug -> override -> role; foreign pointers are preserved.
            let report = unsafe { control.restore(&self.links) };
            let completion = if report.stopped.is_some() || report.pending_restore != 0 {
                Err(format!(
                    "control restore pending={}; stop={:?}",
                    report.pending_restore, report.stopped
                ))
            } else {
                Ok(())
            };
            self.remember(NativeReceipt::Control(report), completion)?;
        }
        for beast in [true, false] {
            let mut session = if beast {
                self.beast_policy.take()
            } else {
                self.body_policy.take()
            };
            if let Some(policy) = &mut session {
                // SAFETY: Restore exact still-owned bits and preserve native/foreign unselected values.
                let report = unsafe { policy.restore(&access(self)) };
                let completion = report
                    .stopped
                    .as_ref()
                    .map_or(Ok(()), |s| Err(s.error.to_string()));
                self.receipts.push_back(NativeReceipt::Policy(report));
                if beast {
                    self.beast_policy = session;
                } else {
                    self.body_policy = session;
                }
                completion?;
            }
        }
        self.linked = false;
        if let Some(beast) = self.beast {
            self.validate_session(true)?;
            if !self.ready_marks()? {
                return Ok(false);
            }
            if self.parking.is_none() {
                self.parking = Some(
                    CharacterPolicySession::capture(
                        &access(self),
                        beast,
                        &actor_edits(false, None),
                    )
                    .map_err(err)?
                    .ok_or_else(absent)?,
                );
                let mut parking = self.parking.take().unwrap();
                // SAFETY: Own registered beast; parking journal remains with the retirement record.
                let report = unsafe { parking.apply(&access(self)) };
                self.parking = Some(parking);
                self.policy_receipt(report)?;
            }
            self.retirement = Retirement::Requested;
            return Ok(false);
        }
        self.finish_restore()
    }
}
fn effect_snapshot<M: AddressSpace>(
    objects: &GameObjectCatalog<'_, '_, M>,
    id: CharacterIdentity,
) -> shikifsrs_runtime::Result<EffectSnapshot> {
    let effects = objects
        .character_effects(id, 1024)?
        .ok_or(Error::NativeOperationRejected("effect container absent"))?;
    Ok(EffectSnapshot {
        identity: (id.world_address, id.address, id.native_identifier),
        ids: effects
            .entries
            .into_iter()
            .filter_map(|node| (node.native_id > 0).then_some(node.native_id))
            .collect::<BTreeSet<_>>(),
    })
}
pub(super) fn resolve_handle<M: AddressSpace>(
    objects: &GameObjectCatalog<'_, '_, M>,
    handle: NativeCharacterHandle,
    epoch: u64,
) -> shikifsrs_runtime::Result<Option<CharacterIdentity>> {
    if handle.is_absent() {
        return Ok(None);
    }
    let world = objects.world()?;
    let mut result = None;
    for selection in [
        WorldCollectionSelection::Players,
        WorldCollectionSelection::Embedded10f38,
        WorldCollectionSelection::Embedded10f90,
        WorldCollectionSelection::Creation,
        WorldCollectionSelection::OpenField,
    ]
    .into_iter()
    .chain((0..197).map(WorldCollectionSelection::Indexed))
    {
        if let Some(collection) = objects.world_collection(selection, epoch, LIMIT)? {
            for member in collection.set.characters {
                if member.native_identifier != handle.0 {
                    continue;
                }
                let identity = CharacterIdentity {
                    address: member.character.address,
                    world_address: collection.world_address,
                    native_identifier: member.native_identifier,
                    epoch,
                };
                if result.is_some_and(|old| old != identity) {
                    return Err(Error::NativeOperationRejected("ambiguous full handle"));
                }
                if objects
                    .character_registration(identity, collection.set.address, LIMIT)?
                    .is_none()
                {
                    return Err(Error::NativeOperationRejected("handle member disappeared"));
                }
                result = Some(identity);
            }
        }
    }
    if objects.world()? != world {
        return Err(Error::NativeOperationRejected(
            "world changed during handle lookup",
        ));
    }
    Ok(result)
}

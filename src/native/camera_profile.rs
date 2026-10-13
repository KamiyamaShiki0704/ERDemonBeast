//! Runtime camera overlays; only exact owned values are conditionally restored.
use shikifsrs_elden_ring::{
    parameter_fields::{ParameterField, ScalarValue},
    parameter_overlays::{FieldKey, ParameterAccess, ParameterOverlays, ReplaceOutcome},
};
use std::collections::BTreeSet;

#[derive(Default)]
pub(super) struct CameraProfile {
    overlays: ParameterOverlays,
    applied_ids: BTreeSet<u32>,
}
impl CameraProfile {
    pub(super) fn pending(&self) -> usize {
        self.overlays.pending()
    }
    /// Caller holds the parameter session's exact live schema, exclusive phase and row lifetime.
    pub(super) unsafe fn apply<A: ParameterAccess>(
        &mut self,
        access: &A,
        ids: &[u32],
        distance: f32,
        height: f32,
    ) -> Result<(), String> {
        if distance == 0.0 {
            return Ok(());
        }
        if !distance.is_finite() || !height.is_finite() {
            return Err("invalid camera profile".into());
        }
        for &id in ids {
            if self.applied_ids.contains(&id) {
                continue;
            }
            if self.applied_ids.len() == 32 {
                return Err("camera profile row bound exceeded".into());
            }
            let keys =
                [ParameterField::CameraDistance, ParameterField::CameraHeight].map(|field| {
                    FieldKey {
                        field,
                        variant: 0,
                        row_id: id,
                    }
                });
            let Some(before_distance) = access.observe(keys[0]).map_err(|e| e.to_string())? else {
                continue;
            };
            let Some(before_height) = access.observe(keys[1]).map_err(|e| e.to_string())? else {
                continue;
            };
            if before_distance.identity != before_height.identity {
                return Err("camera row replaced between field copies".into());
            }
            let before = [before_distance.bits, before_height.bits].map(f32::from_bits);
            if before.iter().any(|v| !v.is_finite()) {
                return Err("invalid camera parameter row".into());
            }
            for (key, value) in keys
                .into_iter()
                .zip([before[0].max(distance), before[1].max(height)])
            {
                // SAFETY: Same exclusive live parameter contract; the SDK journals each actual store.
                match unsafe { self.overlays.apply(access, key, ScalarValue::F32(value)) }
                    .map_err(|e| e.to_string())?
                {
                    ReplaceOutcome::Applied => (),
                    ReplaceOutcome::Conflict => return Err("camera profile superseded".into()),
                    ReplaceOutcome::Unavailable => {
                        return Err("camera profile row unavailable".into());
                    }
                }
            }
            self.applied_ids.insert(id);
        }
        Ok(())
    }
    /// Caller retains row/resource lifetime and the parameter session's exclusive restore phase.
    pub(super) unsafe fn restore<A: ParameterAccess>(&mut self, access: &A) -> Result<(), String> {
        // SAFETY: Forwarded live parameter session contract; foreign values/resources stay untouched.
        let report = unsafe { self.overlays.restore(access) };
        if report.pending != 0 {
            return Err(format!(
                "camera restore pending={}; failures={:?}",
                report.pending, report.failures
            ));
        }
        self.applied_ids.clear();
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use shikifsrs_elden_ring::parameter_overlays::{FieldSnapshot, RowIdentity};
    use shikifsrs_runtime::{Error, Result};
    use std::cell::{Cell, RefCell};
    struct Memory {
        fields: RefCell<[FieldSnapshot; 2]>,
        writes: Cell<usize>,
        fail_height: Cell<bool>,
    }
    impl Memory {
        fn new(distance: f32, height: f32) -> Self {
            let identity = RowIdentity {
                epoch: 1,
                resource: 8,
                header: 16,
                index: 24,
                row: 32,
            };
            Self {
                fields: RefCell::new(
                    [ParameterField::CameraDistance, ParameterField::CameraHeight].map(|field| {
                        FieldSnapshot {
                            key: FieldKey {
                                field,
                                variant: 0,
                                row_id: 0,
                            },
                            identity,
                            bits: if field == ParameterField::CameraDistance {
                                distance
                            } else {
                                height
                            }
                            .to_bits(),
                        }
                    }),
                ),
                writes: Cell::new(0),
                fail_height: Cell::new(false),
            }
        }
        fn index(key: FieldKey) -> Option<usize> {
            if key.row_id != 0 || key.variant != 0 {
                return None;
            }
            match key.field {
                ParameterField::CameraDistance => Some(0),
                ParameterField::CameraHeight => Some(1),
                _ => None,
            }
        }
    }
    // SAFETY: Only original owned RefCell storage; no game allocations, native calls or concurrent writes.
    unsafe impl ParameterAccess for Memory {
        fn observe(&self, key: FieldKey) -> Result<Option<FieldSnapshot>> {
            Ok(Self::index(key).map(|i| self.fields.borrow()[i]))
        }
        unsafe fn compare_replace(
            &self,
            expected: FieldSnapshot,
            bits: u32,
        ) -> Result<ReplaceOutcome> {
            let Some(i) = Self::index(expected.key) else {
                return Ok(ReplaceOutcome::Unavailable);
            };
            if i == 1 && self.fail_height.get() {
                return Err(Error::InvalidProfile(
                    "original simulated second-field failure",
                ));
            }
            let mut fields = self.fields.borrow_mut();
            if fields[i] != expected {
                return Ok(ReplaceOutcome::Conflict);
            }
            fields[i].bits = bits;
            self.writes.set(self.writes.get() + 1);
            Ok(ReplaceOutcome::Applied)
        }
    }
    #[test]
    fn camera_profile_preserves_max_policy_and_foreign_later_values() {
        let access = Memory::new(4.1, 1.42);
        let mut profile = CameraProfile::default();
        // SAFETY: Original owned synthetic parameter session only.
        unsafe {
            profile.apply(&access, &[0, 0, 1], 7.0, 2.6).unwrap();
        }
        assert_eq!(profile.pending(), 2);
        assert_eq!(access.writes.get(), 2);
        // Same ID is not re-applied during subsequent maintenance.
        access.fields.borrow_mut()[0].bits = 8.5f32.to_bits();
        // SAFETY: Same original isolated storage; foreign value emulates another owner's later edit.
        unsafe {
            profile.apply(&access, &[0], 10.0, 4.0).unwrap();
            profile.restore(&access).unwrap();
        }
        assert_eq!(f32::from_bits(access.fields.borrow()[0].bits), 8.5);
        assert_eq!(f32::from_bits(access.fields.borrow()[1].bits), 1.42);
        assert_eq!(profile.pending(), 0);
    }
    #[test]
    fn replaced_resource_and_signed_zero_are_preserved_by_exact_restore() {
        let access = Memory::new(-0.0, 1.0);
        let mut profile = CameraProfile::default();
        // SAFETY: Original isolated storage, no live parameter access.
        unsafe {
            profile.apply(&access, &[0], 7.0, 2.0).unwrap();
        }
        access.fields.borrow_mut()[0].bits = 0.0f32.to_bits();
        access.fields.borrow_mut()[1].identity.epoch += 1;
        // SAFETY: Same isolated synthetic resource reload.
        unsafe {
            profile.restore(&access).unwrap();
        }
        assert_eq!(access.fields.borrow()[0].bits, 0.0f32.to_bits());
        assert_eq!(f32::from_bits(access.fields.borrow()[1].bits), 2.0);
        assert_eq!(profile.pending(), 0);
    }
    #[test]
    fn partial_apply_retains_the_actual_first_store() {
        let access = Memory::new(4.0, 1.0);
        let mut profile = CameraProfile::default();
        access.fail_height.set(true);
        // SAFETY: Original isolated storage with an injected failure before the second store.
        assert!(unsafe { profile.apply(&access, &[0], 7.0, 2.0) }.is_err());
        assert_eq!(profile.pending(), 1);
        assert_eq!(access.writes.get(), 1);
        // SAFETY: First owned field remains valid and can be conditionally restored.
        unsafe {
            profile.restore(&access).unwrap();
        }
        assert_eq!(f32::from_bits(access.fields.borrow()[0].bits), 4.0);
        assert_eq!(profile.pending(), 0);
    }
    #[test]
    fn failed_restore_retains_the_failed_field_and_does_not_repeat_completed_restores() {
        let access = Memory::new(4.0, 1.0);
        let mut profile = CameraProfile::default();
        // SAFETY: Original owned synthetic rows only.
        unsafe {
            profile.apply(&access, &[0], 7.0, 2.0).unwrap();
        }
        access.fail_height.set(true);
        // SAFETY: Simulated failure before the first reverse store; no game state.
        assert!(unsafe { profile.restore(&access) }.is_err());
        assert_eq!(profile.pending(), 1);
        assert_eq!(access.writes.get(), 3);
        access.fail_height.set(false);
        // SAFETY: Same retained original row lifetime after synthetic recovery.
        unsafe {
            profile.restore(&access).unwrap();
        }
        assert_eq!(profile.pending(), 0);
        assert_eq!(access.writes.get(), 4);
        assert_eq!(access.fields.borrow()[0].bits, 4.0f32.to_bits());
        assert_eq!(access.fields.borrow()[1].bits, 1.0f32.to_bits());
    }
    #[test]
    fn zero_distance_skips_and_smaller_values_do_not_lower_existing_rows() {
        let access = Memory::new(8.0, 3.0);
        let mut profile = CameraProfile::default();
        // SAFETY: Original isolated parameter storage.
        unsafe {
            profile.apply(&access, &[0], 0.0, 10.0).unwrap();
            profile.apply(&access, &[0], 7.0, 2.0).unwrap();
        }
        assert_eq!(f32::from_bits(access.fields.borrow()[0].bits), 8.0);
        assert_eq!(f32::from_bits(access.fields.borrow()[1].bits), 3.0);
        assert_eq!(profile.pending(), 0);
    }
}

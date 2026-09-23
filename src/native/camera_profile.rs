//! Reversible runtime camera parameter overlay, scoped to the owned form.
//! Never changes regulation.bin. Reloaded rows and subsequent edits are preserved.
use super::*;
use eldenring::{cs::LockCamParam, param::LOCK_CAM_PARAM_ST};
#[derive(Clone, Copy)]
struct Values {
    distance: f32,
    height: f32,
}
impl Values {
    fn read(row: &LOCK_CAM_PARAM_ST) -> Self {
        Self {
            distance: row.cam_dist_target(),
            height: row.chr_org_offset_y(),
        }
    }
}
struct Saved {
    id: u32,
    address: usize,
    before: Values,
    after: Values,
}
#[derive(Default)]
pub(super) struct CameraProfile {
    saved: Vec<Saved>,
}
fn restore_value(current: f32, after: f32, before: f32) -> f32 {
    if current.to_bits() == after.to_bits() {
        before
    } else {
        current
    }
}
impl CameraProfile {
    pub(super) fn apply(&mut self, ids: &[u32], distance: f32, height: f32) -> Result<(), String> {
        if distance == 0.0 {
            return Ok(());
        }
        let repo = unsafe { SoloParamRepository::instance_mut() }.map_err(|e| e.to_string())?;
        for &id in ids {
            if self.saved.iter().any(|p| p.id == id) {
                continue;
            }
            let Some(row) = repo.get_mut::<LockCamParam>(id) else {
                continue;
            };
            let address = row as *mut _ as usize;
            checked_object(address, size_of::<LOCK_CAM_PARAM_ST>())?;
            let before = Values::read(row);
            if !before.distance.is_finite() || !before.height.is_finite() {
                return Err("invalid camera parameter row".into());
            }
            let after = Values {
                distance: before.distance.max(distance),
                height: before.height.max(height),
            };
            self.saved.push(Saved {
                id,
                address,
                before,
                after,
            });
            row.set_cam_dist_target(after.distance);
            row.set_chr_org_offset_y(after.height);
            log(format!(
                "camera profile row={id} distance={:.2}->{:.2} height={:.2}->{:.2}",
                before.distance, after.distance, before.height, after.height
            ));
        }
        Ok(())
    }
    pub(super) fn restore(&mut self) -> Result<(), String> {
        if self.saved.is_empty() {
            return Ok(());
        }
        let repo = unsafe { SoloParamRepository::instance_mut() }.map_err(|e| e.to_string())?;
        for p in &self.saved {
            if let Some(row) = repo.get_mut::<LockCamParam>(p.id) {
                if row as *mut _ as usize != p.address {
                    continue;
                }
                checked_object(p.address, size_of::<LOCK_CAM_PARAM_ST>())?;
                row.set_cam_dist_target(restore_value(
                    row.cam_dist_target(),
                    p.after.distance,
                    p.before.distance,
                ));
                row.set_chr_org_offset_y(restore_value(
                    row.chr_org_offset_y(),
                    p.after.height,
                    p.before.height,
                ));
            }
        }
        self.saved.clear();
        log("camera profile restored (later edits preserved)");
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn camera_restore_keeps_other_mods_later_value() {
        assert_eq!(restore_value(7.0, 7.0, 4.1), 4.1);
        assert_eq!(restore_value(8.5, 7.0, 4.1), 8.5);
        assert_eq!(restore_value(2.6, 2.6, 1.42), 1.42);
    }
}

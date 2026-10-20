//! One health pool for the validated player/owned-beast pair. HP is gameplay
//! state, not a rollback patch. Callers validate both identities before access.
use eldenring::cs::CSChrDataModule;
use std::mem::offset_of;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Vitals {
    hp: i32,
    max: i32,
    uncapped: i32,
    base: i32,
}

fn read_vitals(address: usize) -> Result<Vitals, String> {
    let bytes = super::read(address + offset_of!(CSChrDataModule, hp), 16)?;
    let value = |i| i32::from_ne_bytes(bytes[i..i + 4].try_into().unwrap());
    let vitals = Vitals {
        hp: value(0),
        max: value(4),
        uncapped: value(8),
        base: value(12),
    };
    if vitals.max <= 0 || vitals.uncapped <= 0 || vitals.base <= 0 {
        return Err("invalid shared health maximum/base".into());
    }
    Ok(vitals)
}

fn write_vitals(address: usize, vitals: Vitals) -> Result<(), String> {
    let bytes: Vec<_> = [vitals.hp, vitals.max, vitals.uncapped, vitals.base]
        .into_iter()
        .flat_map(i32::to_ne_bytes)
        .collect();
    super::write(address + offset_of!(CSChrDataModule, hp), &bytes)
}

pub(super) struct HealthLink {
    last_beast: Vitals,
    dead: bool,
    stopped: bool,
    trace_count: u8,
}

impl HealthLink {
    pub(super) fn seed(body: usize, beast: usize) -> Result<Self, String> {
        if body == beast {
            return Err("shared health owners alias".into());
        }
        let vitals = read_vitals(body)?;
        if vitals.hp <= 0 {
            return Err("player died before health link".into());
        }
        let vitals = Vitals {
            hp: vitals.hp.min(vitals.max),
            ..vitals
        };
        write_vitals(beast, vitals)?;
        super::log(format!("health linked hp={} max={}", vitals.hp, vitals.max));
        Ok(Self {
            last_beast: vitals,
            dead: false,
            stopped: false,
            trace_count: 0,
        })
    }

    /// Returns true when shared HP is zero. A failed write permanently stops
    /// this link, so emergency cleanup cannot retry and apply a debit twice.
    pub(super) fn sync(&mut self, body: usize, beast: usize) -> Result<bool, String> {
        if self.stopped {
            return Err("shared health write failed; link stopped".into());
        }
        if body == beast {
            return Err("shared health owners alias".into());
        }
        let player = read_vitals(body)?;
        let observed = read_vitals(beast)?;
        // Max-HP reductions can clamp both objects. Do not debit that clamp a
        // second time; only merge the beast's change beyond its new ceiling.
        let baseline = self.last_beast.hp.min(observed.max);
        let delta = i64::from(observed.hp) - i64::from(baseline);
        self.dead |= player.hp <= 0 || observed.hp <= 0;
        let hp = if self.dead {
            0
        } else {
            (i64::from(player.hp) + delta).clamp(0, i64::from(player.max)) as i32
        };
        self.dead |= hp == 0;
        let shared = Vitals { hp, ..player };
        if hp != player.hp
            && let Err(error) =
                super::write(body + offset_of!(CSChrDataModule, hp), &hp.to_ne_bytes())
        {
            self.stopped = true;
            return Err(error);
        }
        if shared != observed
            && let Err(error) = write_vitals(beast, shared)
        {
            self.stopped = true;
            return Err(error);
        }
        if self.trace_count < 64 && (delta != 0 || shared != self.last_beast) {
            self.trace_count += 1;
            super::log(format!(
                "health sync body={} beast={} delta={delta} shared={hp}/{} dead={}",
                player.hp, observed.hp, player.max, self.dead
            ));
        }
        self.last_beast = shared;
        Ok(self.dead)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Byte storage only: never construct a CSChrDataModule with null NonNull
    // members. The actual native read/write helpers operate on these pages.
    struct Storage(Vec<u64>);
    impl Storage {
        fn new(hp: i32, max: i32) -> Self {
            let result = Self(vec![0; 0x200 / 8]);
            write_vitals(
                result.address(),
                Vitals {
                    hp,
                    max,
                    uncapped: max,
                    base: max,
                },
            )
            .unwrap();
            result
        }
        fn address(&self) -> usize {
            self.0.as_ptr() as usize
        }
        fn vitals(&self) -> Vitals {
            read_vitals(self.address()).unwrap()
        }
        fn hp(&self, hp: i32) {
            write_vitals(
                self.address(),
                Vitals {
                    hp,
                    ..self.vitals()
                },
            )
            .unwrap();
        }
    }
    #[test]
    fn half_health_entry_damage_healing_exit_and_three_reentries() {
        let body = Storage::new(600, 1500);
        for _ in 0..3 {
            let beast = Storage::new(2000, 2000);
            let start = body.vitals().hp;
            let mut link = HealthLink::seed(body.address(), beast.address()).unwrap();
            assert_eq!(beast.vitals(), body.vitals());
            beast.hp(start - 100);
            assert!(!link.sync(body.address(), beast.address()).unwrap());
            assert_eq!(body.vitals().hp, start - 100);
            for _ in 0..60 {
                link.sync(body.address(), beast.address()).unwrap();
            }
            assert_eq!(body.vitals().hp, start - 100, "same injury debited twice");
            beast.hp(start - 50);
            link.sync(body.address(), beast.address()).unwrap();
            assert_eq!(body.vitals().hp, start - 50);
            // Final restore-time sync is deliberately the same operation.
            link.sync(body.address(), beast.address()).unwrap();
            assert_eq!(body.vitals().hp, start - 50, "exit restored initial HP");
        }
    }
    #[test]
    fn concurrent_player_healing_and_beast_damage_are_both_kept() {
        let body = Storage::new(600, 1500);
        let beast = Storage::new(10, 10);
        let mut link = HealthLink::seed(body.address(), beast.address()).unwrap();
        body.hp(650);
        beast.hp(500);
        link.sync(body.address(), beast.address()).unwrap();
        assert_eq!(body.vitals().hp, 550);
        assert_eq!(beast.vitals().hp, 550);
        body.hp(530); // poison on the protected body remains real damage
        link.sync(body.address(), beast.address()).unwrap();
        assert_eq!(beast.vitals().hp, 530);
    }
    #[test]
    fn max_hp_clamp_is_not_charged_twice_and_overhealing_is_capped() {
        let body = Storage::new(900, 1000);
        let beast = Storage::new(10, 10);
        let mut link = HealthLink::seed(body.address(), beast.address()).unwrap();
        let lowered = Vitals {
            hp: 500,
            max: 500,
            uncapped: 500,
            base: 500,
        };
        write_vitals(body.address(), lowered).unwrap();
        write_vitals(beast.address(), lowered).unwrap();
        link.sync(body.address(), beast.address()).unwrap();
        assert_eq!(body.vitals().hp, 500);
        beast.hp(700);
        link.sync(body.address(), beast.address()).unwrap();
        assert_eq!(beast.vitals().hp, 500);
    }
    #[test]
    fn either_owner_death_is_sticky_even_with_same_frame_healing() {
        for killed_beast in [true, false] {
            let body = Storage::new(600, 1500);
            let beast = Storage::new(10, 10);
            let mut link = HealthLink::seed(body.address(), beast.address()).unwrap();
            if killed_beast {
                beast.hp(0);
                body.hp(900);
            } else {
                body.hp(0);
                beast.hp(900);
            }
            assert!(link.sync(body.address(), beast.address()).unwrap());
            assert_eq!((body.vitals().hp, beast.vitals().hp), (0, 0));
            body.hp(500);
            beast.hp(500);
            assert!(link.sync(body.address(), beast.address()).unwrap());
            assert_eq!((body.vitals().hp, beast.vitals().hp), (0, 0));
        }
    }
    #[test]
    fn rejects_alias_invalid_max_and_dead_entry() {
        let body = Storage::new(0, 100);
        let beast = Storage::new(50, 100);
        assert!(HealthLink::seed(body.address(), body.address()).is_err());
        assert!(HealthLink::seed(body.address(), beast.address()).is_err());
        body.hp(50);
        super::super::write(
            body.address() + offset_of!(CSChrDataModule, max_hp),
            &0_i32.to_ne_bytes(),
        )
        .unwrap();
        assert!(HealthLink::seed(body.address(), beast.address()).is_err());
        assert_eq!(beast.vitals().hp, 50);
    }
}

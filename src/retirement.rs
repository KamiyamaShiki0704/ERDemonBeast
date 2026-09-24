//! Local adaptation of pinned Summon retirement; original retained in vendor/.
//! Detach first, observe native readers drain, then destroy only from FrameEnd.
use eldenring::cs::{ChrIns, FieldInsHandle, WorldChrMan};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use windows::Win32::System::{Diagnostics::Debug::ReadProcessMemory, Threading::GetCurrentProcess};

pub(crate) const TASK_OFFSETS: [usize; 6] = [0x418, 0x448, 0x478, 0x4a8, 0x4d8, 0x508];
pub(crate) const VECTOR_OFFSETS: [usize; 3] = [0x1f1b0, 0x1f1d0, 0x1f200];

#[derive(Default)]
pub(crate) struct DrainGate {
    quiet_frames: u8,
}
impl DrainGate {
    fn observe(&mut self, references: Option<usize>, proxies_clear: bool) -> bool {
        if references != Some(0) || !proxies_clear {
            self.quiet_frames = 0;
            return false;
        }
        self.quiet_frames = self.quiet_frames.saturating_add(1);
        self.quiet_frames >= 2
    }
}

pub(crate) struct Retired {
    pub world: usize,
    pub player: usize,
    pub character: usize,
    pub owner: usize,
    pub destructor: usize,
    pub handle: FieldInsHandle,
    pub gate: DrainGate,
    pub last_observation: Option<(Option<usize>, bool)>,
}
static QUEUE: Mutex<Vec<Retired>> = Mutex::new(Vec::new());
static LAST_DIAGNOSTIC: Mutex<Option<Instant>> = Mutex::new(None);

pub(crate) fn pending() -> bool {
    QUEUE.lock().map_or(true, |q| !q.is_empty())
}
pub(crate) fn enqueue(value: Retired) {
    if let Ok(mut last) = LAST_DIAGNOSTIC.lock() {
        *last = None;
    }
    // Poisoning is not recoverable by freeing a potentially live engine object.
    if let Ok(mut queue) = QUEUE.lock() {
        queue.push(value);
    }
}

/// World transitions can invalidate resources even when this allocation was
/// detached. Leak quarantined storage rather than invoking a stale destructor.
pub(crate) fn quarantine_all(reason: &str) {
    if let Ok(mut queue) = QUEUE.lock()
        && !queue.is_empty()
    {
        crate::log_line(format_args!(
            "summon retirement: phase=quarantined count={} reason={reason}; allocations retained until process exit",
            queue.len()
        ));
        queue.clear();
    }
}

pub(crate) fn player_address(world: &WorldChrMan) -> usize {
    world
        .main_player
        .as_ref()
        .map_or(0, |p| p.as_ref() as *const _ as usize)
}

fn read_words(address: usize, count: usize) -> Option<Vec<usize>> {
    let size = count.checked_mul(8)?;
    if address == 0 || count > 131072 || address.checked_add(size).is_none() {
        return None;
    }
    let mut words = vec![0_usize; count];
    let mut copied = 0;
    // Copy the full span, including valid spans crossing VirtualQuery regions.
    // Never dereference engine storage after a separate readability check.
    let result = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            address as *const _,
            words.as_mut_ptr().cast(),
            size,
            Some(&mut copied),
        )
    };
    (result.is_ok() && copied == size).then_some(words)
}

fn word(address: usize) -> Option<usize> {
    let mut value = 0_usize;
    let mut copied = 0;
    let result = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            address as *const _,
            (&mut value as *mut usize).cast(),
            8,
            Some(&mut copied),
        )
    };
    (result.is_ok() && copied == 8).then_some(value)
}

#[cfg(test)]
fn count_vector(world: usize, offset: usize, stride: usize, pointer: usize) -> Option<usize> {
    scan_vector(world, offset, stride, pointer).ok()
}

fn scan_vector(
    world: usize,
    offset: usize,
    stride: usize,
    pointer: usize,
) -> Result<usize, String> {
    let header = read_words(world + offset + 8, 3)
        .ok_or_else(|| format!("vector+{offset:#x}:header-unreadable"))?;
    let (first, last, end) = (header[0], header[1], header[2]);
    let invalid = || {
        format!(
            "vector+{offset:#x}:invalid-bounds first={first:#x} last={last:#x} end={end:#x} stride={stride}"
        )
    };
    let bytes = last.checked_sub(first).ok_or_else(invalid)?;
    if last > end || bytes % stride != 0 || bytes / stride > 65536 {
        return Err(invalid());
    }
    let count = if bytes == 0 {
        0
    } else {
        let values = read_words(first, bytes / 8).ok_or_else(|| {
            format!("vector+{offset:#x}:data-unreadable first={first:#x} bytes={bytes}")
        })?;
        values
            .iter()
            .step_by(stride / 8)
            .filter(|value| **value == pointer)
            .count()
    };
    if read_words(world + offset + 8, 3).as_ref() != Some(&header) {
        return Err(format!("vector+{offset:#x}:header-changed"));
    }
    Ok(count)
}

fn references(world: usize, pointer: usize) -> Result<usize, String> {
    let mut count = 0;
    for (offset, stride) in VECTOR_OFFSETS.into_iter().zip([8, 16, 8]) {
        count += scan_vector(world, offset, stride, pointer)?;
    }
    // WorldChrMan_PostPhysics also traverses this (character,next) chain.
    let mut node = word(world + 0x1e6a8).ok_or("chain+0x1e6a8:head-unreadable")?;
    let head = node;
    let mut visited = 0;
    while node != 0 {
        if visited >= 65536 {
            return Err(format!("chain+0x1e6a8:limit-or-cycle node={node:#x}"));
        }
        let values = read_words(node, 2).ok_or_else(|| {
            format!("chain+0x1e6a8:node-unreadable node={node:#x} visited={visited}")
        })?;
        count += usize::from(values[0] == pointer);
        node = values[1];
        visited += 1;
    }
    if word(world + 0x1e6a8) != Some(head) {
        return Err("chain+0x1e6a8:head-changed".into());
    }
    Ok(count)
}

/// Must only be called by the FrameEnd callback, after native frame work.
/// No shared engine list is edited; the engine rebuilds lists after detachment.
pub(crate) fn poll(world: &mut WorldChrMan) {
    let Ok(mut queue) = QUEUE.lock() else {
        return;
    };
    let current_world = world as *mut _ as usize;
    let current_player = player_address(world);
    let mut i = 0;
    while i < queue.len() {
        let item = &mut queue[i];
        // Do not dereference an orphan after a world/player transition. Keep its
        // storage quarantined rather than risking a second free during teardown.
        if item.world != current_world || item.player != current_player || current_player == 0 {
            crate::log_line(format_args!(
                "summon retirement: handle={} phase=quarantined reason=world-or-player-changed; allocation retained until process exit",
                item.handle
            ));
            queue.swap_remove(i);
            continue;
        }
        let scan = references(current_world, item.character);
        let refs = scan.as_ref().ok().copied();
        let proxies_clear = TASK_OFFSETS
            .iter()
            .all(|offset| word(item.character + offset + 0x10) == Some(0));
        let observation = (refs, proxies_clear);
        let log_due = LAST_DIAGNOSTIC.lock().is_ok_and(|mut last| {
            if last.is_none_or(|time| time.elapsed() >= Duration::from_secs(2)) {
                *last = Some(Instant::now());
                true
            } else {
                false
            }
        });
        if log_due {
            crate::log_line(format_args!(
                "summon retirement: handle={} phase=draining references={refs:?} task_proxies_clear={proxies_clear} read_error={:?}",
                item.handle,
                scan.as_ref().err()
            ));
            item.last_observation = Some(observation);
        }
        if !item.gate.observe(refs, proxies_clear) {
            i += 1;
            continue;
        }
        let item = queue.swap_remove(i);
        crate::log_line(format_args!(
            "summon retirement: handle={} phase=before-destructor character={:#x} quiet_frame_ends=2",
            item.handle, item.character
        ));
        // The lookup and both vtables were verified before detaching ownership.
        let destructor: unsafe extern "C" fn(*mut ChrIns, u32) =
            unsafe { std::mem::transmute(item.destructor) };
        let owner = unsafe { &*(item.owner as *const eldenring::dlkr::DLAllocator) };
        crate::chr_lifecycle::teardown_with_owner(
            |delete| unsafe { destructor(item.character as *mut ChrIns, delete as u32) },
            || (owner.vftable.deallocate)(owner, item.character as *const u8),
        );
        // The slot was detached earlier and may now be reused; NEVER clear it here.
        crate::log_line(format_args!(
            "summon removed: handle={} outcome=destroyed retirement=frame-end-drained",
            item.handle
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_reader_blocks_free_until_two_quiet_frame_ends() {
        let mut gate = DrainGate::default();
        assert!(!gate.observe(Some(1), true)); // exact failure pattern: old list owns pointer
        assert!(!gate.observe(Some(0), false)); // still registered task
        assert!(!gate.observe(Some(0), true));
        assert!(!gate.observe(None, true)); // failed observation resets progress
        assert!(!gate.observe(Some(0), true));
        assert!(gate.observe(Some(0), true));
    }
    #[test]
    fn bound_world_vectors_match_native_consumers() {
        assert_eq!(
            std::mem::offset_of!(WorldChrMan, chr_inses_by_distance),
            0x1f1d0
        );
        assert_eq!(
            std::mem::offset_of!(WorldChrMan, chr_inses_by_update_priority),
            0x1f200
        );
        assert_eq!(
            std::mem::size_of::<eldenring::cs::ChrInsDistanceEntry>(),
            16
        );
    }
}

#[cfg(test)]
mod boundary_regression {
    use super::*;
    use windows::Win32::System::Memory::{
        MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_PROTECTION_FLAGS, PAGE_READONLY, PAGE_READWRITE,
        VirtualAlloc, VirtualFree, VirtualProtect,
    };

    #[test]
    fn readable_vector_across_regions_must_allow_retirement() {
        // A valid vector spans two readable VirtualQuery regions, e.g. pages
        // with different protections. Both entries no longer reference beast.
        let memory = unsafe { VirtualAlloc(None, 8192, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) };
        assert!(!memory.is_null());
        let base = memory as usize;
        let first = base + 4096 - 8;
        unsafe {
            (first as *mut usize).write(0);
            ((first + 8) as *mut usize).write(0);
        }
        let mut old = PAGE_PROTECTION_FLAGS::default();
        unsafe {
            VirtualProtect((base + 4096) as *const _, 4096, PAGE_READONLY, &mut old).unwrap();
        }
        let vector = [0_usize, first, first + 16, first + 16];
        let mut gate = DrainGate::default();
        let mut freed = false;
        for _ in 0..180 {
            freed |= gate.observe(count_vector(vector.as_ptr() as usize, 0, 8, 0x1234), true);
        }
        unsafe {
            VirtualFree(memory, 0, MEM_RELEASE).unwrap();
        }
        assert!(
            freed,
            "F7 cleanup remained blocked by a fully readable zero-reference vector"
        );
    }

    #[test]
    fn unreadable_and_live_references_still_block_release() {
        let invalid = [0_usize, 8, 16, 16];
        let error = scan_vector(invalid.as_ptr() as usize, 0, 8, 0x1234).unwrap_err();
        assert!(error.contains("data-unreadable"));
        let live = [0x1234_usize];
        let first = live.as_ptr() as usize;
        let header = [0, first, first + 8, first + 8];
        let mut gate = DrainGate::default();
        assert!(!gate.observe(Some(0), true));
        assert!(!gate.observe(count_vector(invalid.as_ptr() as usize, 0, 8, 0x1234), true));
        assert!(!gate.observe(count_vector(header.as_ptr() as usize, 0, 8, 0x1234), true));
        assert!(!gate.observe(Some(0), true));
        assert!(gate.observe(Some(0), true));
    }
}

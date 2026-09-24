//! Bounded copy-based traversal of the pinned SpecialEffectEntry list.
use std::collections::BTreeSet;
pub(super) fn collect(
    mut node: usize,
    mut read: impl FnMut(usize) -> Result<(i32, usize), String>,
) -> Result<BTreeSet<i32>, String> {
    let mut seen = BTreeSet::new();
    let mut ids = BTreeSet::new();
    while node != 0 {
        if seen.len() >= 1024 || !seen.insert(node) {
            return Err("SpEffect list cycle or limit".into());
        }
        let (id, next) = read(node)?;
        if id > 0 {
            ids.insert(id);
        }
        node = next;
    }
    Ok(ids)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_list_duplicate_ids_and_faults() {
        let ids = collect(8, |n| {
            Ok(match n {
                8 => (10, 16),
                16 => (10, 24),
                24 => (20, 0),
                _ => panic!(),
            })
        })
        .unwrap();
        assert_eq!(ids, BTreeSet::from([10, 20]));
        assert!(collect(8, |_| Ok((10, 8))).is_err());
        assert!(collect(8, |_| Err("unreadable".into())).is_err());
        assert!(collect(8, |n| Ok((10, n + 8))).is_err());
    }
}

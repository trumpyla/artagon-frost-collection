//! Owning parallel traversal preserves order and non-Clone ownership.
#![cfg(feature = "rayon-1")]
#![allow(
    clippy::mutable_key_type,
    reason = "drop counters do not participate in member identity"
)]

use frost::adaptive::OrderedRepr;
use frost::prelude::*;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

#[derive(Debug)]
struct OwnedMember {
    id: usize,
    drops: Arc<[AtomicUsize]>,
}

impl Drop for OwnedMember {
    fn drop(&mut self) {
        self.drops[self.id].fetch_add(1, AtomicOrdering::Relaxed);
    }
}

impl PartialEq for OwnedMember {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for OwnedMember {}

impl PartialOrd for OwnedMember {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OwnedMember {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

fn counters(count: usize) -> Arc<[AtomicUsize]> {
    (0..count).map(|_| AtomicUsize::new(0)).collect()
}

#[test]
fn owning_parallel_ordered_set_keeps_order_and_drops_each_member_once() {
    let drops = counters(16);
    let members: FrozenOrderedSet<_> = (0..16)
        .rev()
        .map(|id| OwnedMember {
            id,
            drops: Arc::clone(&drops),
        })
        .collect();
    let parallel = members.into_par_iter();
    assert_eq!(parallel.len(), 16);
    let result: Vec<_> = parallel.collect();
    assert_eq!(
        result.iter().map(|member| member.id).collect::<Vec<_>>(),
        (0..16).collect::<Vec<_>>()
    );
    assert!(
        drops
            .iter()
            .all(|count| count.load(AtomicOrdering::Relaxed) == 0)
    );
    drop(result);
    assert!(
        drops
            .iter()
            .all(|count| count.load(AtomicOrdering::Relaxed) == 1)
    );
}

#[test]
fn owning_parallel_adaptive_ordered_map_preserves_both_representations() {
    for threshold in [0, 32] {
        let drops = counters(16);
        let mut map = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(threshold));
        for id in (0..16).rev() {
            map.insert(
                id,
                OwnedMember {
                    id,
                    drops: Arc::clone(&drops),
                },
            );
        }
        assert_eq!(
            map.representation(),
            if threshold == 0 {
                OrderedRepr::Tree
            } else {
                OrderedRepr::Sorted
            }
        );
        let parallel = map.into_par_iter();
        assert_eq!(parallel.len(), 16);
        let result: Vec<_> = parallel.collect();
        assert_eq!(
            result
                .iter()
                .map(|(key, value)| (*key, value.id))
                .collect::<Vec<_>>(),
            (0..16).map(|id| (id, id)).collect::<Vec<_>>()
        );
        assert!(
            drops
                .iter()
                .all(|count| count.load(AtomicOrdering::Relaxed) == 0)
        );
        drop(result);
        assert!(
            drops
                .iter()
                .all(|count| count.load(AtomicOrdering::Relaxed) == 1)
        );
    }
}

#[test]
fn truncated_parallel_set_traversal_drops_the_unvisited_suffix() {
    let drops = counters(16);
    let members: FrozenOrderedSet<_> = (0..16)
        .map(|id| OwnedMember {
            id,
            drops: Arc::clone(&drops),
        })
        .collect();
    let result: Vec<_> = members.into_par_iter().take(5).collect();
    assert_eq!(
        result.iter().map(|member| member.id).collect::<Vec<_>>(),
        (0..5).collect::<Vec<_>>()
    );
    for (id, count) in drops.iter().enumerate() {
        assert_eq!(count.load(AtomicOrdering::Relaxed), usize::from(id >= 5));
    }
    drop(result);
    assert!(
        drops
            .iter()
            .all(|count| count.load(AtomicOrdering::Relaxed) == 1)
    );
}

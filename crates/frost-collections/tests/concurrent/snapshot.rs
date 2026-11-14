use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

use frost::concurrent::{SnapshotGuard, SnapshotStore, SnapshotValue};
use frost::family::FrozenVec;

#[derive(Debug)]
struct DropProbe {
    value: usize,
    drops: Arc<AtomicUsize>,
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
    }
}

fn tracked_version(value: usize, drops: &Arc<AtomicUsize>) -> FrozenVec<DropProbe> {
    FrozenVec::from([DropProbe {
        value,
        drops: Arc::clone(drops),
    }])
}

#[test]
fn publication_preserves_owned_versions() {
    let store = SnapshotStore::new(FrozenVec::from([1, 2]));
    let original = store.load();
    let replaced = store.publish(FrozenVec::from([3, 4]));
    assert!(Arc::ptr_eq(&original, &replaced));
    assert_eq!(original.as_slice(), &[1, 2]);
    let latest = store.load();
    assert_eq!(latest.as_slice(), &[3, 4]);
    assert!(!Arc::ptr_eq(&original, &latest));
    let extracted = store.into_inner();
    assert!(Arc::ptr_eq(&latest, &extracted));
    assert_eq!(original.as_slice(), &[1, 2]);
    let empty = SnapshotStore::<FrozenVec<usize>>::default();
    assert_eq!(empty.load().as_slice(), &[] as &[usize]);
}

#[test]
fn guard_and_peek_observe_the_selected_version() {
    let store = SnapshotStore::new(FrozenVec::from([1, 2, 3]));
    let original = store.load();
    let guard: SnapshotGuard<FrozenVec<i32>> = store.load_guard();
    assert!(std::ptr::eq(guard.as_ref(), original.as_ref()));

    let calls = Cell::new(0);
    assert_eq!(
        store.peek(|version| {
            calls.set(calls.get() + 1);
            assert!(std::ptr::eq(version, original.as_ref()));
            version.as_slice().iter().sum::<i32>()
        }),
        6
    );
    assert_eq!(calls.get(), 1);

    store.publish(FrozenVec::from([4, 5, 6]));
    assert_eq!(guard.as_slice(), &[1, 2, 3]);
    let retained = guard.into_owned();
    assert!(Arc::ptr_eq(&original, &retained));
    assert_eq!(store.peek(|version| version.as_slice()[0]), 4);
}

#[test]
fn many_guards_remain_valid_after_publication_and_store_drop() {
    let store = SnapshotStore::new(FrozenVec::from([17, 23]));
    let original = store.load();
    // More than the pinned publisher's fast slots exercises its owned fallback
    // without making the slot count part of Frost's public contract.
    let guards: Vec<_> = (0..64).map(|_| store.load_guard()).collect();
    store.publish(FrozenVec::from([29, 31]));
    drop(store);

    for guard in guards {
        assert_eq!(guard.as_slice(), &[17, 23]);
        assert!(Arc::ptr_eq(&original, &guard.into_owned()));
    }
}

#[test]
fn conditional_publication_compares_identity_and_returns_displaced_version() {
    let store = SnapshotStore::new(FrozenVec::from([7]));
    let stale = store.load();
    let first_displaced = store.publish(FrozenVec::from([7]));
    let current = store.load();
    assert!(Arc::ptr_eq(&stale, &first_displaced));
    assert_eq!(stale.as_slice(), current.as_slice());
    assert!(!Arc::ptr_eq(&stale, &current));

    let (rejected, observed) = store
        .publish_if_current(&stale, FrozenVec::from([8]))
        .unwrap_err();
    assert_eq!(rejected.as_slice(), &[8]);
    assert!(Arc::ptr_eq(&current, &observed));
    assert!(Arc::ptr_eq(&current, &store.load()));

    let displaced = store.publish_if_current(&current, rejected).unwrap();
    assert!(Arc::ptr_eq(&current, &displaced));
    assert_eq!(store.peek(|version| version.as_slice()[0]), 8);
    assert_eq!(stale.as_slice(), &[7]);
}

#[test]
fn rejected_non_clone_candidate_keeps_its_storage_and_is_dropped_once() {
    let old_drops = Arc::new(AtomicUsize::new(0));
    let current_drops = Arc::new(AtomicUsize::new(0));
    let candidate_drops = Arc::new(AtomicUsize::new(0));
    let store = SnapshotStore::new(tracked_version(0, &old_drops));
    let stale = store.load();
    store.publish(tracked_version(1, &current_drops));

    let candidate = tracked_version(2, &candidate_drops);
    let candidate_storage = candidate.as_slice().as_ptr();
    let (rejected, observed) = store.publish_if_current(&stale, candidate).unwrap_err();
    assert_eq!(rejected.as_slice().as_ptr(), candidate_storage);
    assert_eq!(rejected.as_slice()[0].value, 2);
    assert_eq!(observed.as_slice()[0].value, 1);
    assert_eq!(candidate_drops.load(Ordering::Relaxed), 0);

    drop(rejected);
    assert_eq!(candidate_drops.load(Ordering::Relaxed), 1);
    drop(store);
    assert_eq!(current_drops.load(Ordering::Relaxed), 0);
    drop(observed);
    assert_eq!(current_drops.load(Ordering::Relaxed), 1);
    assert_eq!(old_drops.load(Ordering::Relaxed), 0);
    drop(stale);
    assert_eq!(old_drops.load(Ordering::Relaxed), 1);
}

#[test]
fn successful_conditional_publication_transfers_non_clone_ownership_once() {
    let old_drops = Arc::new(AtomicUsize::new(0));
    let next_drops = Arc::new(AtomicUsize::new(0));
    let store = SnapshotStore::new(tracked_version(0, &old_drops));
    let expected = store.load();
    let previous = store
        .publish_if_current(&expected, tracked_version(1, &next_drops))
        .unwrap();
    assert!(Arc::ptr_eq(&expected, &previous));
    assert_eq!(old_drops.load(Ordering::Relaxed), 0);
    assert_eq!(next_drops.load(Ordering::Relaxed), 0);
    drop(expected);
    assert_eq!(old_drops.load(Ordering::Relaxed), 0);
    drop(previous);
    assert_eq!(old_drops.load(Ordering::Relaxed), 1);
    assert_eq!(next_drops.load(Ordering::Relaxed), 0);
    drop(store);
    assert_eq!(next_drops.load(Ordering::Relaxed), 1);
}

#[test]
fn update_retries_after_a_forced_conflict_and_returns_its_actual_predecessor() {
    let store = Arc::new(SnapshotStore::new(FrozenVec::from([0_usize])));
    let deriving = Arc::new(Barrier::new(2));
    let published = Arc::new(Barrier::new(2));
    let original = store.load();
    let writer = {
        let store = Arc::clone(&store);
        let deriving = Arc::clone(&deriving);
        let published = Arc::clone(&published);
        std::thread::spawn(move || {
            let mut observations = Vec::new();
            let previous = store.update(|version| {
                let value = version.as_slice()[0];
                observations.push(value);
                if observations.len() == 1 {
                    deriving.wait();
                    published.wait();
                }
                FrozenVec::from([value + 1])
            });
            (previous, observations)
        })
    };

    deriving.wait();
    let displaced = store.publish(FrozenVec::from([10]));
    let intervening = store.load();
    published.wait();

    let (previous, observations) = writer.join().unwrap();
    assert_eq!(observations, [0, 10]);
    assert!(Arc::ptr_eq(&original, &displaced));
    assert!(Arc::ptr_eq(&intervening, &previous));
    assert_eq!(store.peek(|version| version.as_slice()[0]), 11);
    assert_eq!(original.as_slice(), &[0]);
}

#[test]
fn update_panic_leaves_that_attempt_unpublished_and_the_store_usable() {
    let store = SnapshotStore::new(FrozenVec::from([3_usize]));
    let original = store.load();
    let calls = Cell::new(0);
    let result = catch_unwind(AssertUnwindSafe(|| {
        store.update(|_| {
            calls.set(calls.get() + 1);
            panic!("intentional derivation panic");
        });
    }));
    assert!(result.is_err());
    assert_eq!(calls.get(), 1);
    assert!(Arc::ptr_eq(&original, &store.load()));

    let previous = store.update(|version| FrozenVec::from([version.as_slice()[0] + 1]));
    assert!(Arc::ptr_eq(&original, &previous));
    assert_eq!(store.peek(|version| version.as_slice()[0]), 4);
}

#[test]
fn panic_on_a_retry_preserves_the_competing_publication() {
    let store = Arc::new(SnapshotStore::new(FrozenVec::from([0_usize])));
    let deriving = Arc::new(Barrier::new(2));
    let published = Arc::new(Barrier::new(2));
    let writer = {
        let store = Arc::clone(&store);
        let deriving = Arc::clone(&deriving);
        let published = Arc::clone(&published);
        std::thread::spawn(move || {
            let mut calls = 0;
            let result = catch_unwind(AssertUnwindSafe(|| {
                store.update(|version| {
                    calls += 1;
                    if calls == 1 {
                        deriving.wait();
                        published.wait();
                        FrozenVec::from([version.as_slice()[0] + 1])
                    } else {
                        panic!("intentional retry panic");
                    }
                });
            }));
            assert!(result.is_err());
            calls
        })
    };

    deriving.wait();
    store.publish(FrozenVec::from([10]));
    let intervening = store.load();
    published.wait();
    assert_eq!(writer.join().unwrap(), 2);
    assert!(Arc::ptr_eq(&intervening, &store.load()));
}

#[test]
fn peek_unwinding_releases_its_guard_and_calls_the_callback_once() {
    let old_drops = Arc::new(AtomicUsize::new(0));
    let next_drops = Arc::new(AtomicUsize::new(0));
    let store = SnapshotStore::new(tracked_version(0, &old_drops));
    let calls = Cell::new(0);
    let result = catch_unwind(AssertUnwindSafe(|| {
        store.peek(|version| {
            calls.set(calls.get() + 1);
            // Snapshot reads hold no live-entry lock. Publication can retire
            // this version while the callback still reads it safely.
            store.publish(tracked_version(1, &next_drops));
            assert_eq!(version.as_slice()[0].value, 0);
            assert_eq!(old_drops.load(Ordering::Relaxed), 0);
            panic!("intentional reader panic");
        });
    }));
    assert!(result.is_err());
    assert_eq!(calls.get(), 1);
    assert_eq!(old_drops.load(Ordering::Relaxed), 1);
    assert_eq!(store.peek(|version| version.as_slice()[0].value), 1);
    drop(store);
    assert_eq!(next_drops.load(Ordering::Relaxed), 1);
}

#[test]
fn competing_updates_have_a_complete_displaced_version_history() {
    const WRITERS: usize = 8;
    const UPDATES: usize = 250;
    let store = Arc::new(SnapshotStore::new(FrozenVec::from([0_usize])));
    let original = store.load();
    let barrier = Arc::new(Barrier::new(WRITERS));
    let writers: Vec<_> = (0..WRITERS)
        .map(|_| {
            let store = Arc::clone(&store);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                (0..UPDATES)
                    .map(|_| store.update(|version| FrozenVec::from([version.as_slice()[0] + 1])))
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let mut displaced: Vec<_> = writers
        .into_iter()
        .flat_map(|writer| writer.join().unwrap())
        .collect();
    displaced.sort_unstable_by_key(|version| version.as_slice()[0]);
    assert!(Arc::ptr_eq(&original, &displaced[0]));
    assert_eq!(displaced.len(), WRITERS * UPDATES);
    for (expected, version) in displaced.iter().enumerate() {
        assert_eq!(version.as_slice(), &[expected]);
    }
    assert_eq!(
        store.peek(|version| version.as_slice()[0]),
        WRITERS * UPDATES
    );
}

#[test]
fn readers_hold_one_version_across_a_synchronized_publication() {
    let store = Arc::new(SnapshotStore::new(FrozenVec::from([0, 0])));
    let loaded = Arc::new(Barrier::new(5));
    let published = Arc::new(Barrier::new(5));
    let readers: Vec<_> = (0..4)
        .map(|_| {
            let store = Arc::clone(&store);
            let loaded = Arc::clone(&loaded);
            let published = Arc::clone(&published);
            std::thread::spawn(move || {
                let held = store.load_guard();
                loaded.wait();
                published.wait();
                assert_eq!(held.as_slice(), &[0, 0]);
                assert_eq!(store.peek(|version| version.as_slice().to_vec()), [1, 1]);
            })
        })
        .collect();
    loaded.wait();
    store.publish(FrozenVec::from([1, 1]));
    published.wait();
    for reader in readers {
        reader.join().unwrap();
    }
}

#[test]
fn readers_observe_one_complete_published_version() {
    let store = Arc::new(SnapshotStore::new(FrozenVec::from([0_usize, 0])));
    let retained = store.load();
    let barrier = Arc::new(Barrier::new(5));
    let writer = {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            for generation in 1..=2_000 {
                store.publish(FrozenVec::from([generation, generation]));
            }
        })
    };
    let readers: Vec<_> = (0..4)
        .map(|_| {
            let store = Arc::clone(&store);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                for iteration in 0..4_000 {
                    let check = |snapshot: &FrozenVec<usize>| {
                        let values = snapshot.as_slice();
                        assert_eq!(values.len(), 2);
                        assert_eq!(values[0], values[1]);
                        assert!(values[0] <= 2_000);
                    };
                    match iteration % 3 {
                        0 => check(&store.load()),
                        1 => check(&store.load_guard()),
                        _ => store.peek(check),
                    }
                }
            })
        })
        .collect();
    writer.join().unwrap();
    for reader in readers {
        reader.join().unwrap();
    }
    assert_eq!(store.load().as_slice(), &[2_000, 2_000]);
    assert_eq!(retained.as_slice(), &[0, 0]);
}

#[test]
fn newly_frozen_mutable_profiles_preserve_older_published_versions() {
    use frost::adaptive::{
        AdaptiveHash, AdaptiveHashMap, AdaptiveOrdered, AdaptiveOrderedMap, HashRepr, InlineVec,
    };

    let inline: InlineVec<u8, 2> = [1, 2].into_iter().collect();
    let sequences = SnapshotStore::new(inline.freeze());
    let old_sequence = sequences.load();
    let spilled: InlineVec<u8, 2> = [3, 4, 5].into_iter().collect();
    sequences.publish(spilled.freeze());
    assert!(old_sequence.is_inline());
    assert_eq!(old_sequence.as_slice(), &[1, 2]);
    assert!(!sequences.load().is_inline());
    assert_eq!(sequences.load().as_slice(), &[3, 4, 5]);

    let mut small = AdaptiveHashMap::with_policy(AdaptiveHash::linear_up_to(2));
    small.insert(String::from("first"), 1);
    let hashes = SnapshotStore::new(small.freeze());
    let old_hash = hashes.load();
    let mut larger = (*old_hash).clone().thaw_with(AdaptiveHash::linear_up_to(2));
    larger.insert(String::from("second"), 2);
    larger.insert(String::from("third"), 3);
    hashes.publish(larger.freeze());
    assert_eq!(old_hash.representation(), HashRepr::Linear);
    assert_eq!(old_hash.get("second"), None);
    assert_eq!(hashes.load().representation(), HashRepr::Table);
    assert_eq!(hashes.load().get("third"), Some(&3));

    let mut sorted = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(2));
    sorted.insert("b", 2);
    sorted.insert("a", 1);
    let ordered = SnapshotStore::new(sorted.freeze());
    let old_ordered = ordered.load();
    let mut tree =
        AdaptiveOrderedMap::from_frozen((*old_ordered).clone(), AdaptiveOrdered::sorted_up_to(0));
    tree.insert("c", 3);
    ordered.publish(tree.freeze());
    assert_eq!(old_ordered.len(), 2);
    assert_eq!(old_ordered.get("c"), None);
    assert_eq!(
        ordered.load().keys().copied().collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
}

#[test]
fn built_in_frozen_collections_are_valid_snapshot_payloads() {
    fn accepts<T: SnapshotValue>() {}
    accepts::<frost::Frozen<Vec<usize>>>();
    accepts::<FrozenVec<usize>>();
    accepts::<frost::family::FrozenOrderedMap<usize, usize>>();
    accepts::<frost::family::FrozenOrderedSet<usize>>();
    accepts::<frost::family::FrozenHashMap<usize, usize>>();
    accepts::<frost::family::FrozenHashSet<usize>>();
    accepts::<frost::adaptive::InlineFrozenVec<usize, 4>>();
    accepts::<frost::adaptive::AdaptiveFrozenHashMap<usize, usize>>();
}

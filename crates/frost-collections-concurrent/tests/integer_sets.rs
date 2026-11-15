//! Integer sets published through the snapshot store. These cases were split
//! from the integer package's `integer_sets` target because they need
//! `frost::concurrent`; the helpers are copies of that target's helpers.
#![cfg(all(feature = "int-set", feature = "arc-swap-1"))]

// The package under test, under the name its users import.
extern crate frost_collections_concurrent as frost;

use core::fmt::Debug;
use frost::integer::{FrozenIntSet, IntKey, IntSetPolicy};

fn checked<K: IntKey>(values: &[K], policy: IntSetPolicy) -> FrozenIntSet<K> {
    FrozenIntSet::try_from_iter(values.iter().copied(), policy).unwrap()
}

fn assert_members<K: IntKey + Debug>(actual: &FrozenIntSet<K>, expected: &[K]) {
    assert_eq!(actual.iter().collect::<Vec<_>>(), expected);
    assert_eq!(actual.cardinality(), u64::try_from(expected.len()).unwrap());
    assert_eq!(actual.try_len(), Some(expected.len()));
    assert_eq!(actual.is_empty(), expected.is_empty());
    assert_eq!(
        actual.bounds(),
        expected.first().copied().zip(expected.last().copied())
    );
    for member in expected {
        assert!(actual.contains(member));
    }
}

#[test]
fn snapshot_publication_retains_integer_profile_versions() {
    use frost::concurrent::SnapshotStore;
    let original = checked(&[1_u64, 65], IntSetPolicy::dense());
    let addition = checked(&[65_u64, 130], IntSetPolicy::roaring());
    let store = SnapshotStore::new(original);
    let old = store.load_guard();
    let displaced = store.update(|snapshot| snapshot.union(&addition));
    assert_members(&old, &[1, 65]);
    assert_members(&displaced, &[1, 65]);
    assert_eq!(
        store.peek(|snapshot| snapshot.iter().collect::<Vec<_>>()),
        [1, 65, 130]
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ConcurrentId(u64);

impl IntKey for ConcurrentId {
    type Word = u64;
    fn encode(self) -> u64 {
        self.0
    }
    fn decode(word: u64) -> Option<Self> {
        Some(Self(word))
    }
}

#[test]
fn concurrent_integer_updates_publish_complete_versions_without_lost_ids() {
    use frost::concurrent::SnapshotStore;
    use std::sync::Barrier;

    const WRITERS: usize = 4;
    const UPDATES: u64 = 64;
    const BASE: u64 = 1_000_000_000_000_000;
    let store = SnapshotStore::new(checked(&[ConcurrentId(BASE - 1)], IntSetPolicy::dense()));
    let retained = store.load();
    let start = Barrier::new(WRITERS + 2);
    std::thread::scope(|scope| {
        let reader = scope.spawn(|| {
            let initial = store.load_guard();
            start.wait();
            // Fixed work keeps the regression bounded even if a writer panics.
            // Each individually loaded version must remain complete and sorted.
            for _ in 0..256 {
                let current = store.load_guard();
                let values = current.iter().collect::<Vec<_>>();
                assert_eq!(values.first(), Some(&ConcurrentId(BASE - 1)));
                assert!(values.windows(2).all(|pair| pair[0] < pair[1]));
                assert!(values.iter().all(|id| {
                    (BASE - 1..BASE + u64::try_from(WRITERS).unwrap() * UPDATES).contains(&id.0)
                }));
                assert_eq!(current.cardinality(), u64::try_from(values.len()).unwrap());
                std::thread::yield_now();
            }
            assert_members(&initial, &[ConcurrentId(BASE - 1)]);
        });
        let writers: Vec<_> = (0..WRITERS)
            .map(|writer| {
                let store = &store;
                let start = &start;
                scope.spawn(move || {
                    start.wait();
                    for step in 0..UPDATES {
                        let id =
                            ConcurrentId(BASE + u64::try_from(writer).unwrap() * UPDATES + step);
                        let singleton = checked(&[id], IntSetPolicy::roaring());
                        store.update(|current| current.union(&singleton));
                    }
                })
            })
            .collect();
        start.wait();
        for writer in writers {
            writer.join().unwrap();
        }
        reader.join().unwrap();
    });
    let expected: Vec<_> = (BASE - 1..BASE + u64::try_from(WRITERS).unwrap() * UPDATES)
        .map(ConcurrentId)
        .collect();
    assert_members(&store.load(), &expected);
    assert_members(&retained, &[ConcurrentId(BASE - 1)]);
}

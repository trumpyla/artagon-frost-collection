//! Integer output policies under repeated snapshot updates. This case was
//! split from the integer package's `integer_output` target because it needs
//! `frost::concurrent`; the helpers are copies of that target's helpers.
#![cfg(all(feature = "int-set", feature = "arc-swap-1"))]

// The package under test, under the name its users import.
extern crate frost_collections_concurrent as frost;

use frost::integer::{FrozenIntSet, IntKey, IntSetPolicy, IntSetRepr};
use std::collections::BTreeSet;
use std::fmt::Debug;

fn set<K: IntKey>(values: impl IntoIterator<Item = K>, policy: IntSetPolicy) -> FrozenIntSet<K> {
    FrozenIntSet::try_from_iter(values, policy).unwrap()
}

fn assert_members<K: IntKey + Debug>(actual: &FrozenIntSet<K>, expected: &BTreeSet<K>) {
    assert_eq!(
        actual.iter().collect::<Vec<_>>(),
        expected.iter().copied().collect::<Vec<_>>()
    );
    assert_eq!(actual.cardinality(), u64::try_from(expected.len()).unwrap());
    assert_eq!(
        actual.bounds(),
        expected.first().copied().zip(expected.last().copied())
    );
    for value in expected {
        assert!(actual.contains(value));
    }
}

#[test]
fn repeated_snapshot_updates_preserve_dense_storage_and_old_versions() {
    use frost::concurrent::SnapshotStore;
    let initial: BTreeSet<_> = (0_u64..8192).step_by(2).collect();
    let store = SnapshotStore::new(set(initial.iter().copied(), IntSetPolicy::dense()));
    let retained = store.load();
    let mut expected = initial.clone();
    for value in (1_u64..64).step_by(2) {
        let addition = set([value], IntSetPolicy::roaring());
        let previous = store.update(|snapshot| snapshot.union(&addition));
        assert_members(&previous, &expected);
        expected.insert(value);
        let current = store.load();
        assert_eq!(current.representation(), IntSetRepr::Dense);
        assert_members(&current, &expected);
    }
    assert_members(&retained, &initial);
}

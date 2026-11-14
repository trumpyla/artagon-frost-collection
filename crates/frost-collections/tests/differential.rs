//! Differential property tests against the std reference collections.
#![cfg(feature = "std")]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Bound;

use frost::prelude::*;
use proptest::prelude::*;

fn insert_reference(entries: &[(u8, u16)]) -> BTreeMap<u8, u16> {
    let mut m = BTreeMap::new();
    for &(k, v) in entries {
        m.insert(k, v);
    }
    m
}

fn bound() -> impl Strategy<Value = Bound<u8>> {
    prop_oneof![
        Just(Bound::Unbounded),
        any::<u8>().prop_map(Bound::Included),
        any::<u8>().prop_map(Bound::Excluded),
    ]
}

fn valid(lo: Bound<u8>, hi: Bound<u8>) -> bool {
    use Bound::*;
    match (lo, hi) {
        (Excluded(a), Excluded(b)) => a < b,
        (Included(a) | Excluded(a), Included(b) | Excluded(b)) => a <= b,
        _ => true,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    #[test]
    fn ordered_map_matches_btree(entries in prop::collection::vec(any::<(u8, u16)>(), 0..200),
                                 lo in bound(), hi in bound()) {
        let reference = insert_reference(&entries);
        let f: FrozenOrderedMap<u8, u16> = entries.iter().copied().collect();
        prop_assert_eq!(f.len(), reference.len());
        prop_assert!(f.iter().eq(reference.iter()));
        for q in 0..=u8::MAX {
            prop_assert_eq!(f.get(&q), reference.get(&q));
        }
        if valid(lo, hi) {
            prop_assert!(f.range((lo, hi)).eq(reference.range((lo, hi))));
            prop_assert!(f.range((lo, hi)).rev().eq(reference.range((lo, hi)).rev()));
        }
        // Panic behaviour is part of the contract: identical for every bound.
        let ours = std::panic::catch_unwind(|| f.range((lo, hi)).count());
        let theirs = std::panic::catch_unwind(|| reference.range((lo, hi)).count());
        prop_assert_eq!(ours.ok(), theirs.ok());
        let compact = reference.clone().compact();
        prop_assert_eq!(&compact, &f);
        prop_assert_eq!(f.clone().into_btree_map(), reference.clone());
        // Round trip through the mutable family keeps contents and order.
        prop_assert_eq!(f.clone().thaw().freeze(), f);
    }

    #[test]
    fn ordered_set_matches_btree(items in prop::collection::vec(any::<u8>(), 0..200),
                                 lo in bound(), hi in bound()) {
        let reference: BTreeSet<u8> = items.iter().copied().collect();
        let f: FrozenOrderedSet<u8> = items.iter().copied().collect();
        prop_assert!(f.iter().eq(reference.iter()));
        for q in 0..=u8::MAX {
            prop_assert_eq!(f.contains(&q), reference.contains(&q));
        }
        if valid(lo, hi) {
            prop_assert!(f.range((lo, hi)).eq(reference.range((lo, hi))));
        }
        let ours = std::panic::catch_unwind(|| f.range((lo, hi)).count());
        let theirs = std::panic::catch_unwind(|| reference.range((lo, hi)).count());
        prop_assert_eq!(ours.ok(), theirs.ok());
    }

    #[test]
    fn adaptive_hash_matches_hashmap(entries in prop::collection::vec(any::<(u8, u16)>(), 0..64),
                                     threshold in 0usize..32) {
        let reference: HashMap<u8, u16> = entries.iter().copied().collect();
        let f = reference.clone().freeze_with(AdaptiveHash::linear_up_to(threshold));
        prop_assert_eq!(f.len(), reference.len());
        for q in 0..=u8::MAX {
            prop_assert_eq!(f.get(&q), reference.get(&q));
        }
        let mut n = 0usize;
        f.for_each_lookup((0..=u8::MAX).collect::<Vec<_>>().iter(), |q, v| {
            assert_eq!(v, reference.get(q));
            n += 1;
        });
        prop_assert_eq!(n, 256);
        let other = reference.clone().freeze_with(AdaptiveHash::linear_up_to(usize::MAX - threshold));
        prop_assert_eq!(&f, &other);
        prop_assert_eq!(f.clone().into_hash_map(), reference.clone());
        prop_assert_eq!(f.thaw().freeze().into_hash_map(), reference);
    }

    #[test]
    fn inline_vec_roundtrip(items in prop::collection::vec(".{0,4}", 0..12)) {
        let f: InlineFrozenVec<String, 4> = items.clone().freeze_with(Inline::<4>);
        prop_assert_eq!(f.is_inline(), items.len() <= 4);
        prop_assert_eq!(f.as_slice(), items.as_slice());
        let c = f.clone();
        prop_assert_eq!(&c, &f);
        prop_assert_eq!(c.thaw(), items.clone());
        prop_assert_eq!(f.thaw(), items);
    }

    #[test]
    fn frozen_vec_roundtrip(items in prop::collection::vec(any::<i64>(), 0..100)) {
        let f: FrozenVec<i64> = items.clone().compact();
        prop_assert_eq!(&*f, items.as_slice());
        prop_assert_eq!(f.thaw(), items);
    }
}

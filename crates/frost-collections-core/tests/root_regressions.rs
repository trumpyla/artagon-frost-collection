//! Independent review regressions for optional adapters and family API bounds.

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

#[cfg(feature = "std")]
#[test]
fn mutable_hash_set_debug_does_not_require_a_debug_hasher() {
    use std::hash::{BuildHasher, DefaultHasher};

    use frost::mutable::MutableHashSet;

    // A hashing policy is not part of the set's displayed elements. Native
    // HashSet and the frozen family both support this non-Debug policy.
    struct OpaqueHasher;

    impl BuildHasher for OpaqueHasher {
        type Hasher = DefaultHasher;

        fn build_hasher(&self) -> Self::Hasher {
            DefaultHasher::new()
        }
    }

    let mut set = MutableHashSet::with_hasher(OpaqueHasher);
    set.insert(7);
    assert_eq!(format!("{set:?}"), "{7}");
    let frozen = set.freeze();
    assert_eq!(format!("{frozen:?}"), "{7}");
}

#[cfg(feature = "alloc")]
#[test]
fn already_valid_ordered_set_can_freeze_without_new_ordering_bounds() {
    use frost::family::FrozenOrderedSet;
    use frost::mutable::MutableOrderedSet;
    use frost::prelude::FreezeExt;

    struct NoTraits;

    let frozen: FrozenOrderedSet<NoTraits> = MutableOrderedSet::new().freeze();
    assert!(frozen.is_empty());
    let frozen: FrozenOrderedSet<NoTraits> = MutableOrderedSet::default().compact();
    assert!(frozen.is_empty());
}

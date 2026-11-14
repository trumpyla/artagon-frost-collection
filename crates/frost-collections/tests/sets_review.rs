//! Set contracts added while merging the two reviewed designs.

#[cfg(feature = "alloc")]
#[test]
fn ordered_set_strict_construction_and_mutation_match_native_semantics() {
    use frost::family::{FrozenOrderedSet, MutableOrderedSet};

    let rejected = MutableOrderedSet::try_from_unique([4, 2, 4, 2]).unwrap_err();
    assert_eq!(rejected.item, 2);
    assert_eq!(
        FrozenOrderedSet::try_from_unique([3, 3]).unwrap_err().item,
        3
    );
    let input = vec![3, 1];
    let address = input.as_ptr();
    let rejected = FrozenOrderedSet::try_from_sorted_unique(input).unwrap_err();
    assert_eq!(rejected.input, [3, 1]);
    assert_eq!(rejected.input.as_ptr(), address);

    let mut set = MutableOrderedSet::try_from_unique([5, 1, 3, 2, 4]).unwrap();
    assert_eq!(set.range(2..5).copied().collect::<Vec<_>>(), [2, 3, 4]);
    assert_eq!((set.first(), set.last()), (Some(&1), Some(&5)));
    assert_eq!(set.pop_first(), Some(1));
    assert_eq!(set.pop_last(), Some(5));
    set.retain(|value| value % 2 == 0);
    let mut restored: MutableOrderedSet<_> = set.freeze().thaw();
    assert!(restored.insert(6));
    assert_eq!(restored.into_btree_set(), [2, 4, 6].into());
}

#[cfg(feature = "std")]
#[test]
fn hash_set_rejection_and_reservation_preserve_members_and_hasher() {
    use frost::family::MutableHashSet;
    use std::hash::{BuildHasher, DefaultHasher};

    struct Policy(u32);
    impl BuildHasher for Policy {
        type Hasher = DefaultHasher;
        fn build_hasher(&self) -> Self::Hasher {
            DefaultHasher::new()
        }
    }

    let rejected = MutableHashSet::try_from_unique_with_hasher([1, 2, 1], Policy(17)).unwrap_err();
    assert_eq!(rejected.item, 1);
    let mut set = MutableHashSet::from_iter_with_hasher([1_u8, 2, 3], Policy(23));
    assert!(set.try_reserve(usize::MAX).is_err());
    assert_eq!(set.len(), 3);
    assert_eq!(set.hasher().0, 23);
    set.retain(|value| *value != 2);
    set.shrink_to_fit();
    let frozen = set.freeze();
    assert_eq!(frozen.hasher().0, 23);
    assert!(frozen.contains(&1) && frozen.contains(&3));
    let native = frozen.thaw().into_hash_set();
    assert_eq!(native.len(), 2);
    assert_eq!(native.hasher().0, 23);
}

#[cfg(feature = "hashbrown-0_17")]
#[test]
fn hashbrown_set_supports_borrowed_mutation_capabilities() {
    use frost::backend::hashbrown::HashSet;
    use frost::write::{SetInsert, SetRemove};

    let mut set = HashSet::with_hasher(std::hash::RandomState::new());
    assert!(SetInsert::insert(&mut set, String::from("alpha")));
    assert!(!SetInsert::insert(&mut set, String::from("alpha")));
    assert!(SetRemove::remove(&mut set, "alpha"));
    assert!(!SetRemove::remove(&mut set, "alpha"));
}

#[cfg(feature = "indexmap-2")]
#[test]
fn insertion_ordered_set_keeps_explicit_removal_choice() {
    use frost::backend::indexmap::IndexSet;
    use frost::write::SetInsert;

    let mut set = IndexSet::with_hasher(std::hash::RandomState::new());
    for value in ["b", "a", "c", "a"] {
        SetInsert::insert(&mut set, value);
    }
    assert_eq!(set.iter().copied().collect::<Vec<_>>(), ["b", "a", "c"]);
    assert!(set.shift_remove("b"));
    assert_eq!(set.iter().copied().collect::<Vec<_>>(), ["a", "c"]);
}

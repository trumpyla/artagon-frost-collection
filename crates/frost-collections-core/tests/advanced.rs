//! Contract tests for optional persistent, bitmap, and generational collections.

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

#[cfg(feature = "imbl-7")]
mod persistent {
    use core::cmp::Ordering;
    use core::hash::{Hash, Hasher};
    use frost::read::{DequeRead, Elements, Entries, OrderedElements, OrderedEntries, ValueRef};
    use frost::write::{MapInsert, MapRemove, Push, SetInsert, SetRemove};
    use frost::{FreezeWith, Retain};

    #[derive(Clone, Debug)]
    struct Key {
        id: u32,
        tag: &'static str,
    }

    impl PartialEq for Key {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }
    impl Eq for Key {}
    impl PartialOrd for Key {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for Key {
        fn cmp(&self, other: &Self) -> Ordering {
            self.id.cmp(&other.id)
        }
    }
    impl Hash for Key {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.id.hash(state);
        }
    }

    impl core::borrow::Borrow<u32> for Key {
        fn borrow(&self) -> &u32 {
            &self.id
        }
    }

    #[test]
    fn persistent_sets_keep_first_representatives_and_remove_by_borrowed_query() {
        let mut hashed = imbl::HashSet::new();
        let mut ordered = imbl::OrdSet::new();
        for (tag, inserted) in [("first", true), ("later", false)] {
            assert_eq!(SetInsert::insert(&mut hashed, Key { id: 7, tag }), inserted);
            assert_eq!(
                SetInsert::insert(&mut ordered, Key { id: 7, tag }),
                inserted
            );
        }
        let hash_snapshot = hashed.clone().freeze_with(Retain);
        let ordered_snapshot = ordered.clone().freeze_with(Retain);
        assert_eq!(hashed.iter().next().map(|key| key.tag), Some("first"));
        assert_eq!(ordered.iter().next().map(|key| key.tag), Some("first"));
        assert!(SetRemove::remove(&mut hashed, &7_u32));
        assert!(SetRemove::remove(&mut ordered, &7_u32));
        assert!(!SetRemove::remove(&mut hashed, &7_u32));
        assert!(!SetRemove::remove(&mut ordered, &7_u32));
        assert!(hashed.is_empty());
        assert!(ordered.is_empty());
        assert!(hash_snapshot.contains(&7_u32));
        assert!(ordered_snapshot.contains(&7_u32));
    }

    #[test]
    fn borrowed_removal_returns_stored_entries_and_keeps_persistent_snapshots() {
        let mut hashed = imbl::HashMap::new();
        let mut ordered = imbl::OrdMap::new();
        hashed.insert(String::from("alpha"), String::from("value"));
        ordered.insert(String::from("alpha"), String::from("value"));
        let hash_snapshot = hashed.clone().freeze_with(Retain);
        let ordered_snapshot = ordered.clone().freeze_with(Retain);

        assert_eq!(
            MapRemove::remove_entry(&mut hashed, "alpha"),
            Some((String::from("alpha"), String::from("value")))
        );
        assert_eq!(
            MapRemove::remove_entry(&mut ordered, "alpha"),
            Some((String::from("alpha"), String::from("value")))
        );
        assert_eq!(MapRemove::remove(&mut hashed, "alpha"), None);
        assert_eq!(MapRemove::remove(&mut ordered, "alpha"), None);
        assert!(hashed.is_empty());
        assert!(ordered.is_empty());
        assert_eq!(
            hash_snapshot.get("alpha").map(String::as_str),
            Some("value")
        );
        assert_eq!(
            ordered_snapshot.get("alpha").map(String::as_str),
            Some("value")
        );
    }

    #[test]
    fn equivalent_insert_retains_original_stored_key() {
        let mut hashed = imbl::HashMap::new();
        let mut ordered = imbl::OrdMap::new();
        for (key, value, expected) in [
            (
                Key {
                    id: 7,
                    tag: "original",
                },
                10,
                None,
            ),
            (
                Key {
                    id: 7,
                    tag: "replacement",
                },
                20,
                Some(10),
            ),
        ] {
            assert_eq!(MapInsert::insert(&mut hashed, key.clone(), value), expected);
            assert_eq!(MapInsert::insert(&mut ordered, key, value), expected);
        }
        let query = Key {
            id: 7,
            tag: "query",
        };
        let hashed = hashed.freeze_with(Retain);
        let ordered = ordered.freeze_with(Retain);
        assert_eq!(
            hashed
                .get_key_value(&query)
                .map(|(key, value)| (key.tag, *value)),
            Some(("original", 20))
        );
        assert_eq!(
            ordered
                .get_key_value(&query)
                .map(|(key, value)| (key.tag, *value)),
            Some(("original", 20))
        );
    }

    #[test]
    fn borrowed_lookups_and_persistent_snapshots_are_independent() {
        let mut map = imbl::HashMap::new();
        map.insert(String::from("alpha"), 1);
        let mut clone = map.clone();
        let frozen = map.freeze_with(Retain);
        clone.insert(String::from("alpha"), 2);
        clone.insert(String::from("beta"), 3);
        assert_eq!(frozen.get("alpha"), Some(&1));
        assert_eq!(frozen.get("beta"), None);
        assert!(frozen.contains_key("alpha"));
        assert_eq!(Entries::entries(&frozen).count(), 1);
        let mut recovered = frozen.into_inner();
        recovered.insert(String::from("gamma"), 4);
        assert_eq!(recovered.get("gamma"), Some(&4));

        let mut set = imbl::HashSet::new();
        set.insert(String::from("alpha"));
        let mut clone = set.clone();
        let frozen = set.freeze_with(Retain);
        clone.remove("alpha");
        clone.insert(String::from("beta"));
        assert!(frozen.contains("alpha"));
        assert!(!frozen.contains("beta"));
        assert_eq!(Elements::elements(&frozen).count(), 1);
    }

    #[test]
    fn ordered_maps_and_sets_preserve_order_and_borrowed_queries() {
        let map: imbl::OrdMap<String, i32> =
            [(String::from("zeta"), 3), (String::from("alpha"), 1)]
                .into_iter()
                .collect();
        let frozen = map.freeze_with(Retain);
        assert_eq!(frozen.get("alpha"), Some(&1));
        assert_eq!(frozen.get("missing"), None);
        assert_eq!(
            OrderedEntries::first_key_value(&frozen).map(|(key, value)| (key.as_str(), *value)),
            Some(("alpha", 1))
        );
        assert_eq!(
            OrderedEntries::last_key_value(&frozen).map(|(key, value)| (key.as_str(), *value)),
            Some(("zeta", 3))
        );
        assert_eq!(
            frozen
                .iter()
                .map(|(key, _)| key.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );

        let set: imbl::OrdSet<String> = [String::from("zeta"), String::from("alpha")]
            .into_iter()
            .collect();
        let mut clone = set.clone();
        let frozen = set.freeze_with(Retain);
        clone.remove("alpha");
        assert_eq!(frozen.get("alpha").map(String::as_str), Some("alpha"));
        assert!(frozen.contains("zeta"));
        assert_eq!(
            OrderedElements::first_element(&frozen).map(String::as_str),
            Some("alpha")
        );
        assert_eq!(
            OrderedElements::last_element(&frozen).map(String::as_str),
            Some("zeta")
        );
        assert_eq!(
            frozen.iter().map(String::as_str).collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );
    }

    #[test]
    fn vector_retains_snapshot_and_exposes_sequence_capabilities() {
        let mut vector = imbl::Vector::new();
        Push::push(&mut vector, 11);
        Push::push(&mut vector, 22);
        let mut clone = vector.clone();
        let frozen = vector.freeze_with(Retain);
        clone.set(0, 99);
        clone.push_back(33);
        assert_eq!(frozen.get(0), Some(&11));
        assert_eq!(frozen.get(2), None);
        assert_eq!(ValueRef::get_ref(&frozen, &1), Some(&22));
        assert_eq!(DequeRead::front(&frozen), Some(&11));
        assert_eq!(DequeRead::back(&frozen), Some(&22));
        assert_eq!(
            Elements::elements(&frozen).copied().collect::<Vec<_>>(),
            [11, 22]
        );
        assert_eq!(frozen.iter().copied().collect::<Vec<_>>(), [11, 22]);
    }
}

#[cfg(feature = "roaring-0_12")]
mod roaring_sets {
    use frost::read::{Cardinality, SetContains, ValueElements};
    use frost::write::{SetInsert, SetRemove};
    use frost::{FreezeWith, Retain};

    #[test]
    fn set_mutation_capabilities_report_membership_changes() {
        fn exercise<S, T>(set: &mut S, value: T)
        where
            S: SetInsert<Item = T> + SetRemove<T> + Cardinality,
            T: Copy,
        {
            assert!(SetInsert::insert(set, value));
            assert!(!SetInsert::insert(set, value));
            assert_eq!(set.cardinality(), 1);
            assert!(SetRemove::remove(set, &value));
            assert!(!SetRemove::remove(set, &value));
            assert_eq!(set.cardinality(), 0);
        }

        exercise(&mut roaring::RoaringBitmap::new(), u32::MAX);
        exercise(&mut roaring::RoaringTreemap::new(), u64::MAX);
    }

    #[test]
    fn bitmap_iterates_values_and_keeps_cardinality_wide() {
        let bitmap: roaring::RoaringBitmap = [u32::MAX, 1, 65_537].into_iter().collect();
        let mut clone = bitmap.clone();
        let frozen = bitmap.freeze_with(Retain);
        clone.remove(1);
        assert_eq!(Cardinality::cardinality(&frozen), 3_u64);
        assert_eq!(frozen.cardinality(), 3_u64);
        assert!(SetContains::contains(&frozen, &1));
        assert!(frozen.contains(u32::MAX));
        assert_eq!(
            ValueElements::values(&frozen).collect::<Vec<_>>(),
            [1, 65_537, u32::MAX]
        );
        assert_eq!(frozen.iter().collect::<Vec<_>>(), [1, 65_537, u32::MAX]);
    }

    #[test]
    fn treemap_preserves_full_width_keys_and_snapshot() {
        let map: roaring::RoaringTreemap =
            [u64::MAX, 1, u64::from(u32::MAX) + 1].into_iter().collect();
        let mut clone = map.clone();
        let frozen = map.freeze_with(Retain);
        clone.remove(u64::MAX);
        assert_eq!(Cardinality::cardinality(&frozen), 3_u64);
        assert!(frozen.contains(u64::MAX));
        assert!(SetContains::contains(&frozen, &u64::MAX));
        assert_eq!(
            ValueElements::values(&frozen).collect::<Vec<_>>(),
            [1, u64::from(u32::MAX) + 1, u64::MAX]
        );
        assert_eq!(frozen.iter().next_back(), Some(u64::MAX));
        let mut recovered = frozen.into_inner();
        recovered.insert(2);
        assert_eq!(recovered.len(), 4);
    }
}

#[cfg(feature = "fixedbitset-0_5")]
mod dense_sets {
    use frost::read::{Cardinality, SetContains, ValueElements};
    use frost::{FreezeWith, Retain};

    #[test]
    fn padding_bits_are_not_members_and_retention_preserves_storage() {
        let word_bits = usize::try_from(usize::BITS).unwrap();
        for (universe, blocks) in [
            (1, vec![2]),
            (1, vec![3]),
            (5, vec![usize::MAX]),
            (word_bits + 1, vec![usize::MAX; 2]),
        ] {
            let bits = fixedbitset::FixedBitSet::with_capacity_and_blocks(universe, blocks);
            let original_blocks = bits.as_slice().to_vec();
            let expected: Vec<_> = (0..universe)
                .filter(|&index| bits.contains(index))
                .collect();
            assert_eq!(ValueElements::values(&bits).collect::<Vec<_>>(), expected);

            let frozen = bits.freeze_with(Retain);
            assert_eq!(frozen.universe_len(), universe);
            assert_eq!(frozen.cardinality(), u64::try_from(expected.len()).unwrap());
            assert!(!frozen.contains(universe));
            assert_eq!(frozen.ones().collect::<Vec<_>>(), expected);
            assert_eq!(
                frozen.ones().rev().collect::<Vec<_>>(),
                expected.iter().rev().copied().collect::<Vec<_>>()
            );
            assert_eq!(ValueElements::values(&frozen).collect::<Vec<_>>(), expected);
            assert_eq!(frozen.into_inner().as_slice(), original_blocks);
        }
    }

    #[test]
    fn universe_size_and_membership_cardinality_are_distinct() {
        let empty = fixedbitset::FixedBitSet::with_capacity(1024).freeze_with(Retain);
        assert_eq!(empty.universe_len(), 1024);
        assert_eq!(Cardinality::cardinality(&empty), 0);
        assert_eq!(ValueElements::values(&empty).count(), 0);
        let mut bits = empty.into_inner();
        bits.insert(0);
        bits.insert(7);
        bits.insert(1023);
        let mut clone = bits.clone();
        let frozen = bits.freeze_with(Retain);
        clone.clear();
        assert_eq!(frozen.cardinality(), 3);
        assert_eq!(frozen.universe_len(), 1024);
        assert!(SetContains::contains(&frozen, &1023));
        assert!(!frozen.contains(1024));
        assert!(!frozen.contains(usize::MAX));
        assert_eq!(
            ValueElements::values(&frozen).collect::<Vec<_>>(),
            [0, 7, 1023]
        );
        assert_eq!(frozen.ones().collect::<Vec<_>>(), [0, 7, 1023]);
    }
}

#[cfg(feature = "slotmap-1")]
mod generational_maps {
    use frost::read::{Len, ValueRef};
    use frost::{FreezeWith, Retain};
    slotmap::new_key_type! { struct ObjectKey; }

    #[test]
    fn secondary_membership_is_independent_of_primary_removal() {
        let mut primary = slotmap::SlotMap::<ObjectKey, _>::with_key();
        let retired = primary.insert("object");
        let mut secondary = slotmap::SecondaryMap::new();
        secondary.insert(retired, "metadata");
        let frozen = secondary.freeze_with(Retain);

        assert_eq!(primary.remove(retired), Some("object"));
        let replacement = primary.insert("replacement");
        assert_ne!(retired, replacement);
        assert_eq!(primary.get(retired), None);
        assert_eq!(frozen.get(retired), Some(&"metadata"));
        assert!(frozen.contains_key(retired));
        assert_eq!(ValueRef::get_ref(&frozen, &retired), Some(&"metadata"));
        assert_eq!(frozen.get(replacement), None);
        assert_eq!(frozen.len(), 1);

        let mut secondary = frozen.into_inner();
        assert_eq!(secondary.remove(retired), Some("metadata"));
        assert!(secondary.is_empty());
    }

    #[test]
    fn slotmap_rejects_stale_keys_and_clones_do_not_mutate_snapshot() {
        let mut map = slotmap::SlotMap::<ObjectKey, _>::with_key();
        let stale = map.insert("old");
        assert_eq!(map.remove(stale), Some("old"));
        let live = map.insert("current");
        assert_ne!(stale, live);
        let mut clone = map.clone();
        let frozen = map.freeze_with(Retain);
        clone[live] = "changed";
        assert_eq!(frozen.get(stale), None);
        assert!(!frozen.contains_key(stale));
        assert_eq!(frozen.get(live), Some(&"current"));
        assert_eq!(ValueRef::get_ref(&frozen, &live), Some(&"current"));
        assert_eq!(Len::len(&frozen), 1);
        assert_eq!(
            frozen
                .iter()
                .map(|(key, value)| (key, *value))
                .collect::<Vec<_>>(),
            [(live, "current")]
        );
        let mut recovered = frozen.into_inner();
        assert_eq!(recovered.remove(live), Some("current"));
    }

    #[test]
    fn dense_and_secondary_maps_support_copied_key_iteration() {
        let mut primary = slotmap::DenseSlotMap::<ObjectKey, _>::with_key();
        let first = primary.insert(10);
        let second = primary.insert(20);
        let mut secondary = slotmap::SecondaryMap::new();
        secondary.insert(first, "label");
        let frozen_primary = primary.freeze_with(Retain);
        let frozen_secondary = secondary.freeze_with(Retain);
        assert_eq!(frozen_primary.get(second), Some(&20));
        assert_eq!(ValueRef::get_ref(&frozen_primary, &first), Some(&10));
        assert_eq!(
            frozen_primary.iter().map(|(_, value)| *value).sum::<i32>(),
            30
        );
        assert_eq!(frozen_secondary.get(first), Some(&"label"));
        assert_eq!(ValueRef::get_ref(&frozen_secondary, &first), Some(&"label"));
        assert!(!frozen_secondary.contains_key(second));
        assert_eq!(
            frozen_secondary
                .iter()
                .map(|(key, value)| (key, *value))
                .collect::<Vec<_>>(),
            [(first, "label")]
        );
    }
}

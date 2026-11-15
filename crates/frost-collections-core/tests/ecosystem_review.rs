//! Regression tests for independently enabled framework adapters.

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

#[cfg(feature = "serde-1")]
mod serde_contracts {
    use frost::prelude::*;

    #[test]
    fn ordered_sets_roundtrip_and_strict_decoding_rejects_duplicates() {
        #[derive(serde::Deserialize)]
        struct Strict {
            #[serde(deserialize_with = "frost::adapters::serde::sorted_unique::deserialize")]
            members: MutableOrderedSet<u32>,
        }
        let mutable: MutableOrderedSet<u32> = serde_json::from_str("[3,1,3]").unwrap();
        assert_eq!(serde_json::to_string(&mutable).unwrap(), "[1,3]");
        let frozen: FrozenOrderedSet<u32> = serde_json::from_str("[3,1,3]").unwrap();
        assert_eq!(mutable.freeze(), frozen);
        let decoded: Strict = serde_json::from_str(r#"{"members":[1,3]}"#).unwrap();
        assert_eq!(decoded.members.len(), 2);
        assert!(serde_json::from_str::<Strict>(r#"{"members":[1,1]}"#).is_err());
        assert!(serde_json::from_str::<Strict>(r#"{"members":[3,1]}"#).is_err());
    }

    #[test]
    fn map_allocation_ignores_excessive_deserializer_size_hint() {
        struct ExcessiveHint<I>(I);
        impl<I: Iterator> Iterator for ExcessiveHint<I> {
            type Item = I::Item;
            fn next(&mut self) -> Option<Self::Item> {
                self.0.next()
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                (usize::MAX, Some(usize::MAX))
            }
        }
        let access = serde::de::value::MapDeserializer::<_, serde::de::value::Error>::new(
            ExcessiveHint([("a", 7u8)].into_iter()),
        );
        let map =
            <FrozenOrderedMap<String, u8> as serde::Deserialize>::deserialize(access).unwrap();
        assert_eq!(map.get("a"), Some(&7));
    }

    #[cfg(feature = "std")]
    #[test]
    fn hash_sets_roundtrip_without_a_stable_wire_order() {
        let mutable: MutableHashSet<String> = serde_json::from_str(r#"["b","a","b"]"#).unwrap();
        let json = serde_json::to_string(&mutable).unwrap();
        let frozen: FrozenHashSet<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(mutable.freeze(), frozen);
        assert!(frozen.contains("a"));
        assert!(frozen.contains("b"));
        assert_eq!(frozen.len(), 2);
    }
}

#[cfg(feature = "borsh-1")]
mod borsh_contracts {
    use frost::prelude::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn exact_wrapper_preserves_native_decoding_rules() {
        let bytes = borsh::to_vec(&vec![(2u32, 20u8), (1, 10)]).unwrap();
        assert!(borsh::from_slice::<FrozenOrderedMap<u32, u8>>(&bytes).is_err());
        let native = borsh::from_slice::<BTreeMap<u32, u8>>(&bytes);
        let exact = borsh::from_slice::<Frozen<BTreeMap<u32, u8>>>(&bytes).map(Frozen::into_inner);
        match (native, exact) {
            (Ok(native), Ok(exact)) => assert_eq!(native, exact),
            (Err(native), Err(exact)) => assert_eq!(native.kind(), exact.kind()),
            _ => panic!("an exact wrapper must preserve native decoding behavior"),
        }
    }

    #[test]
    fn ordered_set_encodings_are_canonical_and_strict() {
        let mutable: MutableOrderedSet<u32> = [7, 1, 3].into_iter().collect();
        let bytes = borsh::to_vec(&BTreeSet::from([1u32, 3, 7])).unwrap();
        assert_eq!(borsh::to_vec(&mutable).unwrap(), bytes);
        let decoded: MutableOrderedSet<u32> = borsh::from_slice(&bytes).unwrap();
        assert_eq!(decoded, mutable);
        let duplicate = borsh::to_vec(&vec![1u32, 1]).unwrap();
        let unordered = borsh::to_vec(&vec![3u32, 1]).unwrap();
        assert!(borsh::from_slice::<MutableOrderedSet<u32>>(&duplicate).is_err());
        assert!(borsh::from_slice::<MutableOrderedSet<u32>>(&unordered).is_err());
        assert!(borsh::to_vec(&[()].into_iter().collect::<MutableOrderedSet<_>>()).is_err());
    }

    #[cfg(feature = "std")]
    #[test]
    fn hash_set_encoding_matches_ordered_sets_and_rejects_noncanonical_input() {
        let expected = borsh::to_vec(&BTreeSet::from([1u32, 3, 7])).unwrap();
        let mutable: MutableHashSet<u32> = [7, 1, 3].into_iter().collect();
        let frozen: FrozenHashSet<u32> = [3, 7, 1].into_iter().collect();
        assert_eq!(borsh::to_vec(&mutable).unwrap(), expected);
        assert_eq!(borsh::to_vec(&frozen).unwrap(), expected);
        assert_eq!(
            borsh::from_slice::<FrozenHashSet<u32>>(&expected).unwrap(),
            frozen
        );
        assert_eq!(
            borsh::from_slice::<MutableHashSet<u32>>(&expected).unwrap(),
            mutable
        );
        for members in [vec![1u32, 1], vec![3, 1]] {
            let bytes = borsh::to_vec(&members).unwrap();
            assert!(borsh::from_slice::<FrozenHashSet<u32>>(&bytes).is_err());
            assert!(borsh::from_slice::<MutableHashSet<u32>>(&bytes).is_err());
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn hash_decoding_cannot_silently_collapse_eq_equivalent_ordered_keys() {
        #[derive(Debug)]
        struct Key(u8, u8);
        impl PartialEq for Key {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }
        impl Eq for Key {}
        impl std::hash::Hash for Key {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.0.hash(state);
            }
        }
        impl PartialOrd for Key {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for Key {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                (self.0, self.1).cmp(&(other.0, other.1))
            }
        }
        impl borsh::BorshDeserialize for Key {
            fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
                let (id, metadata) =
                    <(u8, u8) as borsh::BorshDeserialize>::deserialize_reader(reader)?;
                Ok(Self(id, metadata))
            }
        }
        // Eq and Ord disagree. The input is ascending under Ord but contains
        // two equivalent hash keys; strict decoding must not discard either.
        let maps = borsh::to_vec(&vec![((1u8, 1u8), 10u8), ((1, 2), 20)]).unwrap();
        assert!(borsh::from_slice::<FrozenHashMap<Key, u8>>(&maps).is_err());
        assert!(borsh::from_slice::<MutableHashMap<Key, u8>>(&maps).is_err());
        assert!(borsh::from_slice::<AdaptiveHashMap<Key, u8>>(&maps).is_err());
        assert!(borsh::from_slice::<AdaptiveFrozenHashMap<Key, u8>>(&maps).is_err());
        let sets = borsh::to_vec(&vec![(1u8, 1u8), (1, 2)]).unwrap();
        assert!(borsh::from_slice::<FrozenHashSet<Key>>(&sets).is_err());
        assert!(borsh::from_slice::<MutableHashSet<Key>>(&sets).is_err());
    }
}

#[cfg(feature = "rayon-1")]
mod rayon_contracts {
    use frost::prelude::*;
    use rayon::prelude::*;
    use std::hash::{BuildHasher, DefaultHasher, Hash, Hasher};
    use std::rc::Rc;

    #[derive(Default)]
    struct LocalHasher(Rc<()>);
    impl BuildHasher for LocalHasher {
        type Hasher = DefaultHasher;
        fn build_hasher(&self) -> Self::Hasher {
            DefaultHasher::new()
        }
    }

    #[test]
    fn collection_and_extension_do_not_send_the_hasher_to_workers() {
        let mut map: MutableHashMap<u32, u32, LocalHasher> =
            (0..20_u32).into_par_iter().map(|key| (key, key)).collect();
        let hasher = Rc::clone(&map.hasher().0);
        map.par_extend([(2, 99), (30, 30)].into_par_iter());
        assert!(Rc::ptr_eq(&hasher, &map.hasher().0));
        assert_eq!(map.get(&2), Some(&99));
        assert_eq!(map.par_iter().count(), 21);
        map.par_iter_mut().for_each(|(_, value)| *value += 1);
        assert_eq!(map.into_par_iter().count(), 21);

        let frozen: FrozenHashMap<u32, u32, LocalHasher> =
            (0..10_u32).into_par_iter().map(|key| (key, key)).collect();
        assert_eq!(frozen.par_iter().count(), 10);
        assert_eq!(frozen.into_par_iter().count(), 10);

        let mut set: MutableHashSet<u32, LocalHasher> = (0..10_u32).into_par_iter().collect();
        set.par_extend([9, 10].into_par_iter());
        assert_eq!(set.par_iter().count(), 11);
        assert_eq!(set.into_par_iter().count(), 11);
        let frozen: FrozenHashSet<u32, LocalHasher> = (0..10_u32).into_par_iter().collect();
        assert_eq!(frozen.par_iter().count(), 10);
        assert_eq!(frozen.into_par_iter().count(), 10);
    }

    #[test]
    fn pure_traversal_needs_neither_hashing_nor_a_hasher() {
        struct NoTraits;
        let mut mutable: MutableHashMap<NoTraits, NoTraits, ()> = MutableHashMap::default();
        assert_eq!(mutable.par_iter().count(), 0);
        assert_eq!(mutable.par_iter_mut().count(), 0);
        assert_eq!(mutable.into_par_iter().count(), 0);
        let frozen: FrozenHashMap<NoTraits, NoTraits, ()> = FrozenHashMap::default();
        assert_eq!(frozen.par_iter().count(), 0);
        assert_eq!(frozen.into_par_iter().count(), 0);
        let mut adaptive = AdaptiveHashMap::<NoTraits, NoTraits, ()>::with_hasher_and_policy(
            (),
            AdaptiveHash::default(),
        );
        assert_eq!(adaptive.par_iter().count(), 0);
        assert_eq!(adaptive.par_iter_mut().count(), 0);
        assert_eq!(adaptive.into_par_iter().count(), 0);
        let members: MutableHashSet<NoTraits, ()> = MutableHashSet::default();
        assert_eq!(members.par_iter().count(), 0);
        assert_eq!(members.into_par_iter().count(), 0);
        let members: FrozenHashSet<NoTraits, ()> = FrozenHashSet::default();
        assert_eq!(members.par_iter().count(), 0);
        assert_eq!(members.into_par_iter().count(), 0);
        let members = MutableOrderedSet::<NoTraits>::new();
        assert_eq!(members.par_iter().count(), 0);
        assert_eq!(members.into_par_iter().count(), 0);
    }

    #[derive(Clone, Debug)]
    struct Key {
        id: u32,
        metadata: u32,
    }
    impl PartialEq for Key {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }
    impl Eq for Key {}
    impl Hash for Key {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.id.hash(state);
        }
    }
    impl PartialOrd for Key {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for Key {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.id.cmp(&other.id)
        }
    }

    #[test]
    fn ordered_parallel_input_retains_first_representative_and_last_value() {
        let input: Vec<_> = (0..4096_u32)
            .map(|n| {
                (
                    Key {
                        id: n % 7,
                        metadata: n,
                    },
                    n,
                )
            })
            .collect();
        let hashed: FrozenHashMap<_, _> = input.par_iter().cloned().collect();
        let ordered: FrozenOrderedMap<_, _> = input.par_iter().cloned().collect();
        let mutable: MutableOrderedMap<_, _> = input.par_iter().cloned().collect();
        for id in 0..7_u32 {
            let query = Key {
                id,
                metadata: u32::MAX,
            };
            let expected = input.iter().rev().find(|(key, _)| key.id == id).unwrap().1;
            for (key, value) in [
                hashed.get_key_value(&query).unwrap(),
                ordered.get_key_value(&query).unwrap(),
                mutable.get_key_value(&query).unwrap(),
            ] {
                assert_eq!(key.metadata, id);
                assert_eq!(*value, expected);
            }
        }
        let ordered: MutableOrderedSet<Key> =
            input.par_iter().map(|(key, _)| key.clone()).collect();
        let hashed: FrozenHashSet<Key> = input.par_iter().map(|(key, _)| key.clone()).collect();
        assert!(ordered.par_iter().all(|key| key.metadata == key.id));
        assert!(hashed.par_iter().all(|key| key.metadata == key.id));
        let mut ordered = ordered;
        ordered.par_extend(
            [
                Key {
                    id: 0,
                    metadata: 99,
                },
                Key { id: 9, metadata: 9 },
            ]
            .into_par_iter(),
        );
        assert_eq!(
            ordered.get(&Key { id: 0, metadata: 0 }).unwrap().metadata,
            0
        );
        assert_eq!(ordered.into_par_iter().count(), 8);
    }
}

#[cfg(feature = "schemars-1")]
#[test]
fn set_schemas_follow_the_same_logical_encoding() {
    use frost::prelude::*;
    let expected = schemars::schema_for!(std::collections::BTreeSet<u32>);
    assert_eq!(schemars::schema_for!(MutableOrderedSet<u32>), expected);
    assert_eq!(schemars::schema_for!(FrozenOrderedSet<u32>), expected);
    #[cfg(feature = "std")]
    {
        assert_eq!(schemars::schema_for!(MutableHashSet<u32>), expected);
        assert_eq!(schemars::schema_for!(FrozenHashSet<u32>), expected);
    }
}

#[cfg(feature = "arbitrary-1")]
#[test]
fn set_generation_matches_the_family_builder() {
    use arbitrary::{Arbitrary, Unstructured};
    use frost::prelude::*;
    let bytes = [1u8, 7, 1, 3, 1, 7, 0];
    let source = Vec::<u8>::arbitrary(&mut Unstructured::new(&bytes)).unwrap();
    assert_eq!(source, [7, 3, 7]);
    let ordered = MutableOrderedSet::<u8>::arbitrary(&mut Unstructured::new(&bytes)).unwrap();
    let hashed = FrozenHashSet::<u8>::arbitrary(&mut Unstructured::new(&bytes)).unwrap();
    let mutable_hashed = MutableHashSet::<u8>::arbitrary(&mut Unstructured::new(&bytes)).unwrap();
    assert_eq!(ordered, source.iter().copied().collect());
    assert_eq!(hashed, source.iter().copied().collect::<FrozenHashSet<_>>());
    assert_eq!(mutable_hashed.freeze(), hashed);
}

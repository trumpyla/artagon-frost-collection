//! Each framework's contract tests run with that feature in isolation.
#![allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::items_after_statements,
    dead_code,
    reason = "test code"
)]

#[cfg(feature = "serde-1")]
mod serde_tests {
    #[cfg(feature = "std")]
    use frost::adaptive::{HashRepr, OrderedRepr};
    use frost::prelude::*;
    use std::collections::BTreeMap;
    #[cfg(feature = "std")]
    use std::collections::HashMap;

    fn json<T: serde::Serialize>(t: &T) -> String {
        serde_json::to_string(t).unwrap()
    }

    #[test]
    fn serde_round_trips_every_family() {
        let v: MutableVec<u8> = vec![3, 1].into();
        assert_eq!(json(&v), "[3,1]");
        assert_eq!(
            serde_json::from_str::<FrozenVec<u8>>("[3,1]").unwrap(),
            [3, 1]
        );
        let i: InlineFrozenVec<u8, 2> = serde_json::from_str("[1,2]").unwrap();
        assert!(i.is_inline());
        let i: InlineVec<u8, 2> = serde_json::from_str("[1,2,3]").unwrap();
        assert!(!i.is_inline());

        let o: FrozenOrderedMap<String, u8> = serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
        assert_eq!(json(&o), r#"{"a":1,"b":2}"#);
        let m: MutableOrderedMap<String, u8> = serde_json::from_str(&json(&o)).unwrap();
        assert_eq!(m.freeze(), o);
        let s: FrozenOrderedSet<u8> = serde_json::from_str("[3,1,3]").unwrap();
        assert_eq!(json(&s), "[1,3]");

        #[cfg(feature = "std")]
        {
            let h: FrozenHashMap<String, u8> = serde_json::from_str(r#"{"x":1}"#).unwrap();
            assert_eq!(json(&h), r#"{"x":1}"#);
        }
        let f: frost::Frozen<BTreeMap<u8, u8>> = serde_json::from_str(r#"{"1":2}"#).unwrap();
        assert_eq!(f.get(&1), Some(&2));
    }

    #[cfg(feature = "std")]
    #[test]
    fn serde_encoding_ignores_representation() {
        let entries: Vec<(u8, u8)> = (0..6).map(|i| (i, i)).collect();
        let lin = entries
            .iter()
            .copied()
            .collect::<HashMap<_, _>>()
            .freeze_with(AdaptiveHash::linear_up_to(8));
        let tab = entries
            .iter()
            .copied()
            .collect::<HashMap<_, _>>()
            .freeze_with(AdaptiveHash::linear_up_to(0));
        assert_eq!(
            (lin.representation(), tab.representation()),
            (HashRepr::Linear, HashRepr::Table)
        );
        let back_l: BTreeMap<u8, u8> = serde_json::from_str(&json(&lin)).unwrap();
        let back_t: BTreeMap<u8, u8> = serde_json::from_str(&json(&tab)).unwrap();
        assert_eq!(back_l, back_t);

        let mut sorted = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(100));
        let mut tree = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(0));
        sorted.extend(entries.iter().copied());
        tree.extend(entries.iter().copied());
        assert_eq!(
            (sorted.representation(), tree.representation()),
            (OrderedRepr::Sorted, OrderedRepr::Tree)
        );
        assert_eq!(json(&sorted), json(&tree));
    }

    #[cfg(feature = "std")]
    #[test]
    fn serde_duplicate_policy_and_strict_helpers() {
        let last: FrozenOrderedMap<String, u8> = serde_json::from_str(r#"{"a":1,"a":2}"#).unwrap();
        assert_eq!(last.get("a"), Some(&2));
        let last: MutableHashMap<String, u8> = serde_json::from_str(r#"{"a":1,"a":2}"#).unwrap();
        assert_eq!(last.get("a"), Some(&2));

        #[derive(serde::Deserialize)]
        struct Strict {
            #[serde(deserialize_with = "frost::adapters::serde::unique::deserialize")]
            h: AdaptiveFrozenHashMap<String, u8>,
            #[serde(deserialize_with = "frost::adapters::serde::sorted_unique::deserialize")]
            o: FrozenOrderedMap<String, u8>,
        }
        assert!(serde_json::from_str::<Strict>(r#"{"h":{"a":1},"o":{"a":1,"b":2}}"#).is_ok());
        assert!(serde_json::from_str::<Strict>(r#"{"h":{"a":1,"a":1},"o":{}}"#).is_err());
        let err = serde_json::from_str::<Strict>(r#"{"h":{},"o":{"b":1,"a":2}}"#)
            .err()
            .unwrap();
        assert!(err.to_string().contains("index 1"), "{err}");
    }

    #[test]
    fn serde_borrowed_keys_work_with_strict_helpers() {
        #[derive(serde::Deserialize)]
        struct Borrowed<'a> {
            #[serde(
                borrow,
                deserialize_with = "frost::adapters::serde::sorted_unique::deserialize"
            )]
            o: FrozenOrderedMap<&'a str, u8>,
        }
        let src = r#"{"o":{"a":1,"b":2}}"#.to_owned();
        let b: Borrowed<'_> = serde_json::from_str(&src).unwrap();
        assert_eq!(b.o.get("b"), Some(&2));
    }

    #[test]
    fn sorted_unique_decoding_covers_all_ordered_maps() {
        // serde sorted_unique for every ordered family
        #[derive(serde::Deserialize)]
        struct S {
            #[serde(deserialize_with = "frost::adapters::serde::sorted_unique::deserialize")]
            m: MutableOrderedMap<String, u8>,
            #[serde(deserialize_with = "frost::adapters::serde::sorted_unique::deserialize")]
            a: AdaptiveOrderedMap<String, u8>,
        }
        assert!(serde_json::from_str::<S>(r#"{"m":{"a":1},"a":{"a":1,"b":2}}"#).is_ok());
        assert!(serde_json::from_str::<S>(r#"{"m":{"b":1,"a":1},"a":{}}"#).is_err());
    }
}

#[cfg(feature = "borsh-1")]
mod borsh_tests {
    use frost::prelude::*;

    #[cfg(feature = "std")]
    #[test]
    fn borsh_is_canonical_and_matches_std_bytes() {
        let entries = [(3u32, 30u8), (1, 10), (2, 20)];
        let std_bytes = borsh::to_vec(&std::collections::BTreeMap::from(entries)).unwrap();
        let hm: FrozenHashMap<u32, u8> = entries.into_iter().collect();
        let om: FrozenOrderedMap<u32, u8> = entries.into_iter().collect();
        let ah: AdaptiveHashMap<u32, u8> = entries.into_iter().collect();
        assert_eq!(borsh::to_vec(&hm).unwrap(), std_bytes);
        assert_eq!(borsh::to_vec(&om).unwrap(), std_bytes);
        assert_eq!(borsh::to_vec(&ah).unwrap(), std_bytes);
        assert_eq!(
            borsh::from_slice::<FrozenOrderedMap<u32, u8>>(&std_bytes).unwrap(),
            om
        );
        assert_eq!(
            borsh::from_slice::<MutableHashMap<u32, u8>>(&std_bytes)
                .unwrap()
                .freeze(),
            hm
        );

        // Non-canonical: same logical map, keys out of order.
        let mut bad = Vec::new();
        borsh::BorshSerialize::serialize(&3u32, &mut bad).unwrap();
        for (k, v) in [(2u32, 20u8), (1, 10), (3, 30)] {
            borsh::BorshSerialize::serialize(&k, &mut bad).unwrap();
            borsh::BorshSerialize::serialize(&v, &mut bad).unwrap();
        }
        assert!(borsh::from_slice::<FrozenOrderedMap<u32, u8>>(&bad).is_err());
        assert!(borsh::from_slice::<FrozenHashMap<u32, u8>>(&bad).is_err());

        let v: InlineFrozenVec<u16, 4> = vec![1, 2, 3].freeze_with(Inline::<4>);
        assert_eq!(
            borsh::to_vec(&v).unwrap(),
            borsh::to_vec(&vec![1u16, 2, 3]).unwrap()
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn borsh_rejects_non_total_hash_key_ordering() {
        // borsh refuses to encode what it would refuse to decode
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct K(u8, u8);
        impl PartialOrd for K {
            fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(o))
            }
        }
        impl Ord for K {
            fn cmp(&self, o: &Self) -> std::cmp::Ordering {
                self.0.cmp(&o.0) // ignores .1: Eq and Ord disagree
            }
        }
        impl borsh::BorshSerialize for K {
            fn serialize<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
                (self.0, self.1).serialize(w)
            }
        }
        let m: FrozenHashMap<K, u8> = [(K(1, 1), 1), (K(1, 2), 2)].into_iter().collect();
        assert_eq!(m.len(), 2);
        assert!(borsh::to_vec(&m).is_err());
    }

    #[test]
    fn ordered_borsh_roundtrip_is_strict_without_std_feature() {
        let map: FrozenOrderedMap<u32, u8> = [(2, 20), (1, 10)].into_iter().collect();
        let expected =
            borsh::to_vec(&std::collections::BTreeMap::from([(1u32, 10u8), (2, 20)])).unwrap();
        assert_eq!(borsh::to_vec(&map).unwrap(), expected);
        assert_eq!(
            borsh::from_slice::<FrozenOrderedMap<u32, u8>>(&expected).unwrap(),
            map
        );
        let unordered = borsh::to_vec(&vec![(2u32, 20u8), (1, 10)]).unwrap();
        assert!(borsh::from_slice::<FrozenOrderedMap<u32, u8>>(&unordered).is_err());
        let duplicates = borsh::to_vec(&vec![1u32, 1]).unwrap();
        assert!(borsh::from_slice::<FrozenOrderedSet<u32>>(&duplicates).is_err());
    }
}

#[cfg(feature = "rayon-1")]
mod rayon_tests {
    use frost::prelude::*;

    #[test]
    fn rayon_collect_equals_sequential_collect() {
        use rayon::prelude::*;
        let data: Vec<(u16, u32)> = (0..20_000u32).map(|i| ((i % 997) as u16, i)).collect();

        let seq_o: FrozenOrderedMap<u16, u32> = data.iter().copied().collect();
        let par_o: FrozenOrderedMap<u16, u32> = data.par_iter().copied().collect();
        assert_eq!(par_o, seq_o);
        let par_mo: MutableOrderedMap<u16, u32> = data.par_iter().copied().collect();
        assert_eq!(par_mo.freeze(), seq_o);
        let par_ao: AdaptiveOrderedMap<u16, u32> = data.par_iter().copied().collect();
        assert_eq!(par_ao.freeze(), seq_o);

        let seq_h: FrozenHashMap<u16, u32> = data.iter().copied().collect();
        let par_h: FrozenHashMap<u16, u32> = data.par_iter().copied().collect();
        assert_eq!(par_h, seq_h);
        let par_ah: AdaptiveFrozenHashMap<u16, u32> = data.par_iter().copied().collect();
        assert_eq!(par_ah.into_hash_map(), seq_h.clone().into_hash_map());

        let s: u64 = seq_o.par_iter().map(|(_, v)| u64::from(*v)).sum();
        assert_eq!(s, seq_o.iter().map(|(_, v)| u64::from(*v)).sum::<u64>());
        assert_eq!(seq_o.par_iter().len(), seq_o.len()); // indexed
    }

    #[test]
    fn rayon_mutable_and_owned_iteration() {
        use rayon::prelude::*;
        let mut v: MutableVec<u32> = (0..1000).collect();
        v.par_iter_mut().for_each(|x| *x *= 2);
        v.par_extend((0..10u32).into_par_iter());
        assert_eq!(v.len(), 1010);
        let total: u64 = v.into_par_iter().map(u64::from).sum();
        assert_eq!(total, 2 * (0..1000u64).sum::<u64>() + (0..10).sum::<u64>());

        let mut a: AdaptiveHashMap<u8, u32> = (0..5u8).map(|k| (k, 1)).collect();
        a.par_iter_mut().for_each(|(_, v)| *v += 1);
        assert!(a.values().all(|v| *v == 2));
        let mut i: InlineVec<u8, 8> = (0..4).collect();
        i.par_iter_mut().for_each(|x| *x += 1);
        assert_eq!(i, [1, 2, 3, 4]);
    }

    #[test]
    fn rayon_mutates_adaptive_ordered_values() {
        // rayon par_iter_mut on the adaptive ordered profile
        use rayon::prelude::*;
        let mut o: AdaptiveOrderedMap<u8, u32> = (0..50u8).map(|k| (k, 1)).collect();
        o.par_iter_mut().for_each(|(_, v)| *v += 1);
        assert!(o.values().all(|v| *v == 2));
    }
}

#[cfg(feature = "arbitrary-1")]
mod arbitrary_tests {
    use frost::adaptive::{HashRepr, OrderedRepr};
    use frost::prelude::*;

    #[test]
    fn arbitrary_reaches_every_representation_with_valid_invariants() {
        use arbitrary::{Arbitrary, Unstructured};
        let mut seen_h = (false, false);
        let mut seen_o = (false, false);
        let mut seen_i = (false, false);
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut bytes = move || -> Vec<u8> {
            (0..512)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    state.to_le_bytes()[3]
                })
                .collect()
        };
        for _ in 0..300 {
            let (b1, b2, b3, b4) = (bytes(), bytes(), bytes(), bytes());
            let h =
                AdaptiveFrozenHashMap::<u8, u8>::arbitrary(&mut Unstructured::new(&b1)).unwrap();
            match h.representation() {
                HashRepr::Linear => seen_h.0 = true,
                _ => seen_h.1 = true,
            }
            let o = AdaptiveOrderedMap::<u8, u8>::arbitrary(&mut Unstructured::new(&b2)).unwrap();
            match o.representation() {
                OrderedRepr::Sorted => seen_o.0 = true,
                _ => seen_o.1 = true,
            }
            let f = FrozenOrderedMap::<u8, u8>::arbitrary(&mut Unstructured::new(&b3)).unwrap();
            assert!(FrozenOrderedMap::try_from_sorted_unique(f.into_sorted_vec()).is_ok());
            let i = InlineFrozenVec::<u8, 2>::arbitrary(&mut Unstructured::new(&b4)).unwrap();
            if i.is_inline() {
                seen_i.0 = true;
            } else {
                seen_i.1 = true;
            }
        }
        assert_eq!(
            (seen_h, seen_o, seen_i),
            ((true, true), (true, true), (true, true))
        );
    }

    #[test]
    fn arbitrary_reaches_grown_states() {
        use arbitrary::{Arbitrary, Unstructured};
        let mut state = 0x2545_F491_4F6C_DD1D_u64;
        let mut seen_spilled_short = false;
        let mut seen_table_short = false;
        for _ in 0..400 {
            let bytes: Vec<u8> = (0..128)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    state.to_le_bytes()[2]
                })
                .collect();
            let v = InlineVec::<u8, 4>::arbitrary(&mut Unstructured::new(&bytes)).unwrap();
            seen_spilled_short |= !v.is_inline() && v.len() <= 4;
            let h = AdaptiveHashMap::<u8, u8>::arbitrary(&mut Unstructured::new(&bytes)).unwrap();
            seen_table_short |=
                h.representation() == HashRepr::Table && h.len() <= h.policy().linear_max;
        }
        assert!(seen_spilled_short && seen_table_short);
    }
}

#[cfg(feature = "schemars-1")]
mod schemars_tests {
    use frost::prelude::*;
    use std::collections::BTreeMap;

    #[test]
    fn schemas_match_the_std_type_with_the_same_encoding() {
        let ours = schemars::schema_for!(FrozenOrderedMap<String, u32>);
        let std = schemars::schema_for!(BTreeMap<String, u32>);
        assert_eq!(ours, std);
        let ours = schemars::schema_for!(InlineFrozenVec<u8, 4>);
        let std = schemars::schema_for!(Vec<u8>);
        assert_eq!(ours, std);
        #[cfg(feature = "std")]
        {
            let ours = schemars::schema_for!(AdaptiveHashMap<String, bool>);
            let std = schemars::schema_for!(std::collections::HashMap<String, bool>);
            assert_eq!(ours, std);
        }
    }
}

//! Mutable families and per-sub-category adaptive profiles: contracts,
//! freeze/thaw pairing, promotion/demotion, and differential checks.
#![cfg(feature = "std")]
#![allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::items_after_statements,
    dead_code,
    reason = "test code"
)]

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

use std::collections::{BTreeMap, HashMap};

use frost::adaptive::{HashRepr, OrderedRepr};
use frost::prelude::*;
use frost::read::{Entries, LookupRef};
use frost::write::{MapInsert, MapRemove};
use proptest::prelude::*;

#[test]
fn freeze_thaw_pairs_are_fixed() {
    let v: MutableVec<i32> = (0..5).collect();
    let f: FrozenVec<i32> = v.freeze();
    let _: MutableVec<i32> = f.thaw();

    let h: MutableHashMap<u8, u8> = [(1, 1)].into();
    let fh: FrozenHashMap<u8, u8> = h.freeze();
    let _: MutableHashMap<u8, u8> = fh.thaw();

    let o: MutableOrderedMap<u8, u8> = [(1, 1)].into();
    let fo: FrozenOrderedMap<u8, u8> = o.freeze();
    let _: MutableOrderedMap<u8, u8> = fo.thaw();

    let iv: InlineVec<u8, 4> = [1, 2].into_iter().collect();
    let fi: InlineFrozenVec<u8, 4> = iv.freeze();
    let _: InlineVec<u8, 4> = fi.thaw();

    let ah: AdaptiveHashMap<u8, u8> = [(1, 1)].into_iter().collect();
    let fah: AdaptiveFrozenHashMap<u8, u8> = ah.freeze();
    let _: AdaptiveHashMap<u8, u8> = fah.thaw();

    let ao: AdaptiveOrderedMap<u8, u8> = [(1, 1)].into_iter().collect();
    let fao: FrozenOrderedMap<u8, u8> = ao.freeze();
    let _ = AdaptiveOrderedMap::from_frozen(fao, AdaptiveOrdered::default());
}

#[test]
fn mutable_ordered_from_iter_keeps_first_key_representative() {
    #[derive(Clone, Copy, Debug)]
    struct K(u8, u8);
    impl PartialEq for K {
        fn eq(&self, o: &Self) -> bool {
            self.0 == o.0
        }
    }
    impl Eq for K {}
    impl PartialOrd for K {
        fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(o))
        }
    }
    impl Ord for K {
        fn cmp(&self, o: &Self) -> std::cmp::Ordering {
            self.0.cmp(&o.0)
        }
    }
    let input = [(K(1, 10), 'a'), (K(1, 11), 'b')];
    let ours: MutableOrderedMap<K, char> = input.into_iter().collect();
    let adaptive: AdaptiveOrderedMap<K, char> = input.into_iter().collect();
    assert_eq!(
        ours.first_key_value().map(|(k, v)| (k.1, *v)),
        Some((10, 'b'))
    );
    assert_eq!(
        adaptive.first_key_value().map(|(k, v)| (k.1, *v)),
        Some((10, 'b'))
    );
}

#[test]
fn entry_apis_hide_the_backend() {
    let mut h: MutableHashMap<&str, u32> = MutableHashMap::new();
    for w in ["a", "b", "a"] {
        *h.entry(w).or_default() += 1;
    }
    assert_eq!(h["a"], 2);
    let mut o: MutableOrderedMap<&str, u32> = MutableOrderedMap::new();
    o.entry("x").and_modify(|v| *v += 1).or_insert(7);
    o.entry("x").and_modify(|v| *v += 1).or_insert(7);
    assert_eq!(o["x"], 8);
}

#[test]
fn inline_vec_spills_once_and_demotes_only_explicitly() {
    let mut v: InlineVec<String, 2> = InlineVec::new();
    v.push("a".into());
    v.push("b".into());
    assert!(v.is_inline());
    v.push("c".into());
    assert!(!v.is_inline());
    v.pop();
    assert!(!v.is_inline(), "no oscillation at the boundary");
    v.shrink_to_fit();
    assert!(v.is_inline());
    v.insert(0, "z".into());
    assert_eq!(v, ["z", "a", "b"].map(String::from));
    assert_eq!(v.remove(1), "a");
    v.retain(|s| s != "z");
    assert_eq!(v, ["b".to_owned()]);
}

#[test]
fn adaptive_hash_promotes_on_new_key_only() {
    let mut m: AdaptiveHashMap<u8, u8> =
        AdaptiveHashMap::with_policy(AdaptiveHash::linear_up_to(2));
    m.insert(1, 1);
    m.insert(2, 2);
    m.insert(2, 20); // replacement: no promotion
    assert_eq!(m.representation(), HashRepr::Linear);
    m.insert(3, 3);
    assert_eq!(m.representation(), HashRepr::Table);
    m.remove(&3);
    assert_eq!(m.representation(), HashRepr::Table);
    m.shrink_to_fit();
    assert_eq!(m.representation(), HashRepr::Linear);
    assert_eq!(m.get(&2), Some(&20));
}

#[test]
fn adaptive_ordered_promotes_and_keeps_order() {
    let mut m = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(3));
    for k in [5, 1, 3] {
        m.insert(k, k * 10);
    }
    assert_eq!(m.representation(), OrderedRepr::Sorted);
    m.insert(4, 40);
    assert_eq!(m.representation(), OrderedRepr::Tree);
    assert_eq!(m.keys().copied().collect::<Vec<_>>(), [1, 3, 4, 5]);
    assert_eq!(m.pop_first(), Some((1, 10)));
    m.shrink_to_fit();
    assert_eq!(m.representation(), OrderedRepr::Sorted);
}

fn generic_upsert<M>(m: &mut M, k: u8) -> Option<u32>
where
    M: MapInsert<Key = u8, Value = u32> + MapRemove<u8> + LookupRef<u8, Value = u32>,
{
    let old = m.get(&k).copied();
    m.insert(k, old.unwrap_or(0) + 1);
    if k.is_multiple_of(7) {
        m.remove(&k);
    }
    old
}

#[test]
fn write_capabilities_work_across_every_mutable_map() {
    let mut a = MutableHashMap::new();
    let mut b = MutableOrderedMap::new();
    let mut c: AdaptiveHashMap<u8, u32> = AdaptiveHashMap::new();
    let mut d = AdaptiveOrderedMap::new();
    let mut e = HashMap::new();
    for k in (0..50u8).chain(0..50) {
        let r = generic_upsert(&mut e, k);
        assert_eq!(generic_upsert(&mut a, k), r);
        assert_eq!(generic_upsert(&mut b, k), r);
        assert_eq!(generic_upsert(&mut c, k), r);
        assert_eq!(generic_upsert(&mut d, k), r);
    }
    let sum = |m: &dyn Fn() -> u32| m();
    let total: u32 = e.values().sum();
    assert_eq!(sum(&|| a.entries().map(|(_, v)| *v).sum()), total);
    assert_eq!(sum(&|| d.entries().map(|(_, v)| *v).sum()), total);
}

#[derive(Debug, Clone)]
enum Op {
    Insert(u8, u16),
    Remove(u8),
    Shrink,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        6 => (any::<u8>(), any::<u16>()).prop_map(|(k, v)| Op::Insert(k % 40, v)),
        3 => any::<u8>().prop_map(|k| Op::Remove(k % 40)),
        1 => Just(Op::Shrink),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    #[test]
    fn adaptive_maps_match_std_under_random_ops(ops in prop::collection::vec(op(), 0..200),
                                                th in 0usize..12, ts in 0usize..12) {
        let mut h_ref = HashMap::new();
        let mut b_ref = BTreeMap::new();
        let mut h: AdaptiveHashMap<u8, u16> = AdaptiveHashMap::with_policy(AdaptiveHash::linear_up_to(th));
        let mut o = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(ts));
        for op in ops {
            match op {
                Op::Insert(k, v) => {
                    prop_assert_eq!(h.insert(k, v), h_ref.insert(k, v));
                    prop_assert_eq!(o.insert(k, v), b_ref.insert(k, v));
                }
                Op::Remove(k) => {
                    prop_assert_eq!(h.remove(&k), h_ref.remove(&k));
                    prop_assert_eq!(o.remove(&k), b_ref.remove(&k));
                }
                Op::Shrink => {
                    h.shrink_to_fit();
                    o.shrink_to_fit();
                }
            }
            prop_assert_eq!(h.len(), h_ref.len());
            prop_assert!(o.iter().eq(b_ref.iter()));
        }
        for k in 0..40 {
            prop_assert_eq!(h.get(&k), h_ref.get(&k));
            prop_assert_eq!(o.range(k..).next(), b_ref.range(k..).next());
        }
        prop_assert_eq!(h.freeze().into_hash_map(), h_ref);
        prop_assert_eq!(o.freeze().into_btree_map(), b_ref);
    }

    #[test]
    fn inline_vec_matches_vec_under_random_ops(ops in prop::collection::vec((0u8..5, any::<u8>()), 0..100)) {
        let mut r: Vec<u8> = Vec::new();
        let mut v: InlineVec<u8, 4> = InlineVec::new();
        for (op, x) in ops {
            match op {
                0 | 1 => { r.push(x); v.push(x); }
                2 => { prop_assert_eq!(v.pop(), r.pop()); }
                3 if !r.is_empty() => {
                    let i = usize::from(x) % r.len();
                    prop_assert_eq!(v.remove(i), r.remove(i));
                }
                3 => {}
                _ => {
                    let i = usize::from(x) % (r.len() + 1);
                    r.insert(i, x);
                    v.insert(i, x);
                }
            }
            prop_assert_eq!(v.as_slice(), r.as_slice());
        }
        v.retain(|x| x % 2 == 0);
        r.retain(|x| x % 2 == 0);
        prop_assert_eq!(v.freeze().into_vec(), r);
    }
}

// --- regressions from the second adversarial review -------------------------

#[test]
fn append_keeps_self_keys_on_every_toolchain() {
    #[derive(Clone, Debug)]
    struct K(u8, &'static str);
    impl PartialEq for K {
        fn eq(&self, o: &Self) -> bool {
            self.0 == o.0
        }
    }
    impl Eq for K {}
    impl PartialOrd for K {
        fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(o))
        }
    }
    impl Ord for K {
        fn cmp(&self, o: &Self) -> std::cmp::Ordering {
            self.0.cmp(&o.0)
        }
    }
    let mut a: MutableOrderedMap<K, u8> = [(K(1, "self"), 1)].into();
    let mut b: MutableOrderedMap<K, u8> = [(K(1, "other"), 2), (K(2, "new"), 3)].into();
    a.append(&mut b);
    assert!(b.is_empty());
    let (k, v) = a.first_key_value().unwrap();
    assert_eq!((k.1, *v), ("self", 2));
    assert_eq!(a.len(), 2);
}

#[test]
fn inline_retain_panic_keeps_unvisited_elements_like_vec() {
    let mut iv: InlineVec<String, 4> = ["a", "b", "c"].map(String::from).into_iter().collect();
    let mut v: Vec<String> = iv.as_slice().to_vec();
    let boom = |s: &String| {
        assert!(s != "b", "boom");
        true
    };
    let r1 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| iv.retain(boom)));
    let r2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| v.retain(boom)));
    assert!(r1.is_err() && r2.is_err());
    assert_eq!(iv, v);
    assert_eq!(iv.len(), 3);
}

#[test]
fn inline_reserve_overflow_panics_like_vec() {
    let r = std::panic::catch_unwind(|| {
        let mut iv: InlineVec<u8, 4> = InlineVec::new();
        iv.push(1);
        iv.reserve(usize::MAX);
    });
    let msg = r.unwrap_err();
    let msg = msg
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| msg.downcast_ref::<&str>().copied())
        .unwrap_or_default();
    assert!(msg.contains("capacity overflow"), "{msg}");
}

#[test]
fn adaptive_api_parity() {
    let mut h: AdaptiveHashMap<&str, u8> = [("a", 1)].into();
    h.reserve(100); // promotes: linear table cannot hold 101 under the default policy
    assert_eq!(h.representation(), HashRepr::Table);
    assert_eq!(h["a"], 1);
    h.extend([(&"b", &2)]);
    assert_eq!(h.keys().count(), 2);
    let f = h.freeze();
    assert_eq!(f["b"], 2);
    assert_eq!(f.values().copied().sum::<u8>(), 3);

    let mut o: AdaptiveOrderedMap<u8, u8> = [(2, 20), (1, 10)].into();
    *o.get_or_insert_with(3, || 0) += 30;
    for (_, v) in o.range_mut(2..) {
        *v += 1;
    }
    for v in o.values_mut() {
        *v += 1;
    }
    assert_eq!(o.values().copied().collect::<Vec<_>>(), [11, 22, 32]);
    assert_eq!(o[&3], 32);
    assert_eq!(o.into_iter().map(|(k, _)| k).collect::<Vec<_>>(), [1, 2, 3]);

    let iv: InlineVec<u8, 2> = [1, 2].into_iter().collect();
    let fv = iv.clone().freeze();
    assert_eq!(iv, fv);
    assert_eq!(fv, iv);
}

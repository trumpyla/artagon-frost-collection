//! Semantic-contract tests: representatives, duplicates, order, hashing
//! policy, recoverable failure, capability-generic algorithms.
#![cfg(feature = "std")]
#![allow(
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::reversed_empty_ranges,
    reason = "test code; reversed range is the case under test"
)]

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::hash::{BuildHasherDefault, DefaultHasher, Hash, Hasher};

use frost::adaptive::HashRepr;
use frost::family::DuplicateKey;
use frost::prelude::*;
use frost::write::{MapInsert, Push, TryPush};

/// Key whose identity (`tag`) is invisible to `Eq`/`Ord`/`Hash`.
#[derive(Clone, Copy, Debug)]
struct Tagged {
    id: u8,
    tag: u8,
}
impl PartialEq for Tagged {
    fn eq(&self, o: &Self) -> bool {
        self.id == o.id
    }
}
impl Eq for Tagged {}
impl PartialOrd for Tagged {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Tagged {
    fn cmp(&self, o: &Self) -> Ordering {
        self.id.cmp(&o.id)
    }
}
impl Hash for Tagged {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.id.hash(h);
    }
}
impl Borrow<u8> for Tagged {
    fn borrow(&self) -> &u8 {
        &self.id
    }
}

fn t(id: u8, tag: u8) -> Tagged {
    Tagged { id, tag }
}

#[test]
fn family_builders_keep_first_key_last_value() {
    let input = [(t(1, 10), "a"), (t(2, 20), "b"), (t(1, 11), "c")];

    let ordered: FrozenOrderedMap<Tagged, &str> = input.into_iter().collect();
    let (k, v) = ordered.get_key_value(&1).unwrap();
    assert_eq!((k.tag, *v), (10, "c"));

    let hashed: FrozenHashMap<Tagged, &str> = input.into_iter().collect();
    let (k, v) = hashed.get_key_value(&1).unwrap();
    assert_eq!((k.tag, *v), (10, "c"));

    let adaptive: AdaptiveFrozenHashMap<Tagged, &str> = input.into_iter().collect();
    let (k, v) = adaptive.get_key_value(&1).unwrap();
    assert_eq!((k.tag, *v), (10, "c"));

    let set: FrozenOrderedSet<Tagged> = [t(3, 1), t(3, 2), t(1, 9)].into_iter().collect();
    assert_eq!(set.get(&3).unwrap().tag, 1);
}

#[test]
fn sealing_preserves_already_stored_representatives() {
    let mut b = BTreeMap::new();
    b.insert(t(1, 10), 1);
    b.insert(t(1, 11), 2); // BTreeMap::insert keeps the stored key
    let compact = b.compact();
    assert_eq!(
        compact.get_key_value(&1).map(|(k, v)| (k.tag, *v)),
        Some((10, 2))
    );
}

#[test]
fn strict_builders_return_the_rejected_entry() {
    let err = FrozenOrderedMap::try_from_unique([(t(5, 1), 'x'), (t(5, 2), 'y')]).unwrap_err();
    assert_eq!((err.key.tag, err.value), (2, 'y'));

    let err: DuplicateKey<Tagged, char> =
        FrozenHashMap::<Tagged, char>::try_from_unique([(t(5, 1), 'x'), (t(5, 2), 'y')])
            .unwrap_err();
    assert_eq!((err.key.tag, err.value), (2, 'y'));

    assert!(FrozenOrderedMap::try_from_unique([(1, ()), (2, ())]).is_ok());
}

#[test]
fn recoverable_failure_returns_source_unchanged() {
    let v = vec![String::from("a"), String::from("b")];
    let ptr = v.as_ptr();
    let err = v.try_freeze_with(ExactLen::<3>).unwrap_err();
    assert_eq!((err.error.expected, err.error.actual), (3, 2));
    assert_eq!(err.input.as_ptr(), ptr, "same allocation handed back");

    let ok = err.input.try_freeze_with(ExactLen::<2>).unwrap();
    assert_eq!(ok.as_array(), &["a", "b"]);

    let bad = vec![(1, 'a'), (1, 'b')];
    let rej = FrozenOrderedMap::try_from_sorted_unique(bad.clone()).unwrap_err();
    assert_eq!(rej.input, bad);
    assert_eq!(rej.error.index, 1);
}

#[test]
fn seal_is_o1_and_keeps_capacity_compact_drops_it() {
    let mut v = Vec::with_capacity(64);
    v.extend([1, 2, 3]);
    let ptr = v.as_ptr();
    let sealed = v.seal();
    assert_eq!((sealed.as_ptr(), sealed.capacity()), (ptr, 64));
    let compact: FrozenVec<i32> = sealed.compact();
    assert_eq!(&*compact, &[1, 2, 3]);

    // len == capacity: no reallocation.
    let exact = vec![7u64; 5];
    let ptr = exact.as_ptr();
    let f: FrozenVec<u64> = exact.compact();
    assert_eq!(f.as_ptr(), ptr);
    let back = f.thaw();
    assert_eq!((back.as_ptr(), back.capacity()), (ptr, 5));
}

#[test]
fn every_std_family_has_a_retaining_freeze() {
    assert_eq!(vec![1].seal().len(), 1);
    assert_eq!(vec![1].into_boxed_slice().seal().len(), 1);
    assert_eq!([1, 2].seal().len(), 2);

    let d: VecDeque<_> = (0..5).collect();
    let fd = d.seal();
    assert_eq!((fd.front(), fd.back(), fd[2]), (Some(&0), Some(&4), 2));
    let (a, b) = fd.as_slices();
    assert_eq!([a, b].concat(), [0, 1, 2, 3, 4]);

    let l: LinkedList<_> = (0..3).collect();
    assert_eq!(l.seal().iter().copied().collect::<Vec<_>>(), [0, 1, 2]);

    let h = HashMap::from([("k", 1)]).seal();
    assert_eq!(h["k"], 1);
    let s = HashSet::from([1, 2]).seal();
    assert!(s.contains(&1));

    let b = BTreeMap::from([(2, 'b'), (1, 'a')]).seal();
    assert_eq!(b.first_key_value(), Some((&1, &'a')));
    let bs = BTreeSet::from([3, 1]).seal();
    assert_eq!(bs.range(2..).copied().collect::<Vec<_>>(), [3]);

    let heap = BinaryHeap::from([1, 5, 3]).seal();
    assert_eq!(heap.peek(), Some(&5));
}

#[test]
fn heap_order_vs_sorted_order_is_explicit() {
    let h = BinaryHeap::from([4, 1, 3, 2]);
    let sorted: FrozenVec<i32> = h.clone().freeze_with(Sorted);
    assert_eq!(&*sorted, &[1, 2, 3, 4]);
    let retained = h.seal();
    assert_eq!(retained.as_slice().len(), 4); // storage order; not asserted sorted
}

#[test]
fn deque_compact_is_contiguous_in_logical_order() {
    let mut d = VecDeque::with_capacity(4);
    d.extend([2, 3]);
    d.push_front(1);
    d.push_front(0); // likely wrapped ring
    let f: FrozenVec<i32> = d.compact();
    assert_eq!(&*f, &[0, 1, 2, 3]);
}

#[test]
fn ordered_range_matches_btree_including_panics() {
    let b: BTreeMap<i32, i32> = (0..20).map(|i| (i * 2, i)).collect();
    let f = b.clone().compact();
    for (lo, hi) in [(0, 0), (3, 9), (-5, 100), (8, 8), (39, 40)] {
        assert!(b.range(lo..hi).eq(f.range(lo..hi)));
        assert!(b.range(lo..=hi).eq(f.range(lo..=hi)));
        assert!(b.range(lo..).rev().eq(f.range(lo..).rev()));
    }
    let r = std::panic::catch_unwind(|| f.range(5..3).count());
    assert!(r.is_err());
    use std::ops::Bound::Excluded;
    let r = std::panic::catch_unwind(|| f.range((Excluded(4), Excluded(4))).count());
    assert!(r.is_err());
}

#[test]
fn borrowed_string_queries() {
    let m: BTreeMap<String, u8> = [("alpha".into(), 1), ("beta".into(), 2)].into();
    let f = m.compact();
    assert_eq!(f.get("beta"), Some(&2));
    assert_eq!(f["alpha"], 1);
    use std::ops::Bound::{Included, Unbounded};
    let r: Vec<_> = f.range::<str, _>((Included("b"), Unbounded)).collect();
    assert_eq!(r.len(), 1);
}

#[test]
fn adaptive_representations_are_logically_equal_and_keep_hasher() {
    type S = BuildHasherDefault<DefaultHasher>;
    let src: HashMap<u32, u32, S> = (0..6).map(|i| (i, i * i)).collect();
    let lin = src.clone().freeze_with(AdaptiveHash::linear_up_to(8));
    let tab = src.clone().freeze_with(AdaptiveHash::linear_up_to(2));
    assert_eq!(lin.representation(), HashRepr::Linear);
    assert_eq!(tab.representation(), HashRepr::Table);
    assert_eq!(lin, tab);
    assert_eq!(lin.iter().map(|(_, v)| v).sum::<u32>(), 55); // fold path
    let thawed: HashMap<u32, u32, S> = lin.into_hash_map();
    assert_eq!(thawed, src);

    let mut got = Vec::new();
    tab.for_each_lookup([1, 9, 5].iter(), |q, v| got.push((*q, v.copied())));
    assert_eq!(got, [(1, Some(1)), (9, None), (5, Some(25))]);
}

#[test]
fn inline_vec_is_canonical_for_equality_and_hash() {
    use std::hash::BuildHasher;
    let a: InlineFrozenVec<u8, 4> = vec![1, 2].freeze_with(Inline::<4>);
    let b: InlineFrozenVec<u8, 1> = vec![1, 2].freeze_with(Inline::<1>);
    assert!(a.is_inline() && !b.is_inline());
    assert_eq!(a, b);
    let s = std::hash::RandomState::new();
    let c: InlineFrozenVec<u8, 1> = vec![1, 2].freeze_with(Inline::<1>);
    let a1: InlineFrozenVec<u8, 1> = InlineFrozenVec::from_vec(vec![1, 2]);
    assert_eq!(s.hash_one(&a1), s.hash_one(&c));
    assert_eq!(s.hash_one(a.as_slice()), s.hash_one(&c));
}

// --- capability-generic algorithms ----------------------------------------

fn sum_values<M>(m: &M) -> u64
where
    M: Entries<Value = u32>,
{
    m.entries().map(|(_, v)| u64::from(*v)).sum()
}

fn count_hits<M, Q>(m: &M, qs: &[&Q]) -> usize
where
    M: LookupRef<Q>,
    Q: ?Sized,
{
    qs.iter().filter(|q| m.contains_key(q)).count()
}

fn checksum<C: AsRef<[u32]> + ?Sized>(c: &C) -> u64 {
    c.as_ref().iter().map(|&x| u64::from(x)).sum()
}

#[test]
fn algorithms_accept_mutable_and_frozen_alike() {
    let h: HashMap<String, u32> = [("a".into(), 1), ("b".into(), 2)].into();
    let b: BTreeMap<String, u32> = h.clone().into_iter().collect();
    assert_eq!(sum_values(&h), 3);
    assert_eq!(sum_values(&h.clone().seal()), 3);
    assert_eq!(sum_values(&h.clone().compact()), 3);
    assert_eq!(sum_values(&b.clone().compact()), 3);
    assert_eq!(
        sum_values(&h.clone().freeze_with(AdaptiveHash::default())),
        3
    );

    let qs: [&str; 3] = ["a", "zz", "b"];
    assert_eq!(count_hits::<_, str>(&h, &qs), 2);
    assert_eq!(count_hits::<_, str>(&b.compact(), &qs), 2);

    let v = vec![1u32, 2, 3];
    assert_eq!(checksum(&v), 6);
    assert_eq!(checksum(&v.clone().seal()), 6);
    assert_eq!(checksum(&v.clone().compact()), 6);
    assert_eq!(checksum(&v.freeze_with(Inline::<8>)), 6);
}

#[test]
fn erased_lookup_is_dyn_compatible() {
    let a: FrozenOrderedMap<String, u32> = [("x".to_owned(), 1)].into_iter().collect();
    let b: FrozenHashMap<String, u32> = [("y".to_owned(), 2)].into_iter().collect();
    let maps: [&dyn LookupRef<str, Key = String, Value = u32>; 2] = [&a, &b];
    assert_eq!(maps.iter().filter_map(|m| m.get("y")).sum::<u32>(), 2);

    let set: FrozenOrderedSet<u32> = [3, 1, 2].into_iter().collect();
    let bset = BTreeSet::from([9u32]).seal();
    let sets: [&dyn SetGet<u32, Item = u32>; 2] = [&set, &bset];
    assert_eq!(sets.iter().filter(|s| s.contains(&9)).count(), 1);
    let ord: &dyn OrderedElements<Item = u32> = &set;
    assert_eq!(ord.last_element(), Some(&3));
    let dq = VecDeque::from([1u32, 2]).seal();
    let d: &dyn DequeRead<Item = u32> = &dq;
    assert_eq!(d.back(), Some(&2));
}

#[test]
fn mutation_traits_on_backends() {
    let mut v = Vec::new();
    Push::push(&mut v, 1);
    TryPush::try_push(&mut v, 2).unwrap();
    assert_eq!(v, [1, 2]);

    let mut m = BTreeMap::new();
    assert_eq!(MapInsert::insert(&mut m, t(1, 1), 'a'), None);
    assert_eq!(MapInsert::insert(&mut m, t(1, 2), 'b'), Some('a'));
    assert_eq!(m.keys().next().unwrap().tag, 1, "stored key kept");
}

#[cfg(feature = "hashbrown-0_17")]
#[test]
fn hashbrown_adapter_uses_equivalent() {
    let mut m: hashbrown::HashMap<String, i32> = hashbrown::HashMap::new();
    m.insert("a".into(), 1);
    let f = m.seal();
    assert_eq!(f.get("a"), Some(&1));
    assert_eq!(count_hits::<_, str>(&f, &["a", "b"]), 1);
}

#[cfg(feature = "indexmap-2")]
#[test]
fn indexmap_adapter_keeps_insertion_order_and_positions() {
    let mut m = indexmap::IndexMap::new();
    m.insert("z", 1);
    m.insert("a", 2);
    m.insert("z", 3); // keeps position 0
    let f = m.compact();
    assert_eq!(f.get_index(0), Some((&"z", &3)));
    assert_eq!(f.get_index_of("a"), Some(1));
    assert_eq!(f.keys().copied().collect::<Vec<_>>(), ["z", "a"]);
}

#[test]
fn layout_report() {
    use std::mem::size_of;
    // Facts, not performance claims; printed so the design doc can cite them.
    println!("Vec<u32>                         {}", size_of::<Vec<u32>>());
    println!(
        "Frozen<Vec<u32>>                 {}",
        size_of::<Frozen<Vec<u32>>>()
    );
    println!(
        "FrozenVec<u32>                   {}",
        size_of::<FrozenVec<u32>>()
    );
    println!(
        "InlineFrozenVec<u32, 2>          {}",
        size_of::<InlineFrozenVec<u32, 2>>()
    );
    println!(
        "InlineFrozenVec<u32, 4>          {}",
        size_of::<InlineFrozenVec<u32, 4>>()
    );
    println!(
        "InlineFrozenVec<u64, 8>          {}",
        size_of::<InlineFrozenVec<u64, 8>>()
    );
    println!(
        "HashMap<u32,u32>                 {}",
        size_of::<HashMap<u32, u32>>()
    );
    println!(
        "FrozenHashMap<u32,u32>           {}",
        size_of::<FrozenHashMap<u32, u32>>()
    );
    println!(
        "AdaptiveFrozenHashMap<u32,u32>   {}",
        size_of::<AdaptiveFrozenHashMap<u32, u32>>()
    );
    println!(
        "FrozenOrderedMap<u32,u32>        {}",
        size_of::<FrozenOrderedMap<u32, u32>>()
    );
    println!(
        "BTreeMap<u32,u32>                {}",
        size_of::<BTreeMap<u32, u32>>()
    );
    println!("MutableVec<u32> {}", size_of::<MutableVec<u32>>());
    println!("InlineVec<u32, 4> {}", size_of::<InlineVec<u32, 4>>());
    println!(
        "MutableHashMap<u32,u32> {}",
        size_of::<MutableHashMap<u32, u32>>()
    );
    println!(
        "AdaptiveHashMap<u32,u32> {}",
        size_of::<AdaptiveHashMap<u32, u32>>()
    );
    println!(
        "MutableOrderedMap<u32,u32> {}",
        size_of::<MutableOrderedMap<u32, u32>>()
    );
    println!(
        "AdaptiveOrderedMap<u32,u32> {}",
        size_of::<AdaptiveOrderedMap<u32, u32>>()
    );
    assert_eq!(size_of::<Frozen<Vec<u32>>>(), size_of::<Vec<u32>>());
    assert_eq!(size_of::<FrozenVec<u32>>(), 2 * size_of::<usize>());
}

// Auto traits propagate; no profile variant narrows them.
const _: () = {
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<Frozen<Vec<String>>>();
    send_sync::<FrozenVec<String>>();
    send_sync::<InlineFrozenVec<String, 4>>();
    send_sync::<AdaptiveFrozenHashMap<String, String>>();
    send_sync::<FrozenOrderedMap<String, String>>();
};

// --- regressions from the adversarial review ------------------------------

#[test]
fn empty_ordered_range_never_panics_like_btree() {
    use std::ops::Bound::Excluded;
    let empty_b: BTreeMap<i32, i32> = BTreeMap::new();
    let empty_f: FrozenOrderedMap<i32, i32> = FrozenOrderedMap::default();
    assert_eq!(empty_b.range(5..1).count(), 0);
    assert_eq!(empty_f.range(5..1).count(), 0);
    assert_eq!(empty_f.range((Excluded(3), Excluded(3))).count(), 0);
    let empty_s: FrozenOrderedSet<i32> = FrozenOrderedSet::default();
    assert_eq!(empty_s.range(5..1).count(), 0);
}

/// `last`, `nth`, `min`, `max`, and `count` are forwarded to the backend, which
/// answers them from one end. They must agree with a complete traversal for
/// every wrapper, from the start and after partial consumption.
#[test]
fn iterator_end_queries_match_a_complete_traversal() {
    use frost::adaptive::{AdaptiveOrdered, AdaptiveOrderedMap, OrderedRepr};
    fn check<I>(iterator: &I, context: &str)
    where
        I: Iterator + Clone,
        I::Item: Ord + Clone + core::fmt::Debug,
    {
        for skipped in [0_usize, 1, 3] {
            let mut iterator = iterator.clone();
            for _ in 0..skipped {
                iterator.next();
            }
            let all: Vec<I::Item> = iterator.clone().collect();
            assert_eq!(iterator.clone().count(), all.len(), "{context}: count");
            assert_eq!(
                iterator.clone().last(),
                all.last().cloned(),
                "{context}: last"
            );
            assert_eq!(
                iterator.clone().min(),
                all.iter().cloned().min(),
                "{context}: min"
            );
            assert_eq!(
                iterator.clone().max(),
                all.iter().cloned().max(),
                "{context}: max"
            );
            for n in [0, 1, all.len().saturating_sub(1), all.len(), all.len() + 3] {
                let mut cursor = iterator.clone();
                assert_eq!(cursor.nth(n), all.get(n).cloned(), "{context}: nth({n})");
                let rest: Vec<I::Item> = cursor.collect();
                assert_eq!(
                    rest,
                    all[(n + 1).min(all.len())..],
                    "{context}: after nth({n})"
                );
            }
        }
    }
    for size in [0_u32, 1, 2, 7, 40] {
        let entries: Vec<(u32, u32)> = (0..size).map(|key| (key * 3, 1_000 - key)).collect();
        let ordered: MutableOrderedMap<u32, u32> = entries.iter().copied().collect();
        check(&ordered.iter(), "MutableOrderedMap::iter");
        check(&ordered.keys(), "MutableOrderedMap::keys");
        check(&ordered.values(), "MutableOrderedMap::values");
        check(&ordered.range(3..90), "MutableOrderedMap::range");
        let ordered_set: MutableOrderedSet<u32> = entries.iter().map(|(key, _)| *key).collect();
        check(&ordered_set.iter(), "MutableOrderedSet::iter");
        let frozen = ordered.clone().freeze();
        check(&frozen.iter(), "FrozenOrderedMap::iter");
        check(&frozen.keys(), "FrozenOrderedMap::keys");
        check(&frozen.values(), "FrozenOrderedMap::values");
        check(&frozen.range(3..90), "FrozenOrderedMap::range");
        for (policy, repr) in [
            (AdaptiveOrdered::sorted_up_to(64), OrderedRepr::Sorted),
            (AdaptiveOrdered::sorted_up_to(0), OrderedRepr::Tree),
        ] {
            let mut adaptive = AdaptiveOrderedMap::with_policy(policy);
            adaptive.extend(entries.iter().copied());
            if size > 0 {
                assert_eq!(adaptive.representation(), repr);
            }
            check(&adaptive.iter(), "AdaptiveOrderedMap::iter");
            check(&adaptive.keys(), "AdaptiveOrderedMap::keys");
            check(&adaptive.values(), "AdaptiveOrderedMap::values");
            check(&adaptive.range(3..90), "AdaptiveOrderedMap::range");
            check(&adaptive.range(..), "AdaptiveOrderedMap::range(..)");
        }
        let hashed: MutableHashMap<u32, u32> = entries.iter().copied().collect();
        check(&hashed.iter(), "MutableHashMap::iter");
        check(&hashed.keys(), "MutableHashMap::keys");
        check(&hashed.values(), "MutableHashMap::values");
        let hashed_set: MutableHashSet<u32> = entries.iter().map(|(key, _)| *key).collect();
        check(&hashed_set.iter(), "MutableHashSet::iter");
    }
}

/// Collecting into the mutable ordered families stages and sorts the input.
/// The result must equal entry-by-entry insertion: the first key
/// representative, the last value, ascending order, and the same members,
/// for sorted, reversed, and random arrival with many duplicate keys.
#[test]
fn collected_ordered_families_match_entry_by_entry_insertion() {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for case in 0..300 {
        let length = usize::try_from(next() % 400).unwrap();
        let keys = 1 + next() % 60;
        let mut input: Vec<(Tagged, u32)> = (0..length)
            .map(|index| {
                let id = u8::try_from(next() % keys).unwrap();
                let tag = u8::try_from(index % 251).unwrap();
                (t(id, tag), u32::try_from(index).unwrap())
            })
            .collect();
        match case % 3 {
            0 => input.sort_by_key(|(key, _)| key.id), // stable: arrival order kept
            1 => input.sort_by_key(|(key, _)| core::cmp::Reverse(key.id)),
            _ => {}
        }
        let mut reference: MutableOrderedMap<Tagged, u32> = MutableOrderedMap::new();
        let mut reference_set: MutableOrderedSet<Tagged> = MutableOrderedSet::new();
        for &(key, value) in &input {
            reference.insert(key, value);
            reference_set.insert(key);
        }
        let collected: MutableOrderedMap<Tagged, u32> = input.iter().copied().collect();
        let collected_set: MutableOrderedSet<Tagged> = input.iter().map(|(key, _)| *key).collect();
        let entries = |map: &MutableOrderedMap<Tagged, u32>| {
            map.iter()
                .map(|(key, value)| (key.id, key.tag, *value))
                .collect::<Vec<_>>()
        };
        let members = |set: &MutableOrderedSet<Tagged>| {
            set.iter().map(|key| (key.id, key.tag)).collect::<Vec<_>>()
        };
        assert_eq!(entries(&collected), entries(&reference), "case {case}");
        assert_eq!(
            members(&collected_set),
            members(&reference_set),
            "case {case}"
        );
        // The collected tree is an ordinary tree afterwards.
        let mut grown = collected;
        grown.insert(t(200, 1), 7);
        assert_eq!(grown.len(), reference.len() + 1);
        assert_eq!(grown.last_key_value().map(|(key, _)| key.id), Some(200));
    }
}

/// Bulk extension must keep the contracts of entry-by-entry insertion.
#[test]
fn bulk_extension_keeps_representatives_policies_and_partial_progress() {
    use frost::adaptive::{AdaptiveHash, AdaptiveHashMap, InlineVec};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    // Hash set: the first representative stays, with or without earlier
    // members, and the reservation is only a lower bound on capacity.
    let mut set: MutableHashSet<Tagged> = MutableHashSet::default();
    set.extend([t(1, 10), t(2, 20), t(1, 11)]);
    set.extend((0..200).map(|id| t(id, 99)));
    assert_eq!(set.len(), 200);
    assert_eq!(set.get(&1_u8).map(|key| key.tag), Some(10));
    assert_eq!(set.get(&2_u8).map(|key| key.tag), Some(20));
    assert_eq!(set.get(&3_u8).map(|key| key.tag), Some(99));
    assert!(set.capacity() >= set.len());
    let collected: MutableHashSet<Tagged> = [t(5, 1), t(5, 2)].into_iter().collect();
    assert_eq!(collected.get(&5_u8).map(|key| key.tag), Some(1));

    // Adaptive hash map: promotion depends on distinct keys. A long input of
    // duplicate keys must stay linear; one key more than the policy promotes.
    let tagged = |index: u32, keys: u32| {
        t(
            u8::try_from(index % keys).unwrap(),
            u8::try_from(index % 251).unwrap(),
        )
    };
    let policy = AdaptiveHash::linear_up_to(4);
    let mut linear = AdaptiveHashMap::with_policy(policy);
    linear.extend((0..1_000_u32).map(|index| (tagged(index, 4), index)));
    assert_eq!(linear.representation(), HashRepr::Linear);
    assert_eq!(linear.len(), 4);
    assert_eq!(
        linear
            .get_key_value(&1_u8)
            .map(|(key, value)| (key.tag, *value)),
        Some((1, 997))
    );
    let mut table = AdaptiveHashMap::with_policy(policy);
    table.extend((0..1_000_u32).map(|index| (tagged(index, 5), index)));
    assert_eq!(table.representation(), HashRepr::Table);
    assert_eq!(table.len(), 5);
    assert_eq!(
        table
            .get_key_value(&4_u8)
            .map(|(key, value)| (key.tag, *value)),
        Some((4, 999))
    );
    let mut grown = AdaptiveHashMap::with_policy(policy);
    grown.extend((0..5_000_u32).map(|index| (index, index + 1)));
    grown.extend((0..5_000_u32).map(|index| (index, index + 2)));
    assert_eq!(grown.len(), 5_000);
    assert!((0..5_000).all(|index| grown.get(&index) == Some(&(index + 2))));

    // Inline sequence: inline, crossing the inline limit, and spilled appends
    // all match `Vec`, including the elements kept when a clone panics.
    for start in [0_u32, 2, 4, 9] {
        let mut inline: InlineVec<u32, 4> = InlineVec::new();
        let mut expected = Vec::new();
        for value in 0..start {
            inline.push(value);
            expected.push(value);
        }
        for chunk in [&[][..], &[7], &[8, 9, 10], &[11; 40]] {
            inline.extend_from_slice(chunk);
            expected.extend_from_slice(chunk);
            assert_eq!(inline.as_slice(), expected.as_slice());
        }
    }
    #[derive(Debug, PartialEq)]
    struct Fragile(u32);
    impl Clone for Fragile {
        fn clone(&self) -> Self {
            assert!(self.0 != 3, "clone refused");
            Self(self.0)
        }
    }
    let source: Vec<Fragile> = (0..6).map(Fragile).collect();
    for start in [0_u32, 10] {
        let mut inline: InlineVec<Fragile, 4> = InlineVec::new();
        let mut expected = Vec::new();
        for value in 100..100 + start {
            inline.push(Fragile(value));
            expected.push(Fragile(value));
        }
        assert!(catch_unwind(AssertUnwindSafe(|| inline.extend_from_slice(&source))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| expected.extend_from_slice(&source))).is_err());
        assert_eq!(
            inline.as_slice(),
            expected.as_slice(),
            "clones before the panic are kept"
        );
    }
}

/// A std tree emptied by removal keeps its root and validates range bounds,
/// unlike `BTreeMap::new()`. The adaptive map documents that an empty map
/// never panics, so its Tree form must not expose that difference: both
/// representations answer an inverted or empty-excluded range with nothing.
#[test]
fn adaptive_ordered_range_on_an_emptied_tree_matches_the_sorted_form() {
    use frost::adaptive::{AdaptiveOrdered, AdaptiveOrderedMap, OrderedRepr};
    use std::ops::Bound::Excluded;
    let emptied = |policy: AdaptiveOrdered, how: usize| {
        let mut map = AdaptiveOrderedMap::with_policy(policy);
        map.insert(1, 10);
        map.insert(2, 20);
        match how {
            0 => {
                map.remove(&1);
                map.remove(&2);
            }
            1 => {
                map.pop_first();
                map.pop_last();
            }
            _ => map.retain(|_, _| false),
        }
        assert!(map.is_empty());
        map
    };
    for how in 0..3 {
        let mut sorted = emptied(AdaptiveOrdered::sorted_up_to(4), how);
        let mut tree = emptied(AdaptiveOrdered::sorted_up_to(0), how);
        assert_eq!(sorted.representation(), OrderedRepr::Sorted);
        assert_eq!(tree.representation(), OrderedRepr::Tree);
        for map in [&mut sorted, &mut tree] {
            assert_eq!(map.range(5..1).count(), 0);
            assert_eq!(map.range((Excluded(3), Excluded(3))).count(), 0);
            assert_eq!(map.range_mut(5..1).count(), 0);
            assert_eq!(map.range_mut((Excluded(3), Excluded(3))).count(), 0);
            assert_eq!(map.range(..).next_back(), None);
            // The map is still usable, and a nonempty map validates bounds.
            map.insert(7, 70);
            assert_eq!(map.range(..).collect::<Vec<_>>(), [(&7, &70)]);
            assert_eq!(map.range_mut(7..=7).count(), 1);
        }
        assert_eq!(sorted, tree);
    }
}

/// `Ord` sees only `id`; `PartialEq` sees `id` and `tag` (inconsistent on
/// purpose). Families must dedup by `Ord`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OrdOnlyId {
    id: u8,
    tag: u8,
}
impl PartialOrd for OrdOnlyId {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for OrdOnlyId {
    fn cmp(&self, o: &Self) -> Ordering {
        self.id.cmp(&o.id)
    }
}

#[test]
fn ordered_families_dedup_by_ord_not_partial_eq() {
    let a = OrdOnlyId { id: 1, tag: 1 };
    let b = OrdOnlyId { id: 1, tag: 2 };
    let m: FrozenOrderedMap<OrdOnlyId, u8> = [(a, 10), (b, 20)].into_iter().collect();
    assert_eq!(m.len(), 1);
    assert_eq!(m.first_key_value(), Some((&a, &20)));
    // The crate accepts its own output.
    assert!(FrozenOrderedMap::try_from_sorted_unique(m.into_sorted_vec()).is_ok());
    let s: FrozenOrderedSet<OrdOnlyId> = [a, b].into_iter().collect();
    assert_eq!(s.iter().copied().collect::<Vec<_>>(), [a]);
}

#[test]
fn defaults_do_not_require_element_default() {
    struct NoDefault;
    assert!(FrozenVec::<NoDefault>::default().is_empty());
    assert!(FrozenHashMap::<u32, NoDefault>::default().is_empty());
    assert!(AdaptiveFrozenHashMap::<u32, NoDefault>::default().is_empty());
    assert!(FrozenOrderedMap::<u32, NoDefault>::default().is_empty());
}

#[test]
fn prelude_does_not_shadow_std_first_entry() {
    let mut m = BTreeMap::from([(1, 'a'), (2, 'b')]);
    // Would resolve to a `&self` trait method returning a tuple if the
    // capability trait reused the name `first_entry`.
    let e = m.first_entry().unwrap();
    assert_eq!(e.remove(), 'a');
    assert_eq!(OrderedEntries::last_key_value(&m), Some((&2, &'b')));
}

#[test]
fn rejected_input_and_slice_equality() {
    let r = vec![1, 2].try_freeze_with(ExactLen::<3>).unwrap_err();
    let (input, why) = r.into_parts();
    assert_eq!((input, why.actual), (vec![1, 2], 2));

    let f: FrozenVec<i32> = vec![1, 2].compact();
    assert_eq!(f, vec![1, 2]);
    assert_eq!(f, [1, 2]);
    assert_eq!(f, *[1, 2].as_slice());
    let i: InlineFrozenVec<i32, 2> = vec![1, 2].freeze_with(Inline::<2>);
    assert_eq!(i, vec![1, 2]);
}

thread_local! {
    static HASH_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// A key that counts how often it is hashed. A table that grows hashes every
/// stored key again, so the count shows a rehash without a timing assertion.
#[derive(Clone, Copy, PartialEq, Eq)]
struct CountedKey(u32);

impl Hash for CountedKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        HASH_CALLS.with(|calls| calls.set(calls.get() + 1));
        self.0.hash(state);
    }
}

fn hash_calls_during(run: impl FnOnce()) -> usize {
    HASH_CALLS.with(|calls| calls.set(0));
    run();
    HASH_CALLS.with(std::cell::Cell::get)
}

/// Extending a nonempty hash family with keys it already holds must not
/// reserve for all of them: that doubles the table and hashes every stored key
/// again. The standard collections reserve half the hint for a nonempty table.
#[test]
fn hash_extension_reserves_like_the_standard_collections() {
    for len in [100_u32, 1000, 1500, 5000, 40_000] {
        let keys = || (0..len).map(CountedKey);
        let entries = || keys().map(|key| (key, key.0));

        let mut native: HashMap<CountedKey, u32> = entries().collect();
        let mut map: MutableHashMap<CountedKey, u32> = entries().collect();
        assert_eq!(map.capacity(), native.capacity(), "{len}");
        let expected = hash_calls_during(|| native.extend(entries()));
        let actual = hash_calls_during(|| map.extend(entries()));
        assert_eq!(actual, expected, "map hash calls, {len}");
        assert_eq!(map.capacity(), native.capacity(), "map capacity, {len}");

        let mut native: HashSet<CountedKey> = keys().collect();
        let mut set: MutableHashSet<CountedKey> = keys().collect();
        assert_eq!(set.capacity(), native.capacity(), "{len}");
        let expected = hash_calls_during(|| native.extend(keys()));
        let actual = hash_calls_during(|| set.extend(keys()));
        assert_eq!(actual, expected, "set hash calls, {len}");
        assert_eq!(set.capacity(), native.capacity(), "set capacity, {len}");

        // Fresh keys still get the whole hint when the family starts empty.
        let mut native: HashMap<CountedKey, u32> = HashMap::new();
        let mut map: MutableHashMap<CountedKey, u32> = MutableHashMap::new();
        let expected = hash_calls_during(|| native.extend(entries()));
        let actual = hash_calls_during(|| map.extend(entries()));
        assert_eq!(actual, expected, "empty map hash calls, {len}");
    }

    // 1000 keys leave room for half as many again, so re-inserting them hashes
    // each key once in every hash family, including the adaptive table.
    let len = 1000_u32;
    let entries = || (0..len).map(|id| (CountedKey(id), id));
    let mut map: MutableHashMap<CountedKey, u32> = entries().collect();
    assert_eq!(hash_calls_during(|| map.extend(entries())), len as usize);
    let mut set: MutableHashSet<CountedKey> = (0..len).map(CountedKey).collect();
    assert_eq!(
        hash_calls_during(|| set.extend((0..len).map(CountedKey))),
        len as usize
    );
    let mut adaptive: AdaptiveHashMap<CountedKey, u32> = entries().collect();
    assert_eq!(adaptive.representation(), HashRepr::Table);
    assert_eq!(
        hash_calls_during(|| adaptive.extend(entries())),
        len as usize
    );
    assert_eq!(adaptive.len(), len as usize);
    assert!(entries().all(|(key, value)| adaptive.get(&key) == Some(&value)));
}

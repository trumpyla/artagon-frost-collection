//! Bounded differential and ownership obligations for public mutable APIs.
//! The exhaustive domain is five initial vectors and every three-step trace
//! over sixteen operations; this is not a proof over arbitrary programs.
#![cfg(feature = "alloc")]

use std::borrow::Borrow;
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::iter::FusedIterator;
use std::rc::Rc;

use frost::family::ordered::Entry;
use frost::prelude::*;
use frost::read::{Entries, Len, LookupRef};
use frost::write::{MapInsert, MapRemove};

fn vector_step(actual: &mut MutableVec<i32>, expected: &mut Vec<i32>, op: u8) {
    match op {
        0 => {
            actual.push(3);
            expected.push(3);
        }
        1 => {
            assert_eq!(actual.pop(), expected.pop());
        }
        2 => {
            actual.insert(actual.len() / 2, 4);
            expected.insert(expected.len() / 2, 4);
        }
        3 if !expected.is_empty() => {
            assert_eq!(
                actual.remove(actual.len() / 2),
                expected.remove(expected.len() / 2)
            );
        }
        4 if !expected.is_empty() => {
            assert_eq!(actual.swap_remove(0), expected.swap_remove(0));
        }
        5 => {
            actual.truncate(2);
            expected.truncate(2);
        }
        6 => {
            actual.retain(|v| v % 2 == 0);
            expected.retain(|v| v % 2 == 0);
        }
        7 => {
            actual.retain_mut(|v| {
                *v += 1;
                *v % 3 != 0
            });
            expected.retain_mut(|v| {
                *v += 1;
                *v % 3 != 0
            });
        }
        8 => {
            actual.dedup();
            expected.dedup();
        }
        9 => {
            actual.dedup_by_key(|v| *v % 2);
            expected.dedup_by_key(|v| *v % 2);
        }
        10 => {
            let mut actual_tail = actual.split_off(actual.len() / 2);
            let mut expected_tail = expected.split_off(expected.len() / 2);
            assert_eq!(actual_tail.as_slice(), expected_tail.as_slice());
            actual.append(&mut actual_tail);
            expected.append(&mut expected_tail);
            assert!(actual_tail.is_empty());
        }
        11 => {
            // Dropping a partially consumed drain must still remove the range.
            let start = expected.len() / 3;
            let end = expected.len();
            let mut actual_drain = actual.drain(start..end);
            let mut expected_drain = expected.drain(start..end);
            assert_eq!(actual_drain.len(), expected_drain.len());
            assert_eq!(actual_drain.next(), expected_drain.next());
            assert_eq!(actual_drain.next_back(), expected_drain.next_back());
            assert_eq!(actual_drain.size_hint(), expected_drain.size_hint());
        }
        12 => {
            actual.resize_with(5, || 2);
            expected.resize_with(5, || 2);
        }
        13 => {
            actual.extend_from_slice(&[1, 1, 2]);
            expected.extend_from_slice(&[1, 1, 2]);
        }
        14 => {
            actual.as_mut_slice().reverse();
            expected.as_mut_slice().reverse();
        }
        15 => {
            actual.clear();
            expected.clear();
        }
        _ => {}
    }
    assert_eq!(actual.as_slice(), expected.as_slice(), "operation {op}");
    assert_eq!(actual.len(), expected.len());
    assert_eq!(actual.is_empty(), expected.is_empty());
}

#[test]
fn mutable_vec_exhausts_bounded_operation_traces() {
    for initial in [
        vec![],
        vec![0],
        vec![1, 1],
        vec![0, 1, 2],
        vec![3, 3, 2, 1, 0],
    ] {
        for first in 0..16 {
            for second in 0..16 {
                for third in 0..16 {
                    let mut actual = MutableVec::from_vec(initial.clone());
                    let mut expected = initial.clone();
                    for op in [first, second, third] {
                        vector_step(&mut actual, &mut expected, op);
                    }
                    assert_eq!(actual.freeze().thaw().into_vec(), expected);
                }
            }
        }
    }
}

#[test]
fn mutable_vec_capacity_failure_preserves_contents() {
    let mut values = MutableVec::with_capacity(4);
    values.extend([1_u64, 2, 3]);
    values.reserve(5);
    assert!(values.capacity() >= values.len() + 5);
    values.try_reserve(2).unwrap();
    let capacity = values.capacity();
    // A non-ZST capacity overflow is deterministic and never attempts OOM.
    assert!(values.try_reserve(usize::MAX).is_err());
    assert_eq!(values.as_slice(), [1, 2, 3]);
    assert_eq!(values.capacity(), capacity);
    values.shrink_to_fit();
    assert!(values.capacity() >= values.len());
    assert!(values.capacity() <= capacity);
}

struct Tracked {
    id: usize,
    drops: Rc<Vec<Cell<usize>>>,
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.drops[self.id].set(self.drops[self.id].get() + 1);
    }
}

#[test]
fn mutable_vec_partial_consumption_drops_each_non_clone_value_once() {
    let drops = Rc::new((0..8).map(|_| Cell::new(0)).collect::<Vec<_>>());
    let mut values: MutableVec<_> = (0..8)
        .map(|id| Tracked {
            id,
            drops: Rc::clone(&drops),
        })
        .collect();
    {
        let mut removed = values.drain(2..6);
        assert_eq!(removed.len(), 4);
        let front = removed.next().unwrap();
        let back = removed.next_back().unwrap();
        assert_eq!((front.id, back.id), (2, 5));
        assert_eq!(removed.len(), 2);
        assert!(drops.iter().all(|count| count.get() == 0));
    }
    assert_eq!(
        drops.iter().map(Cell::get).collect::<Vec<_>>(),
        [0, 0, 1, 1, 1, 1, 0, 0]
    );
    let mut tail = values.split_off(2);
    values.append(&mut tail);
    assert!(tail.is_empty());
    let mut remaining = values.freeze().thaw().into_iter();
    assert_eq!(remaining.len(), 4);
    assert_eq!(remaining.next().unwrap().id, 0);
    assert_eq!(remaining.next_back().unwrap().id, 7);
    assert_eq!(remaining.size_hint(), (2, Some(2)));
    drop(remaining);
    assert!(drops.iter().all(|count| count.get() == 1));
}

#[derive(Debug)]
struct Key {
    name: String,
    representative: u8,
}

fn key(name: &str, representative: u8) -> Key {
    Key {
        name: name.to_owned(),
        representative,
    }
}

impl Borrow<str> for Key {
    fn borrow(&self) -> &str {
        &self.name
    }
}
impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
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
        self.name.cmp(&other.name)
    }
}
impl std::hash::Hash for Key {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[test]
fn ordered_entry_preserves_representatives_and_lazy_callbacks() {
    let mut map = MutableOrderedMap::new();
    let calls = Cell::new(0);
    let entry = map.entry(key("alpha", 1));
    assert_eq!(entry.key().representative, 1);
    *entry
        .and_modify(|_| panic!("vacant entry modified"))
        .or_insert_with_key(|stored| {
            calls.set(calls.get() + 1);
            assert_eq!(stored.representative, 1);
            10
        }) += 1;
    assert_eq!(calls.get(), 1);
    assert_eq!(
        *map.entry(key("alpha", 2))
            .or_insert_with(|| panic!("occupied default evaluated")),
        11
    );
    assert_eq!(
        *map.entry(key("alpha", 3))
            .or_insert_with_key(|_| panic!("occupied key closure evaluated")),
        11
    );
    map.entry(key("alpha", 4))
        .and_modify(|value| *value += 1)
        .or_default();
    assert_eq!(
        map.get_key_value("alpha")
            .map(|(stored, value)| (stored.representative, *value)),
        Some((1, 12))
    );

    match map.entry(key("alpha", 5)) {
        Entry::Occupied(mut entry) => {
            assert_eq!(entry.key().representative, 1);
            assert_eq!(entry.get(), &12);
            *entry.get_mut() += 1;
            assert_eq!(entry.insert(20), 13);
            *entry.into_mut() += 2;
        }
        Entry::Vacant(_) => panic!("existing key was vacant"),
    }
    match map.entry(key("alpha", 6)) {
        Entry::Occupied(entry) => {
            let (stored, value) = entry.remove_entry();
            assert_eq!((stored.representative, value), (1, 22));
        }
        Entry::Vacant(_) => panic!("existing key was vacant"),
    }
    match map.entry(key("beta", 7)) {
        Entry::Vacant(entry) => {
            assert_eq!(entry.key().representative, 7);
            assert_eq!(entry.into_key().name, "beta");
        }
        Entry::Occupied(_) => panic!("missing key was occupied"),
    }
    assert!(map.is_empty());
    map.entry(key("beta", 8)).or_insert_with(|| {
        calls.set(calls.get() + 1);
        30
    });
    assert_eq!(calls.get(), 2);
    match map.entry(key("beta", 9)) {
        Entry::Occupied(entry) => assert_eq!(entry.remove(), 30),
        Entry::Vacant(_) => panic!("existing key was vacant"),
    }
    assert!(map.is_empty());
}

fn assert_exact_fused<I: ExactSizeIterator + FusedIterator>(mut iter: I) {
    let initial = iter.len();
    for remaining in (1..=initial).rev() {
        assert_eq!(iter.size_hint(), (remaining, Some(remaining)));
        assert!(iter.next().is_some());
    }
    assert_eq!(iter.len(), 0);
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());
}

#[test]
fn ordered_mutation_ranges_and_named_iterators_match_btree() {
    let mut map: MutableOrderedMap<i32, i32> = [(4, 40), (1, 10), (3, 30), (2, 20)].into();
    let mut reference: BTreeMap<i32, i32> = [(4, 40), (1, 10), (3, 30), (2, 20)].into();
    for (_, value) in map.range_mut(2..=3) {
        *value += 1;
    }
    for (_, value) in reference.range_mut(2..=3) {
        *value += 1;
    }
    for value in map.values_mut() {
        *value *= 2;
    }
    for value in reference.values_mut() {
        *value *= 2;
    }
    for (key, value) in &mut map {
        *value += *key;
    }
    for (key, value) in &mut reference {
        *value += *key;
    }
    assert!(map.iter().eq(reference.iter()));
    assert!(map.range(2..).rev().eq(reference.range(2..).rev()));
    let mut iter = map.iter();
    assert_eq!(iter.next(), reference.first_key_value());
    assert_eq!(iter.next_back(), reference.last_key_value());
    assert_eq!(iter.len(), 2);
    assert_exact_fused(iter);
    assert_exact_fused(map.keys());
    assert_exact_fused(map.values());
    assert_exact_fused(map.iter_mut());
    assert_exact_fused(map.values_mut());
    *map.get_mut(&3).unwrap() += 1;
    *reference.get_mut(&3).unwrap() += 1;
    map[&2] -= 1;
    reference.insert(2, reference[&2] - 1);
    map.retain(|key, value| {
        *value += 1;
        key % 2 == 0
    });
    reference.retain(|key, value| {
        *value += 1;
        key % 2 == 0
    });
    assert_eq!(map.pop_first(), reference.pop_first());
    let tail = map.split_off(&3);
    let reference_tail = reference.split_off(&3);
    assert_eq!(tail.into_map(), reference_tail);
    assert_eq!(map.pop_last(), reference.pop_last());
    assert!(map.is_empty());
    assert_eq!(map.remove_entry(&99), None);
}

fn borrowed_map_contract<M>(mut map: M)
where
    M: MapInsert<Key = Key, Value = u32>
        + MapRemove<str>
        + LookupRef<str, Value = u32>
        + Entries
        + Len,
{
    assert_eq!(MapInsert::insert(&mut map, key("same", 1), 10), None);
    assert_eq!(MapInsert::insert(&mut map, key("same", 2), 20), Some(10));
    assert_eq!(LookupRef::get(&map, "same"), Some(&20));
    assert_eq!(
        LookupRef::get_key_value(&map, "same").map(|(stored, _)| stored.representative),
        Some(1)
    );
    assert_eq!(map.len(), 1);
    assert_eq!(map.entries().count(), 1);
    assert!(MapRemove::remove_entry(&mut map, "absent").is_none());
    let (stored, value) = MapRemove::remove_entry(&mut map, "same").unwrap();
    assert_eq!((stored.representative, value), (1, 20));
    assert_eq!(MapRemove::remove(&mut map, "same"), None);
    MapInsert::insert(&mut map, key("last", 3), 30);
    assert_eq!(MapRemove::remove(&mut map, "last"), Some(30));
    assert!(map.is_empty());
}

#[test]
fn borrowed_map_capabilities_preserve_stored_keys() {
    borrowed_map_contract(MutableOrderedMap::new());
    borrowed_map_contract(BTreeMap::new());
    #[cfg(feature = "std")]
    {
        borrowed_map_contract(MutableHashMap::new());
        borrowed_map_contract(std::collections::HashMap::new());
    }
}

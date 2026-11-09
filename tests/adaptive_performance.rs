//! Operation-count and semantic regressions for adaptive entry lookups.
#![cfg(feature = "alloc")]
#![allow(
    clippy::mutable_key_type,
    reason = "the cell counts comparisons without changing key identity"
)]

#[cfg(feature = "std")]
use frost::adaptive::HashRepr;
use frost::adaptive::OrderedRepr;
use frost::prelude::*;
use std::cell::Cell;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

#[derive(Debug)]
struct CountedKey {
    id: u32,
    representative: u8,
    comparisons: Rc<Cell<usize>>,
}

impl PartialEq for CountedKey {
    fn eq(&self, other: &Self) -> bool {
        self.comparisons.set(self.comparisons.get() + 1);
        self.id == other.id
    }
}

impl Eq for CountedKey {}

impl PartialOrd for CountedKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CountedKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.comparisons.set(self.comparisons.get() + 1);
        self.id.cmp(&other.id)
    }
}

impl Hash for CountedKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

fn key(id: u32, representative: u8, comparisons: &Rc<Cell<usize>>) -> CountedKey {
    CountedKey {
        id,
        representative,
        comparisons: Rc::clone(comparisons),
    }
}

#[test]
fn ordered_entry_searches_the_sorted_representation_once() {
    let comparisons = Rc::new(Cell::new(0));
    let mut map = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(8));
    for id in 0..4 {
        map.insert(key(id, 1, &comparisons), id);
    }

    for id in [0, 3, 9] {
        comparisons.set(0);
        let previous = map.get(&key(id, 2, &comparisons)).copied();
        let lookup_comparisons = comparisons.get();
        comparisons.set(0);
        let calls = Cell::new(0);
        let value = map.get_or_insert_with(key(id, 2, &comparisons), || {
            calls.set(calls.get() + 1);
            99
        });
        assert_eq!(*value, previous.unwrap_or(99));
        assert_eq!(comparisons.get(), lookup_comparisons);
        assert_eq!(calls.get(), usize::from(previous.is_none()));
        let stored = map.get_key_value(&key(id, 3, &comparisons)).unwrap().0;
        assert_eq!(
            stored.representative,
            if previous.is_some() { 1 } else { 2 }
        );
    }
    assert_eq!(map.representation(), OrderedRepr::Sorted);
}

#[cfg(feature = "std")]
#[test]
fn hash_entry_searches_the_linear_representation_once() {
    let comparisons = Rc::new(Cell::new(0));
    let mut map = AdaptiveHashMap::with_policy(AdaptiveHash::linear_up_to(8));
    for id in 0..4 {
        map.insert(key(id, 1, &comparisons), id);
    }

    for id in [0, 3, 9] {
        comparisons.set(0);
        let previous = map.get(&key(id, 2, &comparisons)).copied();
        let lookup_comparisons = comparisons.get();
        comparisons.set(0);
        let calls = Cell::new(0);
        let value = map.get_or_insert_with(key(id, 2, &comparisons), || {
            calls.set(calls.get() + 1);
            99
        });
        assert_eq!(*value, previous.unwrap_or(99));
        assert_eq!(comparisons.get(), lookup_comparisons);
        assert_eq!(calls.get(), usize::from(previous.is_none()));
        let stored = map.get_key_value(&key(id, 3, &comparisons)).unwrap().0;
        assert_eq!(
            stored.representative,
            if previous.is_some() { 1 } else { 2 }
        );
    }
    assert_eq!(map.representation(), HashRepr::Linear);
}

#[test]
fn ordered_entry_preserves_promotion_and_closure_panic_behavior() {
    let mut map = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(2));
    map.insert(1, 10);
    map.insert(2, 20);
    assert_eq!(
        *map.get_or_insert_with(2, || panic!("hit must not call closure")),
        20
    );
    assert_eq!(map.representation(), OrderedRepr::Sorted);

    let failure = catch_unwind(AssertUnwindSafe(|| {
        map.get_or_insert_with(3, || panic!("insertion failed"));
    }));
    assert!(failure.is_err());
    assert_eq!(map.representation(), OrderedRepr::Tree);
    assert_eq!(
        map.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
        [(1, 10), (2, 20)]
    );
    assert_eq!(*map.get_or_insert_with(3, || 30), 30);
    assert_eq!(*map.get_or_insert_with(3, || panic!("tree hit")), 30);
}

#[cfg(feature = "std")]
#[test]
fn hash_entry_preserves_promotion_and_closure_panic_behavior() {
    let mut map = AdaptiveHashMap::with_policy(AdaptiveHash::linear_up_to(2));
    map.insert(1, 10);
    map.insert(2, 20);
    assert_eq!(
        *map.get_or_insert_with(2, || panic!("hit must not call closure")),
        20
    );
    assert_eq!(map.representation(), HashRepr::Linear);

    let failure = catch_unwind(AssertUnwindSafe(|| {
        map.get_or_insert_with(3, || panic!("insertion failed"));
    }));
    assert!(failure.is_err());
    assert_eq!(map.representation(), HashRepr::Table);
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&1), Some(&10));
    assert_eq!(map.get(&2), Some(&20));
    assert_eq!(map.get(&3), None);
    assert_eq!(*map.get_or_insert_with(3, || 30), 30);
    assert_eq!(*map.get_or_insert_with(3, || panic!("table hit")), 30);
}

#[test]
fn ordered_entry_panicking_before_promotion_keeps_the_map() {
    let mut map = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(8));
    map.insert(1, 10);
    let failure = catch_unwind(AssertUnwindSafe(|| {
        map.get_or_insert_with(2, || panic!("insertion failed"));
    }));
    assert!(failure.is_err());
    assert_eq!(map.representation(), OrderedRepr::Sorted);
    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&1), Some(&10));
}

#[cfg(feature = "std")]
#[test]
fn hash_entry_panicking_before_promotion_keeps_the_map() {
    let mut map = AdaptiveHashMap::with_policy(AdaptiveHash::linear_up_to(8));
    map.insert(1, 10);
    let failure = catch_unwind(AssertUnwindSafe(|| {
        map.get_or_insert_with(2, || panic!("insertion failed"));
    }));
    assert!(failure.is_err());
    assert_eq!(map.representation(), HashRepr::Linear);
    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&1), Some(&10));
}

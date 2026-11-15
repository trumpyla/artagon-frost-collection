//! Mutable/frozen set pairing and representative laws.
#![cfg(feature = "alloc")]

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

use core::borrow::Borrow;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use frost::mutable::MutableOrderedSet;
use frost::prelude::*;
use frost::write::{SetInsert, SetRemove};

#[derive(Debug)]
struct Member {
    key: String,
    label: &'static str,
}
impl Borrow<str> for Member {
    fn borrow(&self) -> &str {
        &self.key
    }
}
impl PartialEq for Member {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for Member {}
impl PartialOrd for Member {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Member {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}
impl Hash for Member {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.key.hash(h);
    }
}

fn member(label: &'static str) -> Member {
    Member {
        key: String::from("key"),
        label,
    }
}

#[test]
fn ordered_set_keeps_first_rep_and_its_frozen_pair() {
    let mut values = MutableOrderedSet::new();
    assert!(SetInsert::insert(&mut values, member("first")));
    assert!(!SetInsert::insert(&mut values, member("second")));
    assert_eq!(values.get("key").unwrap().label, "first");
    let frozen: FrozenOrderedSet<Member> = values.freeze();
    assert_eq!(frozen.get("key").unwrap().label, "first");
    let mut thawed: MutableOrderedSet<Member> = frozen.thaw();
    assert!(SetRemove::remove(&mut thawed, "key"));
    assert!(!SetRemove::remove(&mut thawed, "key"));
    assert!(thawed.is_empty());
}

#[test]
fn ordered_set_iterators_match_sorted_reference() {
    let mut values: MutableOrderedSet<_> = [5, 3, 1, 3].into_iter().collect();
    values.extend([9, 7]);
    assert_eq!(values.iter().len(), 5);
    assert_eq!(
        values.iter().rev().copied().collect::<Vec<_>>(),
        [9, 7, 5, 3, 1]
    );
    assert_eq!(
        values.range_elements(3..=7).copied().collect::<Vec<_>>(),
        [3, 5, 7]
    );
    assert_eq!(values.first_element(), Some(&1));
    assert_eq!(values.last_element(), Some(&9));
    assert_eq!(values.take(&3), Some(3));
    assert_eq!(values.into_iter().rev().collect::<Vec<_>>(), [9, 7, 5, 1]);
}

#[test]
fn defaults_and_iterator_clones_do_not_require_element_traits() {
    #[derive(Debug)]
    struct NotDefault;
    let empty: MutableOrderedSet<NotDefault> = MutableOrderedSet::default();
    let iter = empty.iter();
    assert_eq!(iter.clone().count(), 0);
    assert!(empty.is_empty());
}

#[cfg(feature = "std")]
#[test]
fn hash_set_keeps_first_rep_and_the_exact_hasher() {
    use frost::mutable::MutableHashSet;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::BuildHasher;
    struct CustomHasher(u32);
    impl BuildHasher for CustomHasher {
        type Hasher = DefaultHasher;
        fn build_hasher(&self) -> Self::Hasher {
            DefaultHasher::new()
        }
    }
    let mut values = MutableHashSet::with_hasher(CustomHasher(23));
    assert!(SetInsert::insert(&mut values, member("first")));
    assert!(!SetInsert::insert(&mut values, member("second")));
    values.reserve(32);
    let frozen: FrozenHashSet<Member, CustomHasher> = values.freeze();
    assert_eq!(frozen.get("key").unwrap().label, "first");
    let mut thawed: MutableHashSet<Member, CustomHasher> = frozen.thaw();
    assert_eq!(thawed.hasher().0, 23);
    assert_eq!(thawed.take("key").unwrap().label, "first");
    assert!(thawed.is_empty());
}

#[cfg(feature = "std")]
#[test]
fn hash_set_equality_and_consuming_iteration_are_logical() {
    use frost::mutable::MutableHashSet;
    let left: MutableHashSet<_> = [3, 1, 2, 1].into_iter().collect();
    let right: MutableHashSet<_> = [2, 3, 1].into_iter().collect();
    assert_eq!(left, right);
    assert_eq!(left.iter().len(), 3);
    assert_eq!(left.iter().clone().count(), 3);
    let mut values = left.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    assert_eq!(values, [1, 2, 3]);
}

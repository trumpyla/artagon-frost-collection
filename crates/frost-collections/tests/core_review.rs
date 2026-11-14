//! Regressions from the review of mutable adaptive collection profiles.
#![cfg(feature = "alloc")]

use frost::prelude::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn inline_zero_sized_reservation_checks_length_overflow() {
    let mut values: InlineVec<(), 4> = InlineVec::new();
    values.push(());
    let failure = catch_unwind(AssertUnwindSafe(|| values.reserve(usize::MAX)));
    assert!(
        failure.is_err(),
        "len + additional must not overflow for ZSTs"
    );
    assert!(values.is_inline());
    assert_eq!(values.as_slice(), &[()]);

    // A zero-sized allocation may legitimately reserve usize::MAX elements
    // when it is empty. Only the logical length overflow must be rejected.
    let mut empty: InlineVec<(), 4> = InlineVec::new();
    empty.reserve(usize::MAX);
    assert!(empty.is_empty());
    assert_eq!(empty.capacity(), usize::MAX);
}

#[test]
fn small_inline_collection_does_not_consult_heap_reservation_hints() {
    struct NoHint(std::ops::Range<u8>);

    impl Iterator for NoHint {
        type Item = u8;

        fn next(&mut self) -> Option<u8> {
            self.0.next()
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            panic!("a fitting inline collection does not need a heap reservation");
        }
    }

    let mutable: InlineVec<_, 4> = NoHint(0..4).collect();
    let frozen: InlineFrozenVec<_, 4> = NoHint(0..4).collect();
    assert!(mutable.is_inline() && frozen.is_inline());
    assert_eq!(mutable.as_slice(), &[0, 1, 2, 3]);
    assert_eq!(frozen.as_slice(), mutable.as_slice());
}

#[test]
fn inline_collection_stops_at_the_first_none() {
    struct NonFused(u8);

    impl Iterator for NonFused {
        type Item = u8;

        fn next(&mut self) -> Option<u8> {
            self.0 += 1;
            match self.0 {
                1 => Some(7),
                3 => Some(9),
                _ => None,
            }
        }
    }

    let mutable: InlineVec<_, 2> = NonFused(0).collect();
    let frozen: InlineFrozenVec<_, 2> = NonFused(0).collect();
    assert_eq!(mutable.as_slice(), &[7]);
    assert_eq!(frozen.as_slice(), &[7]);
}

#[test]
fn ordered_freezing_does_not_require_new_key_bounds() {
    struct NoTraits;

    let frozen: FrozenOrderedMap<NoTraits, NoTraits> = MutableOrderedMap::new().freeze();
    assert!(frozen.is_empty());
    let frozen: FrozenOrderedMap<NoTraits, NoTraits> = MutableOrderedMap::new().compact();
    assert!(frozen.is_empty());
    let adaptive =
        frost::adaptive::AdaptiveOrderedMap::from(MutableOrderedMap::<NoTraits, NoTraits>::new());
    assert!(adaptive.freeze().is_empty());
}

#[test]
fn ordered_duplicate_reduction_drops_each_value_once_on_panic() {
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Value {
        id: usize,
        drops: Rc<RefCell<Vec<usize>>>,
    }

    impl Drop for Value {
        fn drop(&mut self) {
            self.drops.borrow_mut().push(self.id);
            assert_ne!(self.id, 0, "replaced value's destructor panicked");
        }
    }

    let drops = Rc::new(RefCell::new(Vec::new()));
    let failure = catch_unwind(AssertUnwindSafe(|| {
        [(1, 0), (1, 1), (2, 2)]
            .map(|(key, id)| {
                (
                    key,
                    Value {
                        id,
                        drops: Rc::clone(&drops),
                    },
                )
            })
            .into_iter()
            .collect::<FrozenOrderedMap<_, _>>()
    }));
    assert!(failure.is_err());
    drops.borrow_mut().sort_unstable();
    assert_eq!(*drops.borrow(), [0, 1, 2]);
}

#[test]
fn explicit_native_extraction_preserves_owned_values() {
    let vector = MutableVec::from_vec(vec![String::from("owned")]);
    assert_eq!(vector.into_vec(), ["owned"]);
    let ordered =
        MutableOrderedMap::from_map(std::collections::BTreeMap::from([(3, String::from("map"))]));
    assert_eq!(ordered.into_map().get(&3).map(String::as_str), Some("map"));
    let frozen: FrozenOrderedMap<_, _> = [(3, String::from("frozen"))].into_iter().collect();
    assert_eq!(
        frozen.into_map().get(&3).map(String::as_str),
        Some("frozen")
    );
}

#[test]
fn mutable_ordered_strict_building_uses_input_order_and_index_mut_borrows_keys() {
    let duplicate = MutableOrderedMap::try_from_unique([
        (9, "first nine"),
        (3, "first three"),
        (9, "second nine"),
        (3, "second three"),
    ])
    .unwrap_err();
    assert_eq!((duplicate.key, duplicate.value), (9, "second nine"));

    let mut map = MutableOrderedMap::try_from_unique([(String::from("key"), 1)]).unwrap();
    map["key"] += 2;
    assert_eq!(map.get("key"), Some(&3));
}

#[test]
fn thawing_zero_sized_values_keeps_the_standard_capacity_convention() {
    let frozen: FrozenVec<_> = [(), (), ()].into();
    let mutable = frozen.thaw();
    assert_eq!(mutable.len(), 3);
    assert_eq!(mutable.capacity(), usize::MAX);
}

#[cfg(feature = "std")]
mod hashed {
    use super::*;
    use std::cell::Cell;
    use std::cmp::Ordering;
    use std::hash::{BuildHasher, DefaultHasher, Hash, Hasher};
    use std::rc::Rc;

    #[derive(Clone, Debug)]
    struct Key(u8, &'static str);

    impl PartialEq for Key {
        fn eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }
    impl Eq for Key {}
    impl Hash for Key {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.0.hash(state);
        }
    }

    #[test]
    fn every_hash_builder_preserves_first_key_and_last_value() {
        let entries = [(Key(1, "first"), 10), (Key(1, "later"), 20)];
        let query = Key(1, "query");
        let from_array = MutableHashMap::from(entries.clone());
        let mutable: MutableHashMap<_, _> = entries.clone().into_iter().collect();
        let frozen: FrozenHashMap<_, _> = entries.clone().into_iter().collect();
        let adaptive: AdaptiveFrozenHashMap<_, _> = entries.into_iter().collect();
        for (key, value) in [
            from_array.get_key_value(&query).unwrap(),
            mutable.get_key_value(&query).unwrap(),
            frozen.get_key_value(&query).unwrap(),
            adaptive.get_key_value(&query).unwrap(),
        ] {
            assert_eq!((key.1, *value), ("first", 20));
        }
        let mut extended = mutable;
        extended.extend([(Key(1, "extended"), 30)]);
        let native = extended.freeze().into_map();
        assert_eq!(
            native
                .get_key_value(&query)
                .map(|(key, value)| (key.1, *value)),
            Some(("first", 30))
        );
        assert_eq!(from_array.into_map().len(), 1);
    }

    struct ChosenHasher(u64);

    impl BuildHasher for ChosenHasher {
        type Hasher = DefaultHasher;

        fn build_hasher(&self) -> Self::Hasher {
            let mut hasher = DefaultHasher::new();
            hasher.write_u64(self.0);
            hasher
        }
    }

    #[test]
    fn explicit_hash_builders_need_neither_clone_nor_default_hashers() {
        let mutable = MutableHashMap::from_iter_with_hasher(
            [(Key(1, "first"), 10), (Key(1, "later"), 20)],
            ChosenHasher(41),
        );
        assert_eq!(mutable.hasher().0, 41);
        let frozen = mutable.freeze();
        assert_eq!(frozen.hasher().0, 41);
        assert_eq!(
            frozen
                .get_key_value(&Key(1, "query"))
                .map(|(k, v)| (k.1, *v)),
            Some(("first", 20))
        );
        let native = frozen.thaw().into_map();
        let mut mutable = MutableHashMap::from_map(native);
        mutable[&Key(1, "query")] = 30;
        assert_eq!(mutable.hasher().0, 41);
        assert_eq!(mutable.get(&Key(1, "query")), Some(&30));

        let frozen =
            FrozenHashMap::from_iter_with_hasher([(String::from("key"), 7)], ChosenHasher(43));
        assert_eq!(frozen.hasher().0, 43);
        assert_eq!(frozen.get("key"), Some(&7));

        let mut strict = MutableHashMap::try_from_unique_with_hasher(
            [(String::from("key"), 1)],
            ChosenHasher(47),
        )
        .unwrap();
        strict["key"] += 2;
        assert_eq!(strict.get("key"), Some(&3));
        assert_eq!(strict.freeze().hasher().0, 47);

        let strict = FrozenHashMap::try_from_unique_with_hasher(
            [(String::from("key"), 5)],
            ChosenHasher(53),
        )
        .unwrap();
        assert_eq!(strict.hasher().0, 53);
        assert_eq!(strict.get("key"), Some(&5));
    }

    #[test]
    fn strict_hash_builders_return_the_rejected_input_entry() {
        let entries = [(Key(1, "first"), 10), (Key(1, "rejected"), 20)];
        let mutable = MutableHashMap::try_from_unique_with_hasher(entries.clone(), ChosenHasher(1))
            .unwrap_err();
        assert_eq!((mutable.key.1, mutable.value), ("rejected", 20));
        let frozen = FrozenHashMap::try_from_unique_with_hasher(entries.clone(), ChosenHasher(2))
            .unwrap_err();
        assert_eq!((frozen.key.1, frozen.value), ("rejected", 20));
        let mutable = MutableHashMap::<_, _>::try_from_unique(entries.clone()).unwrap_err();
        assert_eq!((mutable.key.1, mutable.value), ("rejected", 20));
        let frozen = FrozenHashMap::<_, _>::try_from_unique(entries).unwrap_err();
        assert_eq!((frozen.key.1, frozen.value), ("rejected", 20));
    }

    #[test]
    fn frozen_adaptive_thaw_and_extraction_do_not_clone_the_hasher() {
        struct HasherPolicy(u8);
        struct NoKeyTraits;

        impl Default for HasherPolicy {
            fn default() -> Self {
                Self(37)
            }
        }

        impl BuildHasher for HasherPolicy {
            type Hasher = DefaultHasher;

            fn build_hasher(&self) -> Self::Hasher {
                DefaultHasher::new()
            }
        }

        let frozen = AdaptiveFrozenHashMap::<u8, u8, HasherPolicy>::default();
        let mutable = frozen.thaw_with(AdaptiveHash::linear_up_to(3));
        assert_eq!(mutable.hasher().0, 37);
        assert_eq!(mutable.policy().linear_max, 3);
        assert_eq!(mutable.get(&1), None);

        let frozen = AdaptiveFrozenHashMap::<u8, u8, HasherPolicy>::default();
        assert_eq!(frozen.into_hash_map().hasher().0, 37);

        let frozen = AdaptiveFrozenHashMap::<NoKeyTraits, (), HasherPolicy>::default();
        assert!(frozen.thaw().is_empty());
    }

    struct PanicKey {
        id: u8,
        panic_hash: Rc<Cell<bool>>,
        panic_order: Rc<Cell<bool>>,
    }

    impl PartialEq for PanicKey {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }
    impl Eq for PanicKey {}
    impl Hash for PanicKey {
        fn hash<H: Hasher>(&self, state: &mut H) {
            assert!(!self.panic_hash.get(), "hashing during promotion panicked");
            self.id.hash(state);
        }
    }
    impl PartialOrd for PanicKey {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for PanicKey {
        fn cmp(&self, other: &Self) -> Ordering {
            assert!(
                !self.panic_order.get() || self.id >= 2 || other.id >= 2,
                "comparison of existing entries during promotion panicked"
            );
            self.id.cmp(&other.id)
        }
    }

    struct Dropped(Rc<Cell<usize>>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn hash_promotion_panic_keeps_a_valid_map_and_drops_values_once() {
        let panic_hash = Rc::new(Cell::new(false));
        let panic_order = Rc::new(Cell::new(false));
        let dropped = Rc::new(Cell::new(0));
        let key = |id| PanicKey {
            id,
            panic_hash: Rc::clone(&panic_hash),
            panic_order: Rc::clone(&panic_order),
        };
        let mut map = AdaptiveHashMap::with_policy(AdaptiveHash::linear_up_to(2));
        map.insert(key(0), Dropped(Rc::clone(&dropped)));
        map.insert(key(1), Dropped(Rc::clone(&dropped)));
        panic_hash.set(true);
        let failure = catch_unwind(AssertUnwindSafe(|| {
            map.insert(key(2), Dropped(Rc::clone(&dropped)));
        }));
        assert!(failure.is_err());
        panic_hash.set(false);
        // This checks validity and ownership, not a transactional rollback
        // guarantee: failed representation rebuilding can discard entries.
        map.insert(key(3), Dropped(Rc::clone(&dropped)));
        assert!(map.get(&key(3)).is_some());
        assert_eq!(map.len(), map.iter().count());
        drop(map);
        assert_eq!(dropped.get(), 4);
    }

    #[test]
    fn ordered_promotion_panic_keeps_a_valid_map_and_drops_values_once() {
        let panic_hash = Rc::new(Cell::new(false));
        let panic_order = Rc::new(Cell::new(false));
        let dropped = Rc::new(Cell::new(0));
        let key = |id| PanicKey {
            id,
            panic_hash: Rc::clone(&panic_hash),
            panic_order: Rc::clone(&panic_order),
        };
        let mut map = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(2));
        map.insert(key(0), Dropped(Rc::clone(&dropped)));
        map.insert(key(1), Dropped(Rc::clone(&dropped)));
        panic_order.set(true);
        let failure = catch_unwind(AssertUnwindSafe(|| {
            map.insert(key(2), Dropped(Rc::clone(&dropped)));
        }));
        assert!(failure.is_err());
        panic_order.set(false);
        map.insert(key(3), Dropped(Rc::clone(&dropped)));
        assert!(map.get(&key(3)).is_some());
        assert_eq!(map.len(), map.iter().count());
        drop(map);
        assert_eq!(dropped.get(), 4);
    }
}

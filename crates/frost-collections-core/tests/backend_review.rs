//! Boundary, ownership, and capability regressions for backend adapters.

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

#[cfg(feature = "arrayvec-0_7")]
#[test]
fn arrayvec_exact_length_is_independent_of_inline_capacity() {
    use frost::backend::arrayvec::ArrayVec;
    use frost::prelude::{ExactLen, Frozen, TryFreezeWith};

    let mut source = ArrayVec::<String, 4>::new();
    source.push(String::from("a"));
    source.push(String::from("b"));
    let frozen: Frozen<[String; 2]> = source.try_freeze_with(ExactLen::<2>).unwrap();
    assert_eq!(frozen.as_array(), &["a", "b"]);

    let empty: Frozen<[String; 0]> = ArrayVec::<String, 4>::new()
        .try_freeze_with(ExactLen::<0>)
        .unwrap();
    assert_eq!(empty.len(), 0);
    let empty: Frozen<[String; 0]> = ArrayVec::<String, 0>::new()
        .try_freeze_with(ExactLen::<0>)
        .unwrap();
    assert!(empty.is_empty());
}

#[cfg(feature = "arrayvec-0_7")]
#[test]
fn arrayvec_exact_length_failure_preserves_owned_elements() {
    use std::cell::Cell;
    use std::rc::Rc;

    use frost::backend::arrayvec::ArrayVec;
    use frost::prelude::{ExactLen, TryFreezeWith};

    struct Counted {
        value: usize,
        drops: Rc<Cell<usize>>,
    }

    impl Drop for Counted {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    let drops = Rc::new(Cell::new(0));
    let mut source = ArrayVec::<Counted, 4>::new();
    source.extend((0..3).map(|value| Counted {
        value,
        drops: Rc::clone(&drops),
    }));
    let Err(rejected) = source.try_freeze_with(ExactLen::<2>) else {
        panic!("three elements must not convert to an array of length two");
    };
    assert_eq!((rejected.error.expected, rejected.error.actual), (2, 3));
    assert_eq!(drops.get(), 0);
    assert_eq!(rejected.input.capacity(), 4);
    assert_eq!(
        rejected
            .input
            .iter()
            .map(|item| item.value)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    let exact = rejected.input.try_freeze_with(ExactLen::<3>).unwrap();
    assert_eq!(drops.get(), 0);
    drop(exact);
    assert_eq!(drops.get(), 3);

    let oversized = ArrayVec::<u8, 2>::new()
        .try_freeze_with(ExactLen::<4>)
        .unwrap_err();
    assert_eq!((oversized.error.expected, oversized.error.actual), (4, 0));
    assert_eq!(oversized.input.capacity(), 2);
}

#[test]
fn value_and_cardinality_read_traits_remain_dyn_compatible() {
    use frost::read::{Cardinality, LookupValue, ValueRef};

    struct Decoded;

    impl ValueRef<str> for Decoded {
        type Value = [u8];

        fn get_ref(&self, query: &str) -> Option<&[u8]> {
            (query == "stored").then_some(&[1, 2, 3])
        }
    }

    impl LookupValue<str> for Decoded {
        type Value = u64;

        fn get_value(&self, query: &str) -> Option<u64> {
            (query == "stored").then_some(u64::MAX)
        }
    }

    impl Cardinality for Decoded {
        fn cardinality(&self) -> u64 {
            u64::MAX
        }
    }

    let borrow: &dyn ValueRef<str, Value = [u8]> = &Decoded;
    assert_eq!(borrow.get_ref("stored"), Some([1, 2, 3].as_slice()));
    let decoded: &dyn LookupValue<str, Value = u64> = &Decoded;
    assert_eq!(decoded.get_value("stored"), Some(u64::MAX));
    let count: &dyn Cardinality = &Decoded;
    assert_eq!(count.cardinality(), u64::MAX);
}

#[cfg(feature = "fst-0_4")]
#[test]
fn fst_read_traits_work_for_borrowed_bytes_and_seal_for_owned_bytes() {
    use frost::backend::fst::{Map, Set};
    use frost::prelude::{FreezeExt, LookupValue, SetContains};

    let owned = Map::from_iter([("", 0), ("a", 7)]).unwrap();
    let bytes = owned.as_fst().as_bytes();
    let borrowed = Map::new(bytes).unwrap();
    let erased: &dyn LookupValue<str, Value = u64> = &borrowed;
    assert_eq!(erased.get_value(""), Some(0));
    assert_eq!(erased.get_value("a"), Some(7));
    let owned_bytes = owned.into_fst().into_inner().into_boxed_slice();
    let frozen = Map::new(owned_bytes).unwrap().seal();
    assert_eq!(frozen.get("a"), Some(7));

    let set = Set::from_iter(["", "a"]).unwrap();
    let borrowed = Set::new(set.as_fst().as_bytes()).unwrap();
    assert!(SetContains::contains(&borrowed, ""));
    assert!(SetContains::contains(&borrowed, "a"));
}

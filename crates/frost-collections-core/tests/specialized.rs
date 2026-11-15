//! Tests that also run with one backend feature enabled and no defaults.

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

use frost::prelude::*;

#[test]
fn core_arrays_need_neither_alloc_nor_std() {
    let frozen = [String::from("a"), String::from("b")].seal();
    assert_eq!(frozen.get(1).map(String::as_str), Some("b"));
    assert_eq!(frozen.elements().count(), 2);
    let mut array = frozen.into_inner();
    array.swap(0, 1);
    assert_eq!(array[0], "b");
}

#[cfg(feature = "arrayvec-0_7")]
#[test]
fn arrayvec_failure_returns_ownership_and_keeps_content() {
    use frost::backend::arrayvec::ArrayVec;
    use frost::write::TryPush;
    let mut values = ArrayVec::<String, 2>::new();
    TryPush::try_push(&mut values, String::from("a")).unwrap();
    TryPush::try_push(&mut values, String::from("b")).unwrap();
    let rejected = TryPush::try_push(&mut values, String::from("c")).unwrap_err();
    assert_eq!(rejected.value, "c");
    assert_eq!(values.as_slice(), ["a", "b"]);
    let frozen = values.try_freeze_with(ExactLen::<2>).unwrap();
    assert_eq!(frozen.as_array(), &["a", "b"]);

    let mut short = ArrayVec::<u32, 2>::new();
    short.push(7);
    let rejected = short.try_freeze_with(ExactLen::<2>).unwrap_err();
    assert_eq!((rejected.error.expected, rejected.error.actual), (2, 1));
    assert_eq!(rejected.input.as_slice(), &[7]);
    let mut zero = ArrayVec::<u8, 0>::new();
    assert_eq!(TryPush::try_push(&mut zero, 9).unwrap_err().value, 9);
    assert!(zero.seal().is_empty());
}

#[cfg(feature = "smallvec-1")]
#[test]
fn smallvec_freeze_preserves_spill_state_and_compact_values() {
    use frost::backend::smallvec::SmallVec;
    use frost::write::TryPush;
    for length in [0, 2, 4, 5, 32] {
        let mut values = SmallVec::<[String; 4]>::new();
        for index in 0..length {
            TryPush::try_push(&mut values, index.to_string()).unwrap();
        }
        let spilled = values.spilled();
        let frozen = values.seal();
        assert_eq!(frozen.spilled(), spilled);
        assert_eq!(frozen.len(), length);
        let compact = frozen.into_inner().compact();
        assert_eq!(compact.len(), length);
        assert!(
            compact
                .iter()
                .enumerate()
                .all(|(i, value)| *value == i.to_string())
        );
    }
}

#[cfg(feature = "phf-0_14")]
#[test]
fn static_maps_and_sets_preserve_their_query_and_order_contracts() {
    use frost::backend::phf;
    let map = phf::phf_map! { "ready" => 1_u32, "done" => 2_u32 }.seal();
    assert_eq!(map.get("done"), Some(&2));
    assert_eq!(
        LookupRef::get_key_value(&map, "ready"),
        Some((&"ready", &1))
    );
    assert_eq!(map.entries().count(), 2);
    assert_eq!(map.get("missing"), None);

    let map = phf::phf_ordered_map! { "z" => 1_u32, "a" => 2_u32 }.seal();
    assert_eq!(
        map.entries().map(|(k, _)| *k).collect::<Vec<_>>(),
        ["z", "a"]
    );
    assert_eq!(IndexedEntries::get_index(&map, 0), Some((&"z", &1)));
    assert_eq!(IndexOf::get_index_of(&map, "a"), Some(1));
    assert_eq!(IndexedEntries::get_index(&map, 2), None);

    let set = phf::phf_set! { "a", "b" }.seal();
    assert_eq!(set.get("a"), Some(&"a"));
    assert!(set.contains("b"));
    assert!(!set.contains("c"));
    assert_eq!(set.elements().count(), 2);
    let ordered = phf::phf_ordered_set! { "z", "a" }.seal();
    assert_eq!(ordered.elements().copied().collect::<Vec<_>>(), ["z", "a"]);
}

#[cfg(feature = "fst-0_4")]
#[test]
fn fst_lookup_decodes_values_and_streams_lend_keys() {
    use frost::backend::fst::{Map, Set, Streamer};
    let map = Map::from_iter([(b"a".as_slice(), 7), (b"ab".as_slice(), u64::MAX)])
        .unwrap()
        .seal();
    assert_eq!(LookupValue::get_value(&map, "a"), Some(7));
    assert_eq!(map.get("ab"), Some(u64::MAX));
    assert_eq!(map.get("z"), None);
    assert_eq!(map.len(), 2);
    let mut stream = map.stream();
    assert_eq!(stream.next(), Some((b"a".as_slice(), 7)));
    assert_eq!(stream.next(), Some((b"ab".as_slice(), u64::MAX)));
    assert_eq!(stream.next(), None);

    let set = Set::from_iter(["a", "ab"]).unwrap().seal();
    assert!(SetContains::contains(&set, "ab"));
    assert!(!set.contains("b"));
    assert_eq!(set.stream().into_strs().unwrap(), ["a", "ab"]);
    assert!(Map::from_iter([("z", 1), ("a", 2)]).is_err());
    assert!(Set::from_iter(["z", "a"]).is_err());
}

#[cfg(all(feature = "std", feature = "hashbrown-0_17"))]
#[test]
fn hashbrown_sets_compact_with_the_supplied_hasher() {
    use frost::backend::hashbrown::HashSet;
    use std::collections::hash_map::RandomState;
    let mut values = HashSet::with_hasher(RandomState::new());
    values.insert(String::from("member"));
    let frozen = values.compact();
    assert_eq!(frozen.get("member").map(String::as_str), Some("member"));
    assert!(frozen.contains("member"));
}

#[cfg(all(feature = "std", feature = "indexmap-2"))]
#[test]
fn indexset_compact_retains_positions() {
    use frost::backend::indexmap::IndexSet;
    let values: IndexSet<_> = ["z", "a", "z"].into_iter().collect();
    let frozen = values.compact();
    assert_eq!(frozen.get_index(0), Some(&"z"));
    assert_eq!(frozen.get("a"), Some(&"a"));
    assert_eq!(frozen.iter().copied().collect::<Vec<_>>(), ["z", "a"]);
}

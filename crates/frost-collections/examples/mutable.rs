//! Build, freeze, and thaw without changing public collection families.

use frost::mutable::{MutableOrderedMap, MutableVec};

fn demo() {
    let mut sequence = MutableVec::with_capacity(8);
    sequence.extend(["plan", "verify"]);
    sequence.push("refine");
    let frozen = sequence.freeze();
    assert_eq!(frozen.as_slice(), &["plan", "verify", "refine"]);
    let mut sequence: MutableVec<&str> = frozen.thaw();
    assert_eq!(sequence.pop(), Some("refine"));

    let mut ordered = MutableOrderedMap::new();
    for word in ["compile", "test", "compile"] {
        *ordered.entry(word).or_default() += 1_u32;
    }
    let frozen = ordered.freeze();
    assert_eq!(frozen.get("compile"), Some(&2));
    assert_eq!(
        frozen.keys().copied().collect::<Vec<_>>(),
        ["compile", "test"]
    );
    let mut ordered: MutableOrderedMap<&str, u32> = frozen.thaw();
    ordered
        .entry("test")
        .and_modify(|count| *count += 1)
        .or_insert(1);
    assert_eq!(ordered.get("test"), Some(&2));

    #[cfg(feature = "std")]
    {
        use frost::mutable::MutableHashMap;

        let mut hashed = MutableHashMap::new();
        hashed
            .entry(String::from("worker"))
            .or_insert_with_key(String::len);
        let frozen = hashed.freeze();
        assert_eq!(frozen.get("worker"), Some(&6));
        let mut hashed: MutableHashMap<String, usize> = frozen.thaw();
        assert_eq!(hashed.remove("worker"), Some(6));
        assert!(hashed.is_empty());
    }
}

fn main() {
    demo();
}

#[test]
fn complete_example_is_tested() {
    main();
}

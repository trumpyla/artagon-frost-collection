//! PHF declaration order and FST compressed values have different contracts.
use frost::backend::{fst, phf};
use frost::prelude::*;

fn main() {
    let commands = phf::phf_ordered_map! { "verify" => 1_u64, "refine" => 2_u64 }.seal();
    assert_eq!(commands.get("verify"), Some(&1));
    assert_eq!(
        IndexedEntries::get_index(&commands, 0),
        Some((&"verify", &1))
    );

    let encoded = fst::Map::from_iter([("refine", 2), ("verify", 1)])
        .unwrap()
        .seal();
    assert_eq!(encoded.get_value("verify"), Some(1));
    assert_eq!(encoded.get_value("missing"), None);
    assert_eq!(encoded.len(), commands.len());
}

#[test]
fn example() {
    main();
}

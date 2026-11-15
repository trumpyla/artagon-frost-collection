//! Persistent snapshots, value-yielding bitsets, and generational keys.
//!
//! Run with `cargo run --example advanced --features
//! imbl-7,roaring-0_12,fixedbitset-0_5,slotmap-1`.

use frost::backend::{fixedbitset, imbl, roaring, slotmap};
use frost::read::{Cardinality, ValueElements};
use frost::{FreezeWith, Retain};

fn main() {
    let mut catalog = imbl::OrdMap::new();
    catalog.insert(String::from("queued"), 3);
    let mut next_catalog = catalog.clone();
    let snapshot = catalog.freeze_with(Retain);
    next_catalog.insert(String::from("queued"), 4);
    assert_eq!(snapshot.get("queued"), Some(&3));
    assert_eq!(next_catalog.get("queued"), Some(&4));

    let sparse: roaring::RoaringTreemap = [7, u64::MAX].into_iter().collect();
    let sparse = sparse.freeze_with(Retain);
    assert_eq!(Cardinality::cardinality(&sparse), 2);
    assert_eq!(
        ValueElements::values(&sparse).collect::<Vec<_>>(),
        [7, u64::MAX]
    );

    let mut dense = fixedbitset::FixedBitSet::with_capacity(256);
    dense.insert(7);
    let dense = dense.freeze_with(Retain);
    assert_eq!(dense.universe_len(), 256);
    assert_eq!(dense.cardinality(), 1);
    assert_eq!(dense.ones().collect::<Vec<_>>(), [7]);

    let mut objects = slotmap::SlotMap::new();
    let old_key = objects.insert("retired");
    objects.remove(old_key);
    let current_key = objects.insert("ready");
    let objects = objects.freeze_with(Retain);
    assert_eq!(objects.get(old_key), None);
    assert_eq!(objects.get(current_key), Some(&"ready"));
    println!(
        "Persistent snapshot: {snapshot:?}; sparse members: {}; dense universe: {}; current object: {:?}",
        sparse.cardinality(),
        dense.universe_len(),
        objects.get(current_key)
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn executes_advanced_example() {
        super::main();
    }
}

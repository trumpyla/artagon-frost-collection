//! `FROZEN-STABLE` from `docs/design/algebra/frozen.md`, against a real frozen family.

use core::cell::Cell;
use frost::family::FrozenVec;

/// A frozen sequence fixes its structure, not the state inside its elements.
#[test]
fn frozen_structure_does_not_freeze_interior_state() {
    let frozen: FrozenVec<Cell<u8>> = vec![Cell::new(1)].into();
    let before: Vec<u8> = frozen.iter().map(Cell::get).collect();
    frozen[0].set(2);
    let after: Vec<u8> = frozen.iter().map(Cell::get).collect();
    assert_eq!(frozen.len(), 1);
    assert_eq!((before, after), (vec![1], vec![2]));
}

/// Materializing references keeps them tied to their source.
#[test]
fn materialized_references_still_borrow_their_source() {
    let source = [String::from("a"), String::from("b")];
    let refs: FrozenVec<&String> = source.iter().collect::<Vec<_>>().into();
    assert!(core::ptr::eq(refs[0], source.as_ptr()));
}

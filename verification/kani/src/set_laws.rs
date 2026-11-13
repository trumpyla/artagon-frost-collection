//! Refinement of `SET-REP` and `SET-BOOLEAN` (`docs/design/algebra/sets.md`) on
//! the real `FrozenOrderedSet` merge kernels.
//!
//! Each element carries a key and a label; equality and order use the key only,
//! so the label records which stored object an operation returned. The
//! abstraction maps a set to the label stored for each key. Each proof asserts
//! that the abstraction of the real result equals the reference operation on
//! the abstractions of the operands, for every operand mask and every label.
//! The bounds are three keys and one level of nesting.

use alloc::vec::Vec;
use core::cmp::Ordering;
use frost::family::FrozenOrderedSet;

const KEYS: u8 = 3;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Tagged {
    key: u8,
    label: u8,
}

impl PartialEq for Tagged {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for Tagged {}
impl PartialOrd for Tagged {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Tagged {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    Union,
    Intersection,
    Difference,
    Xor,
}

pub(crate) const OPS: [Op; 4] = [Op::Union, Op::Intersection, Op::Difference, Op::Xor];

/// The label stored for each key, or `None` when the key is absent.
type Slots = [Option<u8>; KEYS as usize];

/// The reference rule: left on union and intersection, the left operand on
/// difference, the sole surviving operand on symmetric difference.
fn reference(op: Op, a: Slots, b: Slots) -> Slots {
    core::array::from_fn(|k| match op {
        Op::Union => a[k].or(b[k]),
        Op::Intersection => a[k].filter(|_| b[k].is_some()),
        Op::Difference => a[k].filter(|_| b[k].is_none()),
        Op::Xor => match (a[k], b[k]) {
            (Some(x), None) | (None, Some(x)) => Some(x),
            _ => None,
        },
    })
}

/// The abstraction. It also asserts that keys arrive strictly ascending, so
/// each key is yielded at most once.
fn slots<'a>(found: impl IntoIterator<Item = &'a Tagged>) -> Slots {
    let mut out = [None; KEYS as usize];
    let mut last: Option<u8> = None;
    for item in found {
        assert!(last < Some(item.key), "keys must be strictly ascending");
        assert!(item.key < KEYS);
        out[usize::from(item.key)] = Some(item.label);
        last = Some(item.key);
    }
    out
}

fn set(mask: u8, labels: [u8; KEYS as usize]) -> FrozenOrderedSet<Tagged> {
    let items: Vec<Tagged> = (0..KEYS)
        .filter(|k| mask & (1 << k) != 0)
        .map(|key| Tagged {
            key,
            label: labels[usize::from(key)],
        })
        .collect();
    FrozenOrderedSet::try_from_sorted_unique(items).unwrap()
}

fn apply<'a>(
    op: Op,
    a: &'a FrozenOrderedSet<Tagged>,
    b: &'a FrozenOrderedSet<Tagged>,
) -> Vec<&'a Tagged> {
    match op {
        Op::Union => a.union(b).collect(),
        Op::Intersection => a.intersection(b).collect(),
        Op::Difference => a.difference(b).collect(),
        Op::Xor => a.symmetric_difference(b).collect(),
    }
}

fn materialize(found: Vec<&Tagged>) -> FrozenOrderedSet<Tagged> {
    FrozenOrderedSet::try_from_sorted_unique(found.into_iter().copied().collect()).unwrap()
}

fn borrowed_from(set: &FrozenOrderedSet<Tagged>, item: &Tagged) -> bool {
    set.as_slice()
        .as_ptr_range()
        .contains(&core::ptr::from_ref(item))
}

/// One operation: the result refines the reference, and every yielded object
/// is borrowed from the operand that holds its key, the left one when both do.
pub(crate) fn representatives(op: Op, masks: [u8; 2], labels: [[u8; KEYS as usize]; 2]) {
    assert!(masks[0] < 1 << KEYS && masks[1] < 1 << KEYS);
    let (a, b) = (set(masks[0], labels[0]), set(masks[1], labels[1]));
    let found = apply(op, &a, &b);
    let expected = reference(op, slots(a.iter()), slots(b.iter()));
    for item in &found {
        if masks[0] & (1 << item.key) != 0 {
            assert!(borrowed_from(&a, item));
        } else {
            assert!(borrowed_from(&b, item));
        }
    }
    assert_eq!(slots(found), expected);
}

/// Both groupings of a union or an intersection, with each intermediate
/// materialized as a real frozen set, select the same stored objects as the
/// reference applied left to right.
pub(crate) fn nested(op: Op, masks: [u8; 3], labels: [[u8; KEYS as usize]; 3]) {
    assert!(matches!(op, Op::Union | Op::Intersection));
    assert!(masks.iter().all(|&m| m < 1 << KEYS));
    let a = set(masks[0], labels[0]);
    let b = set(masks[1], labels[1]);
    let c = set(masks[2], labels[2]);
    let ab = materialize(apply(op, &a, &b));
    let bc = materialize(apply(op, &b, &c));
    let expected = reference(
        op,
        reference(op, slots(a.iter()), slots(b.iter())),
        slots(c.iter()),
    );
    assert_eq!(slots(apply(op, &ab, &c)), expected);
    assert_eq!(slots(apply(op, &a, &bc)), expected);
}

#[cfg(kani)]
mod proofs {
    use super::{Op, nested, representatives};

    fn op() -> Op {
        match kani::any::<u8>() % 4 {
            0 => Op::Union,
            1 => Op::Intersection,
            2 => Op::Difference,
            _ => Op::Xor,
        }
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn ordered_set_operations_refine_the_representative_rule() {
        let masks: [u8; 2] = kani::any();
        kani::assume(masks[0] < 8 && masks[1] < 8);
        representatives(op(), masks, kani::any());
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn nested_ordered_union_and_intersection_keep_the_first_representative() {
        let masks: [u8; 3] = kani::any();
        kani::assume(masks.iter().all(|&m| m < 8));
        let op = if kani::any() {
            Op::Union
        } else {
            Op::Intersection
        };
        nested(op, masks, kani::any());
    }
}

#[cfg(test)]
mod tests {
    use super::{OPS, Op, nested, representatives};

    const LABELS: [[u8; 3]; 3] = [[10, 11, 12], [20, 21, 22], [30, 31, 32]];

    #[test]
    fn representative_helpers_execute_against_production_code() {
        for a in 0..8 {
            for b in 0..8 {
                for op in OPS {
                    representatives(op, [a, b], [LABELS[0], LABELS[1]]);
                }
                for c in 0..8 {
                    for op in [Op::Union, Op::Intersection] {
                        nested(op, [a, b, c], LABELS);
                    }
                }
            }
        }
    }
}

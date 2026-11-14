//! Refinement of `SET-REP` and `SET-BOOLEAN` (`docs/design/algebra/sets.md`) on
//! the real `FrozenOrderedSet` merge kernels.
//!
//! Each element carries a key and a label; equality and order use the key only,
//! so the label records which stored object an operation returned. The
//! abstraction maps a set to the label stored for each key. Each proof asserts
//! that the abstraction of the real result equals the reference operation on
//! the abstractions of the operands, for every operand mask. Labels are fixed
//! and distinct per operand: the kernels never read them. The bounds are three
//! keys and one level of nesting. At three keys only the merge strategy runs:
//! the seeking strategies of intersection and difference need an operand at
//! least 32 times longer than the other and are outside these proofs.

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

const EMPTY: Slots = [None; KEYS as usize];

/// The reference rule: left on union and intersection, the left operand on
/// difference, the sole surviving operand on symmetric difference.
fn reference(op: Op, a: Slots, b: Slots) -> Slots {
    let mut out = EMPTY;
    for k in 0..usize::from(KEYS) {
        out[k] = match (op, a[k], b[k]) {
            (Op::Union, Some(x), _) | (Op::Union, None, Some(x)) => Some(x),
            (Op::Intersection, Some(x), Some(_)) => Some(x),
            (Op::Difference, Some(x), None) => Some(x),
            (Op::Xor, Some(x), None) | (Op::Xor, None, Some(x)) => Some(x),
            _ => None,
        };
    }
    out
}

/// The abstraction. It also asserts that keys arrive strictly ascending, so
/// each key is yielded at most once.
fn slots<'a>(found: impl Iterator<Item = &'a Tagged>) -> Slots {
    let mut out = EMPTY;
    let mut next = 0;
    for item in found {
        assert!(next <= item.key && item.key < KEYS);
        out[usize::from(item.key)] = Some(item.label);
        next = item.key + 1;
    }
    out
}

/// Builds the set for a concrete `mask`, so the allocation has a fixed size.
fn build(mask: u8, labels: [u8; KEYS as usize]) -> FrozenOrderedSet<Tagged> {
    let mut items = Vec::with_capacity(mask.count_ones() as usize);
    for key in 0..KEYS {
        if mask & (1 << key) != 0 {
            items.push(Tagged {
                key,
                label: labels[usize::from(key)],
            });
        }
    }
    match FrozenOrderedSet::try_from_sorted_unique(items) {
        Ok(set) => set,
        Err(_) => panic!("keys are pushed in ascending order"),
    }
}

/// Dispatches a possibly symbolic `mask` to one `build` call per concrete
/// value. A symbolic allocation size exhausts the model checker's memory; a
/// choice among eight fixed-size allocations does not.
fn set(mask: u8, labels: [u8; KEYS as usize]) -> FrozenOrderedSet<Tagged> {
    for candidate in 0..1 << KEYS {
        if candidate == mask {
            return build(candidate, labels);
        }
    }
    panic!("mask out of range")
}

/// The abstraction of `op(a, b)`, read straight off the real iterator.
fn apply(op: Op, a: &FrozenOrderedSet<Tagged>, b: &FrozenOrderedSet<Tagged>) -> Slots {
    match op {
        Op::Union => slots(a.union(b)),
        Op::Intersection => slots(a.intersection(b)),
        Op::Difference => slots(a.difference(b)),
        Op::Xor => slots(a.symmetric_difference(b)),
    }
}

/// `op(a, b)` as a real frozen set holding copies of the yielded objects.
fn materialize(
    op: Op,
    a: &FrozenOrderedSet<Tagged>,
    b: &FrozenOrderedSet<Tagged>,
) -> FrozenOrderedSet<Tagged> {
    let found = apply(op, a, b);
    let (mut mask, mut labels) = (0, [0; KEYS as usize]);
    for key in 0..KEYS {
        if let Some(label) = found[usize::from(key)] {
            mask |= 1 << key;
            labels[usize::from(key)] = label;
        }
    }
    set(mask, labels)
}

/// One operation: the result refines the reference. Labels are distinct per
/// operand, so the label of each yielded object names the operand it came from.
pub(crate) fn representatives(op: Op, masks: [u8; 2], labels: [[u8; KEYS as usize]; 2]) {
    assert!(masks[0] < 1 << KEYS && masks[1] < 1 << KEYS);
    let (a, b) = (set(masks[0], labels[0]), set(masks[1], labels[1]));
    let expected = reference(op, slots(a.iter()), slots(b.iter()));
    assert!(apply(op, &a, &b) == expected);
}

/// Both groupings of a union or an intersection, with each intermediate
/// materialized as a real frozen set, select the same stored objects as the
/// reference applied left to right.
pub(crate) fn nested(op: Op, masks: [u8; 3], labels: [[u8; KEYS as usize]; 3]) {
    assert!(matches!(op, Op::Union | Op::Intersection));
    assert!(masks[0] < 1 << KEYS && masks[1] < 1 << KEYS && masks[2] < 1 << KEYS);
    let a = set(masks[0], labels[0]);
    let b = set(masks[1], labels[1]);
    let c = set(masks[2], labels[2]);
    let expected = reference(
        op,
        reference(op, slots(a.iter()), slots(b.iter())),
        slots(c.iter()),
    );
    let ab = materialize(op, &a, &b);
    assert!(apply(op, &ab, &c) == expected);
    let bc = materialize(op, &b, &c);
    assert!(apply(op, &a, &bc) == expected);
}

/// Distinct labels per operand. The kernels never read a label, so fixed
/// distinct values identify the source of every yielded object.
const LABELS: [[u8; KEYS as usize]; 3] = [[10, 11, 12], [20, 21, 22], [30, 31, 32]];

#[cfg(kani)]
mod proofs {
    use super::{LABELS, Op, nested, representatives};

    // Union and symmetric difference are one merge loop, so their operand
    // masks stay symbolic. Intersection and difference choose a strategy from
    // the operand lengths; with symbolic lengths the model checker explores
    // the seeking strategies as well and exhausts memory, so those proofs
    // enumerate every mask concretely inside the model checker instead. That
    // costs minutes per 64 pairs, so nested intersection (512 triples) has no
    // proof here; the native test below runs every triple.

    fn symbolic_pair(op: Op) {
        let masks: [u8; 2] = kani::any();
        kani::assume(masks[0] < 8 && masks[1] < 8);
        representatives(op, masks, [LABELS[0], LABELS[1]]);
    }

    fn every_pair(op: Op) {
        for a in 0..8 {
            for b in 0..8 {
                representatives(op, [a, b], [LABELS[0], LABELS[1]]);
            }
        }
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn ordered_union_refines_the_representative_rule() {
        symbolic_pair(Op::Union);
    }

    #[kani::proof]
    #[kani::unwind(9)]
    fn ordered_intersection_refines_the_representative_rule() {
        every_pair(Op::Intersection);
    }

    #[kani::proof]
    #[kani::unwind(9)]
    fn ordered_difference_refines_the_representative_rule() {
        every_pair(Op::Difference);
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn ordered_symmetric_difference_refines_the_representative_rule() {
        symbolic_pair(Op::Xor);
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn nested_ordered_union_keeps_the_first_representative() {
        let masks: [u8; 3] = kani::any();
        kani::assume(masks[0] < 8 && masks[1] < 8 && masks[2] < 8);
        nested(Op::Union, masks, LABELS);
    }
}

#[cfg(test)]
mod tests {
    use super::{LABELS, OPS, Op, nested, representatives};

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

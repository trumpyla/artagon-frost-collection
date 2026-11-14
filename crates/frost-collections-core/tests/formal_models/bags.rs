//! `BAG-LAWS`, `BAG-SUPPORT`, and `COUNT-PARTIAL` from `docs/design/algebra/bags.md`.
//!
//! Frost implements no bag yet; these check the proposed definitions.

use core::iter::repeat_n;
use itertools::Itertools;

/// A bag over a fixed set of keys: slot `k` is the multiplicity of key `k`.
type Bag = Vec<u32>;
type CountOp = fn(u32, u32) -> u32;

fn sum(a: u32, b: u32) -> u32 {
    a + b
}
fn max_union(a: u32, b: u32) -> u32 {
    a.max(b)
}
fn intersection(a: u32, b: u32) -> u32 {
    a.min(b)
}
fn difference(a: u32, b: u32) -> u32 {
    a.saturating_sub(b)
}
fn absolute_difference(a: u32, b: u32) -> u32 {
    a.abs_diff(b)
}

fn pointwise(a: &Bag, b: &Bag, op: CountOp) -> Bag {
    a.iter().zip(b).map(|(&x, &y)| op(x, y)).collect()
}

fn support(bag: &Bag) -> u32 {
    bag.iter()
        .enumerate()
        .filter(|(_, n)| **n > 0)
        .map(|(k, _)| 1 << k)
        .sum()
}

fn bags() -> Vec<Bag> {
    repeat_n(0..4, 2).multi_cartesian_product().collect()
}

/// Scalar identities that tie the operations to each other. They pin each
/// definition, unlike associativity, which several wrong definitions share.
fn identities_hold(min: CountOp, monus: CountOp, max: CountOp, abs: CountOp) -> bool {
    (0..4).all(|a| {
        (0..4).all(|b| {
            min(a, b) + monus(a, b) == a
                && max(a, b) == a + monus(b, a)
                && abs(a, b) == monus(a, b) + monus(b, a)
                && (min(a, b) == a || min(a, b) == b)
        })
    })
}

/// `BAG-LAWS` over every pair and triple of two-key bags with counts 0..=3.
#[test]
fn count_operations_satisfy_their_laws() {
    let all = bags();
    for a in &all {
        assert_eq!(pointwise(a, a, absolute_difference), vec![0; 2]);
        for b in &all {
            for op in [sum, max_union, intersection, absolute_difference] {
                assert_eq!(pointwise(a, b, op), pointwise(b, a, op));
            }
            for c in &all {
                for op in [sum, max_union, intersection] {
                    assert_eq!(
                        pointwise(&pointwise(a, b, op), c, op),
                        pointwise(a, &pointwise(b, c, op), op)
                    );
                }
                let (bc_max, bc_min) = (pointwise(b, c, max_union), pointwise(b, c, intersection));
                assert_eq!(
                    pointwise(a, &bc_min, max_union),
                    pointwise(
                        &pointwise(a, b, max_union),
                        &pointwise(a, c, max_union),
                        intersection
                    )
                );
                assert_eq!(
                    pointwise(a, &bc_max, intersection),
                    pointwise(
                        &pointwise(a, b, intersection),
                        &pointwise(a, c, intersection),
                        max_union
                    )
                );
            }
        }
    }
    assert!(identities_hold(
        intersection,
        difference,
        max_union,
        absolute_difference
    ));
    // Absolute difference is not Boolean XOR: it does not reassociate.
    assert_eq!(absolute_difference(absolute_difference(1, 1), 2), 2);
    assert_eq!(absolute_difference(1, absolute_difference(1, 2)), 0);
    // Difference is not an additive inverse.
    assert_ne!(difference(1, 2) + 2, 1);
}

/// Wrong definitions that still associate are rejected by the identities.
#[test]
fn identities_reject_wrong_count_definitions() {
    let product: CountOp = |a, b| a * b;
    let left: CountOp = |a, _| a;
    assert!(!identities_hold(
        product,
        difference,
        max_union,
        absolute_difference
    ));
    assert!(!identities_hold(
        intersection,
        left,
        max_union,
        absolute_difference
    ));
}

/// `BAG-SUPPORT`: support maps sum and max to union and min to intersection,
/// but not monus to difference or absolute difference to XOR.
#[test]
fn support_preserves_only_the_lattice_and_sum() {
    let all = bags();
    for a in &all {
        for b in &all {
            assert_eq!(support(&pointwise(a, b, sum)), support(a) | support(b));
            assert_eq!(
                support(&pointwise(a, b, max_union)),
                support(a) | support(b)
            );
            assert_eq!(
                support(&pointwise(a, b, intersection)),
                support(a) & support(b)
            );
        }
    }
    let (two, one) = (vec![2], vec![1]);
    assert_eq!(support(&pointwise(&two, &one, difference)), 1);
    assert_eq!(support(&two) & !support(&one), 0);
    let (one, two) = (vec![1], vec![2]);
    assert_eq!(support(&pointwise(&one, &two, absolute_difference)), 1);
    assert_eq!(support(&one) ^ support(&two), 0);
}

fn checked(bound: u32, a: Option<u32>, b: Option<u32>) -> Option<u32> {
    a.zip(b).map(|(x, y)| x + y).filter(|&total| total <= bound)
}

/// `COUNT-PARTIAL`: with a counter bounded by `M`, both groupings of a checked
/// sum succeed exactly when the total fits, and then agree.
#[test]
fn checked_sum_succeeds_in_either_grouping_exactly_when_the_total_fits() {
    for bound in 0..8 {
        for a in 0..=bound {
            for b in 0..=bound {
                for c in 0..=bound {
                    let expected = Some(a + b + c).filter(|&t| t <= bound);
                    assert_eq!(
                        checked(bound, checked(bound, Some(a), Some(b)), Some(c)),
                        expected
                    );
                    assert_eq!(
                        checked(bound, Some(a), checked(bound, Some(b), Some(c))),
                        expected
                    );
                }
            }
        }
    }
    let wrapping = |a: u32, b: u32| Some((a + b) % 4);
    assert_ne!(
        wrapping(3, 1),
        checked(3, Some(3), Some(1)),
        "wrapping is a different operation"
    );
}

/// The left-present representative policy: a positive result keeps the left
/// object when the left count is positive, otherwise the right one.
fn left_present(a: (u32, char), b: (u32, char), op: CountOp) -> (u32, Option<char>) {
    let count = op(a.0, b.0);
    let rep = (count > 0).then_some(if a.0 > 0 { a.1 } else { b.1 });
    (count, rep)
}

/// The policy associates for sum, max and min on counts 0..=3.
#[test]
fn left_present_representatives_associate_for_sum_max_and_min() {
    let carry = |(count, rep): (u32, Option<char>), fallback| (count, rep.unwrap_or(fallback));
    for op in [sum, max_union, intersection] {
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let (a, b, c) = ((x, 'a'), (y, 'b'), (z, 'c'));
                    let left = left_present(carry(left_present(a, b, op), '-'), c, op);
                    let right = left_present(a, carry(left_present(b, c, op), '-'), op);
                    assert_eq!(left, right, "{x} {y} {z}");
                }
            }
        }
    }
}

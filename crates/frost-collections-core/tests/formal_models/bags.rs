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

/// `BAG-MONUS`: the ten identities of bag difference, as a function of the
/// difference operation, over every triple of counts 0..=6.
fn monus_identities_hold(monus: CountOp) -> bool {
    (0..7).all(|a| {
        (0..7).all(|b| {
            (0..7).all(|c| {
                monus(a + b, b) == a
                    && monus(a, b + c) == monus(monus(a, b), c)
                    && monus(a, b) + b == a.max(b)
                    && monus(a, b) + a.min(b) == a
                    && monus(a, monus(a, b)) == a.min(b)
                    && monus(a.max(b), c) == monus(a, c).max(monus(b, c))
                    && monus(a.min(b), c) == monus(a, c).min(monus(b, c))
                    && monus(a, b.max(c)) == monus(a, b).min(monus(a, c))
                    && monus(a, b.min(c)) == monus(a, b).max(monus(a, c))
                    && monus(b, a) * c == monus(b * c, a * c)
            })
        })
    })
}

/// The identities hold for truncated subtraction and reject absolute difference.
#[test]
fn bag_difference_satisfies_the_monus_identities() {
    assert!(monus_identities_hold(difference));
    assert!(!monus_identities_hold(absolute_difference));
}

type Ternary = fn(u32, u32, u32) -> u32;

/// A triple of inputs with the differing left and right values.
type Difference = ([u32; 3], u32, u32);

/// The first triple below `n`, in lexicographic order, on which the two sides
/// differ, with both values.
fn first_difference(n: u32, lhs: Ternary, rhs: Ternary) -> Option<Difference> {
    repeat_n(0..n, 3)
        .multi_cartesian_product()
        .map(|t| {
            (
                [t[0], t[1], t[2]],
                lhs(t[0], t[1], t[2]),
                rhs(t[0], t[1], t[2]),
            )
        })
        .find(|(_, left, right)| left != right)
}

/// `BAG-MONUS`: four identities of set difference hold on every triple of
/// subsets of three keys, and fail for bags when union is read as sum and
/// intersection as min. The first failing triple of counts is recorded.
#[test]
fn set_difference_identities_fail_for_bags() {
    let set_forms: [(Ternary, Ternary); 4] = [
        (|a, b, _| (a | b) & !b, |a, b, _| a & !b),
        (|a, b, c| (a | b) & !c, |a, b, c| (a & !c) | (b & !c)),
        (|a, b, c| a & (b & !c), |a, b, c| (a & b) & !c),
        (|a, b, c| a & !(b & !c), |a, b, c| (a & !b) | (a & c)),
    ];
    for (lhs, rhs) in set_forms {
        assert_eq!(first_difference(8, lhs, rhs), None);
    }

    let bag_forms: [(Ternary, Ternary, Difference); 4] = [
        // (A sum B) monus B is A, not A monus B.
        (
            |a, b, _| difference(a + b, b),
            |a, b, _| difference(a, b),
            ([1, 1, 0], 1, 0),
        ),
        // Monus does not distribute over bag sum.
        (
            |a, b, c| difference(a + b, c),
            |a, b, c| difference(a, c) + difference(b, c),
            ([1, 1, 1], 1, 0),
        ),
        // Min with a difference is not the difference of the min.
        (
            |a, b, c| a.min(difference(b, c)),
            |a, b, c| difference(a.min(b), c),
            ([1, 2, 1], 1, 0),
        ),
        // Nested difference does not split into difference and min.
        (
            |a, b, c| difference(a, difference(b, c)),
            |a, b, c| difference(a, b).max(a.min(c)),
            ([1, 2, 1], 0, 1),
        ),
    ];
    for (lhs, rhs, expected) in bag_forms {
        assert_eq!(first_difference(7, lhs, rhs), Some(expected));
    }
}

/// Whether `monus(b, a)` is the least `z` with `max(a, z) >= b` on the chain
/// `0 < 1 < 2`, where max is the sum.
fn is_chain_monus(monus: CountOp) -> bool {
    (0..3).all(|a| (0..3).all(|b| (0..3).all(|z| (a.max(z) >= b) == (z >= monus(b, a)))))
}

/// A different weight algebra: the chain `0 < 1 < 2` with max as sum and min
/// as product. Its monus is not numeric subtraction, and product does not
/// distribute over it, so that identity of `BAG-MONUS` is specific to counts.
#[test]
fn product_does_not_distribute_over_monus_on_the_max_min_chain() {
    let chain_monus: CountOp = |b, a| if b <= a { 0 } else { b };
    assert!(is_chain_monus(chain_monus));
    assert!(!is_chain_monus(difference));
    assert_eq!(
        first_difference(
            3,
            |a, b, c| (if b <= a { 0 } else { b }).min(c),
            |a, b, c| if b.min(c) <= a.min(c) { 0 } else { b.min(c) },
        ),
        Some(([1, 2, 1], 1, 0))
    );
}

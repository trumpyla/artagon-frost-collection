//! `DOM-IMAGE`, `DOM-CONGRUENCE`, `DOM-VALID`, and `DOM-NORMALIZE` from
//! `docs/design/algebra/domains.md`, plus subcarrier closure from the formal model.

use super::common::{Key3, is_subset, members};
use core::iter::repeat_n;
use exhaust::Exhaust;
use itertools::Itertools;

const KEYS: u32 = 3;

/// Every total function from three keys to three keys, as `f[key]`.
fn functions() -> Vec<Vec<u32>> {
    <[Key3; 3]>::exhaust()
        .map(|f| f.map(Key3::index).to_vec())
        .collect()
}

fn image(f: &[u32], set: u32) -> u32 {
    members(set, KEYS)
        .map(|key| 1 << f[key as usize])
        .fold(0, |acc, bit| acc | bit)
}

fn preimage(f: &[u32], set: u32) -> u32 {
    (0..KEYS)
        .filter(|&key| set & (1 << f[key as usize]) != 0)
        .map(|key| 1 << key)
        .sum()
}

fn injective(f: &[u32]) -> bool {
    (0..KEYS).all(|x| (0..x).all(|y| f[x as usize] != f[y as usize]))
}

/// Direct image always preserves union. It preserves intersection, difference
/// and XOR for every pair exactly when the conversion is injective; otherwise
/// the error has a fixed direction.
#[test]
fn direct_image_needs_injectivity_beyond_union() {
    let mut saw_collapsing = false;
    for f in functions() {
        let mut keeps = [true; 3];
        for a in 0..1 << KEYS {
            for b in 0..1 << KEYS {
                assert_eq!(image(&f, a | b), image(&f, a) | image(&f, b));
                keeps[0] &= image(&f, a & b) == image(&f, a) & image(&f, b);
                keeps[1] &= image(&f, a & !b) == image(&f, a) & !image(&f, b);
                keeps[2] &= image(&f, a ^ b) == image(&f, a) ^ image(&f, b);
                assert!(is_subset(image(&f, a & b), image(&f, a) & image(&f, b)));
                assert!(is_subset(image(&f, a) & !image(&f, b), image(&f, a & !b)));
                assert!(is_subset(image(&f, a) ^ image(&f, b), image(&f, a ^ b)));
                assert_eq!(preimage(&f, a | b), preimage(&f, a) | preimage(&f, b));
                assert_eq!(preimage(&f, a & b), preimage(&f, a) & preimage(&f, b));
                assert_eq!(preimage(&f, a & !b), preimage(&f, a) & !preimage(&f, b));
                assert_eq!(preimage(&f, a ^ b), preimage(&f, a) ^ preimage(&f, b));
            }
        }
        assert_eq!(keeps, [injective(&f); 3], "f = {f:?}");
        saw_collapsing |= !injective(&f);
    }
    assert!(saw_collapsing);
}

/// Grant and deny rules over collapsed keys. Converting before the
/// intersection can invent a grant; converting before the difference can remove one.
#[test]
fn collapsing_conversion_changes_grant_and_deny_answers() {
    let collapse = [0, 0, 0];
    let rule = |granted: u32, scope: u32, denied: u32| (granted & scope) & !denied;
    let exact = |g, s, d| image(&collapse, rule(g, s, d));
    let early = |g, s, d| {
        rule(
            image(&collapse, g),
            image(&collapse, s),
            image(&collapse, d),
        )
    };
    // Distinct granted and scoped keys that share a class: a grant appears.
    assert_eq!((exact(0b001, 0b010, 0), early(0b001, 0b010, 0)), (0, 1));
    // A denied key that shares a class with a granted one: a grant disappears.
    assert_eq!(
        (exact(0b001, 0b001, 0b010), early(0b001, 0b001, 0b010)),
        (1, 0)
    );
}

fn push(f: &[u32], bag: &[u32]) -> Vec<u32> {
    (0..KEYS)
        .map(|l| {
            (0..KEYS)
                .filter(|&k| f[k as usize] == l)
                .map(|k| bag[k as usize])
                .sum()
        })
        .collect()
}

/// Bag pushforward adds counts along each fiber. It preserves sum for every
/// conversion and max, min and monus only for injective ones.
#[test]
fn bag_pushforward_preserves_sum_and_needs_injectivity_for_the_rest() {
    let bags: Vec<Vec<u32>> = repeat_n(0..3, KEYS as usize)
        .multi_cartesian_product()
        .collect();
    let pointwise = |a: &[u32], b: &[u32], op: fn(u32, u32) -> u32| -> Vec<u32> {
        a.iter().zip(b).map(|(&x, &y)| op(x, y)).collect()
    };
    let ops: [fn(u32, u32) -> u32; 3] = [u32::max, u32::min, u32::saturating_sub];
    for f in functions() {
        let mut keeps = [true; 3];
        for a in &bags {
            for b in &bags {
                assert_eq!(
                    push(&f, &pointwise(a, b, |x, y| x + y)),
                    pointwise(&push(&f, a), &push(&f, b), |x, y| x + y)
                );
                for (keep, op) in keeps.iter_mut().zip(ops) {
                    *keep &=
                        push(&f, &pointwise(a, b, op)) == pointwise(&push(&f, a), &push(&f, b), op);
                }
            }
        }
        assert_eq!(keeps, [injective(&f); 3], "f = {f:?}");
    }
}

/// A filter on stored values commutes with conversion to key classes exactly
/// when it is constant on each class.
#[test]
fn class_filters_commute_with_conversion_exactly_when_congruent() {
    for f in functions() {
        for predicate in 0..1 << KEYS {
            let congruent = (0..KEYS).all(|x| {
                (0..KEYS).all(|y| {
                    f[x as usize] != f[y as usize]
                        || ((predicate >> x) & 1) == ((predicate >> y) & 1)
                })
            });
            // On classes, keep a class when any member satisfies the predicate.
            let class_predicate = image(&f, predicate);
            let commutes = (0..1 << KEYS)
                .all(|set| image(&f, set & predicate) == image(&f, set) & class_predicate);
            assert_eq!(
                commutes, congruent,
                "f = {f:?}, predicate = {predicate:03b}"
            );
        }
    }
}

fn percentage(value: f64) -> bool {
    value.fract() == 0.0 && (0.0..=100.0).contains(&value)
}

/// `DOM-VALID`: the admitted percentages, from a table rather than a restated predicate.
#[test]
fn percentage_validation_matches_its_boundary_table() {
    for (value, valid) in [
        (-1.0, false),
        (0.0, true),
        (1.0, true),
        (99.0, true),
        (100.0, true),
        (101.0, false),
        (0.5, false),
        (f64::NAN, false),
        (80.0 + 50.0, false),
    ] {
        assert_eq!(percentage(value), valid, "{value}");
    }
}

/// `DOM-NORMALIZE`: normalization preserves meaning and is idempotent on its
/// outputs; idempotence alone does not preserve identity.
#[test]
fn normalization_preserves_meaning_but_idempotence_alone_does_not() {
    let normalize = |raw: &str| raw.trim().parse::<u32>().ok();
    for (raw, meaning) in [(" 7 ", 7), ("7", 7), ("\t42\n", 42), ("0", 0)] {
        assert_eq!(normalize(raw), Some(meaning));
        assert_eq!(normalize(&meaning.to_string()), Some(meaning));
    }
    let constant = |_: u32| 0_u32;
    assert!((0..3).all(|x| constant(constant(x)) == constant(x)));
    let classes: std::collections::BTreeSet<u32> = (0..3).map(constant).collect();
    assert_eq!(
        classes.len(),
        1,
        "an idempotent normalizer can collapse every key"
    );
}

/// Closure on the full carrier does not give closure on a restricted one.
#[test]
fn subcarrier_restrictions_are_not_closed() {
    let nonempty = |set: u32| set != 0;
    assert!(nonempty(0b01) && nonempty(0b10) && !nonempty(0b01 & 0b10));
    let at_most_one = |set: u32| set.count_ones() <= 1;
    assert!(at_most_one(0b01) && at_most_one(0b10) && !at_most_one(0b01 | 0b10));
    // IEEE equality is not reflexive, so floats are not lawful keys as they are.
    let nan = f64::NAN;
    assert!(nan.partial_cmp(&nan).is_none());
}

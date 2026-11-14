//! `SET-BOOLEAN` and `SET-REP` from `docs/design/algebra/sets.md`, checked against
//! the model and against `FrozenOrderedSet` and `FrozenHashSet`.

use super::common::{is_subset, members};
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use frost::family::{FrozenHashSet, FrozenOrderedSet};
use itertools::iproduct;

const KEYS: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Union,
    Intersection,
    Difference,
    Xor,
}

const OPS: [Op; 4] = [Op::Union, Op::Intersection, Op::Difference, Op::Xor];

/// A set with stored representatives: slot `k` holds the label of the object
/// stored for key `k`, or `None` when `k` is absent.
type Rep = Vec<Option<char>>;

/// A rule choosing the result slot from the two operand slots.
type Rule = fn(Option<char>, Option<char>, Op) -> Option<char>;

/// The documented rules: left on union and intersection, the left operand on
/// difference, and the sole surviving operand on symmetric difference.
fn frost_rule(left: Option<char>, right: Option<char>, op: Op) -> Option<char> {
    match op {
        Op::Union => left.or(right),
        Op::Intersection => left.filter(|_| right.is_some()),
        Op::Difference => left.filter(|_| right.is_none()),
        Op::Xor => match (left, right) {
            (Some(_), None) => left,
            (None, Some(_)) => right,
            _ => None,
        },
    }
}

fn combine(rule: Rule, a: &Rep, b: &Rep, op: Op) -> Rep {
    a.iter().zip(b).map(|(&x, &y)| rule(x, y, op)).collect()
}

/// The set of `n` keys selected by `mask`, each stored with `label`.
fn labelled(mask: u32, n: u32, label: char) -> Rep {
    (0..n)
        .map(|k| (mask & (1 << k) != 0).then_some(label))
        .collect()
}

fn mask(set: &Rep) -> u32 {
    set.iter()
        .enumerate()
        .filter(|(_, slot)| slot.is_some())
        .map(|(k, _)| 1 << k)
        .sum()
}

fn apply(op: Op, a: u32, b: u32) -> u32 {
    match op {
        Op::Union => a | b,
        Op::Intersection => a & b,
        Op::Difference => a & !b,
        Op::Xor => a ^ b,
    }
}

/// A key whose equality, order and hash use only `id`; `label` records which
/// operand supplied the stored object.
#[derive(Clone, Copy, Debug)]
struct Key {
    id: u32,
    label: char,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Key {}
impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}
impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

fn keys(set: &Rep) -> Vec<Key> {
    set.iter()
        .enumerate()
        .filter_map(|(id, slot)| {
            slot.map(|label| Key {
                id: u32::try_from(id).expect("few keys"),
                label,
            })
        })
        .collect()
}

/// A result as a slot vector, from stored objects in any order.
fn slots<'a>(n: u32, found: impl IntoIterator<Item = &'a Key>) -> Rep {
    let mut out = vec![None; n as usize];
    for key in found {
        assert!(
            out[key.id as usize].is_none(),
            "key {} emitted twice",
            key.id
        );
        out[key.id as usize] = Some(key.label);
    }
    out
}

fn ordered_result<'a>(
    a: &'a FrozenOrderedSet<Key>,
    b: &'a FrozenOrderedSet<Key>,
    op: Op,
) -> Vec<&'a Key> {
    match op {
        Op::Union => a.union(b).collect(),
        Op::Intersection => a.intersection(b).collect(),
        Op::Difference => a.difference(b).collect(),
        Op::Xor => a.symmetric_difference(b).collect(),
    }
}

fn hash_result<'a>(a: &'a FrozenHashSet<Key>, b: &'a FrozenHashSet<Key>, op: Op) -> Vec<&'a Key> {
    match op {
        Op::Union => a.union(b).collect(),
        Op::Intersection => a.intersection(b).collect(),
        Op::Difference => a.difference(b).collect(),
        Op::Xor => a.symmetric_difference(b).collect(),
    }
}

fn ordered_len(a: &FrozenOrderedSet<Key>, b: &FrozenOrderedSet<Key>, op: Op) -> usize {
    match op {
        Op::Union => a.union_len(b),
        Op::Intersection => a.intersection_len(b),
        Op::Difference => a.difference_len(b),
        Op::Xor => a.symmetric_difference_len(b),
    }
}

fn hash_len(a: &FrozenHashSet<Key>, b: &FrozenHashSet<Key>, op: Op) -> usize {
    match op {
        Op::Union => a.union_len(b),
        Op::Intersection => a.intersection_len(b),
        Op::Difference => a.difference_len(b),
        Op::Xor => a.symmetric_difference_len(b),
    }
}

/// `SET-BOOLEAN` at membership level over every subset of four keys.
#[test]
fn membership_laws_hold_for_every_subset_of_four_keys() {
    let all = 0..1 << KEYS;
    for a in all.clone() {
        assert_eq!(a | a, a);
        assert_eq!(a & a, a);
        assert_eq!(a ^ a, 0);
        let empty = apply(Op::Difference, a, a);
        assert_eq!(empty, 0);
        assert_eq!(a | empty, a);
        assert_eq!(a & empty, empty);
        for b in all.clone() {
            let shared = (a & b).count_ones();
            assert_eq!(a | b, b | a);
            assert_eq!(a & b, b & a);
            assert_eq!(a ^ b, b ^ a);
            assert_eq!(
                (a | b).count_ones(),
                a.count_ones() + (b.count_ones() - shared)
            );
            assert_eq!((a & !b).count_ones(), a.count_ones() - shared);
            assert_eq!(
                (a ^ b).count_ones(),
                (a.count_ones() - shared) + (b.count_ones() - shared)
            );
            assert_eq!(a | (a & b), a);
            assert_eq!(a & (a | b), a);
            for c in all.clone() {
                assert_eq!((a | b) | c, a | (b | c));
                assert_eq!((a & b) & c, a & (b & c));
                assert_eq!((a ^ b) ^ c, a ^ (b ^ c));
                assert_eq!(a & (b | c), (a & b) | (a & c));
                assert_eq!(a | (b & c), (a | b) & (a | c));
            }
        }
    }
    // Difference is neither commutative nor associative.
    assert_ne!(apply(Op::Difference, 1, 0), apply(Op::Difference, 0, 1));
    let d = |x, y| apply(Op::Difference, x, y);
    assert_ne!(d(d(1, 1), 1), d(1, d(1, 1)));
}

/// Both frozen set families against the model, for every pair of subsets of
/// four keys whose stored objects record their operand.
#[test]
fn frozen_set_families_match_the_model_on_every_pair() {
    for a_mask in 0..1 << KEYS {
        let a = labelled(a_mask, KEYS, 'a');
        let ordered_a: FrozenOrderedSet<Key> = keys(&a).into_iter().collect();
        let hash_a: FrozenHashSet<Key> = keys(&a).into_iter().collect();
        for b_mask in 0..1 << KEYS {
            let b = labelled(b_mask, KEYS, 'b');
            let ordered_b: FrozenOrderedSet<Key> = keys(&b).into_iter().collect();
            let hash_b: FrozenHashSet<Key> = keys(&b).into_iter().collect();
            for op in OPS {
                let expected = combine(frost_rule, &a, &b, op);
                let ordered = ordered_result(&ordered_a, &ordered_b, op);
                assert!(
                    ordered.windows(2).all(|w| w[0].id < w[1].id),
                    "{op:?} not ascending"
                );
                assert_eq!(
                    slots(KEYS, ordered),
                    expected,
                    "ordered {op:?} {a_mask:b} {b_mask:b}"
                );
                assert_eq!(
                    slots(KEYS, hash_result(&hash_a, &hash_b, op)),
                    expected,
                    "hash {op:?}"
                );
                let count = apply(op, a_mask, b_mask).count_ones() as usize;
                assert_eq!(ordered_len(&ordered_a, &ordered_b, op), count);
                assert_eq!(hash_len(&hash_a, &hash_b, op), count);
            }
            assert_eq!(ordered_a.is_subset(&ordered_b), is_subset(a_mask, b_mask));
            assert_eq!(hash_a.is_subset(&hash_b), is_subset(a_mask, b_mask));
            assert_eq!(ordered_a.is_disjoint(&ordered_b), a_mask & b_mask == 0);
        }
    }
}

/// `SET-REP`: nested expressions keep the left representative for union and
/// intersection, checked by materializing each intermediate frozen set.
#[test]
fn nested_union_and_intersection_keep_the_first_operand_representative() {
    const N: u32 = 3;
    let materialize =
        |found: Vec<&Key>| -> FrozenOrderedSet<Key> { found.into_iter().copied().collect() };
    for (a_mask, b_mask, c_mask) in triples(N) {
        let (a, b, c) = (
            labelled(a_mask, N, 'a'),
            labelled(b_mask, N, 'b'),
            labelled(c_mask, N, 'c'),
        );
        let (fa, fb, fc): (
            FrozenOrderedSet<Key>,
            FrozenOrderedSet<Key>,
            FrozenOrderedSet<Key>,
        ) = (
            keys(&a).into_iter().collect(),
            keys(&b).into_iter().collect(),
            keys(&c).into_iter().collect(),
        );
        for op in [Op::Union, Op::Intersection] {
            let model_left = combine(frost_rule, &combine(frost_rule, &a, &b, op), &c, op);
            let model_right = combine(frost_rule, &a, &combine(frost_rule, &b, &c, op), op);
            assert_eq!(
                model_left, model_right,
                "{op:?} associates on representatives"
            );
            let ab = materialize(ordered_result(&fa, &fb, op));
            let bc = materialize(ordered_result(&fb, &fc, op));
            assert_eq!(slots(N, ordered_result(&ab, &fc, op)), model_left);
            assert_eq!(slots(N, ordered_result(&fa, &bc, op)), model_right);
        }
    }
}

fn triples(n: u32) -> impl Iterator<Item = (u32, u32, u32)> {
    iproduct!(0..1 << n, 0..1 << n, 0..1 << n)
}

/// Laws that hold for membership but not for the selected object.
#[test]
fn representative_level_counterexamples_are_real() {
    let one = |label| labelled(1, 1, label);
    let (a, b, c, none) = (one('a'), one('b'), one('c'), labelled(0, 1, 'c'));
    let f = frost_rule;
    // XOR reassociation and cancellation.
    let left = combine(f, &combine(f, &a, &b, Op::Xor), &c, Op::Xor);
    let right = combine(f, &a, &combine(f, &b, &c, Op::Xor), Op::Xor);
    assert_eq!((left, right), (vec![Some('c')], vec![Some('a')]));
    let cancelled = combine(f, &combine(f, &a, &b, Op::Xor), &a, Op::Xor);
    assert_eq!(cancelled, vec![Some('a')]);
    assert_ne!(
        cancelled, b,
        "cancelling A to leave B selects a different object"
    );
    // Commutation.
    assert_ne!(combine(f, &a, &b, Op::Union), combine(f, &b, &a, Op::Union));
    // Absorption and distribution with the left operand second.
    assert_eq!(
        combine(f, &combine(f, &b, &a, Op::Intersection), &a, Op::Union),
        vec![Some('b')]
    );
    let lhs = combine(f, &combine(f, &b, &none, Op::Intersection), &a, Op::Union);
    let rhs = combine(
        f,
        &combine(f, &b, &a, Op::Union),
        &combine(f, &none, &a, Op::Union),
        Op::Intersection,
    );
    assert_eq!((lhs, rhs), (vec![Some('a')], vec![Some('b')]));
}

/// The left-first orientations of absorption and distribution do hold.
#[test]
fn left_first_absorption_and_distribution_hold_on_representatives() {
    const N: u32 = 3;
    let f = frost_rule;
    for (a_mask, b_mask, c_mask) in triples(N) {
        let (a, b, c) = (
            labelled(a_mask, N, 'a'),
            labelled(b_mask, N, 'b'),
            labelled(c_mask, N, 'c'),
        );
        assert_eq!(
            combine(f, &a, &combine(f, &a, &b, Op::Intersection), Op::Union),
            a
        );
        assert_eq!(
            combine(f, &a, &combine(f, &b, &c, Op::Union), Op::Intersection),
            combine(
                f,
                &combine(f, &a, &b, Op::Intersection),
                &combine(f, &a, &c, Op::Intersection),
                Op::Union
            )
        );
        assert_eq!(
            combine(f, &a, &combine(f, &b, &c, Op::Intersection), Op::Union),
            combine(
                f,
                &combine(f, &a, &b, Op::Union),
                &combine(f, &a, &c, Op::Union),
                Op::Intersection
            )
        );
        assert_eq!(mask(&combine(f, &a, &b, Op::Xor)), a_mask ^ b_mask);
    }
}

/// Associativity alone does not pin which side supplies the object: a
/// right-biased rule also associates. The pinned cases reject it.
#[test]
fn associativity_does_not_pin_the_representative_rule() {
    fn right_rule(left: Option<char>, right: Option<char>, op: Op) -> Option<char> {
        match op {
            Op::Union => right.or(left),
            Op::Intersection => right.filter(|_| left.is_some()),
            _ => frost_rule(left, right, op),
        }
    }
    let associates = |rule: Rule| {
        triples(3).all(|(x, y, z)| {
            let (a, b, c) = (
                labelled(x, 3, 'a'),
                labelled(y, 3, 'b'),
                labelled(z, 3, 'c'),
            );
            [Op::Union, Op::Intersection].iter().all(|&op| {
                combine(rule, &combine(rule, &a, &b, op), &c, op)
                    == combine(rule, &a, &combine(rule, &b, &c, op), op)
            })
        })
    };
    let pinned = |rule: Rule| {
        rule(Some('a'), Some('b'), Op::Union) == Some('a')
            && rule(None, Some('b'), Op::Union) == Some('b')
            && rule(Some('a'), Some('b'), Op::Intersection) == Some('a')
    };
    assert!(associates(frost_rule) && pinned(frost_rule));
    assert!(associates(right_rule) && !pinned(right_rule));
}

/// Members found by mask helpers match the slot model.
#[test]
fn bit_and_slot_encodings_agree() {
    for set in 0..1 << KEYS {
        let rep = labelled(set, KEYS, 'a');
        assert_eq!(mask(&rep), set);
        assert_eq!(
            members(set, KEYS).count(),
            usize::try_from(set.count_ones()).expect("small")
        );
    }
}

/// Union of sets with stored representatives, over 64 keys.
type Sampled = std::collections::BTreeMap<u8, char>;

fn sampled_union(
    rule: fn(Option<char>, Option<char>) -> Option<char>,
    a: &Sampled,
    b: &Sampled,
) -> Sampled {
    (0..64)
        .filter_map(|k| rule(a.get(&k).copied(), b.get(&k).copied()).map(|label| (k, label)))
        .collect()
}

/// `SET-REP` for union, checked by proptest over 64 keys with a fixed seed,
/// and whether the rule passes or fails with a shrunk counterexample.
fn sampled_union_law(
    rule: fn(Option<char>, Option<char>) -> Option<char>,
) -> Result<(), proptest::test_runner::TestError<(Sampled, Sampled, Sampled)>> {
    use proptest::prelude::*;
    use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
    let operand = |label: char| {
        prop::collection::btree_set(0_u8..64, 0..=64)
            .prop_map(move |keys| keys.into_iter().map(|k| (k, label)).collect::<Sampled>())
    };
    let config = Config {
        failure_persistence: None,
        ..Config::default()
    };
    // A fixed seed: the shrunk counterexample below must not depend on the run.
    let rng = TestRng::deterministic_rng(RngAlgorithm::ChaCha);
    let mut runner = TestRunner::new_with_rng(config, rng);
    runner.run(&(operand('a'), operand('b'), operand('c')), |(a, b, c)| {
        let grouped_left = sampled_union(rule, &sampled_union(rule, &a, &b), &c);
        let grouped_right = sampled_union(rule, &a, &sampled_union(rule, &b, &c));
        prop_assert_eq!(&grouped_left, &grouped_right);
        for (k, label) in &grouped_left {
            let first = [&a, &b, &c].into_iter().find_map(|s| s.get(k));
            prop_assert_eq!(Some(label), first);
        }
        Ok(())
    })
}

#[test]
fn sampled_union_keeps_the_first_representative_and_rejects_the_right_rule() {
    use proptest::test_runner::TestError;
    assert!(sampled_union_law(Option::or).is_ok());
    match sampled_union_law(|left, right| right.or(left)) {
        Err(TestError::Fail(_, (a, b, c))) => {
            assert!(
                a.len() + b.len() + c.len() <= 2,
                "not shrunk: {a:?} {b:?} {c:?}"
            );
        }
        other => panic!("the right-biased rule was not rejected: {other:?}"),
    }
}

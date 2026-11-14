//! `CLOSURE-STABLE` and `CTX-KEY` from
//! `docs/design/algebra/execution-and-computed-sets.md`.
//!
//! Frost implements no weighted closure and no result cache; these check the
//! proposed definitions on small weight algebras and a three-outcome toy
//! permission. The stability theorem itself is cited by the chapter, not proved
//! here: only its instances on the stated bounds are checked.

use std::collections::BTreeMap;

/// A commutative semiring of weights. `MinPlus` uses `u128::MAX` as infinity.
#[derive(Clone, Copy, Debug)]
enum Weights {
    Naturals,
    Booleans,
    MinPlus,
    /// Counts that saturate at the given cap.
    Capped(u128),
}

impl Weights {
    fn zero(self) -> u128 {
        match self {
            Self::MinPlus => u128::MAX,
            _ => 0,
        }
    }

    fn one(self) -> u128 {
        match self {
            Self::MinPlus => 0,
            _ => 1,
        }
    }

    fn plus(self, a: u128, b: u128) -> u128 {
        match self {
            Self::Naturals => a + b,
            Self::Booleans => a | b,
            Self::MinPlus => a.min(b),
            Self::Capped(cap) => cap.min(a + b),
        }
    }

    fn times(self, a: u128, b: u128) -> u128 {
        match self {
            Self::Naturals => a * b,
            Self::Booleans => a & b,
            Self::MinPlus => a.saturating_add(b),
            Self::Capped(cap) => cap.min(a * b),
        }
    }
}

/// `1 + u + u^2 + ... + u^p`, with the sum given separately so that a wrong
/// sum can be tried.
fn star(weights: Weights, plus: impl Fn(u128, u128) -> u128, u: u128, p: u32) -> u128 {
    let (mut total, mut power) = (weights.one(), weights.one());
    for _ in 0..p {
        power = weights.times(power, u);
        total = plus(total, power);
    }
    total
}

/// The least `p` up to 30 at which the partial sums stop changing.
fn stable_at(weights: Weights, plus: impl Fn(u128, u128) -> u128 + Copy, u: u128) -> Option<u32> {
    (0..=30).find(|&p| star(weights, plus, u, p) == star(weights, plus, u, p + 1))
}

/// `stable_at` with the algebra's own sum.
fn stable(weights: Weights, u: u128) -> Option<u32> {
    stable_at(weights, |a, b| weights.plus(a, b), u)
}

/// Whether no positive natural up to 4 is `p`-stable for any `p` up to 30
/// under the given sum.
fn positive_naturals_never_stabilize(plus: impl Fn(u128, u128) -> u128 + Copy) -> bool {
    (1..=4).all(|u| stable_at(Weights::Naturals, plus, u).is_none())
}

/// `CLOSURE-STABLE`: zero is stable in the naturals and no positive natural
/// is; the Booleans and min-plus are 0-stable; a count capped at `M` is
/// `p`-stable for some `p <= M`.
#[test]
fn stability_separates_the_weight_algebras() {
    let nat = Weights::Naturals;
    assert_eq!(stable(nat, 0), Some(0));
    assert!(positive_naturals_never_stabilize(|a, b| a + b));
    for u in 1..=4 {
        for p in 0..30 {
            assert!(star(nat, |a, b| a + b, u, p) < star(nat, |a, b| a + b, u, p + 1));
        }
    }
    // A sum that is really bitwise or makes the naturals look stable.
    assert!(!positive_naturals_never_stabilize(|a, b| a | b));

    for u in 0..=1 {
        assert_eq!(stable(Weights::Booleans, u), Some(0));
    }
    for u in 0..5 {
        assert_eq!(stable(Weights::MinPlus, u), Some(0));
    }
    for cap in 1..=5 {
        for u in 0..=cap {
            let at = stable(Weights::Capped(cap), u);
            assert!(at.is_some_and(|p| u128::from(p) <= cap), "cap {cap}, u {u}");
        }
    }
}

type Times = fn(u128, u128, u128) -> u128;

/// Whether capping at `M` commutes with the sum and with the given product,
/// for caps 1..=5 and operands 0..=20.
fn capping_commutes(times: Times) -> bool {
    (1..=5).all(|cap: u128| {
        (0..=20).all(|a: u128| {
            (0..=20).all(|b: u128| {
                cap.min(a + b) == cap.min(cap.min(a) + cap.min(b))
                    && cap.min(a * b) == times(cap, cap.min(a), cap.min(b))
            })
        })
    })
}

/// Capping is a semiring homomorphism from the naturals, which is why a
/// capped count may stand in for an exact one. A product that is not capped
/// again is rejected.
#[test]
fn capping_commutes_with_sum_and_product() {
    assert!(capping_commutes(|cap, a, b| cap.min(a * b)));
    assert!(!capping_commutes(|_, a, b| a * b));
}

const NODES: usize = 3;
type Matrix = [[u128; NODES]; NODES];

fn matrix(entry: impl Fn(usize, usize) -> u128) -> Matrix {
    core::array::from_fn(|i| core::array::from_fn(|j| entry(i, j)))
}

/// One round of `T := I + A.T`.
fn step(weights: Weights, edges: &Matrix, closure: &Matrix) -> Matrix {
    matrix(|i, j| {
        let identity = if i == j {
            weights.one()
        } else {
            weights.zero()
        };
        (0..NODES).fold(identity, |total, k| {
            weights.plus(total, weights.times(edges[i][k], closure[k][j]))
        })
    })
}

/// The closure after each of `rounds` naive rounds from zero.
fn iterate(weights: Weights, edges: &Matrix, rounds: usize) -> Vec<Matrix> {
    let mut closure = matrix(|_, _| weights.zero());
    (0..rounds)
        .map(|_| {
            closure = step(weights, edges, &closure);
            closure
        })
        .collect()
}

/// The first round that repeats the round before it.
fn fixed_after(seen: &[Matrix]) -> Option<usize> {
    (1..seen.len()).find(|&i| seen[i] == seen[i - 1])
}

fn total(closure: &Matrix) -> u128 {
    closure.iter().flatten().sum()
}

/// `CLOSURE-STABLE` on three-node graphs: path counts on a cycle never reach
/// a fixed point over the naturals, while Boolean reachability on the same
/// cycle, and path counts on an acyclic chain, are fixed within `n + 1` rounds.
#[test]
fn weighted_closure_diverges_on_a_cycle_over_the_naturals() {
    let unit = |weights: Weights, on: bool| if on { weights.one() } else { weights.zero() };
    let cycle = |weights| matrix(|i, j| unit(weights, j == (i + 1) % NODES));
    let chain = |weights| matrix(|i, j| unit(weights, j == i + 1));

    let nat_cycle = iterate(Weights::Naturals, &cycle(Weights::Naturals), 24);
    let bool_cycle = iterate(Weights::Booleans, &cycle(Weights::Booleans), 24);
    let nat_chain = iterate(Weights::Naturals, &chain(Weights::Naturals), 24);

    assert_eq!(fixed_after(&nat_cycle), None);
    assert!(fixed_after(&bool_cycle).is_some_and(|round| round <= NODES + 1));
    assert!(fixed_after(&nat_chain).is_some_and(|round| round <= NODES + 1));

    // The support of the diverging counts is the Boolean closure.
    let support = matrix(|i, j| u128::from(nat_cycle[23][i][j] > 0));
    assert_eq!(support, bool_cycle[23]);

    for round in 0..23 {
        assert!(total(&nat_cycle[round]) < total(&nat_cycle[round + 1]));
    }
    assert_eq!((total(&nat_cycle[22]), total(&nat_cycle[23])), (69, 72));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    Member,
    Conditional,
    NotMember,
}

/// A request: the subject and the caveat input, which may be missing.
type Request = (&'static str, Option<bool>);

/// One permission: an uncaveated path intersected with a caveated path.
fn evaluate((subject, caveat): Request) -> Answer {
    match (subject, caveat) {
        ("alice", Some(true)) => Answer::Member,
        ("alice", None) => Answer::Conditional,
        _ => Answer::NotMember,
    }
}

fn requests() -> Vec<Request> {
    ["alice", "bob"]
        .into_iter()
        .flat_map(|subject| [Some(true), Some(false), None].map(|caveat| (subject, caveat)))
        .collect()
}

/// Whether a memo keyed by `key_of` returns the direct answer for the second
/// request after any first request, over all ordered pairs.
fn memo_is_transparent<K: Ord>(key_of: fn(Request) -> K) -> bool {
    requests().into_iter().all(|first| {
        requests().into_iter().all(|second| {
            let mut store = BTreeMap::new();
            store.insert(key_of(first), evaluate(first));
            *store
                .entry(key_of(second))
                .or_insert_with(|| evaluate(second))
                == evaluate(second)
        })
    })
}

/// `CTX-KEY`: a key that holds the request context returns the direct answer;
/// a key that omits it returns a stale one.
#[test]
fn a_cache_key_must_include_the_request_context() {
    assert!(memo_is_transparent(|request| request));
    assert!(!memo_is_transparent(|(subject, _)| subject));

    let mut by_subject = BTreeMap::new();
    by_subject.insert("alice", evaluate(("alice", Some(true))));
    assert_eq!(by_subject["alice"], Answer::Member);
    assert_eq!(evaluate(("alice", None)), Answer::Conditional);
}

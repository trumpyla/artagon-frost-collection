//! `REL-COMP`, `BAGREL-COMP`, and `MM-SUPPORT` from `docs/design/algebra/multimaps.md`.
//!
//! Relations over two-by-two keys: a Boolean relation is a four-bit mask with
//! bit `2a + b` for the pair `(a, b)`; a weighted relation stores four counts.

use super::common::{subsets, tuples};
use std::collections::BTreeMap;

const N: usize = 2;
/// Number of pairs over `N` by `N` keys.
const PAIRS: u32 = 4;

/// Boolean composition, using every middle key or only the first.
fn compose_with(r: u32, s: u32, middles: usize) -> u32 {
    let mut out = 0;
    for a in 0..N {
        for c in 0..N {
            for b in 0..middles {
                if r & (1 << (a * N + b)) != 0 && s & (1 << (b * N + c)) != 0 {
                    out |= 1 << (a * N + c);
                }
            }
        }
    }
    out
}

fn compose(r: u32, s: u32) -> u32 {
    compose_with(r, s, N)
}

fn weighted(r: &[u32], s: &[u32]) -> Vec<u32> {
    let mut out = vec![0; N * N];
    for a in 0..N {
        for c in 0..N {
            out[a * N + c] = (0..N).map(|b| r[a * N + b] * s[b * N + c]).sum();
        }
    }
    out
}

fn weighted_max(r: &[u32], s: &[u32]) -> Vec<u32> {
    let mut out = vec![0; N * N];
    for a in 0..N {
        for c in 0..N {
            out[a * N + c] = (0..N)
                .map(|b| r[a * N + b] * s[b * N + c])
                .max()
                .unwrap_or(0);
        }
    }
    out
}

fn support(w: &[u32]) -> u32 {
    w.iter()
        .enumerate()
        .filter(|(_, n)| **n > 0)
        .map(|(i, _)| 1 << i)
        .sum()
}

/// `REL-COMP` over every Boolean relation on two-by-two keys.
#[test]
fn boolean_composition_associates_and_distributes_over_union() {
    assert_eq!(N * N, 4);
    let all = subsets(PAIRS);
    for r in all.clone() {
        for s in all.clone() {
            for t in all.clone() {
                assert_eq!(compose(compose(r, s), t), compose(r, compose(s, t)));
                assert_eq!(compose(r, s | t), compose(r, s) | compose(r, t));
                assert_eq!(compose(r | s, t), compose(r, t) | compose(s, t));
            }
        }
    }
}

/// `BAGREL-COMP` with weights 0..=2, cross-checked against Boolean composition
/// on supports. Weighted composition counts every derivation.
#[test]
fn weighted_composition_associates_and_projects_to_boolean_composition() {
    let all = tuples(3, N * N);
    for r in &all {
        for s in &all {
            assert_eq!(support(&weighted(r, s)), compose(support(r), support(s)));
        }
    }
    for r in all.iter().step_by(3) {
        for s in all.iter().step_by(2) {
            for t in all.iter().step_by(5) {
                assert_eq!(weighted(&weighted(r, s), t), weighted(r, &weighted(s, t)));
            }
        }
    }
    // Two middle witnesses for (0, 0): weighted keeps 2, Boolean keeps 1.
    let (r, s) = (vec![1, 1, 0, 0], vec![1, 0, 1, 0]);
    assert_eq!(weighted(&r, &s)[0], 2);
    assert_eq!(compose(support(&r), support(&s)) & 1, 1);
}

/// Wrong compositions: one that keeps only the first middle key fails the
/// cross-check; one that takes the maximum loses the witness count.
#[test]
fn wrong_compositions_are_rejected() {
    let all = tuples(3, N * N);
    let first_only_agrees = all.iter().all(|r| {
        all.iter()
            .all(|s| support(&weighted(r, s)) == compose_with(support(r), support(s), 1))
    });
    assert!(!first_only_agrees);
    let (r, s) = (vec![1, 1, 0, 0], vec![1, 0, 1, 0]);
    assert_ne!(weighted_max(&r, &s)[0], weighted(&r, &s)[0]);
}

/// `MM-SUPPORT`: the proposed family treats an empty fiber as absent; a native
/// map keeps the key until it is normalized. Grouping loses interleaving.
#[test]
fn empty_fibers_and_grouping_are_explicit_conversions() {
    let mut native: BTreeMap<char, Vec<u32>> = BTreeMap::new();
    native.insert('k', Vec::new());
    let normalized: BTreeMap<char, Vec<u32>> = native
        .clone()
        .into_iter()
        .filter(|(_, f)| !f.is_empty())
        .collect();
    assert!(native.contains_key(&'k') && !normalized.contains_key(&'k'));

    let entries = [('a', 1), ('b', 2), ('a', 3)];
    let mut grouped: BTreeMap<char, Vec<u32>> = BTreeMap::new();
    for (k, v) in entries {
        grouped.entry(k).or_default().push(v);
    }
    let flattened: Vec<(char, u32)> = grouped
        .iter()
        .flat_map(|(&k, vs)| vs.iter().map(move |&v| (k, v)))
        .collect();
    assert_eq!(flattened, [('a', 1), ('a', 3), ('b', 2)]);
    assert_ne!(flattened, entries);
}

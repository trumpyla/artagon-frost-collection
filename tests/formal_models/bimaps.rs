//! `BIMAP-CLOSED`, `BIMAP-IDENTITY`, and `BIMAP-PARTIAL` from
//! `docs/design/algebra/bimaps.md`, over every relation on three-by-three keys.

use super::common::subsets;

const N: u32 = 3;

fn pairs(r: u32) -> impl Iterator<Item = (u32, u32)> {
    (0..N * N)
        .filter(move |i| r & (1 << i) != 0)
        .map(|i| (i / N, i % N))
}

fn relation(pairs: impl IntoIterator<Item = (u32, u32)>) -> u32 {
    pairs.into_iter().map(|(a, b)| 1 << (a * N + b)).sum()
}

/// Both projections unique, or only the left one.
fn valid_with(r: u32, check_right: bool) -> bool {
    let all: Vec<_> = pairs(r).collect();
    let unique = |side: fn(&(u32, u32)) -> u32| {
        let mut seen: Vec<u32> = all.iter().map(side).collect();
        seen.sort_unstable();
        seen.windows(2).all(|w| w[0] != w[1])
    };
    unique(|p| p.0) && (!check_right || unique(|p| p.1))
}

fn valid(r: u32) -> bool {
    valid_with(r, true)
}

fn inverse(r: u32) -> u32 {
    relation(pairs(r).map(|(a, b)| (b, a)))
}

fn compose(r: u32, s: u32) -> u32 {
    relation(
        pairs(r).flat_map(|(a, b)| pairs(s).filter(move |p| p.0 == b).map(move |(_, c)| (a, c))),
    )
}

fn checked_union(r: Option<u32>, s: Option<u32>) -> Option<u32> {
    r.zip(s).map(|(r, s)| r | s).filter(|&u| valid(u))
}

fn bimaps() -> Vec<u32> {
    subsets(N * N).filter(|&r| valid(r)).collect()
}

/// Closure, inverses, and local identities over all 34 partial bijections.
#[test]
fn partial_bijections_are_closed_under_the_listed_operations() {
    let all = bimaps();
    assert_eq!(all.len(), 34);
    for &r in &all {
        assert_eq!(inverse(inverse(r)), r);
        let domain = relation(pairs(r).map(|(a, _)| (a, a)));
        let range = relation(pairs(r).map(|(_, b)| (b, b)));
        assert_eq!(compose(r, inverse(r)), domain);
        assert_eq!(compose(inverse(r), r), range);
        for keys in subsets(N) {
            assert!(valid(relation(
                pairs(r).filter(|(a, _)| keys & (1 << a) != 0)
            )));
            assert!(valid(relation(
                pairs(r).filter(|(_, b)| keys & (1 << b) != 0)
            )));
        }
        for &s in &all {
            assert!(valid(r & s) && valid(r & !s) && valid(compose(r, s)));
            assert_eq!(inverse(compose(r, s)), compose(inverse(s), inverse(r)));
            for &t in &all {
                assert_eq!(compose(compose(r, s), t), compose(r, compose(s, t)));
            }
        }
    }
}

/// `BIMAP-PARTIAL`: checked union succeeds in either grouping exactly when the
/// three-way union is a partial bijection.
#[test]
fn checked_union_succeeds_exactly_when_the_whole_union_is_valid() {
    let all = bimaps();
    for &r in &all {
        for &s in &all {
            for &t in &all {
                let expected = Some(r | s | t).filter(|&u| valid(u));
                assert_eq!(
                    checked_union(checked_union(Some(r), Some(s)), Some(t)),
                    expected
                );
                assert_eq!(
                    checked_union(Some(r), checked_union(Some(s), Some(t))),
                    expected
                );
            }
        }
    }
}

/// Wrong definitions fail: left-only validity admits too many relations, and
/// a union that never reports a conflict accepts an invalid result.
#[test]
fn wrong_bimap_definitions_are_rejected() {
    assert_ne!(subsets(N * N).filter(|&r| valid_with(r, false)).count(), 34);
    let never_conflicts = |r: u32, s: u32| r | s;
    let (r, s) = (relation([(0, 0)]), relation([(0, 1)]));
    assert!(!valid(never_conflicts(r, s)));
    assert_eq!(checked_union(Some(r), Some(s)), None);
}

/// Union is not closed, checked XOR is not associative even for definedness,
/// and pair difference is not difference by left key.
#[test]
fn bimap_counterexamples_are_real() {
    let left_conflict = relation([(0, 0), (0, 1)]);
    let right_conflict = relation([(0, 0), (1, 0)]);
    assert!(!valid(left_conflict) && !valid(right_conflict));

    let checked_xor =
        |r: Option<u32>, s: Option<u32>| r.zip(s).map(|(r, s)| r ^ s).filter(|&x| valid(x));
    let (r, s) = (Some(relation([(0, 0)])), Some(relation([(0, 1)])));
    assert_eq!(checked_xor(checked_xor(r, s), s), None);
    assert_eq!(checked_xor(r, checked_xor(s, s)), r);

    let (r, s) = (relation([(0, 0)]), relation([(0, 1)]));
    let by_left_key = relation(pairs(r).filter(|(a, _)| pairs(s).all(|(x, _)| x != *a)));
    assert_eq!((r & !s, by_left_key), (r, 0));
}

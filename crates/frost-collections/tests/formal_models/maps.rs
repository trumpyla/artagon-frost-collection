//! `MAP-LIFT` and the join laws from `docs/design/algebra/maps.md`.
//!
//! Frost implements no map merge or join yet, so these check the proposed
//! definitions only. Present-`None` is checked against `FrozenOrderedMap`.

use core::iter::repeat_n;
use exhaust::Exhaust;
use frost::family::FrozenOrderedMap;
use itertools::Itertools;

/// A map over a fixed set of keys: slot `k` is the value bound to `k`, or `None`.
type Map = Vec<Option<u32>>;

/// A collision resolver; `None` deletes the binding.
type Resolver = Box<dyn Fn(u32, u32) -> Option<u32>>;

/// The resolver lifted to absence: absence is the identity.
fn lift(resolver: &Resolver, left: Option<u32>, right: Option<u32>) -> Option<u32> {
    match (left, right) {
        (None, other) | (other, None) => other,
        (Some(value), Some(other)) => resolver(value, other),
    }
}

fn merge(r: &Resolver, a: &Map, b: &Map) -> Map {
    a.iter().zip(b).map(|(&x, &y)| lift(r, x, y)).collect()
}

/// Every map from `keys` keys to values `0..values`.
fn maps(keys: usize, values: u32) -> Vec<Map> {
    repeat_n((0..=values).map(|v| v.checked_sub(1)), keys)
        .multi_cartesian_product()
        .collect()
}

/// Every resolver from `{0, 1}²` to `{absent, 0, 1}`, as a table indexed by `2v + w`.
fn resolvers() -> Vec<Resolver> {
    <[Option<bool>; 4]>::exhaust()
        .map(|table| {
            Box::new(move |v: u32, w: u32| table[(v * 2 + w) as usize].map(u32::from)) as Resolver
        })
        .collect()
}

/// `MAP-LIFT`: pointwise merge is associative exactly when the lifted resolver
/// is associative on values and absence, checked for all 81 resolvers on two
/// values over every map on two keys.
#[test]
fn merge_associates_exactly_when_the_lifted_resolver_does() {
    let scalars = [None, Some(0), Some(1)];
    let all = maps(2, 2);
    let mut associative = 0;
    for r in resolvers() {
        let lifted = scalars.iter().all(|&x| {
            scalars.iter().all(|&y| {
                scalars
                    .iter()
                    .all(|&z| lift(&r, lift(&r, x, y), z) == lift(&r, x, lift(&r, y, z)))
            })
        });
        let merged = all.iter().all(|a| {
            all.iter().all(|b| {
                all.iter()
                    .all(|c| merge(&r, &merge(&r, a, b), c) == merge(&r, a, &merge(&r, b, c)))
            })
        });
        assert_eq!(lifted, merged);
        associative += usize::from(lifted);
    }
    assert!(associative > 0 && associative < 81);
}

/// A resolver that is associative on values but can delete is not
/// associative once absence takes part.
#[test]
fn deleting_and_subtracting_resolvers_do_not_reassociate() {
    let delete_equal: Resolver = Box::new(|v, w| (v != w).then_some(v));
    let (a, b) = (vec![Some(0)], vec![Some(1)]);
    assert_eq!(
        merge(&delete_equal, &merge(&delete_equal, &a, &a), &b),
        vec![Some(1)]
    );
    assert_eq!(
        merge(&delete_equal, &a, &merge(&delete_equal, &a, &b)),
        vec![None]
    );
    let subtract: Resolver = Box::new(u32::checked_sub);
    let (ten, three, two) = (vec![Some(10)], vec![Some(3)], vec![Some(2)]);
    assert_eq!(
        merge(&subtract, &merge(&subtract, &ten, &three), &two),
        vec![Some(5)]
    );
    assert_eq!(
        merge(&subtract, &ten, &merge(&subtract, &three, &two)),
        vec![Some(9)]
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Aligned {
    Left(u32),
    Right(u32),
    Both(u32, u32),
}

fn align(a: &Map, b: &Map) -> Vec<Option<Aligned>> {
    a.iter()
        .zip(b)
        .map(|pair| match pair {
            (Some(x), None) => Some(Aligned::Left(*x)),
            (None, Some(y)) => Some(Aligned::Right(*y)),
            (Some(x), Some(y)) => Some(Aligned::Both(*x, *y)),
            (None, None) => None,
        })
        .collect()
}

fn join<V: Copy, W: Copy>(a: &[Option<V>], b: &[Option<W>]) -> Vec<Option<(V, W)>> {
    a.iter().zip(b).map(|(x, y)| x.zip(*y)).collect()
}

fn domain<T>(map: &[Option<T>]) -> u32 {
    map.iter()
        .enumerate()
        .filter(|(_, v)| v.is_some())
        .map(|(k, _)| 1 << k)
        .sum()
}

/// Joins: the inner join covers common keys and reassociates only up to the
/// product reshaping; alignment covers every key once with its tag.
#[test]
fn joins_and_alignment_have_the_stated_domains_and_shapes() {
    let all = maps(2, 3);
    for a in &all {
        for b in &all {
            assert_eq!(domain(&join(a, b)), domain(a) & domain(b));
            assert_eq!(domain(&align(a, b)), domain(a) | domain(b));
            let swapped: Vec<_> = join(b, a)
                .into_iter()
                .map(|p| p.map(|(y, x)| (x, y)))
                .collect();
            assert_eq!(join(a, b), swapped);
            for c in &all {
                let left: Vec<_> = join(&join(a, b), c)
                    .into_iter()
                    .map(|p| p.map(|((x, y), z)| (x, y, z)))
                    .collect();
                let right: Vec<_> = join(a, &join(b, c))
                    .into_iter()
                    .map(|p| p.map(|(x, (y, z))| (x, y, z)))
                    .collect();
                assert_eq!(left, right);
            }
        }
    }
    assert_eq!(
        align(&vec![Some(1), None, Some(3)], &vec![None, Some(2), Some(4)]),
        vec![
            Some(Aligned::Left(1)),
            Some(Aligned::Right(2)),
            Some(Aligned::Both(3, 4))
        ]
    );
}

/// A stored `None` is a present binding, not absence, in a real frozen map.
#[test]
fn stored_none_is_not_a_missing_binding() {
    let map: FrozenOrderedMap<u8, Option<u8>> = [(0, None)].into_iter().collect();
    assert_eq!(map.get(&0), Some(&None));
    assert_eq!(map.get(&1), None);
}

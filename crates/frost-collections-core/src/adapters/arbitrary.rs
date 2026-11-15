//! `arbitrary` 1.x (feature `arbitrary-1`): structured fuzzing input.
//!
//! Values are built through the public family contracts (so every invariant
//! holds by construction). Adaptive profiles also draw their policy threshold
//! and a "grown" flag from the input. Generated values exercise both
//! representations and growth-history states, including a spilled `InlineVec`
//! with `len <= N` and a promoted map with
//! `len <= threshold` (states reachable by growth then shrinkage). Frozen
//! profiles use their normal canonical construction paths.

use alloc::vec::Vec;
use core::hash::{BuildHasher, Hash};

use ::arbitrary::{Arbitrary, Result, Unstructured};

use crate::adaptive::{
    AdaptiveFrozenHashMap, AdaptiveHash, AdaptiveHashMap, AdaptiveOrdered, AdaptiveOrderedMap,
    InlineFrozenVec, InlineVec,
};
use crate::family::{
    FrozenHashMap, FrozenHashSet, FrozenOrderedMap, FrozenOrderedSet, FrozenVec, MutableHashMap,
    MutableHashSet, MutableOrderedMap, MutableOrderedSet, MutableVec,
};
use crate::{Eligible, Frozen};

/// Largest threshold drawn for adaptive policies (covers both sides of every
/// boundary for small inputs).
const MAX_DRAWN_THRESHOLD: usize = 16;

impl<'a, B: Eligible + Arbitrary<'a>> Arbitrary<'a> for Frozen<B> {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        B::arbitrary(u).map(Frozen::new_unchecked)
    }
    fn arbitrary_take_rest(u: Unstructured<'a>) -> Result<Self> {
        B::arbitrary_take_rest(u).map(Frozen::new_unchecked)
    }
    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        B::size_hint(depth)
    }
}

macro_rules! via_vec {
    ($([$($g:tt)*] $ty:ty : $item:ty where [$($b:tt)*] => |$v:ident| $build:expr),* $(,)?) => {$(
        impl<'a, $($g)*> Arbitrary<'a> for $ty
        where
            $item: Arbitrary<'a>,
            $($b)*
        {
            fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
                let $v = Vec::<$item>::arbitrary(u)?;
                Ok($build)
            }
            fn arbitrary_take_rest(u: Unstructured<'a>) -> Result<Self> {
                let $v = Vec::<$item>::arbitrary_take_rest(u)?;
                Ok($build)
            }
            fn size_hint(depth: usize) -> (usize, Option<usize>) {
                Vec::<$item>::size_hint(depth)
            }
        }
    )*};
}

via_vec! {
    [T] MutableVec<T>: T where [] => |v| MutableVec::from(v),
    [T] FrozenVec<T>: T where [] => |v| FrozenVec::from(v),

    [T, const N: usize] InlineFrozenVec<T, N>: T where [] => |v| InlineFrozenVec::from_vec(v),
    [T] FrozenOrderedSet<T>: T where [T: Ord] => |v| v.into_iter().collect(),
    [T] MutableOrderedSet<T>: T where [T: Ord] => |v| v.into_iter().collect(),
    [T, S] FrozenHashSet<T, S>: T where [T: Eq + Hash, S: BuildHasher + Default]
        => |v| v.into_iter().collect(),
    [T, S] MutableHashSet<T, S>: T where [T: Eq + Hash, S: BuildHasher + Default]
        => |v| v.into_iter().collect(),
    [K, V] MutableOrderedMap<K, V>: (K, V) where [K: Ord] => |v| v.into_iter().collect(),
    [K, V] FrozenOrderedMap<K, V>: (K, V) where [K: Ord] => |v| v.into_iter().collect(),
    [K, V, S] MutableHashMap<K, V, S>: (K, V) where [K: Eq + Hash, S: BuildHasher + Default]
        => |v| v.into_iter().collect(),
    [K, V, S] FrozenHashMap<K, V, S>: (K, V) where [K: Eq + Hash, S: BuildHasher + Default]
        => |v| v.into_iter().collect(),
}

impl<'a, T: Arbitrary<'a>, const N: usize> Arbitrary<'a> for InlineVec<T, N> {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let grown = bool::arbitrary(u)?;
        let v = Vec::<T>::arbitrary(u)?;
        Ok(if grown {
            InlineVec::from_heap(v)
        } else {
            InlineVec::from_vec(v)
        })
    }
    fn arbitrary_take_rest(mut u: Unstructured<'a>) -> Result<Self> {
        let grown = bool::arbitrary(&mut u)?;
        let v = Vec::<T>::arbitrary_take_rest(u)?;
        Ok(if grown {
            InlineVec::from_heap(v)
        } else {
            InlineVec::from_vec(v)
        })
    }
    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        ::arbitrary::size_hint::and(
            <bool as Arbitrary<'a>>::size_hint(depth),
            Vec::<T>::size_hint(depth),
        )
    }
}

macro_rules! adaptive {
    ($([$($g:tt)*] $ty:ty where [$($b:tt)*] => |$t:ident, $v:ident| $build:expr),* $(,)?) => {$(
        impl<'a, $($g)*> Arbitrary<'a> for $ty
        where
            K: Arbitrary<'a>,
            V: Arbitrary<'a>,
            $($b)*
        {
            fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
                let $t = u.int_in_range(0..=MAX_DRAWN_THRESHOLD)?;
                let grown = bool::arbitrary(u)?;
                let $v = Vec::<(K, V)>::arbitrary(u)?;
                Ok($build(grown))
            }
            fn arbitrary_take_rest(mut u: Unstructured<'a>) -> Result<Self> {
                let $t = u.int_in_range(0..=MAX_DRAWN_THRESHOLD)?;
                let grown = bool::arbitrary(&mut u)?;
                let $v = Vec::<(K, V)>::arbitrary_take_rest(u)?;
                Ok($build(grown))
            }
            fn size_hint(depth: usize) -> (usize, Option<usize>) {
                ::arbitrary::size_hint::and(
                    <u8 as Arbitrary<'a>>::size_hint(depth),
                    Vec::<(K, V)>::size_hint(depth),
                )
            }
        }
    )*};
}

adaptive! {
    [K, V, S] AdaptiveHashMap<K, V, S> where [K: Eq + Hash, S: BuildHasher + Clone + Default]
        => |t, v| |grown: bool| {
            // "Grown": built as a table, then given the drawn policy, as after
            // growth past the threshold followed by removals.
            let start = if grown { 0 } else { t };
            let mut m = AdaptiveHashMap::with_hasher_and_policy(S::default(), AdaptiveHash::linear_up_to(start));
            m.extend(v);
            m.policy = AdaptiveHash::linear_up_to(t);
            m
        },
    [K, V, S] AdaptiveFrozenHashMap<K, V, S> where [K: Eq + Hash, S: BuildHasher + Clone + Default]
        => |t, v| |_grown: bool| {
            let entries: MutableHashMap<K, V, S> = v.into_iter().collect();
            AdaptiveFrozenHashMap::from_map(entries.into(), AdaptiveHash::linear_up_to(t))
        },
    [K, V] AdaptiveOrderedMap<K, V> where [K: Ord]
        => |t, v| |grown: bool| {
            let start = if grown { 0 } else { t };
            let mut m = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(start));
            m.extend(v);
            m.policy = AdaptiveOrdered::sorted_up_to(t);
            m
        },
}

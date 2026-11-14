//! `rayon` 1.x (feature `rayon-1`).
//!
//! * Borrowed and owned parallel iteration for every family and profile.
//!   Mutable iteration is provided for mutable sequences and map values;
//!   frozen elements and set members remain immutable.
//!   Noncontiguous iterators use crate-owned newtypes.
//! * `FromParallelIterator` stages the input and then applies the family
//!   builder: first key representative, last value. Ordered parallel input
//!   preserves that order. Unordered input, including `par_bridge`, has no
//!   defined first or last duplicate; its winning representative/value must
//!   not be used as a deterministic result.
//! * Contiguous families yield rayon's slice iterators, which are indexed.
//!   [`FrozenOrderedMap`] yields an *indexed* entry iterator over its sorted
//!   slice; tree and hash iterators are unindexed.
//! * Native Rayon tree/hash traversal stages entries or references in a Vec.
//!   Slice traversal does not need that staging allocation. Parallel collection
//!   also stages input before sequential duplicate reduction or insertion.
//!
//! ```
//! use frost::prelude::*;
//! use rayon::prelude::*;
//! let m: FrozenOrderedMap<u32, u32> = (0..1000u32).into_par_iter().map(|i| (i % 10, i)).collect();
//! assert_eq!(m.len(), 10);
//! assert_eq!(m.get(&3), Some(&993));                 // last value wins, as sequentially
//! let total: u64 = m.par_iter().map(|(_, v)| u64::from(*v)).sum();
//! assert_eq!(total, (990..1000).sum::<u64>());
//! ```

use alloc::vec::Vec;
use core::fmt;
use core::hash::{BuildHasher, Hash};

use ::rayon::iter::plumbing::{Consumer, ProducerCallback, UnindexedConsumer};
use ::rayon::iter::{
    Either, FromParallelIterator, IndexedParallelIterator, IntoParallelIterator, Map,
    ParallelExtend, ParallelIterator,
};
use ::rayon::slice::ParallelSliceMut;

use crate::adaptive::{
    AdaptiveFrozenHashMap, AdaptiveHashMap, AdaptiveOrdered, AdaptiveOrderedMap, InlineFrozenVec,
    InlineVec,
};
use crate::family::{
    FrozenHashMap, FrozenHashSet, FrozenOrderedMap, FrozenOrderedSet, FrozenVec, MutableHashMap,
    MutableHashSet, MutableOrderedMap, MutableOrderedSet, MutableVec,
};
use crate::{Eligible, Frozen};

macro_rules! par_newtype {
    (@common [$($b:tt)*] $(#[$m:meta])* $name:ident <$($p:tt),*> ($inner:ty) -> $item:ty) => {
        $(#[$m])*
        pub struct $name<$($p),*>($inner);

        // Bounds are on the wrapper's own parameters only (no backend or rayon
        // iterator type in the where-clause), mirroring rayon's own impls.
        impl<$($p),*> ParallelIterator for $name<$($p),*>
        where
            $($b)*
        {
            type Item = $item;
            fn drive_unindexed<C>(self, consumer: C) -> C::Result
            where
                C: UnindexedConsumer<Self::Item>,
            {
                self.0.drive_unindexed(consumer)
            }
            fn opt_len(&self) -> Option<usize> {
                self.0.opt_len()
            }
        }

        impl<$($p),*> fmt::Debug for $name<$($p),*> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }
    };
    (unindexed [$($b:tt)*] $(#[$m:meta])* $name:ident <$($p:tt),*> ($inner:ty) -> $item:ty) => {
        par_newtype!(@common [$($b)*] $(#[$m])* $name<$($p),*>($inner) -> $item);
    };
    (indexed [$($b:tt)*] $(#[$m:meta])* $name:ident <$($p:tt),*> ($inner:ty) -> $item:ty) => {
        par_newtype!(@common [$($b)*] $(#[$m])* $name<$($p),*>($inner) -> $item);

        impl<$($p),*> IndexedParallelIterator for $name<$($p),*>
        where
            $($b)*
        {
            fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
                self.0.drive(consumer)
            }
            fn len(&self) -> usize {
                self.0.len()
            }
            fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
                self.0.with_producer(callback)
            }
        }
    };
}

type PairFn<'a, K, V> = fn(&'a (K, V)) -> (&'a K, &'a V);
type PairMutFn<'a, K, V> = fn(&'a mut (K, V)) -> (&'a K, &'a mut V);

fn pair<K, V>((k, v): &(K, V)) -> (&K, &V) {
    (k, v)
}
fn pair_mut<K, V>((k, v): &mut (K, V)) -> (&K, &mut V) {
    (&*k, v)
}

par_newtype! { indexed [T: Send,]
    /// Owning parallel iterator of the sequence families and profiles.
    SeqIntoParIter<T>(::rayon::vec::IntoIter<T>) -> T
}
par_newtype! { indexed [K: Sync + 'a, V: Sync + 'a,]
    /// Borrowed parallel entries of [`FrozenOrderedMap`] (indexed).
    SortedParIter<'a, K, V>(Map<::rayon::slice::Iter<'a, (K, V)>, PairFn<'a, K, V>>) -> (&'a K, &'a V)
}
par_newtype! { unindexed [K: Sync + 'a, V: Sync + 'a,]
    /// Borrowed parallel entries of the hash families.
    HashParIter<'a, K, V>(::rayon::collections::hash_map::Iter<'a, K, V>) -> (&'a K, &'a V)
}
par_newtype! { unindexed [K: Sync + 'a, V: Send + 'a,]
    /// Mutable parallel entries of [`MutableHashMap`].
    HashParIterMut<'a, K, V>(::rayon::collections::hash_map::IterMut<'a, K, V>) -> (&'a K, &'a mut V)
}
par_newtype! { unindexed [K: Send, V: Send,]
    /// Owning parallel entries of the hash families.
    HashIntoParIter<K, V>(::rayon::collections::hash_map::IntoIter<K, V>) -> (K, V)
}
par_newtype! { unindexed [K: Sync + 'a, V: Sync + 'a,]
    /// Borrowed parallel entries of [`MutableOrderedMap`].
    TreeParIter<'a, K, V>(::rayon::collections::btree_map::Iter<'a, K, V>) -> (&'a K, &'a V)
}
par_newtype! { unindexed [K: Sync + 'a, V: Send + 'a,]
    /// Mutable parallel entries of [`MutableOrderedMap`].
    TreeParIterMut<'a, K, V>(::rayon::collections::btree_map::IterMut<'a, K, V>) -> (&'a K, &'a mut V)
}
par_newtype! { unindexed [K: Send, V: Send,]
    /// Owning parallel entries of [`MutableOrderedMap`].
    TreeIntoParIter<K, V>(::rayon::collections::btree_map::IntoIter<K, V>) -> (K, V)
}
par_newtype! { unindexed [K: Sync + 'a, V: Sync + 'a,]
    /// Borrowed parallel entries of the adaptive hash profiles.
    AdaptiveHashParIter<'a, K, V>(Either<
        Map<::rayon::slice::Iter<'a, (K, V)>, PairFn<'a, K, V>>,
        ::rayon::collections::hash_map::Iter<'a, K, V>,
    >) -> (&'a K, &'a V)
}
par_newtype! { unindexed [K: Sync + Send + 'a, V: Send + 'a,]
    /// Mutable parallel entries of [`AdaptiveHashMap`].
    AdaptiveHashParIterMut<'a, K, V>(Either<
        Map<::rayon::slice::IterMut<'a, (K, V)>, PairMutFn<'a, K, V>>,
        ::rayon::collections::hash_map::IterMut<'a, K, V>,
    >) -> (&'a K, &'a mut V)
}

par_newtype! { unindexed [T: Sync + 'a,]
    /// Borrowed parallel members of the hash-set families.
    HashSetParIter<'a, T>(::rayon::collections::hash_set::Iter<'a, T>) -> &'a T
}
par_newtype! { unindexed [T: Send,]
    /// Owning parallel members of the hash-set families.
    HashSetIntoParIter<T>(::rayon::collections::hash_set::IntoIter<T>) -> T
}
par_newtype! { unindexed [T: Sync + 'a,]
    /// Borrowed parallel members of [`MutableOrderedSet`].
    TreeSetParIter<'a, T>(::rayon::collections::btree_set::Iter<'a, T>) -> &'a T
}
par_newtype! { unindexed [T: Send,]
    /// Owning parallel members of [`MutableOrderedSet`].
    TreeSetIntoParIter<T>(::rayon::collections::btree_set::IntoIter<T>) -> T
}
par_newtype! { unindexed [K: Send, V: Send,]
    /// Owning parallel entries of the adaptive hash profiles.
    AdaptiveHashIntoParIter<K, V>(Either<
        ::rayon::vec::IntoIter<(K, V)>,
        ::rayon::collections::hash_map::IntoIter<K, V>,
    >) -> (K, V)
}
par_newtype! { unindexed [K: Sync + 'a, V: Sync + 'a,]
    /// Borrowed parallel entries of [`AdaptiveOrderedMap`].
    AdaptiveOrderedParIter<'a, K, V>(Either<
        Map<::rayon::slice::Iter<'a, (K, V)>, PairFn<'a, K, V>>,
        ::rayon::collections::btree_map::Iter<'a, K, V>,
    >) -> (&'a K, &'a V)
}

par_newtype! { unindexed [K: Sync + Send + 'a, V: Send + 'a,]
    /// Mutable parallel entries of [`AdaptiveOrderedMap`].
    AdaptiveOrderedParIterMut<'a, K, V>(Either<
        Map<::rayon::slice::IterMut<'a, (K, V)>, PairMutFn<'a, K, V>>,
        ::rayon::collections::btree_map::IterMut<'a, K, V>,
    >) -> (&'a K, &'a mut V)
}

// ---------------------------------------------------------------------------
// Exact frozen wrapper (read-only and owned only; never `&mut`).
// ---------------------------------------------------------------------------

impl<'a, B> IntoParallelIterator for &'a Frozen<B>
where
    &'a B: IntoParallelIterator,
{
    type Iter = <&'a B as IntoParallelIterator>::Iter;
    type Item = <&'a B as IntoParallelIterator>::Item;
    fn into_par_iter(self) -> Self::Iter {
        self.backend().into_par_iter()
    }
}
impl<B: IntoParallelIterator> IntoParallelIterator for Frozen<B> {
    type Iter = B::Iter;
    type Item = B::Item;
    fn into_par_iter(self) -> Self::Iter {
        self.into_inner().into_par_iter()
    }
}
/// Backend semantics (exact-wrapper rule); eligibility-gated.
impl<B, T> FromParallelIterator<T> for Frozen<B>
where
    B: Eligible + FromParallelIterator<T>,
    T: Send,
{
    fn from_par_iter<I: IntoParallelIterator<Item = T>>(iter: I) -> Self {
        Frozen::new_unchecked(B::from_par_iter(iter))
    }
}

// ---------------------------------------------------------------------------
// Sequences
// ---------------------------------------------------------------------------

macro_rules! seq_par {
    ($([$($g:tt)*] $ty:ty => into_vec: |$s:ident| $into_vec:expr),* $(,)?) => {$(
        impl<'a, $($g)*> IntoParallelIterator for &'a $ty
        where
            T: Sync + 'a,
        {
            type Iter = ::rayon::slice::Iter<'a, T>;
            type Item = &'a T;
            fn into_par_iter(self) -> Self::Iter {
                self.as_slice().into_par_iter()
            }
        }
        impl<$($g)*> IntoParallelIterator for $ty
        where
            T: Send,
        {
            type Iter = SeqIntoParIter<T>;
            type Item = T;
            fn into_par_iter(self) -> SeqIntoParIter<T> {
                let $s = self;
                SeqIntoParIter(($into_vec).into_par_iter())
            }
        }
        impl<$($g)*> FromParallelIterator<T> for $ty
        where
            T: Send,
        {
            fn from_par_iter<I: IntoParallelIterator<Item = T>>(iter: I) -> Self {
                Vec::from_par_iter(iter).into()
            }
        }
    )*};
}

seq_par! {
    [T] MutableVec<T> => into_vec: |s| Vec::from(s),
    [T] FrozenVec<T> => into_vec: |s| s.into_vec(),
    [T, const N: usize] InlineVec<T, N> => into_vec: |s| s.into_vec(),
}

impl<'a, T: Sync + 'a, const N: usize> IntoParallelIterator for &'a InlineFrozenVec<T, N> {
    type Iter = ::rayon::slice::Iter<'a, T>;
    type Item = &'a T;
    fn into_par_iter(self) -> Self::Iter {
        self.as_slice().into_par_iter()
    }
}
impl<T: Send, const N: usize> IntoParallelIterator for InlineFrozenVec<T, N> {
    type Iter = SeqIntoParIter<T>;
    type Item = T;
    fn into_par_iter(self) -> SeqIntoParIter<T> {
        SeqIntoParIter(self.into_vec().into_par_iter())
    }
}
impl<T: Send, const N: usize> FromParallelIterator<T> for InlineFrozenVec<T, N> {
    fn from_par_iter<I: IntoParallelIterator<Item = T>>(iter: I) -> Self {
        InlineFrozenVec::from_vec(Vec::from_par_iter(iter))
    }
}

impl<'a, T: Send + 'a> IntoParallelIterator for &'a mut MutableVec<T> {
    type Iter = ::rayon::slice::IterMut<'a, T>;
    type Item = &'a mut T;
    fn into_par_iter(self) -> Self::Iter {
        self.as_mut_slice().into_par_iter()
    }
}
impl<'a, T: Send + 'a, const N: usize> IntoParallelIterator for &'a mut InlineVec<T, N> {
    type Iter = ::rayon::slice::IterMut<'a, T>;
    type Item = &'a mut T;
    fn into_par_iter(self) -> Self::Iter {
        self.as_mut_slice().into_par_iter()
    }
}
impl<T: Send> ParallelExtend<T> for MutableVec<T> {
    fn par_extend<I: IntoParallelIterator<Item = T>>(&mut self, iter: I) {
        self.v.par_extend(iter);
    }
}
impl<T: Send, const N: usize> ParallelExtend<T> for InlineVec<T, N> {
    fn par_extend<I: IntoParallelIterator<Item = T>>(&mut self, iter: I) {
        self.extend(Vec::from_par_iter(iter));
    }
}

// ---------------------------------------------------------------------------
// Hash families
// ---------------------------------------------------------------------------

impl<'a, K: Sync, V: Sync, S> IntoParallelIterator for &'a MutableHashMap<K, V, S> {
    type Iter = HashParIter<'a, K, V>;
    type Item = (&'a K, &'a V);
    fn into_par_iter(self) -> Self::Iter {
        HashParIter((&self.m).into_par_iter())
    }
}
impl<'a, K: Sync, V: Send, S> IntoParallelIterator for &'a mut MutableHashMap<K, V, S> {
    type Iter = HashParIterMut<'a, K, V>;
    type Item = (&'a K, &'a mut V);
    fn into_par_iter(self) -> Self::Iter {
        HashParIterMut((&mut self.m).into_par_iter())
    }
}
impl<K: Send, V: Send, S> IntoParallelIterator for MutableHashMap<K, V, S> {
    type Iter = HashIntoParIter<K, V>;
    type Item = (K, V);
    fn into_par_iter(self) -> Self::Iter {
        HashIntoParIter(self.m.into_par_iter())
    }
}
/// Stage parallel input before applying the family's insertion contract.
/// The hasher is created on the collecting thread and need not be Send.
impl<K: Hash + Eq + Send, V: Send, S: BuildHasher + Default> FromParallelIterator<(K, V)>
    for MutableHashMap<K, V, S>
{
    fn from_par_iter<I: IntoParallelIterator<Item = (K, V)>>(iter: I) -> Self {
        Vec::from_par_iter(iter).into_iter().collect()
    }
}
impl<K: Hash + Eq + Send, V: Send, S: BuildHasher> ParallelExtend<(K, V)>
    for MutableHashMap<K, V, S>
{
    fn par_extend<I: IntoParallelIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.extend(Vec::from_par_iter(iter));
    }
}

impl<'a, K: Sync, V: Sync, S> IntoParallelIterator for &'a FrozenHashMap<K, V, S> {
    type Iter = HashParIter<'a, K, V>;
    type Item = (&'a K, &'a V);
    fn into_par_iter(self) -> Self::Iter {
        HashParIter((&self.map).into_par_iter())
    }
}
impl<K: Send, V: Send, S> IntoParallelIterator for FrozenHashMap<K, V, S> {
    type Iter = HashIntoParIter<K, V>;
    type Item = (K, V);
    fn into_par_iter(self) -> Self::Iter {
        HashIntoParIter(self.map.into_par_iter())
    }
}
impl<K: Hash + Eq + Send, V: Send, S: BuildHasher + Default> FromParallelIterator<(K, V)>
    for FrozenHashMap<K, V, S>
{
    fn from_par_iter<I: IntoParallelIterator<Item = (K, V)>>(iter: I) -> Self {
        MutableHashMap::from_par_iter(iter).freeze()
    }
}

// --- adaptive hash ---------------------------------------------------------

use crate::adaptive::hash_map::{FrozenRepr as FrozenHashRepr, MutableRepr as MutHashRepr};

impl<'a, K: Sync, V: Sync, S> IntoParallelIterator for &'a AdaptiveFrozenHashMap<K, V, S> {
    type Iter = AdaptiveHashParIter<'a, K, V>;
    type Item = (&'a K, &'a V);
    fn into_par_iter(self) -> Self::Iter {
        AdaptiveHashParIter(match &self.repr {
            FrozenHashRepr::Linear { entries, .. } => {
                Either::Left(entries.into_par_iter().map(pair as PairFn<'a, K, V>))
            }
            FrozenHashRepr::Table(m) => Either::Right(m.into_par_iter()),
        })
    }
}
impl<K: Send, V: Send, S> IntoParallelIterator for AdaptiveFrozenHashMap<K, V, S> {
    type Iter = AdaptiveHashIntoParIter<K, V>;
    type Item = (K, V);
    fn into_par_iter(self) -> Self::Iter {
        AdaptiveHashIntoParIter(match self.repr {
            FrozenHashRepr::Linear { entries, .. } => {
                Either::Left(entries.into_vec().into_par_iter())
            }
            FrozenHashRepr::Table(m) => Either::Right(m.into_par_iter()),
        })
    }
}
impl<'a, K: Sync, V: Sync, S> IntoParallelIterator for &'a AdaptiveHashMap<K, V, S> {
    type Iter = AdaptiveHashParIter<'a, K, V>;
    type Item = (&'a K, &'a V);
    fn into_par_iter(self) -> Self::Iter {
        AdaptiveHashParIter(match &self.repr {
            MutHashRepr::Linear { entries, .. } => {
                Either::Left(entries.into_par_iter().map(pair as PairFn<'a, K, V>))
            }
            MutHashRepr::Table(m) => Either::Right(m.into_par_iter()),
        })
    }
}
impl<'a, K: Sync + Send, V: Send, S> IntoParallelIterator for &'a mut AdaptiveHashMap<K, V, S> {
    type Iter = AdaptiveHashParIterMut<'a, K, V>;
    type Item = (&'a K, &'a mut V);
    fn into_par_iter(self) -> Self::Iter {
        AdaptiveHashParIterMut(match &mut self.repr {
            MutHashRepr::Linear { entries, .. } => {
                Either::Left(entries.into_par_iter().map(pair_mut as PairMutFn<'a, K, V>))
            }
            MutHashRepr::Table(m) => Either::Right(m.into_par_iter()),
        })
    }
}
impl<K: Send, V: Send, S> IntoParallelIterator for AdaptiveHashMap<K, V, S> {
    type Iter = AdaptiveHashIntoParIter<K, V>;
    type Item = (K, V);
    fn into_par_iter(self) -> Self::Iter {
        AdaptiveHashIntoParIter(match self.repr {
            MutHashRepr::Linear { entries, .. } => Either::Left(entries.into_par_iter()),
            MutHashRepr::Table(m) => Either::Right(m.into_par_iter()),
        })
    }
}
impl<K: Hash + Eq + Send, V: Send, S: BuildHasher + Clone + Default> FromParallelIterator<(K, V)>
    for AdaptiveFrozenHashMap<K, V, S>
{
    fn from_par_iter<I: IntoParallelIterator<Item = (K, V)>>(iter: I) -> Self {
        Vec::from_par_iter(iter).into_iter().collect()
    }
}
impl<K: Hash + Eq + Send, V: Send, S: BuildHasher + Clone + Default> FromParallelIterator<(K, V)>
    for AdaptiveHashMap<K, V, S>
{
    fn from_par_iter<I: IntoParallelIterator<Item = (K, V)>>(iter: I) -> Self {
        Vec::from_par_iter(iter).into_iter().collect()
    }
}
impl<K: Hash + Eq + Send, V: Send, S: BuildHasher + Clone> ParallelExtend<(K, V)>
    for AdaptiveHashMap<K, V, S>
{
    fn par_extend<I: IntoParallelIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.extend(Vec::from_par_iter(iter));
    }
}

// ---------------------------------------------------------------------------
// Ordered families
// ---------------------------------------------------------------------------

/// Stable parallel sort followed by one in-place duplicate reduction.
/// This preserves the staged first key representative and last value.
fn sorted_entries<K: Ord + Send, V: Send, I: IntoParallelIterator<Item = (K, V)>>(
    iter: I,
) -> FrozenOrderedMap<K, V> {
    let mut v = Vec::from_par_iter(iter);
    v.par_sort_by(|a, b| a.0.cmp(&b.0));
    v.dedup_by(|later, first| {
        if later.0.cmp(&first.0).is_eq() {
            core::mem::swap(&mut first.1, &mut later.1);
            true
        } else {
            false
        }
    });
    FrozenOrderedMap::from_sorted_unique_unchecked(v)
}

impl<'a, K: Sync + 'a, V: Sync + 'a> IntoParallelIterator for &'a FrozenOrderedMap<K, V> {
    type Iter = SortedParIter<'a, K, V>;
    type Item = (&'a K, &'a V);
    fn into_par_iter(self) -> Self::Iter {
        SortedParIter(self.entries.into_par_iter().map(pair as PairFn<'a, K, V>))
    }
}
impl<K: Send, V: Send> IntoParallelIterator for FrozenOrderedMap<K, V> {
    type Iter = SeqIntoParIter<(K, V)>;
    type Item = (K, V);
    fn into_par_iter(self) -> Self::Iter {
        SeqIntoParIter(self.into_sorted_vec().into_par_iter())
    }
}
impl<K: Ord + Send, V: Send> FromParallelIterator<(K, V)> for FrozenOrderedMap<K, V> {
    fn from_par_iter<I: IntoParallelIterator<Item = (K, V)>>(iter: I) -> Self {
        sorted_entries(iter)
    }
}

impl<'a, K: Sync + 'a, V: Sync + 'a> IntoParallelIterator for &'a MutableOrderedMap<K, V> {
    type Iter = TreeParIter<'a, K, V>;
    type Item = (&'a K, &'a V);
    fn into_par_iter(self) -> Self::Iter {
        TreeParIter((&self.m).into_par_iter())
    }
}
impl<'a, K: Sync + 'a, V: Send + 'a> IntoParallelIterator for &'a mut MutableOrderedMap<K, V> {
    type Iter = TreeParIterMut<'a, K, V>;
    type Item = (&'a K, &'a mut V);
    fn into_par_iter(self) -> Self::Iter {
        TreeParIterMut((&mut self.m).into_par_iter())
    }
}
impl<K: Send, V: Send> IntoParallelIterator for MutableOrderedMap<K, V> {
    type Iter = TreeIntoParIter<K, V>;
    type Item = (K, V);
    fn into_par_iter(self) -> Self::Iter {
        TreeIntoParIter(self.m.into_par_iter())
    }
}
impl<K: Ord + Send, V: Send> FromParallelIterator<(K, V)> for MutableOrderedMap<K, V> {
    fn from_par_iter<I: IntoParallelIterator<Item = (K, V)>>(iter: I) -> Self {
        sorted_entries(iter).thaw()
    }
}
impl<K: Ord + Send, V: Send> ParallelExtend<(K, V)> for MutableOrderedMap<K, V> {
    fn par_extend<I: IntoParallelIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.extend(Vec::from_par_iter(iter));
    }
}

use crate::adaptive::ordered_map::Repr as OrdRepr;

impl<'a, K: Sync + 'a, V: Sync + 'a> IntoParallelIterator for &'a AdaptiveOrderedMap<K, V> {
    type Iter = AdaptiveOrderedParIter<'a, K, V>;
    type Item = (&'a K, &'a V);
    fn into_par_iter(self) -> Self::Iter {
        AdaptiveOrderedParIter(match &self.repr {
            OrdRepr::Sorted(v) => Either::Left(v.into_par_iter().map(pair as PairFn<'a, K, V>)),
            OrdRepr::Tree(t) => Either::Right(t.into_par_iter()),
        })
    }
}
impl<'a, K: Sync + Send + 'a, V: Send + 'a> IntoParallelIterator
    for &'a mut AdaptiveOrderedMap<K, V>
{
    type Iter = AdaptiveOrderedParIterMut<'a, K, V>;
    type Item = (&'a K, &'a mut V);
    fn into_par_iter(self) -> Self::Iter {
        AdaptiveOrderedParIterMut(match &mut self.repr {
            OrdRepr::Sorted(v) => {
                Either::Left(v.into_par_iter().map(pair_mut as PairMutFn<'a, K, V>))
            }
            OrdRepr::Tree(t) => Either::Right(t.into_par_iter()),
        })
    }
}
impl<K: Send, V: Send> IntoParallelIterator for AdaptiveOrderedMap<K, V> {
    type Iter = SeqIntoParIter<(K, V)>;
    type Item = (K, V);
    fn into_par_iter(self) -> Self::Iter {
        let entries = match self.repr {
            OrdRepr::Sorted(entries) => entries,
            OrdRepr::Tree(tree) => tree.into_iter().collect(),
        };
        SeqIntoParIter(entries.into_par_iter())
    }
}
impl<K: Ord + Send, V: Send> FromParallelIterator<(K, V)> for AdaptiveOrderedMap<K, V> {
    fn from_par_iter<I: IntoParallelIterator<Item = (K, V)>>(iter: I) -> Self {
        AdaptiveOrderedMap::from_frozen(sorted_entries(iter), AdaptiveOrdered::default())
    }
}
impl<K: Ord + Send, V: Send> ParallelExtend<(K, V)> for AdaptiveOrderedMap<K, V> {
    fn par_extend<I: IntoParallelIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.extend(Vec::from_par_iter(iter));
    }
}

impl<'a, T: Sync + 'a> IntoParallelIterator for &'a FrozenOrderedSet<T> {
    type Iter = ::rayon::slice::Iter<'a, T>;
    type Item = &'a T;
    fn into_par_iter(self) -> Self::Iter {
        self.as_slice().into_par_iter()
    }
}
impl<T: Send> IntoParallelIterator for FrozenOrderedSet<T> {
    type Iter = SeqIntoParIter<T>;
    type Item = T;
    fn into_par_iter(self) -> Self::Iter {
        SeqIntoParIter(self.items.into_vec().into_par_iter())
    }
}
impl<T: Ord + Send> FromParallelIterator<T> for FrozenOrderedSet<T> {
    fn from_par_iter<I: IntoParallelIterator<Item = T>>(iter: I) -> Self {
        let mut v = Vec::from_par_iter(iter);
        v.par_sort();
        v.dedup_by(|later, first| Ord::cmp(&*later, &*first).is_eq());
        FrozenOrderedSet {
            items: v.into_boxed_slice(),
        }
    }
}

impl<'a, T: Sync> IntoParallelIterator for &'a MutableOrderedSet<T> {
    type Iter = TreeSetParIter<'a, T>;
    type Item = &'a T;

    fn into_par_iter(self) -> Self::Iter {
        TreeSetParIter((&self.inner).into_par_iter())
    }
}

impl<T: Send> IntoParallelIterator for MutableOrderedSet<T> {
    type Iter = TreeSetIntoParIter<T>;
    type Item = T;

    fn into_par_iter(self) -> Self::Iter {
        TreeSetIntoParIter(self.inner.into_par_iter())
    }
}

impl<T: Ord + Send> FromParallelIterator<T> for MutableOrderedSet<T> {
    fn from_par_iter<I: IntoParallelIterator<Item = T>>(iter: I) -> Self {
        FrozenOrderedSet::from_par_iter(iter).thaw()
    }
}

impl<T: Ord + Send> ParallelExtend<T> for MutableOrderedSet<T> {
    fn par_extend<I: IntoParallelIterator<Item = T>>(&mut self, iter: I) {
        self.extend(Vec::from_par_iter(iter));
    }
}

macro_rules! hash_set_par {
    ($($ty:ident => $field:ident),* $(,)?) => {$(
        impl<'a, T: Sync, S> IntoParallelIterator for &'a $ty<T, S> {
            type Iter = HashSetParIter<'a, T>;
            type Item = &'a T;

            fn into_par_iter(self) -> Self::Iter {
                HashSetParIter((&self.$field).into_par_iter())
            }
        }

        impl<T: Send, S> IntoParallelIterator for $ty<T, S> {
            type Iter = HashSetIntoParIter<T>;
            type Item = T;

            fn into_par_iter(self) -> Self::Iter {
                HashSetIntoParIter(self.$field.into_par_iter())
            }
        }

        impl<T: Eq + Hash + Send, S: BuildHasher + Default> FromParallelIterator<T> for $ty<T, S> {
            fn from_par_iter<I: IntoParallelIterator<Item = T>>(iter: I) -> Self {
                Vec::from_par_iter(iter).into_iter().collect()
            }
        }
    )*};
}

hash_set_par!(MutableHashSet => inner, FrozenHashSet => set);

impl<T: Eq + Hash + Send, S: BuildHasher> ParallelExtend<T> for MutableHashSet<T, S> {
    fn par_extend<I: IntoParallelIterator<Item = T>>(&mut self, iter: I) {
        self.extend(Vec::from_par_iter(iter));
    }
}

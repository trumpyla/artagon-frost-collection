use alloc::boxed::Box;
use core::borrow::Borrow;
use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::iter::FusedIterator;
use std::collections::{HashMap, hash_map};
use std::hash::RandomState;

use super::AdaptiveHashMap;
use crate::iter::{Keys, Values};
use crate::read::{Entries, Len, LookupRef, MapTypes};
use crate::{FreezeWith, Frozen};

/// Plan: freeze a hash map into [`AdaptiveFrozenHashMap`].
///
/// `linear_max` is the largest entry count stored as a linear table. The
/// default (8) is a placeholder, **not** a measured crossover.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct AdaptiveHash {
    /// Largest length stored linearly.
    pub linear_max: usize,
}

impl AdaptiveHash {
    /// Policy with an explicit threshold.
    pub const fn linear_up_to(linear_max: usize) -> Self {
        Self { linear_max }
    }
}

impl Default for AdaptiveHash {
    fn default() -> Self {
        Self::linear_up_to(8)
    }
}

/// Which representation was selected (diagnostics only; not part of equality).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HashRepr {
    /// Boxed `(K, V)` entries, scanned with `Eq`.
    Linear,
    /// A shrunk `HashMap<K, V, S>`.
    Table,
}

pub(crate) enum Repr<K, V, S> {
    // The hasher is kept so `thaw` and `hasher()` preserve hashing policy
    // regardless of the selected variant.
    Linear { entries: Box<[(K, V)]>, hasher: S },
    Table(HashMap<K, V, S>),
}

/// Frozen hash map that stores small maps as a linear table.
///
/// Contract — identical for both representations:
/// * Lookup: `K: Borrow<Q> + Eq + Hash`, `Q: Hash + Eq`, `S: BuildHasher`.
///   The linear variant uses only `Eq`; by `Borrow`'s contract that agrees
///   with hash-based lookup.
/// * Traversal order: unspecified (and may differ between variants).
/// * Equality: mapping equality across variants.
/// * Hashing policy: `S` is preserved through freeze and [`thaw`](Self::thaw).
///
/// ```
/// use frost::prelude::*;
/// use std::collections::HashMap;
///
/// let small: HashMap<&str, u32> = HashMap::from([("a", 1), ("b", 2)]);
/// let f = small.freeze_with(AdaptiveHash::default());
/// assert_eq!(f.get("b"), Some(&2));
/// assert_eq!(f.representation(), frost::adaptive::HashRepr::Linear);
///
/// // Batched queries match on the representation once.
/// let mut hits = 0;
/// f.for_each_lookup(["a", "z", "b"].iter().copied(), |_, v| hits += v.is_some() as u32);
/// assert_eq!(hits, 2);
/// ```
pub struct AdaptiveFrozenHashMap<K, V, S = RandomState> {
    pub(crate) repr: Repr<K, V, S>,
}

impl<K, V, S> AdaptiveFrozenHashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Clone,
{
    /// Select a representation for `map` under `policy`.
    ///
    /// Cost: linear variant — one allocation of exactly `len` entries and a
    /// move of each entry; table variant — `shrink_to_fit` (may rehash).
    pub fn from_map(mut map: HashMap<K, V, S>, policy: AdaptiveHash) -> Self {
        let repr = if map.len() <= policy.linear_max {
            let hasher = map.hasher().clone();
            Repr::Linear {
                entries: map.into_iter().collect(),
                hasher,
            }
        } else {
            map.shrink_to_fit();
            Repr::Table(map)
        };
        Self { repr }
    }
}

impl<K, V, S> AdaptiveFrozenHashMap<K, V, S> {
    /// Back to the paired mutable profile [`AdaptiveHashMap`] with the
    /// **default** policy. O(1) for both variants (no rebuild).
    ///
    /// The frozen profile does not store the policy it was frozen with; use
    /// [`thaw_with`](Self::thaw_with) to restore a non-default one.
    #[must_use]
    pub fn thaw(self) -> AdaptiveHashMap<K, V, S> {
        self.thaw_with(AdaptiveHash::default())
    }

    /// [`thaw`](Self::thaw) with an explicit growth policy.
    #[must_use]
    pub fn thaw_with(self, policy: AdaptiveHash) -> AdaptiveHashMap<K, V, S> {
        AdaptiveHashMap::from_frozen_repr(self.repr, policy)
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> AdaptiveFrozenHashMap<K, V, S> {
    /// Interop: a std `HashMap` with the same hasher. Linear variant: O(n)
    /// rebuild. Table variant: O(1).
    #[must_use]
    pub fn into_hash_map(self) -> HashMap<K, V, S> {
        match self.repr {
            Repr::Linear { entries, hasher } => {
                let mut m = HashMap::with_capacity_and_hasher(entries.len(), hasher);
                m.extend(entries.into_vec());
                m
            }
            Repr::Table(m) => m,
        }
    }
}

impl<K, V, S> AdaptiveFrozenHashMap<K, V, S> {
    /// Selected representation.
    pub fn representation(&self) -> HashRepr {
        match self.repr {
            Repr::Linear { .. } => HashRepr::Linear,
            Repr::Table(_) => HashRepr::Table,
        }
    }

    /// Entry count.
    pub fn len(&self) -> usize {
        match &self.repr {
            Repr::Linear { entries, .. } => entries.len(),
            Repr::Table(m) => m.len(),
        }
    }

    /// `true` when empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The hasher (kept for both variants).
    pub fn hasher(&self) -> &S {
        match &self.repr {
            Repr::Linear { hasher, .. } => hasher,
            Repr::Table(m) => m.hasher(),
        }
    }

    /// Keys in unspecified order.
    pub fn keys(&self) -> Keys<Iter<'_, K, V>> {
        Keys(self.iter())
    }

    /// Values in unspecified order.
    pub fn values(&self) -> Values<Iter<'_, K, V>> {
        Values(self.iter())
    }

    /// Entries in unspecified order.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter(match &self.repr {
            Repr::Linear { entries, .. } => IterRepr::Linear(entries.iter()),
            Repr::Table(m) => IterRepr::Table(m.iter()),
        })
    }
}

impl<K, V, S> AdaptiveFrozenHashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// Stored key representative and value.
    #[inline]
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match &self.repr {
            Repr::Linear { entries, .. } => linear_find(entries, key),
            Repr::Table(m) => m.get_key_value(key),
        }
    }

    /// Value for `key`.
    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_key_value(key).map(|(_, v)| v)
    }

    /// `true` if present.
    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_key_value(key).is_some()
    }

    /// Batched lookup: matches on the representation **once**, then runs a
    /// monomorphic loop over the selected backend.
    pub fn for_each_lookup<'q, Q, I, F>(&self, queries: I, mut f: F)
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized + 'q,
        I: IntoIterator<Item = &'q Q>,
        F: FnMut(&'q Q, Option<&V>),
    {
        match &self.repr {
            Repr::Linear { entries, .. } => {
                for q in queries {
                    f(q, linear_find(entries, q).map(|(_, v)| v));
                }
            }
            Repr::Table(m) => {
                for q in queries {
                    f(q, m.get(q));
                }
            }
        }
    }
}

#[inline]
fn linear_find<'a, K, V, Q>(entries: &'a [(K, V)], key: &Q) -> Option<(&'a K, &'a V)>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    entries
        .iter()
        .find(|(k, _)| k.borrow() == key)
        .map(|(k, v)| (k, v))
}

impl<K, V, S> FreezeWith<AdaptiveHash> for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Clone,
{
    type Output = AdaptiveFrozenHashMap<K, V, S>;
    fn freeze_with(self, plan: AdaptiveHash) -> Self::Output {
        AdaptiveFrozenHashMap::from_map(self, plan)
    }
}

impl<K, V, S> FreezeWith<AdaptiveHash> for Frozen<HashMap<K, V, S>>
where
    K: Eq + Hash,
    S: BuildHasher + Clone,
{
    type Output = AdaptiveFrozenHashMap<K, V, S>;
    fn freeze_with(self, plan: AdaptiveHash) -> Self::Output {
        AdaptiveFrozenHashMap::from_map(self.into_inner(), plan)
    }
}

/// Builds with the default policy and `S::default()`. First key
/// representative, last value.
impl<K, V, S> FromIterator<(K, V)> for AdaptiveFrozenHashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Clone + Default,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        crate::family::MutableHashMap::<K, V, S>::from_iter(iter)
            .freeze_with(AdaptiveHash::default())
    }
}

/// Empty linear table; no allocation.
impl<K, V, S: Default> Default for AdaptiveFrozenHashMap<K, V, S> {
    fn default() -> Self {
        Self {
            repr: Repr::Linear {
                entries: Box::new([]),
                hasher: S::default(),
            },
        }
    }
}

impl<K: Clone, V: Clone, S: Clone> Clone for AdaptiveFrozenHashMap<K, V, S> {
    fn clone(&self) -> Self {
        Self {
            repr: match &self.repr {
                Repr::Linear { entries, hasher } => Repr::Linear {
                    entries: entries.clone(),
                    hasher: hasher.clone(),
                },
                Repr::Table(m) => Repr::Table(m.clone()),
            },
        }
    }
}

impl<K, V, S, Q> core::ops::Index<&Q> for AdaptiveFrozenHashMap<K, V, S>
where
    K: Borrow<Q> + Eq + Hash,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.get(key)
            .expect("key not present in AdaptiveFrozenHashMap")
    }
}

/// Mapping equality, independent of which representation either side holds.
impl<K, V, S> PartialEq for AdaptiveFrozenHashMap<K, V, S>
where
    K: Eq + Hash,
    V: PartialEq,
    S: BuildHasher,
{
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .all(|(k, v)| other.get(k).is_some_and(|ov| v == ov))
    }
}

impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for AdaptiveFrozenHashMap<K, V, S> {}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for AdaptiveFrozenHashMap<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<'a, K, V, S> IntoIterator for &'a AdaptiveFrozenHashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Entry iterator for [`AdaptiveFrozenHashMap`].
///
/// `next` branches on the variant; `fold` (and so `for_each`, `sum`, `count`,
/// …) matches once and delegates to the concrete iterator's `fold`.
pub struct Iter<'a, K, V>(pub(super) IterRepr<'a, K, V>);

pub(super) enum IterRepr<'a, K, V> {
    Linear(core::slice::Iter<'a, (K, V)>),
    Table(hash_map::Iter<'a, K, V>),
}

// Manual: `derive(Clone)` would demand `K: Clone, V: Clone`.
impl<K, V> Clone for Iter<'_, K, V> {
    fn clone(&self) -> Self {
        Self(match &self.0 {
            IterRepr::Linear(it) => IterRepr::Linear(it.clone()),
            IterRepr::Table(it) => IterRepr::Table(it.clone()),
        })
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Iter<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(Iter::clone(self)).finish()
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            IterRepr::Linear(it) => it.next().map(|(k, v)| (k, v)),
            IterRepr::Table(it) => it.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            IterRepr::Linear(it) => it.size_hint(),
            IterRepr::Table(it) => it.size_hint(),
        }
    }

    #[inline]
    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        match self.0 {
            IterRepr::Linear(it) => it.fold(init, |acc, (k, v)| f(acc, (k, v))),
            IterRepr::Table(it) => it.fold(init, f),
        }
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {}
impl<K, V> FusedIterator for Iter<'_, K, V> {}

impl<K, V, S> Len for AdaptiveFrozenHashMap<K, V, S> {
    fn len(&self) -> usize {
        Self::len(self)
    }
}

impl<K, V, S> MapTypes for AdaptiveFrozenHashMap<K, V, S> {
    type Key = K;
    type Value = V;
}

impl<K, V, S, Q> LookupRef<Q> for AdaptiveFrozenHashMap<K, V, S>
where
    K: Borrow<Q> + Eq + Hash,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> {
        Self::get_key_value(self, query)
    }
}

impl<K, V, S> Entries for AdaptiveFrozenHashMap<K, V, S> {
    type Iter<'a>
        = Iter<'a, K, V>
    where
        Self: 'a;
    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

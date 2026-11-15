//! `AdaptiveHashMap`: mutable *hash-map* profile — a linear table that is
//! promoted to a hash table when it outgrows the policy threshold.

use alloc::vec::{self, Vec};
use core::borrow::Borrow;
use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::iter::FusedIterator;
use std::collections::{HashMap, hash_map};
use std::hash::RandomState;

use super::frozen::{
    AdaptiveFrozenHashMap, AdaptiveHash, HashRepr, Iter, IterRepr, Repr as FrozenRepr,
};
use crate::FreezeWith;
use crate::iter::{Keys, Values};
use crate::read::{Entries, Len, LookupRef, MapTypes};
use crate::write::{MapInsert, MapRemove};

pub(crate) enum Repr<K, V, S> {
    Linear { entries: Vec<(K, V)>, hasher: S },
    Table(HashMap<K, V, S>),
}

/// Mutable hash map that stays a linear `(K, V)` table while small.
///
/// * Same contract as [`AdaptiveFrozenHashMap`] and the hash families:
///   `K: Borrow<Q> + Eq + Hash`, `Q: Hash + Eq`; `insert` keeps the stored key;
///   unspecified order; mapping equality; `S` preserved in both variants.
/// * Promotion happens when inserting a **new** key into a full linear table
///   (`len == policy.linear_max`). It is one-way during mutation (no
///   oscillation); [`shrink_to_fit`](Self::shrink_to_fit) demotes explicitly.
/// * No full `Entry` API: [`get_or_insert_with`](Self::get_or_insert_with)
///   covers the common case without a cross-variant entry type.
///
/// Promotion rebuilds a table and can invoke user hashing/equality code. If
/// that code panics, the map remains valid, but entries moved into the unfinished
/// replacement can be dropped; promotion does not promise transactional rollback.
/// Hasher cloning is required for representation-changing mutation, not lookup,
/// removal, traversal, or thawing a frozen profile.
///
/// ```
/// # extern crate frost_collections_core as frost;
/// use frost::prelude::*;
/// use frost::adaptive::HashRepr;
/// let mut m: AdaptiveHashMap<u32, u32> = AdaptiveHashMap::with_policy(AdaptiveHash::linear_up_to(2));
/// m.insert(1, 10);
/// m.insert(2, 20);
/// assert_eq!(m.representation(), HashRepr::Linear);
/// m.insert(3, 30);                                   // promotes
/// assert_eq!(m.representation(), HashRepr::Table);
/// *m.get_or_insert_with(4, || 0) += 1;
/// let f = m.freeze();                                // re-selects under the policy
/// assert_eq!(f.get(&4), Some(&1));
/// ```
pub struct AdaptiveHashMap<K, V, S = RandomState> {
    pub(crate) repr: Repr<K, V, S>,
    pub(crate) policy: AdaptiveHash,
}

impl<K, V> AdaptiveHashMap<K, V, RandomState> {
    /// Empty, default policy, std's randomized hasher.
    pub fn new() -> Self {
        Self::with_hasher_and_policy(RandomState::new(), AdaptiveHash::default())
    }
    /// Empty with `policy`.
    pub fn with_policy(policy: AdaptiveHash) -> Self {
        Self::with_hasher_and_policy(RandomState::new(), policy)
    }
}

impl<K, V, S: Default> Default for AdaptiveHashMap<K, V, S> {
    fn default() -> Self {
        Self::with_hasher_and_policy(S::default(), AdaptiveHash::default())
    }
}

impl<K, V, S> AdaptiveHashMap<K, V, S> {
    /// Empty with `hasher` and `policy`; starts linear, allocates nothing.
    pub fn with_hasher_and_policy(hasher: S, policy: AdaptiveHash) -> Self {
        Self {
            repr: Repr::Linear {
                entries: Vec::new(),
                hasher,
            },
            policy,
        }
    }

    pub(super) fn from_frozen_repr(repr: FrozenRepr<K, V, S>, policy: AdaptiveHash) -> Self {
        let repr = match repr {
            FrozenRepr::Linear { entries, hasher } => Repr::Linear {
                entries: entries.into_vec(),
                hasher,
            },
            FrozenRepr::Table(m) => Repr::Table(m),
        };
        Self { repr, policy }
    }

    /// Growth policy.
    pub fn policy(&self) -> AdaptiveHash {
        self.policy
    }
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
    /// The hasher.
    pub fn hasher(&self) -> &S {
        match &self.repr {
            Repr::Linear { hasher, .. } => hasher,
            Repr::Table(m) => m.hasher(),
        }
    }
    /// Remove all entries; keeps the current representation.
    pub fn clear(&mut self) {
        match &mut self.repr {
            Repr::Linear { entries, .. } => entries.clear(),
            Repr::Table(m) => m.clear(),
        }
    }
    /// Entries, unspecified order.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter(match &self.repr {
            Repr::Linear { entries, .. } => IterRepr::Linear(entries.iter()),
            Repr::Table(m) => IterRepr::Table(m.iter()),
        })
    }
    /// Entries with mutable values.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut(match &mut self.repr {
            Repr::Linear { entries, .. } => IterMutRepr::Linear(entries.iter_mut()),
            Repr::Table(m) => IterMutRepr::Table(m.iter_mut()),
        })
    }
    /// Keys.
    pub fn keys(&self) -> Keys<Iter<'_, K, V>> {
        Keys(self.iter())
    }
    /// Values.
    pub fn values(&self) -> Values<Iter<'_, K, V>> {
        Values(self.iter())
    }
    /// Mutable values.
    pub fn values_mut(&mut self) -> Values<IterMut<'_, K, V>> {
        Values(self.iter_mut())
    }
    /// Keep entries for which `f` returns `true`.
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut f: F) {
        match &mut self.repr {
            Repr::Linear { entries, .. } => entries.retain_mut(|(k, v)| f(k, v)),
            Repr::Table(m) => m.retain(f),
        }
    }
}

fn linear_pos<K: Borrow<Q>, V, Q: Eq + ?Sized>(entries: &[(K, V)], q: &Q) -> Option<usize> {
    entries.iter().position(|(k, _)| k.borrow() == q)
}

impl<K: Eq + Hash, V, S: BuildHasher + Clone> AdaptiveHashMap<K, V, S> {
    /// Linear → table. `S: Clone` lets the hasher be copied into the table
    /// without a placeholder value.
    fn promote(&mut self) -> &mut HashMap<K, V, S> {
        if let Repr::Linear { entries, hasher } = &mut self.repr {
            let mut m =
                HashMap::with_capacity_and_hasher(entries.len().saturating_mul(2), hasher.clone());
            m.extend(core::mem::take(entries));
            self.repr = Repr::Table(m);
        }
        match &mut self.repr {
            Repr::Table(m) => m,
            Repr::Linear { .. } => unreachable!("promoted above"),
        }
    }

    /// Ensure room for `additional` more entries; promotes if the linear
    /// table could not hold them under the policy.
    pub fn reserve(&mut self, additional: usize) {
        match &mut self.repr {
            Repr::Linear { entries, .. } => {
                if entries.len().saturating_add(additional) > self.policy.linear_max {
                    self.promote().reserve(additional);
                } else {
                    entries.reserve(additional);
                }
            }
            Repr::Table(m) => m.reserve(additional),
        }
    }

    /// Insert; an existing key keeps its stored representative.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let max = self.policy.linear_max;
        match &mut self.repr {
            Repr::Linear { entries, .. } => {
                if let Some(i) = linear_pos(entries, &key) {
                    return Some(core::mem::replace(&mut entries[i].1, value));
                }
                if entries.len() < max {
                    entries.push((key, value));
                    return None;
                }
                self.promote().insert(key, value)
            }
            Repr::Table(m) => m.insert(key, value),
        }
    }

    /// Value for `key`, inserting `f()` if absent (may promote).
    pub fn get_or_insert_with<F: FnOnce() -> V>(&mut self, key: K, f: F) -> &mut V {
        // Keep the lookup result across the promotion decision. The linear
        // entries cannot change unless we promote, in which case the table's
        // entry API performs the required hash lookup instead.
        let position = match &self.repr {
            Repr::Linear { entries, .. } => linear_pos(entries, &key),
            Repr::Table(_) => None,
        };
        if let Repr::Linear { entries, .. } = &self.repr
            && position.is_none()
            && entries.len() >= self.policy.linear_max
        {
            self.promote();
        }
        match &mut self.repr {
            Repr::Linear { entries, .. } => {
                let i = if let Some(i) = position {
                    i
                } else {
                    entries.push((key, f()));
                    entries.len() - 1
                };
                &mut entries[i].1
            }
            Repr::Table(m) => m.entry(key).or_insert_with(f),
        }
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> AdaptiveHashMap<K, V, S> {
    /// Stored key and value.
    pub fn get_key_value<Q: Hash + Eq + ?Sized>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
    {
        match &self.repr {
            Repr::Linear { entries, .. } => linear_pos(entries, key).map(|i| {
                let (k, v) = &entries[i];
                (k, v)
            }),
            Repr::Table(m) => m.get_key_value(key),
        }
    }
    /// Value for `key`.
    pub fn get<Q: Hash + Eq + ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.get_key_value(key).map(|(_, v)| v)
    }
    /// Mutable value for `key`.
    pub fn get_mut<Q: Hash + Eq + ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        match &mut self.repr {
            Repr::Linear { entries, .. } => linear_pos(entries, key).map(|i| &mut entries[i].1),
            Repr::Table(m) => m.get_mut(key),
        }
    }
    /// `true` if present.
    pub fn contains_key<Q: Hash + Eq + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.get_key_value(key).is_some()
    }
    /// Remove, returning the stored key and value. Linear: `swap_remove`
    /// (order is unspecified anyway).
    pub fn remove_entry<Q: Hash + Eq + ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
    {
        match &mut self.repr {
            Repr::Linear { entries, .. } => {
                linear_pos(entries, key).map(|i| entries.swap_remove(i))
            }
            Repr::Table(m) => m.remove_entry(key),
        }
    }
    /// Remove, returning the value.
    pub fn remove<Q: Hash + Eq + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        self.remove_entry(key).map(|(_, v)| v)
    }
}

impl<K: Eq + Hash, V, S: BuildHasher + Clone> AdaptiveHashMap<K, V, S> {
    /// Release excess capacity; a table with `len <= linear_max` is demoted to
    /// a linear table (the only demotion path).
    pub fn shrink_to_fit(&mut self) {
        match &mut self.repr {
            Repr::Linear { entries, .. } => entries.shrink_to_fit(),
            Repr::Table(m) if m.len() <= self.policy.linear_max => {
                let hasher = m.hasher().clone();
                let entries: Vec<(K, V)> = m.drain().collect();
                self.repr = Repr::Linear { entries, hasher };
            }
            Repr::Table(m) => m.shrink_to_fit(),
        }
    }

    /// Freeze into [`AdaptiveFrozenHashMap`], re-selecting the representation
    /// under this map's policy.
    #[must_use]
    pub fn freeze(self) -> AdaptiveFrozenHashMap<K, V, S> {
        let policy = self.policy;
        self.freeze_with(policy)
    }
}

/// Re-select under `plan`, independent of the mutable map's own policy.
impl<K: Eq + Hash, V, S: BuildHasher + Clone> FreezeWith<AdaptiveHash>
    for AdaptiveHashMap<K, V, S>
{
    type Output = AdaptiveFrozenHashMap<K, V, S>;
    fn freeze_with(self, plan: AdaptiveHash) -> Self::Output {
        let repr = match self.repr {
            Repr::Linear { entries, hasher } if entries.len() <= plan.linear_max => {
                FrozenRepr::Linear {
                    entries: entries.into_boxed_slice(),
                    hasher,
                }
            }
            Repr::Linear { entries, hasher } => {
                let mut m = HashMap::with_capacity_and_hasher(entries.len(), hasher);
                m.extend(entries);
                return AdaptiveFrozenHashMap::from_map(m, plan);
            }
            Repr::Table(m) => return AdaptiveFrozenHashMap::from_map(m, plan),
        };
        AdaptiveFrozenHashMap { repr }
    }
}

/// `HashMap::insert` semantics for each entry, default policy.
impl<K: Eq + Hash, V, S: BuildHasher + Clone + Default> FromIterator<(K, V)>
    for AdaptiveHashMap<K, V, S>
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut m = Self::default();
        m.extend(iter);
        m
    }
}
impl<K: Eq + Hash, V, S: BuildHasher + Clone> Extend<(K, V)> for AdaptiveHashMap<K, V, S> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        let mut iter = iter.into_iter();
        let started_empty = self.is_empty();
        // The linear form promotes on distinct keys, not on input length, so
        // it is filled entry by entry. Reserving up front would promote a
        // small map that receives many duplicate keys.
        while matches!(self.repr, Repr::Linear { .. }) {
            let Some((k, v)) = iter.next() else {
                return;
            };
            self.insert(k, v);
        }
        // Once a table exists, size it for the rest of the input. A map that
        // was empty holds only keys from this input, so the rest is new.
        if let Repr::Table(m) = &mut self.repr {
            m.reserve(crate::family::extend_reservation(
                started_empty,
                iter.size_hint().0,
            ));
        }
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<'a, K: Eq + Hash + Copy, V: Copy, S: BuildHasher + Clone> Extend<(&'a K, &'a V)>
    for AdaptiveHashMap<K, V, S>
{
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(*k, *v);
        }
    }
}

impl<K: Eq + Hash, V, const N: usize> From<[(K, V); N]> for AdaptiveHashMap<K, V> {
    fn from(a: [(K, V); N]) -> Self {
        a.into_iter().collect()
    }
}

impl<K, V, S, Q> core::ops::Index<&Q> for AdaptiveHashMap<K, V, S>
where
    K: Borrow<Q> + Eq + Hash,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("key not present in AdaptiveHashMap")
    }
}

impl<K: Clone, V: Clone, S: Clone> Clone for AdaptiveHashMap<K, V, S> {
    fn clone(&self) -> Self {
        Self {
            repr: match &self.repr {
                Repr::Linear { entries, hasher } => Repr::Linear {
                    entries: entries.clone(),
                    hasher: hasher.clone(),
                },
                Repr::Table(m) => Repr::Table(m.clone()),
            },
            policy: self.policy,
        }
    }
}

/// Mapping equality across variants; the policy is not compared.
impl<K: Eq + Hash, V: PartialEq, S: BuildHasher> PartialEq for AdaptiveHashMap<K, V, S> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .all(|(k, v)| other.get(k).is_some_and(|o| v == o))
    }
}
impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for AdaptiveHashMap<K, V, S> {}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for AdaptiveHashMap<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

// --- iterators -------------------------------------------------------------

enum IterMutRepr<'a, K, V> {
    Linear(core::slice::IterMut<'a, (K, V)>),
    Table(hash_map::IterMut<'a, K, V>),
}

/// Entries with mutable values of [`AdaptiveHashMap`].
pub struct IterMut<'a, K, V>(IterMutRepr<'a, K, V>);

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            IterMutRepr::Linear(it) => it.next().map(|(k, v)| (&*k, v)),
            IterMutRepr::Table(it) => it.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            IterMutRepr::Linear(it) => it.size_hint(),
            IterMutRepr::Table(it) => it.size_hint(),
        }
    }
    fn fold<B, F: FnMut(B, Self::Item) -> B>(self, init: B, mut f: F) -> B {
        match self.0 {
            IterMutRepr::Linear(it) => it.fold(init, |acc, (k, v)| f(acc, (&*k, v))),
            IterMutRepr::Table(it) => it.fold(init, f),
        }
    }
}
impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {}
impl<K, V> FusedIterator for IterMut<'_, K, V> {}
impl<K, V> fmt::Debug for IterMut<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IterMut").finish_non_exhaustive()
    }
}

enum IntoIterRepr<K, V> {
    Linear(vec::IntoIter<(K, V)>),
    Table(hash_map::IntoIter<K, V>),
}

/// Owning iterator of the adaptive hash profiles.
pub struct IntoIter<K, V>(IntoIterRepr<K, V>);

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<(K, V)> {
        match &mut self.0 {
            IntoIterRepr::Linear(it) => it.next(),
            IntoIterRepr::Table(it) => it.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            IntoIterRepr::Linear(it) => it.size_hint(),
            IntoIterRepr::Table(it) => it.size_hint(),
        }
    }
    fn fold<B, F: FnMut(B, (K, V)) -> B>(self, init: B, f: F) -> B {
        match self.0 {
            IntoIterRepr::Linear(it) => it.fold(init, f),
            IntoIterRepr::Table(it) => it.fold(init, f),
        }
    }
}
impl<K, V> ExactSizeIterator for IntoIter<K, V> {}
impl<K, V> FusedIterator for IntoIter<K, V> {}
impl<K, V> fmt::Debug for IntoIter<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IntoIter").finish_non_exhaustive()
    }
}

impl<K, V, S> IntoIterator for AdaptiveHashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter(match self.repr {
            Repr::Linear { entries, .. } => IntoIterRepr::Linear(entries.into_iter()),
            Repr::Table(m) => IntoIterRepr::Table(m.into_iter()),
        })
    }
}
impl<K, V, S> IntoIterator for AdaptiveFrozenHashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter(match self.repr {
            FrozenRepr::Linear { entries, .. } => {
                IntoIterRepr::Linear(entries.into_vec().into_iter())
            }
            FrozenRepr::Table(m) => IntoIterRepr::Table(m.into_iter()),
        })
    }
}
impl<'a, K, V, S> IntoIterator for &'a AdaptiveHashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}
impl<'a, K, V, S> IntoIterator for &'a mut AdaptiveHashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}

// --- capabilities ----------------------------------------------------------

impl<K, V, S> Len for AdaptiveHashMap<K, V, S> {
    fn len(&self) -> usize {
        Self::len(self)
    }
}
impl<K, V, S> MapTypes for AdaptiveHashMap<K, V, S> {
    type Key = K;
    type Value = V;
}
impl<K, V, S, Q> LookupRef<Q> for AdaptiveHashMap<K, V, S>
where
    K: Borrow<Q> + Eq + Hash,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> {
        Self::get_key_value(self, query)
    }
}
impl<K, V, S> Entries for AdaptiveHashMap<K, V, S> {
    type Iter<'a>
        = Iter<'a, K, V>
    where
        Self: 'a;
    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}
impl<K: Eq + Hash, V, S: BuildHasher + Clone> MapInsert for AdaptiveHashMap<K, V, S> {
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        Self::insert(self, key, value)
    }
}
impl<K, V, S, Q> MapRemove<Q> for AdaptiveHashMap<K, V, S>
where
    K: Borrow<Q> + Eq + Hash,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    fn remove_entry(&mut self, query: &Q) -> Option<(K, V)> {
        Self::remove_entry(self, query)
    }
}

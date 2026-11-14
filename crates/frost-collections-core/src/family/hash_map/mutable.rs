use alloc::collections::TryReserveError;
use core::borrow::Borrow;
use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::ops::{Index, IndexMut};
use std::collections::{HashMap, hash_map};
use std::hash::RandomState;

use super::FrozenHashMap;
use crate::adaptive::{AdaptiveFrozenHashMap, AdaptiveHash};
use crate::family::DuplicateKey;
use crate::iter::wrap_iter;
use crate::read::{Entries, Len, LookupRef, MapTypes};
use crate::write::{MapInsert, MapRemove};
use crate::{Compact, FreezeWith};

/// Mutable hash map with a private representation.
///
/// Representation today: std `HashMap<K, V, S>`. Signatures use crate-owned
/// iterator and entry types only.
///
/// Contract (shared with [`FrozenHashMap`]): lookup by `K: Borrow<Q>`,
/// `Q: Hash + Eq` under the map's `S`; `insert` keeps the stored key and
/// replaces the value; traversal order unspecified; mapping equality.
///
/// ```
/// # extern crate frost_collections_core as frost;
/// use frost::prelude::*;
/// let mut m: MutableHashMap<String, u32> = MutableHashMap::new();
/// *m.entry("a".into()).or_insert(0) += 1;
/// m.insert("b".into(), 2);
/// let f = m.freeze();                       // FrozenHashMap, shrunk
/// assert_eq!(f.get("a"), Some(&1));
/// let mut back = f.thaw();                  // MutableHashMap again, O(1)
/// back.remove("a");
/// assert_eq!(back.len(), 1);
/// ```
#[derive(Clone)]
pub struct MutableHashMap<K, V, S = RandomState> {
    pub(crate) m: HashMap<K, V, S>,
}

wrap_iter! {
    /// Borrowed entries, unspecified order.
    pub struct Iter<'a, K, V>(hash_map::Iter<'a, K, V>) -> (&'a K, &'a V); exact, clone
}
wrap_iter! {
    /// Entries with mutable values.
    pub struct IterMut<'a, K, V>(hash_map::IterMut<'a, K, V>) -> (&'a K, &'a mut V); exact
}
wrap_iter! {
    /// Borrowed keys.
    pub struct Keys<'a, K, V>(hash_map::Keys<'a, K, V>) -> &'a K; exact, clone
}
wrap_iter! {
    /// Borrowed values.
    pub struct Values<'a, K, V>(hash_map::Values<'a, K, V>) -> &'a V; exact, clone
}
wrap_iter! {
    /// Mutable values.
    pub struct ValuesMut<'a, K, V>(hash_map::ValuesMut<'a, K, V>) -> &'a mut V; exact
}
wrap_iter! {
    /// Owning entries.
    pub struct IntoIter<K, V>(hash_map::IntoIter<K, V>) -> (K, V); exact
}
wrap_iter! {
    /// Draining entries.
    pub struct Drain<'a, K, V>(hash_map::Drain<'a, K, V>) -> (K, V); exact
}

/// A view into one key of a [`MutableHashMap`].
pub enum Entry<'a, K, V> {
    /// The key is present.
    Occupied(OccupiedEntry<'a, K, V>),
    /// The key is absent.
    Vacant(VacantEntry<'a, K, V>),
}

/// Present key.
pub struct OccupiedEntry<'a, K, V>(hash_map::OccupiedEntry<'a, K, V>);
/// Absent key.
pub struct VacantEntry<'a, K, V>(hash_map::VacantEntry<'a, K, V>);

impl<'a, K, V> Entry<'a, K, V> {
    fn wrap(e: hash_map::Entry<'a, K, V>) -> Self {
        match e {
            hash_map::Entry::Occupied(o) => Self::Occupied(OccupiedEntry(o)),
            hash_map::Entry::Vacant(v) => Self::Vacant(VacantEntry(v)),
        }
    }
    /// The entry's key (the stored one when occupied).
    pub fn key(&self) -> &K {
        match self {
            Self::Occupied(o) => o.key(),
            Self::Vacant(v) => v.key(),
        }
    }
    /// Value, inserting `default` if absent.
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Self::Occupied(o) => o.into_mut(),
            Self::Vacant(v) => v.insert(default),
        }
    }
    /// Value, inserting `f()` if absent.
    pub fn or_insert_with<F: FnOnce() -> V>(self, f: F) -> &'a mut V {
        match self {
            Self::Occupied(o) => o.into_mut(),
            Self::Vacant(v) => v.insert(f()),
        }
    }
    /// Value, inserting `f(&key)` if absent.
    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, f: F) -> &'a mut V {
        match self {
            Self::Occupied(o) => o.into_mut(),
            Self::Vacant(v) => {
                let value = f(v.key());
                v.insert(value)
            }
        }
    }
    /// Apply `f` to the value if present.
    #[must_use]
    pub fn and_modify<F: FnOnce(&mut V)>(mut self, f: F) -> Self {
        if let Self::Occupied(o) = &mut self {
            f(o.get_mut());
        }
        self
    }
}

impl<'a, K, V: Default> Entry<'a, K, V> {
    /// Value, inserting `V::default()` if absent.
    pub fn or_default(self) -> &'a mut V {
        self.or_insert_with(V::default)
    }
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    /// Stored key.
    pub fn key(&self) -> &K {
        self.0.key()
    }
    /// Value.
    pub fn get(&self) -> &V {
        self.0.get()
    }
    /// Value, mutably.
    pub fn get_mut(&mut self) -> &mut V {
        self.0.get_mut()
    }
    /// Value with the map's lifetime.
    pub fn into_mut(self) -> &'a mut V {
        self.0.into_mut()
    }
    /// Replace the value (stored key kept), returning the old one.
    pub fn insert(&mut self, value: V) -> V {
        self.0.insert(value)
    }
    /// Remove, returning the value.
    pub fn remove(self) -> V {
        self.0.remove()
    }
    /// Remove, returning the stored key and value.
    pub fn remove_entry(self) -> (K, V) {
        self.0.remove_entry()
    }
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    /// Key that would be inserted.
    pub fn key(&self) -> &K {
        self.0.key()
    }
    /// Take the key back without inserting.
    pub fn into_key(self) -> K {
        self.0.into_key()
    }
    /// Insert and return the value.
    pub fn insert(self, value: V) -> &'a mut V {
        self.0.insert(value)
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Entry<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Occupied(o) => f.debug_tuple("Occupied").field(o).finish(),
            Self::Vacant(v) => f.debug_tuple("Vacant").field(v).finish(),
        }
    }
}
impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for OccupiedEntry<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("key", self.key())
            .field("value", self.get())
            .finish()
    }
}
impl<K: fmt::Debug, V> fmt::Debug for VacantEntry<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry").field(self.key()).finish()
    }
}

impl<K, V> MutableHashMap<K, V, RandomState> {
    /// Empty map with std's randomized hasher.
    pub fn new() -> Self {
        Self { m: HashMap::new() }
    }
    /// Empty map with room for `capacity` entries.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            m: HashMap::with_capacity(capacity),
        }
    }
}

impl<K, V, S: Default> Default for MutableHashMap<K, V, S> {
    fn default() -> Self {
        Self {
            m: HashMap::default(),
        }
    }
}

impl<K, V, S> MutableHashMap<K, V, S> {
    /// Adopt a native map without changing capacity or hashing policy.
    pub fn from_map(map: HashMap<K, V, S>) -> Self {
        Self { m: map }
    }

    /// Empty map with `hasher`.
    pub fn with_hasher(hasher: S) -> Self {
        Self {
            m: HashMap::with_hasher(hasher),
        }
    }
    /// Empty map with room for `capacity` entries and `hasher`.
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        Self {
            m: HashMap::with_capacity_and_hasher(capacity, hasher),
        }
    }
    /// Entry count.
    pub fn len(&self) -> usize {
        self.m.len()
    }
    /// `true` when empty.
    pub fn is_empty(&self) -> bool {
        self.m.is_empty()
    }
    /// Entries that fit without reallocating.
    pub fn capacity(&self) -> usize {
        self.m.capacity()
    }
    /// The hasher.
    pub fn hasher(&self) -> &S {
        self.m.hasher()
    }
    /// Remove all entries; keeps capacity.
    pub fn clear(&mut self) {
        self.m.clear();
    }
    /// Borrowed entries.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter(self.m.iter())
    }
    /// Entries with mutable values.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut(self.m.iter_mut())
    }
    /// Keys.
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys(self.m.keys())
    }
    /// Values.
    pub fn values(&self) -> Values<'_, K, V> {
        Values(self.m.values())
    }
    /// Mutable values.
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut(self.m.values_mut())
    }
    /// Remove and yield every entry; keeps capacity.
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        Drain(self.m.drain())
    }
    /// Keep entries for which `f` returns `true`.
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, f: F) {
        self.m.retain(f);
    }

    /// Extract the native map without rebuilding or cloning its hasher.
    #[must_use]
    pub fn into_map(self) -> HashMap<K, V, S> {
        self.m
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> MutableHashMap<K, V, S> {
    /// Build with an explicit hashing policy, retaining first keys and last values.
    pub fn from_iter_with_hasher<I>(iter: I, hasher: S) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut map = Self::with_hasher(hasher);
        map.extend(iter);
        map
    }

    /// Strict construction with an explicit hashing policy.
    ///
    /// # Errors
    /// Returns the first duplicate entry in input order. Other consumed entries
    /// are dropped; the error does not contain the complete input collection.
    pub fn try_from_unique_with_hasher<I>(iter: I, hasher: S) -> Result<Self, DuplicateKey<K, V>>
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let iter = iter.into_iter();
        let mut map = Self::with_capacity_and_hasher(iter.size_hint().0, hasher);
        for (key, value) in iter {
            if map.contains_key(&key) {
                return Err(DuplicateKey { key, value });
            }
            map.insert(key, value);
        }
        Ok(map)
    }

    /// Strict construction with the default hashing policy.
    ///
    /// # Errors
    /// Returns the first duplicate entry in input order; other consumed entries
    /// are dropped.
    pub fn try_from_unique<I>(iter: I) -> Result<Self, DuplicateKey<K, V>>
    where
        I: IntoIterator<Item = (K, V)>,
        S: Default,
    {
        Self::try_from_unique_with_hasher(iter, S::default())
    }

    /// Reserve room for `additional` entries.
    pub fn reserve(&mut self, additional: usize) {
        self.m.reserve(additional);
    }
    /// Fallible [`reserve`](Self::reserve).
    ///
    /// # Errors
    /// Capacity overflow or allocator failure; nothing changes.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.m.try_reserve(additional)
    }
    /// Release excess capacity.
    pub fn shrink_to_fit(&mut self) {
        self.m.shrink_to_fit();
    }
    /// Insert; if the key is present the stored key is kept and the old value
    /// returned.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.m.insert(key, value)
    }
    /// Entry for `key`.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        Entry::wrap(self.m.entry(key))
    }
    /// Value for `key`.
    pub fn get<Q: Hash + Eq + ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.m.get(key)
    }
    /// Mutable value for `key`.
    pub fn get_mut<Q: Hash + Eq + ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        self.m.get_mut(key)
    }
    /// Stored key and value.
    pub fn get_key_value<Q: Hash + Eq + ?Sized>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
    {
        self.m.get_key_value(key)
    }
    /// `true` if present.
    pub fn contains_key<Q: Hash + Eq + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.m.contains_key(key)
    }
    /// Remove, returning the value.
    pub fn remove<Q: Hash + Eq + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        self.m.remove(key)
    }
    /// Remove, returning the stored key and value.
    pub fn remove_entry<Q: Hash + Eq + ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
    {
        self.m.remove_entry(key)
    }

    /// Freeze into [`FrozenHashMap`]: shrink (may rehash), same hasher.
    #[must_use]
    pub fn freeze(self) -> FrozenHashMap<K, V, S> {
        FrozenHashMap::from_map(self.m)
    }
}

impl<K, V, S> From<HashMap<K, V, S>> for MutableHashMap<K, V, S> {
    fn from(m: HashMap<K, V, S>) -> Self {
        Self { m }
    }
}
impl<K, V, S> From<MutableHashMap<K, V, S>> for HashMap<K, V, S> {
    fn from(m: MutableHashMap<K, V, S>) -> Self {
        m.m
    }
}
impl<K: Eq + Hash, V, const N: usize> From<[(K, V); N]> for MutableHashMap<K, V> {
    fn from(a: [(K, V); N]) -> Self {
        a.into_iter().collect()
    }
}

/// First key representative, last value.
impl<K: Eq + Hash, V, S: BuildHasher + Default> FromIterator<(K, V)> for MutableHashMap<K, V, S> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::from_iter_with_hasher(iter, S::default())
    }
}
impl<K: Eq + Hash, V, S: BuildHasher> Extend<(K, V)> for MutableHashMap<K, V, S> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        self.m.reserve(crate::family::extend_reservation(
            self.m.is_empty(),
            iter.size_hint().0,
        ));
        for (key, value) in iter {
            self.m.insert(key, value);
        }
    }
}
impl<'a, K: Eq + Hash + Copy, V: Copy, S: BuildHasher> Extend<(&'a K, &'a V)>
    for MutableHashMap<K, V, S>
{
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        self.extend(iter.into_iter().map(|(&key, &value)| (key, value)));
    }
}

impl<K, V, S> IntoIterator for MutableHashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter(self.m.into_iter())
    }
}
impl<'a, K, V, S> IntoIterator for &'a MutableHashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}
impl<'a, K, V, S> IntoIterator for &'a mut MutableHashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}

impl<K, V, S, Q> Index<&Q> for MutableHashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
    S: BuildHasher,
{
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        &self.m[key]
    }
}

impl<K: Eq + Hash, V: PartialEq, S: BuildHasher> PartialEq for MutableHashMap<K, V, S> {
    fn eq(&self, other: &Self) -> bool {
        self.m == other.m
    }
}

impl<K, V, S, Q> IndexMut<&Q> for MutableHashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
    S: BuildHasher,
{
    fn index_mut(&mut self, key: &Q) -> &mut V {
        self.get_mut(key)
            .expect("key not present in MutableHashMap")
    }
}
impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for MutableHashMap<K, V, S> {}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for MutableHashMap<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.m.iter()).finish()
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> FreezeWith<Compact> for MutableHashMap<K, V, S> {
    type Output = FrozenHashMap<K, V, S>;
    fn freeze_with(self, _: Compact) -> Self::Output {
        self.freeze()
    }
}
impl<K: Eq + Hash, V, S: BuildHasher + Clone> FreezeWith<AdaptiveHash> for MutableHashMap<K, V, S> {
    type Output = AdaptiveFrozenHashMap<K, V, S>;
    fn freeze_with(self, plan: AdaptiveHash) -> Self::Output {
        AdaptiveFrozenHashMap::from_map(self.m, plan)
    }
}

impl<K, V, S> Len for MutableHashMap<K, V, S> {
    fn len(&self) -> usize {
        self.m.len()
    }
}
impl<K, V, S> MapTypes for MutableHashMap<K, V, S> {
    type Key = K;
    type Value = V;
}
impl<K, V, S, Q> LookupRef<Q> for MutableHashMap<K, V, S>
where
    K: Borrow<Q> + Eq + Hash,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> {
        self.m.get_key_value(query)
    }
}
impl<K, V, S> Entries for MutableHashMap<K, V, S> {
    type Iter<'a>
        = Iter<'a, K, V>
    where
        Self: 'a;
    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}
impl<K: Eq + Hash, V, S: BuildHasher> MapInsert for MutableHashMap<K, V, S> {
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.m.insert(key, value)
    }
}
impl<K, V, S, Q> MapRemove<Q> for MutableHashMap<K, V, S>
where
    K: Borrow<Q> + Eq + Hash,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    fn remove_entry(&mut self, query: &Q) -> Option<(K, V)> {
        self.m.remove_entry(query)
    }
}

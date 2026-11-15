use core::borrow::Borrow;
use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::ops::Index;
use std::collections::HashMap;
use std::hash::RandomState;

use super::{IntoIter, Iter, Keys, MutableHashMap, Values};
use crate::family::DuplicateKey;
use crate::read::{Entries, Len, LookupRef, MapTypes};

/// Frozen hash map family. Representation: a shrunk std `HashMap<K, V, S>`
/// (private; may change without a breaking release).
///
/// Contract
/// * Lookup: `K: Borrow<Q>`, `Q: Hash + Eq`, using the map's own `S`.
/// * Hashing: `S` is kept; the default is std's randomized `RandomState`.
/// * Traversal order: unspecified.
/// * Construction from entries: first key representative, last value.
/// * Equality: mapping equality (`HashMap`'s `PartialEq`).
///
/// ```compile_fail,E0599
/// # extern crate frost_collections_core as frost;
/// use frost::prelude::*;
/// let mut m: FrozenHashMap<&str, i32> = [("a", 1)].into_iter().collect();
/// m.insert("b", 2); // no mutation
/// ```
#[derive(Clone)]
pub struct FrozenHashMap<K, V, S = RandomState> {
    pub(crate) map: HashMap<K, V, S>,
}

// Manual: `derive(Default)` would demand `K: Default, V: Default`.
impl<K, V, S: Default> Default for FrozenHashMap<K, V, S> {
    fn default() -> Self {
        Self {
            map: HashMap::default(),
        }
    }
}

/// Equivalent to the `Compact` plan: shrinks, keeps `S`.
impl<K: Eq + Hash, V, S: BuildHasher> From<HashMap<K, V, S>> for FrozenHashMap<K, V, S> {
    fn from(map: HashMap<K, V, S>) -> Self {
        Self::from_map(map)
    }
}

impl<K, V, S> FrozenHashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    pub(crate) fn from_map(mut map: HashMap<K, V, S>) -> Self {
        map.shrink_to_fit();
        Self { map }
    }

    /// Build with an explicit hashing policy, retaining first keys and last values.
    pub fn from_iter_with_hasher<I>(iter: I, hasher: S) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        MutableHashMap::from_iter_with_hasher(iter, hasher).freeze()
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
        MutableHashMap::try_from_unique_with_hasher(iter, hasher).map(MutableHashMap::freeze)
    }

    /// Strict builder: rejects the first duplicate key.
    ///
    /// # Errors
    /// [`DuplicateKey`] with the offending entry.
    pub fn try_from_unique<I>(iter: I) -> Result<Self, DuplicateKey<K, V>>
    where
        I: IntoIterator<Item = (K, V)>,
        S: Default,
    {
        Self::try_from_unique_with_hasher(iter, S::default())
    }

    /// Value for `key`.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get(key)
    }

    /// Stored key representative and value.
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get_key_value(key)
    }

    /// `true` if present.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.contains_key(key)
    }
}

impl<K, V, S> FrozenHashMap<K, V, S> {
    /// Entry count.
    pub fn len(&self) -> usize {
        self.map.len()
    }
    /// `true` when empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    /// Entries, unspecified order.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter(self.map.iter())
    }
    /// Keys, unspecified order.
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys(self.map.keys())
    }
    /// Values, unspecified order.
    pub fn values(&self) -> Values<'_, K, V> {
        Values(self.map.values())
    }
    /// The hasher.
    pub fn hasher(&self) -> &S {
        self.map.hasher()
    }
    /// Back to the paired mutable family. O(1); same hasher and table.
    #[must_use]
    pub fn thaw(self) -> MutableHashMap<K, V, S> {
        MutableHashMap::from(self.map)
    }

    /// Interop: the std `HashMap`. O(1).
    #[must_use]
    pub fn into_hash_map(self) -> HashMap<K, V, S> {
        self.map
    }

    /// Extract the native map; an alias for [`into_hash_map`](Self::into_hash_map).
    #[must_use]
    pub fn into_map(self) -> HashMap<K, V, S> {
        self.into_hash_map()
    }
}

/// First key representative, last value (`HashMap::insert` semantics).
impl<K, V, S> FromIterator<(K, V)> for FrozenHashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Default,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        MutableHashMap::<K, V, S>::from_iter(iter).freeze()
    }
}

impl<K, V, S> PartialEq for FrozenHashMap<K, V, S>
where
    K: Eq + Hash,
    V: PartialEq,
    S: BuildHasher,
{
    fn eq(&self, other: &Self) -> bool {
        self.map == other.map
    }
}

impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for FrozenHashMap<K, V, S> {}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for FrozenHashMap<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.map.iter()).finish()
    }
}

impl<K, V, S, Q> Index<&Q> for FrozenHashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
    S: BuildHasher,
{
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        &self.map[key]
    }
}

impl<'a, K, V, S> IntoIterator for &'a FrozenHashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K, V, S> IntoIterator for FrozenHashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.map.into_iter())
    }
}

impl<K, V, S> Len for FrozenHashMap<K, V, S> {
    fn len(&self) -> usize {
        self.map.len()
    }
}

impl<K, V, S> MapTypes for FrozenHashMap<K, V, S> {
    type Key = K;
    type Value = V;
}

impl<K, V, S, Q> LookupRef<Q> for FrozenHashMap<K, V, S>
where
    K: Borrow<Q> + Eq + Hash,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> {
        self.map.get_key_value(query)
    }
}

impl<K, V, S> Entries for FrozenHashMap<K, V, S> {
    type Iter<'a>
        = Iter<'a, K, V>
    where
        Self: 'a;
    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

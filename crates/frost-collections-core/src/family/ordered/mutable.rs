use alloc::collections::{BTreeMap, btree_map};
use core::borrow::Borrow;
use core::fmt;
use core::ops::{Index, IndexMut, RangeBounds};

use super::FrozenOrderedMap;
use crate::family::DuplicateKey;
use crate::iter::wrap_iter;
use crate::read::{Entries, Len, LookupRef, MapTypes, OrderedEntries, RangeEntries};
use crate::write::{MapInsert, MapRemove};
use crate::{Compact, FreezeWith};

/// Mutable ordered map with a private representation.
///
/// Representation today: `BTreeMap<K, V>`. Signatures use crate-owned
/// iterator and entry types only.
///
/// Contract (shared with [`FrozenOrderedMap`]): `K: Borrow<Q> + Ord` lookup;
/// ascending traversal; `range` as `BTreeMap::range`; `insert` keeps the
/// stored key; **construction from entries keeps the first key and the last
/// value**, independently of the native backend's bulk construction policy.
///
/// ```
/// # extern crate frost_collections_core as frost;
/// use frost::prelude::*;
/// let mut m = MutableOrderedMap::new();
/// m.insert(2, "b");
/// m.insert(1, "a");
/// *m.entry(3).or_insert("c") = "C";
/// assert_eq!(m.keys().copied().collect::<Vec<_>>(), [1, 2, 3]);
/// let f = m.freeze();                       // FrozenOrderedMap, O(n)
/// assert_eq!(f.get(&3), Some(&"C"));
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MutableOrderedMap<K, V> {
    pub(crate) m: BTreeMap<K, V>,
}

wrap_iter! {
    /// Ascending borrowed entries.
    pub struct Iter<'a, K, V>(btree_map::Iter<'a, K, V>) -> (&'a K, &'a V); double, exact, clone
}
wrap_iter! {
    /// Ascending entries with mutable values.
    pub struct IterMut<'a, K, V>(btree_map::IterMut<'a, K, V>) -> (&'a K, &'a mut V); double, exact
}
wrap_iter! {
    /// Ascending keys.
    pub struct Keys<'a, K, V>(btree_map::Keys<'a, K, V>) -> &'a K; double, exact, clone
}
wrap_iter! {
    /// Values in key order.
    pub struct Values<'a, K, V>(btree_map::Values<'a, K, V>) -> &'a V; double, exact, clone
}
wrap_iter! {
    /// Mutable values in key order.
    pub struct ValuesMut<'a, K, V>(btree_map::ValuesMut<'a, K, V>) -> &'a mut V; double, exact
}
wrap_iter! {
    /// Ascending owning entries.
    pub struct IntoIter<K, V>(btree_map::IntoIter<K, V>) -> (K, V); double, exact
}
wrap_iter! {
    /// Entries in a key range.
    pub struct Range<'a, K, V>(btree_map::Range<'a, K, V>) -> (&'a K, &'a V); double, clone
}
wrap_iter! {
    /// Entries in a key range, mutable values.
    pub struct RangeMut<'a, K, V>(btree_map::RangeMut<'a, K, V>) -> (&'a K, &'a mut V); double
}

/// A view into one key of a [`MutableOrderedMap`].
pub enum Entry<'a, K, V> {
    /// The key is present.
    Occupied(OccupiedEntry<'a, K, V>),
    /// The key is absent.
    Vacant(VacantEntry<'a, K, V>),
}
/// Present key.
pub struct OccupiedEntry<'a, K, V>(btree_map::OccupiedEntry<'a, K, V>);
/// Absent key.
pub struct VacantEntry<'a, K, V>(btree_map::VacantEntry<'a, K, V>);

impl<'a, K: Ord, V> Entry<'a, K, V> {
    fn wrap(e: btree_map::Entry<'a, K, V>) -> Self {
        match e {
            btree_map::Entry::Occupied(o) => Self::Occupied(OccupiedEntry(o)),
            btree_map::Entry::Vacant(v) => Self::Vacant(VacantEntry(v)),
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
impl<'a, K: Ord, V: Default> Entry<'a, K, V> {
    /// Value, inserting `V::default()` if absent.
    pub fn or_default(self) -> &'a mut V {
        self.or_insert_with(V::default)
    }
}
impl<'a, K: Ord, V> OccupiedEntry<'a, K, V> {
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
    /// Replace the value (stored key kept).
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
impl<'a, K: Ord, V> VacantEntry<'a, K, V> {
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
impl<K: fmt::Debug + Ord, V: fmt::Debug> fmt::Debug for Entry<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Occupied(o) => f
                .debug_struct("Occupied")
                .field("key", o.key())
                .field("value", o.get())
                .finish(),
            Self::Vacant(v) => f.debug_tuple("Vacant").field(v.key()).finish(),
        }
    }
}
impl<K: fmt::Debug + Ord, V: fmt::Debug> fmt::Debug for OccupiedEntry<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("key", self.key())
            .field("value", self.get())
            .finish()
    }
}
impl<K: fmt::Debug + Ord, V> fmt::Debug for VacantEntry<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry").field(self.key()).finish()
    }
}

impl<K, V> Default for MutableOrderedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> MutableOrderedMap<K, V> {
    /// Adopt a native tree without rebuilding or cloning entries.
    pub fn from_map(map: BTreeMap<K, V>) -> Self {
        Self { m: map }
    }

    /// Empty; does not allocate.
    pub const fn new() -> Self {
        Self { m: BTreeMap::new() }
    }
    /// Entry count.
    pub fn len(&self) -> usize {
        self.m.len()
    }
    /// `true` when empty.
    pub fn is_empty(&self) -> bool {
        self.m.is_empty()
    }
    /// Remove all entries.
    pub fn clear(&mut self) {
        self.m.clear();
    }
    /// Ascending entries.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter(self.m.iter())
    }
    /// Ascending entries with mutable values.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut(self.m.iter_mut())
    }
    /// Ascending keys.
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys(self.m.keys())
    }
    /// Values in key order.
    pub fn values(&self) -> Values<'_, K, V> {
        Values(self.m.values())
    }
    /// Mutable values in key order.
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut(self.m.values_mut())
    }

    /// Extract the native tree without rebuilding or cloning entries.
    #[must_use]
    pub fn into_map(self) -> BTreeMap<K, V> {
        self.m
    }

    /// Freeze into [`FrozenOrderedMap`]: O(n) flatten, no comparisons.
    #[must_use]
    pub fn freeze(self) -> FrozenOrderedMap<K, V> {
        FrozenOrderedMap::from(self.m)
    }
}

impl<K: Ord, V> MutableOrderedMap<K, V> {
    /// Strict construction in input order.
    ///
    /// # Errors
    /// Returns the first duplicate entry encountered. Other consumed entries
    /// are dropped; the error does not return the complete input collection.
    pub fn try_from_unique<I>(iter: I) -> Result<Self, DuplicateKey<K, V>>
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut map = Self::new();
        for (key, value) in iter {
            if map.contains_key(&key) {
                return Err(DuplicateKey { key, value });
            }
            map.insert(key, value);
        }
        Ok(map)
    }

    /// Insert; an existing key keeps its stored representative.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.m.insert(key, value)
    }
    /// Entry for `key`.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        Entry::wrap(self.m.entry(key))
    }
    /// Value for `key`.
    pub fn get<Q: Ord + ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.m.get(key)
    }
    /// Mutable value for `key`.
    pub fn get_mut<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        self.m.get_mut(key)
    }
    /// Stored key and value.
    pub fn get_key_value<Q: Ord + ?Sized>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
    {
        self.m.get_key_value(key)
    }
    /// `true` if present.
    pub fn contains_key<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.m.contains_key(key)
    }
    /// Remove, returning the value.
    pub fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        self.m.remove(key)
    }
    /// Remove, returning the stored key and value.
    pub fn remove_entry<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
    {
        self.m.remove_entry(key)
    }
    /// Keep entries for which `f` returns `true`.
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, f: F) {
        self.m.retain(f);
    }
    /// Least entry.
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        self.m.first_key_value()
    }
    /// Greatest entry.
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        self.m.last_key_value()
    }
    /// Remove the least entry.
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        self.m.pop_first()
    }
    /// Remove the greatest entry.
    pub fn pop_last(&mut self) -> Option<(K, V)> {
        self.m.pop_last()
    }
    /// Move all entries of `other` into `self`: on collision `self`'s stored
    /// key is kept and `other`'s value wins. Per-entry insertion enforces this
    /// contract independently of the native backend's bulk merge policy.
    pub fn append(&mut self, other: &mut Self) {
        if self.m.is_empty() {
            core::mem::swap(&mut self.m, &mut other.m);
            return;
        }
        for (k, v) in core::mem::take(&mut other.m) {
            self.m.insert(k, v);
        }
    }
    /// Split off entries with keys `>= key`.
    #[must_use]
    pub fn split_off<Q: Ord + ?Sized>(&mut self, key: &Q) -> Self
    where
        K: Borrow<Q>,
    {
        Self {
            m: self.m.split_off(key),
        }
    }
    /// Entries in `range`.
    ///
    /// # Panics
    /// As `BTreeMap::range`.
    pub fn range<Q: Ord + ?Sized, R: RangeBounds<Q>>(&self, range: R) -> Range<'_, K, V>
    where
        K: Borrow<Q>,
    {
        Range(self.m.range(range))
    }
    /// Entries in `range` with mutable values.
    ///
    /// # Panics
    /// As `BTreeMap::range_mut`.
    pub fn range_mut<Q: Ord + ?Sized, R: RangeBounds<Q>>(&mut self, range: R) -> RangeMut<'_, K, V>
    where
        K: Borrow<Q>,
    {
        RangeMut(self.m.range_mut(range))
    }
}

impl<K, V> From<BTreeMap<K, V>> for MutableOrderedMap<K, V> {
    fn from(m: BTreeMap<K, V>) -> Self {
        Self { m }
    }
}
impl<K, V> From<MutableOrderedMap<K, V>> for BTreeMap<K, V> {
    fn from(m: MutableOrderedMap<K, V>) -> Self {
        m.m
    }
}
impl<K: Ord, V, const N: usize> From<[(K, V); N]> for MutableOrderedMap<K, V> {
    fn from(a: [(K, V); N]) -> Self {
        a.into_iter().collect()
    }
}

/// First key representative, last value. The entries are staged, sorted
/// stably, and reduced by the frozen builder, which applies this rule itself;
/// the tree is then built in bulk from strictly ascending entries. This never
/// relies on `BTreeMap::from_iter`'s own duplicate handling, and it costs one
/// sort and O(n) temporary staging instead of a tree insertion per entry.
impl<K: Ord, V> FromIterator<(K, V)> for MutableOrderedMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        super::FrozenOrderedMap::from_iter(iter).thaw()
    }
}
impl<K: Ord, V> Extend<(K, V)> for MutableOrderedMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.m.insert(k, v);
        }
    }
}
impl<'a, K: Ord + Copy, V: Copy> Extend<(&'a K, &'a V)> for MutableOrderedMap<K, V> {
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.m.insert(*k, *v);
        }
    }
}

impl<K, V> IntoIterator for MutableOrderedMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter(self.m.into_iter())
    }
}
impl<'a, K, V> IntoIterator for &'a MutableOrderedMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}
impl<'a, K, V> IntoIterator for &'a mut MutableOrderedMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}

impl<K: Borrow<Q> + Ord, V, Q: Ord + ?Sized> Index<&Q> for MutableOrderedMap<K, V> {
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        &self.m[key]
    }
}

impl<K: Borrow<Q> + Ord, V, Q: Ord + ?Sized> IndexMut<&Q> for MutableOrderedMap<K, V> {
    fn index_mut(&mut self, key: &Q) -> &mut V {
        self.get_mut(key).expect("no entry found for key")
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for MutableOrderedMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.m.iter()).finish()
    }
}

impl<K, V> FreezeWith<Compact> for MutableOrderedMap<K, V> {
    type Output = FrozenOrderedMap<K, V>;
    fn freeze_with(self, _: Compact) -> Self::Output {
        self.freeze()
    }
}

impl<K, V> Len for MutableOrderedMap<K, V> {
    fn len(&self) -> usize {
        self.m.len()
    }
}
impl<K, V> MapTypes for MutableOrderedMap<K, V> {
    type Key = K;
    type Value = V;
}
impl<K: Borrow<Q> + Ord, V, Q: Ord + ?Sized> LookupRef<Q> for MutableOrderedMap<K, V> {
    fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> {
        self.m.get_key_value(query)
    }
}
impl<K, V> Entries for MutableOrderedMap<K, V> {
    type Iter<'a>
        = Iter<'a, K, V>
    where
        Self: 'a;
    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}
impl<K, V> OrderedEntries for MutableOrderedMap<K, V> {
    fn first_key_value(&self) -> Option<(&K, &V)> {
        self.m.iter().next()
    }
    fn last_key_value(&self) -> Option<(&K, &V)> {
        self.m.iter().next_back()
    }
}
impl<K: Borrow<Q> + Ord, V, Q: Ord + ?Sized> RangeEntries<Q> for MutableOrderedMap<K, V> {
    type Range<'a>
        = Range<'a, K, V>
    where
        Self: 'a;
    fn range_entries<R: RangeBounds<Q>>(&self, range: R) -> Self::Range<'_> {
        self.range(range)
    }
}
impl<K: Ord, V> MapInsert for MutableOrderedMap<K, V> {
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.m.insert(key, value)
    }
}
impl<K: Borrow<Q> + Ord, V, Q: Ord + ?Sized> MapRemove<Q> for MutableOrderedMap<K, V> {
    fn remove_entry(&mut self, query: &Q) -> Option<(K, V)> {
        self.m.remove_entry(query)
    }
}

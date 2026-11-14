//! `indexmap::{IndexMap, IndexSet}` (feature `indexmap-2`).
//!
//! Traversal order is insertion order (or the order produced by an explicit
//! reordering operation before freezing). Freezing removes the ambiguity
//! between `swap_remove` and `shift_remove` simply by removing removal.

use core::hash::{BuildHasher, Hash};

use indexmap::{Equivalent, IndexMap, IndexSet, map, set};

use crate::frozen::sealed::Sealed;
use crate::read::{
    ElementType, Elements, Entries, IndexOf, IndexedEntries, Len, LookupRef, MapTypes, SetContains,
    SetGet,
};
use crate::{Compact, Eligible, FreezeWith, Frozen, Retain};

impl<K, V, S> Sealed for IndexMap<K, V, S> {}
impl<K, V, S> Eligible for IndexMap<K, V, S> {}
impl<T, S> Sealed for IndexSet<T, S> {}
impl<T, S> Eligible for IndexSet<T, S> {}

impl<K, V, S> Len for IndexMap<K, V, S> {
    fn len(&self) -> usize {
        IndexMap::len(self)
    }
}

impl<K, V, S> MapTypes for IndexMap<K, V, S> {
    type Key = K;
    type Value = V;
}

impl<K, V, S, Q> LookupRef<Q> for IndexMap<K, V, S>
where
    Q: Hash + Equivalent<K> + ?Sized,
    S: BuildHasher,
{
    fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> {
        IndexMap::get_key_value(self, query)
    }
}

impl<K, V, S> Entries for IndexMap<K, V, S> {
    type Iter<'a>
        = map::Iter<'a, K, V>
    where
        Self: 'a;
    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

impl<K, V, S> IndexedEntries for IndexMap<K, V, S> {
    fn get_index(&self, index: usize) -> Option<(&K, &V)> {
        IndexMap::get_index(self, index)
    }
}

impl<K, V, S, Q> IndexOf<Q> for IndexMap<K, V, S>
where
    Q: Hash + Equivalent<K> + ?Sized,
    S: BuildHasher,
{
    fn get_index_of(&self, query: &Q) -> Option<usize> {
        IndexMap::get_index_of(self, query)
    }
}

impl<K, V, S> FreezeWith<Retain> for IndexMap<K, V, S> {
    type Output = Frozen<Self>;
    fn freeze_with(self, _: Retain) -> Frozen<Self> {
        Frozen::new_unchecked(self)
    }
}

/// Shrinks entries and index in place; order and hasher preserved.
impl<K, V, S> FreezeWith<Compact> for IndexMap<K, V, S> {
    type Output = Frozen<Self>;
    fn freeze_with(mut self, _: Compact) -> Frozen<Self> {
        self.shrink_to_fit();
        Frozen::new_unchecked(self)
    }
}

impl<K, V, S> Frozen<IndexMap<K, V, S>> {
    /// Value for `key`.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: Hash + Equivalent<K> + ?Sized,
        S: BuildHasher,
    {
        self.backend().get(key)
    }
    /// Stored key representative and value.
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        Q: Hash + Equivalent<K> + ?Sized,
        S: BuildHasher,
    {
        self.backend().get_key_value(key)
    }
    /// Entry at insertion position `index`.
    pub fn get_index(&self, index: usize) -> Option<(&K, &V)> {
        self.backend().get_index(index)
    }
    /// Insertion position of `key`.
    pub fn get_index_of<Q>(&self, key: &Q) -> Option<usize>
    where
        Q: Hash + Equivalent<K> + ?Sized,
        S: BuildHasher,
    {
        self.backend().get_index_of(key)
    }
    /// Entries in insertion order.
    pub fn iter(&self) -> map::Iter<'_, K, V> {
        self.backend().iter()
    }
    /// Keys in insertion order.
    pub fn keys(&self) -> map::Keys<'_, K, V> {
        self.backend().keys()
    }
    /// Values in insertion order.
    pub fn values(&self) -> map::Values<'_, K, V> {
        self.backend().values()
    }
    /// Entries as a slice (`indexmap::map::Slice`), insertion order.
    pub fn as_slice(&self) -> &map::Slice<K, V> {
        self.backend().as_slice()
    }
}

impl<T, S> Len for IndexSet<T, S> {
    fn len(&self) -> usize {
        IndexSet::len(self)
    }
}

impl<T, S> ElementType for IndexSet<T, S> {
    type Item = T;
}

impl<T, S> Elements for IndexSet<T, S> {
    type Iter<'a>
        = set::Iter<'a, T>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

impl<T, S, Q> SetContains<Q> for IndexSet<T, S>
where
    Q: Hash + Equivalent<T> + ?Sized,
    S: BuildHasher,
{
    fn contains(&self, query: &Q) -> bool {
        IndexSet::contains(self, query)
    }
}

impl<T, S, Q> SetGet<Q> for IndexSet<T, S>
where
    Q: Hash + Equivalent<T> + ?Sized,
    S: BuildHasher,
{
    fn get(&self, query: &Q) -> Option<&T> {
        IndexSet::get(self, query)
    }
}

impl<T, S> FreezeWith<Retain> for IndexSet<T, S> {
    type Output = Frozen<Self>;
    fn freeze_with(self, _: Retain) -> Frozen<Self> {
        Frozen::new_unchecked(self)
    }
}

impl<T, S> FreezeWith<Compact> for IndexSet<T, S> {
    type Output = Frozen<Self>;
    fn freeze_with(mut self, _: Compact) -> Self::Output {
        self.shrink_to_fit();
        Frozen::new_unchecked(self)
    }
}

impl<T, S> Frozen<IndexSet<T, S>> {
    /// Stored representative of an equivalent query.
    pub fn get<Q>(&self, query: &Q) -> Option<&T>
    where
        Q: Hash + Equivalent<T> + ?Sized,
        S: BuildHasher,
    {
        self.backend().get(query)
    }
    /// Membership.
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        Q: Hash + Equivalent<T> + ?Sized,
        S: BuildHasher,
    {
        self.backend().contains(value)
    }
    /// Element at insertion position `index`.
    pub fn get_index(&self, index: usize) -> Option<&T> {
        self.backend().get_index(index)
    }
    /// Insertion-order traversal.
    pub fn iter(&self) -> set::Iter<'_, T> {
        self.backend().iter()
    }
}

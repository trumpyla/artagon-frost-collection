//! `BTreeMap` / `BTreeSet` (`alloc`). Query relation: `K: Borrow<Q> + Ord`,
//! `Q: Ord`.

use alloc::collections::{BTreeMap, BTreeSet, btree_map, btree_set};
use core::borrow::Borrow;
use core::ops::{Index, RangeBounds};

use crate::family::{FrozenOrderedMap, FrozenOrderedSet};
use crate::frozen::sealed::Sealed;
use crate::read::{
    ElementType, Elements, Entries, Len, LookupRef, MapTypes, OrderedElements, OrderedEntries,
    RangeElements, RangeEntries, SetContains, SetGet,
};
use crate::{Compact, Eligible, FreezeWith, Frozen, Retain};

impl<K, V> Sealed for BTreeMap<K, V> {}
impl<K, V> Eligible for BTreeMap<K, V> {}
impl<T> Sealed for BTreeSet<T> {}
impl<T> Eligible for BTreeSet<T> {}

// ---------------------------------------------------------------------------
// BTreeMap
// ---------------------------------------------------------------------------

impl<K, V> Len for BTreeMap<K, V> {
    fn len(&self) -> usize {
        BTreeMap::len(self)
    }
}

impl<K, V> MapTypes for BTreeMap<K, V> {
    type Key = K;
    type Value = V;
}

impl<K, V, Q> LookupRef<Q> for BTreeMap<K, V>
where
    K: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> {
        BTreeMap::get_key_value(self, query)
    }
    fn get(&self, query: &Q) -> Option<&V> {
        BTreeMap::get(self, query)
    }
}

impl<K, V> Entries for BTreeMap<K, V> {
    type Iter<'a>
        = btree_map::Iter<'a, K, V>
    where
        Self: 'a;
    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

impl<K, V> OrderedEntries for BTreeMap<K, V> {
    fn first_key_value(&self) -> Option<(&K, &V)> {
        self.iter().next()
    }
    fn last_key_value(&self) -> Option<(&K, &V)> {
        self.iter().next_back()
    }
}

impl<K, V, Q> RangeEntries<Q> for BTreeMap<K, V>
where
    K: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    type Range<'a>
        = btree_map::Range<'a, K, V>
    where
        Self: 'a;
    fn range_entries<R: RangeBounds<Q>>(&self, range: R) -> Self::Range<'_> {
        self.range(range)
    }
}

impl<K, V> FreezeWith<Retain> for BTreeMap<K, V> {
    type Output = Frozen<Self>;
    fn freeze_with(self, _: Retain) -> Frozen<Self> {
        Frozen::new_unchecked(self)
    }
}

/// O(n) moves into sorted boxed entries; keys are already unique and sorted,
/// so no comparison is performed.
impl<K, V> FreezeWith<Compact> for BTreeMap<K, V> {
    type Output = FrozenOrderedMap<K, V>;
    fn freeze_with(self, _: Compact) -> FrozenOrderedMap<K, V> {
        FrozenOrderedMap::from_btree(self)
    }
}

impl<K, V> FreezeWith<Compact> for Frozen<BTreeMap<K, V>> {
    type Output = FrozenOrderedMap<K, V>;
    fn freeze_with(self, _: Compact) -> FrozenOrderedMap<K, V> {
        FrozenOrderedMap::from_btree(self.into_inner())
    }
}

impl<K, V> Frozen<BTreeMap<K, V>> {
    /// Value for `key`.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.backend().get(key)
    }
    /// Stored key representative and value.
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.backend().get_key_value(key)
    }
    /// `true` if `key` is present.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.backend().contains_key(key)
    }
    /// Ascending entries.
    pub fn iter(&self) -> btree_map::Iter<'_, K, V> {
        self.backend().iter()
    }
    /// Ascending keys.
    pub fn keys(&self) -> btree_map::Keys<'_, K, V> {
        self.backend().keys()
    }
    /// Values in key order.
    pub fn values(&self) -> btree_map::Values<'_, K, V> {
        self.backend().values()
    }
    /// Entries in `range`.
    ///
    /// # Panics
    /// As `BTreeMap::range`.
    pub fn range<Q, R>(&self, range: R) -> btree_map::Range<'_, K, V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
    {
        self.backend().range(range)
    }
    /// Least entry.
    pub fn first_key_value(&self) -> Option<(&K, &V)>
    where
        K: Ord,
    {
        self.backend().first_key_value()
    }
    /// Greatest entry.
    pub fn last_key_value(&self) -> Option<(&K, &V)>
    where
        K: Ord,
    {
        self.backend().last_key_value()
    }
}

impl<K, V, Q> Index<&Q> for Frozen<BTreeMap<K, V>>
where
    K: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        &self.backend()[key]
    }
}

// ---------------------------------------------------------------------------
// BTreeSet
// ---------------------------------------------------------------------------

impl<T> Len for BTreeSet<T> {
    fn len(&self) -> usize {
        BTreeSet::len(self)
    }
}

impl<T> ElementType for BTreeSet<T> {
    type Item = T;
}

impl<T> Elements for BTreeSet<T> {
    type Iter<'a>
        = btree_set::Iter<'a, T>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

impl<T, Q> SetContains<Q> for BTreeSet<T>
where
    T: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    fn contains(&self, query: &Q) -> bool {
        BTreeSet::contains(self, query)
    }
}

impl<T, Q> SetGet<Q> for BTreeSet<T>
where
    T: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    fn get(&self, query: &Q) -> Option<&T> {
        BTreeSet::get(self, query)
    }
}

impl<T> OrderedElements for BTreeSet<T> {
    fn first_element(&self) -> Option<&T> {
        self.iter().next()
    }
    fn last_element(&self) -> Option<&T> {
        self.iter().next_back()
    }
}

impl<T, Q> RangeElements<Q> for BTreeSet<T>
where
    T: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    type Range<'a>
        = btree_set::Range<'a, T>
    where
        Self: 'a;
    fn range_elements<R: RangeBounds<Q>>(&self, range: R) -> Self::Range<'_> {
        self.range(range)
    }
}

impl<T> FreezeWith<Retain> for BTreeSet<T> {
    type Output = Frozen<Self>;
    fn freeze_with(self, _: Retain) -> Frozen<Self> {
        Frozen::new_unchecked(self)
    }
}

impl<T> FreezeWith<Compact> for BTreeSet<T> {
    type Output = FrozenOrderedSet<T>;
    fn freeze_with(self, _: Compact) -> FrozenOrderedSet<T> {
        FrozenOrderedSet::from_btree(self)
    }
}

impl<T> FreezeWith<Compact> for Frozen<BTreeSet<T>> {
    type Output = FrozenOrderedSet<T>;
    fn freeze_with(self, _: Compact) -> FrozenOrderedSet<T> {
        FrozenOrderedSet::from_btree(self.into_inner())
    }
}

impl<T> Frozen<BTreeSet<T>> {
    /// Membership.
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.backend().contains(value)
    }
    /// Stored representative.
    pub fn get<Q>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.backend().get(value)
    }
    /// Ascending traversal.
    pub fn iter(&self) -> btree_set::Iter<'_, T> {
        self.backend().iter()
    }
    /// Elements in `range`.
    ///
    /// # Panics
    /// As `BTreeSet::range`.
    pub fn range<Q, R>(&self, range: R) -> btree_set::Range<'_, T>
    where
        T: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
    {
        self.backend().range(range)
    }
    /// Least element.
    pub fn first(&self) -> Option<&T>
    where
        T: Ord,
    {
        self.backend().first()
    }
    /// Greatest element.
    pub fn last(&self) -> Option<&T>
    where
        T: Ord,
    {
        self.backend().last()
    }
}

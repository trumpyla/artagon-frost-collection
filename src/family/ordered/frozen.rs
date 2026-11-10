use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::iter::FusedIterator;
use core::ops::{Bound, Index, RangeBounds};

use super::MutableOrderedMap;
use crate::Rejected;
use crate::family::DuplicateKey;
use crate::iter::{Keys, Values, wrap_iter};
use crate::read::{
    ElementType, Elements, Entries, Len, LookupRef, MapTypes, OrderedElements, OrderedEntries,
    RangeElements, RangeEntries, SetContains, SetGet,
};

/// Input to `try_from_sorted_unique` was not strictly ascending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct NotSortedUnique {
    /// First index `i` with `items[i - 1] >= items[i]`.
    pub index: usize,
}

impl fmt::Display for NotSortedUnique {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "input is not strictly ascending at index {}", self.index)
    }
}

impl core::error::Error for NotSortedUnique {}

/// `[start, end)` indices for `range` over a strictly ascending slice, with
/// exactly `BTreeMap::range`'s bound validation (none when empty).
pub(crate) fn range_indices<T, Q, R>(
    items: &[T],
    key: impl Fn(&T) -> &Q,
    range: &R,
) -> (usize, usize)
where
    Q: Ord + ?Sized,
    R: RangeBounds<Q> + ?Sized,
{
    // `BTreeMap::range` on a new or cleared map returns an empty iterator
    // without validating bounds (a tree emptied by removal still validates
    // them). An empty slice never validates.
    if items.is_empty() {
        return (0, 0);
    }
    match (range.start_bound(), range.end_bound()) {
        (Bound::Excluded(s), Bound::Excluded(e)) if s == e => {
            panic!("range start and end are equal and excluded")
        }
        (Bound::Included(s) | Bound::Excluded(s), Bound::Included(e) | Bound::Excluded(e))
            if s > e =>
        {
            panic!("range start is greater than range end")
        }
        _ => {}
    }
    let start = match range.start_bound() {
        Bound::Included(s) => items.partition_point(|x| key(x) < s),
        Bound::Excluded(s) => items.partition_point(|x| key(x) <= s),
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(e) => items.partition_point(|x| key(x) <= e),
        Bound::Excluded(e) => items.partition_point(|x| key(x) < e),
        Bound::Unbounded => items.len(),
    };
    (start, end.max(start))
}

/// First `i` where `items[i-1] >= items[i]`.
fn first_unsorted<T>(items: &[T], cmp: impl Fn(&T, &T) -> Ordering) -> Option<usize> {
    items
        .windows(2)
        .position(|w| cmp(&w[0], &w[1]) != Ordering::Less)
        .map(|i| i + 1)
}

// ===========================================================================
// FrozenOrderedMap
// ===========================================================================

/// Frozen ordered map: strictly ascending boxed entries with binary search.
///
/// Contract (identical to `BTreeMap` where both define it)
/// * Lookup: `K: Borrow<Q> + Ord`, `Q: Ord`.
/// * Traversal and `range`: ascending key order; `range` panics on the same
///   bounds `BTreeMap::range` rejects, and never on an empty map. (A std tree
///   emptied by removal still validates its bounds.)
/// * Lookup and construction use `Ord` equivalence only. Whole-map
///   equality, ordering and hashing compare stored keys with
///   `K: PartialEq`/`Ord`/`Hash`, exactly as `BTreeMap` does.
/// * Construction from entries: **first key representative, last value** —
///   fixed by this type, unlike `BTreeMap::from_iter`, which leaves it
///   unspecified.
/// * Equality / ordering / hashing: over the ascending entry sequence. The
///   representation is canonical, so this is mapping equality.
///
/// ```
/// use frost::prelude::*;
/// let m: FrozenOrderedMap<i32, &str> = [(3, "c"), (1, "a"), (2, "b"), (1, "A")]
///     .into_iter()
///     .collect();
/// assert_eq!(m.get(&1), Some(&"A"));                       // last value wins
/// assert_eq!(m.range(2..).map(|(k, _)| *k).collect::<Vec<_>>(), [2, 3]);
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrozenOrderedMap<K, V> {
    // Invariant: strictly ascending by `K: Ord`.
    pub(crate) entries: Box<[(K, V)]>,
}

impl<K, V> Default for FrozenOrderedMap<K, V> {
    fn default() -> Self {
        Self {
            entries: Box::new([]),
        }
    }
}

impl<K, V> FrozenOrderedMap<K, V> {
    /// Adopt a vector the caller guarantees is strictly ascending.
    pub(crate) fn from_sorted_unique_unchecked(entries: Vec<(K, V)>) -> Self {
        Self {
            entries: entries.into_boxed_slice(),
        }
    }

    /// O(n), no comparisons: a `BTreeMap` is already strictly ascending.
    pub(crate) fn from_btree(map: BTreeMap<K, V>) -> Self {
        Self {
            entries: map.into_iter().collect(),
        }
    }

    /// Entry count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// `true` when empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Ascending entries.
    pub fn iter(&self) -> SortedIter<'_, K, V> {
        SortedIter(self.entries.iter())
    }
    /// Ascending keys.
    pub fn keys(&self) -> Keys<SortedIter<'_, K, V>> {
        Keys(self.iter())
    }
    /// Values in key order.
    pub fn values(&self) -> Values<SortedIter<'_, K, V>> {
        Values(self.iter())
    }
    /// Ascending entries as a slice.
    pub fn as_slice(&self) -> &[(K, V)] {
        &self.entries
    }
    /// Entry at ascending rank `index`.
    pub fn get_index(&self, index: usize) -> Option<(&K, &V)> {
        self.entries.get(index).map(|(k, v)| (k, v))
    }
    /// Least entry.
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        self.entries.first().map(|(k, v)| (k, v))
    }
    /// Greatest entry.
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        self.entries.last().map(|(k, v)| (k, v))
    }
    /// Ascending `(K, V)` vector; O(1).
    #[must_use]
    pub fn into_sorted_vec(self) -> Vec<(K, V)> {
        self.entries.into_vec()
    }
}

impl<K: Ord, V> FrozenOrderedMap<K, V> {
    /// Validate and adopt an already strictly ascending vector. O(n).
    ///
    /// Intended for deserialization and pre-sorted bulk loads.
    ///
    /// # Errors
    /// Returns the vector unchanged if it is not strictly ascending.
    pub fn try_from_sorted_unique(
        entries: Vec<(K, V)>,
    ) -> Result<Self, Rejected<Vec<(K, V)>, NotSortedUnique>> {
        match first_unsorted(&entries, |a, b| a.0.cmp(&b.0)) {
            Some(index) => Err(Rejected {
                input: entries,
                error: NotSortedUnique { index },
            }),
            None => Ok(Self {
                entries: entries.into_boxed_slice(),
            }),
        }
    }

    /// Strict builder. O(n log n).
    ///
    /// # Errors
    /// [`DuplicateKey`] carrying the second occurrence (in input order) of the
    /// smallest duplicated key.
    pub fn try_from_unique<I>(iter: I) -> Result<Self, DuplicateKey<K, V>>
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut v: Vec<(K, V)> = iter.into_iter().collect();
        v.sort_by(|a, b| a.0.cmp(&b.0)); // stable: input order kept among equals
        match first_unsorted(&v, |a, b| a.0.cmp(&b.0)) {
            Some(i) => {
                let (key, value) = v.swap_remove(i);
                Err(DuplicateKey { key, value })
            }
            None => Ok(Self {
                entries: v.into_boxed_slice(),
            }),
        }
    }

    /// Back to the paired mutable family. O(n) bulk build (input is already
    /// strictly ascending, so no duplicate handling is involved).
    #[must_use]
    pub fn thaw(self) -> MutableOrderedMap<K, V> {
        MutableOrderedMap::from(self.into_btree_map())
    }

    /// Interop: a std `BTreeMap`. O(n) bulk build.
    #[must_use]
    pub fn into_btree_map(self) -> BTreeMap<K, V> {
        self.entries.into_vec().into_iter().collect()
    }

    /// Extract the native map; an alias for [`into_btree_map`](Self::into_btree_map).
    #[must_use]
    pub fn into_map(self) -> BTreeMap<K, V> {
        self.into_btree_map()
    }

    fn search<Q>(&self, key: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.entries.binary_search_by(|(k, _)| k.borrow().cmp(key))
    }

    /// Value for `key`. O(log n).
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.get_key_value(key).map(|(_, v)| v)
    }

    /// Stored key representative and value.
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.search(key).ok().map(|i| {
            let (k, v) = &self.entries[i];
            (k, v)
        })
    }

    /// `true` if present.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.search(key).is_ok()
    }

    /// Ascending rank, `binary_search`-style.
    ///
    /// # Errors
    /// `Err(i)` when absent; `i` is where the key would be inserted.
    pub fn rank<Q>(&self, key: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.search(key)
    }

    /// Entries with keys in `range`, ascending. O(log n) to locate.
    ///
    /// # Panics
    /// If `start > end`, or `start == end` with both bounds excluded (as
    /// `BTreeMap::range`).
    pub fn range<Q, R>(&self, range: R) -> SortedIter<'_, K, V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
    {
        let (s, e) = range_indices(&self.entries, |(k, _)| k.borrow(), &range);
        SortedIter(self.entries[s..e].iter())
    }
}

/// First key representative, last value (stable sort + in-place reduction).
impl<K: Ord, V> FromIterator<(K, V)> for FrozenOrderedMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut v: Vec<(K, V)> = iter.into_iter().collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v.dedup_by(|later, first| {
            // Keep the first key while the retained slot acquires the latest
            // value. Vec handles dropping removed pairs and unwinding safely.
            if first.0.cmp(&later.0) == Ordering::Equal {
                core::mem::swap(&mut first.1, &mut later.1);
                true
            } else {
                false
            }
        });
        Self {
            entries: v.into_boxed_slice(),
        }
    }
}

impl<K, V> From<BTreeMap<K, V>> for FrozenOrderedMap<K, V> {
    fn from(map: BTreeMap<K, V>) -> Self {
        Self::from_btree(map)
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for FrozenOrderedMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, V, Q> Index<&Q> for FrozenOrderedMap<K, V>
where
    K: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("key not present in FrozenOrderedMap")
    }
}

impl<'a, K, V> IntoIterator for &'a FrozenOrderedMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = SortedIter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

wrap_iter! {
    /// Ascending owning entries of [`FrozenOrderedMap`].
    pub struct SortedIntoIter<K, V>(alloc::vec::IntoIter<(K, V)>) -> (K, V); double, exact
}
wrap_iter! {
    /// Ascending owning elements of [`FrozenOrderedSet`].
    pub struct SortedSetIntoIter<T>(alloc::vec::IntoIter<T>) -> T; double, exact
}

impl<K, V> IntoIterator for FrozenOrderedMap<K, V> {
    type Item = (K, V);
    type IntoIter = SortedIntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        SortedIntoIter(self.entries.into_vec().into_iter())
    }
}

/// Ascending borrowed entry iterator for [`FrozenOrderedMap`].
pub struct SortedIter<'a, K, V>(core::slice::Iter<'a, (K, V)>);

// Manual: derives would demand `K: Clone`/`V: Clone`.
impl<K, V> Clone for SortedIter<'_, K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for SortedIter<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a, K, V> Iterator for SortedIter<'a, K, V> {
    type Item = (&'a K, &'a V);
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (k, v))
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<K, V> DoubleEndedIterator for SortedIter<'_, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|(k, v)| (k, v))
    }
}
impl<K, V> ExactSizeIterator for SortedIter<'_, K, V> {}
impl<K, V> FusedIterator for SortedIter<'_, K, V> {}

impl<K, V> Len for FrozenOrderedMap<K, V> {
    fn len(&self) -> usize {
        self.entries.len()
    }
}
impl<K, V> MapTypes for FrozenOrderedMap<K, V> {
    type Key = K;
    type Value = V;
}
impl<K, V, Q> LookupRef<Q> for FrozenOrderedMap<K, V>
where
    K: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> {
        Self::get_key_value(self, query)
    }
}
impl<K, V> Entries for FrozenOrderedMap<K, V> {
    type Iter<'a>
        = SortedIter<'a, K, V>
    where
        Self: 'a;
    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}
impl<K, V> OrderedEntries for FrozenOrderedMap<K, V> {
    fn first_key_value(&self) -> Option<(&K, &V)> {
        self.first_key_value()
    }
    fn last_key_value(&self) -> Option<(&K, &V)> {
        self.last_key_value()
    }
}
impl<K, V, Q> RangeEntries<Q> for FrozenOrderedMap<K, V>
where
    K: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    type Range<'a>
        = SortedIter<'a, K, V>
    where
        Self: 'a;
    fn range_entries<R: RangeBounds<Q>>(&self, range: R) -> Self::Range<'_> {
        self.range(range)
    }
}

// ===========================================================================
// FrozenOrderedSet
// ===========================================================================

/// Frozen ordered set: strictly ascending boxed elements.
///
/// Construction from elements keeps the **first** representative of each
/// equivalence class. Equality, ordering and hashing are over the ascending
/// sequence (canonical, so set equality).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrozenOrderedSet<T> {
    // Invariant: strictly ascending by `T: Ord`.
    pub(crate) items: Box<[T]>,
}

impl<T> Default for FrozenOrderedSet<T> {
    fn default() -> Self {
        Self {
            items: Box::new([]),
        }
    }
}

impl<T> FrozenOrderedSet<T> {
    pub(crate) fn from_btree(set: BTreeSet<T>) -> Self {
        Self {
            items: set.into_iter().collect(),
        }
    }
    /// Element count.
    pub fn len(&self) -> usize {
        self.items.len()
    }
    /// `true` when empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// Ascending traversal.
    pub fn iter(&self) -> core::slice::Iter<'_, T> {
        self.items.iter()
    }
    /// Ascending elements as a slice.
    pub fn as_slice(&self) -> &[T] {
        &self.items
    }
    /// Least element.
    pub fn first(&self) -> Option<&T> {
        self.items.first()
    }
    /// Greatest element.
    pub fn last(&self) -> Option<&T> {
        self.items.last()
    }
}

impl<T: Ord> FrozenOrderedSet<T> {
    /// Borrow every member of either set, in ascending order.
    ///
    /// Equivalent members yield this set's stored representative. The iterator
    /// allocates nothing, clones no elements, and uses O(n + m) comparisons and
    /// O(1) auxiliary storage when fully traversed.
    ///
    /// ```
    /// use frost::family::FrozenOrderedSet;
    /// let left: FrozenOrderedSet<_> = [1, 3, 5].into_iter().collect();
    /// let right: FrozenOrderedSet<_> = [2, 3, 4].into_iter().collect();
    /// assert_eq!(left.union(&right).copied().collect::<Vec<_>>(), [1, 2, 3, 4, 5]);
    /// assert_eq!(left.intersection(&right).copied().collect::<Vec<_>>(), [3]);
    /// assert_eq!(left.difference(&right).copied().collect::<Vec<_>>(), [1, 5]);
    /// assert_eq!(left.symmetric_difference(&right).copied().collect::<Vec<_>>(), [1, 2, 4, 5]);
    /// assert_eq!(left.union_len(&right), 5);
    /// assert!(!left.is_disjoint(&right));
    /// ```
    pub fn union<'a>(&'a self, other: &'a Self) -> super::SetUnion<'a, T> {
        super::SetUnion::new(&self.items, &other.items)
    }

    /// Borrow members present in both sets, in ascending order.
    ///
    /// Each result is this set's stored representative. Traversal allocates
    /// nothing, clones no elements, and uses O(n + m) comparisons and O(1)
    /// auxiliary storage.
    pub fn intersection<'a>(&'a self, other: &'a Self) -> super::SetIntersection<'a, T> {
        super::SetIntersection::new(&self.items, &other.items)
    }

    /// Borrow members present in this set but absent from `other`, ascending.
    ///
    /// Traversal allocates nothing, clones no elements, and uses O(n + m)
    /// comparisons and O(1) auxiliary storage.
    pub fn difference<'a>(&'a self, other: &'a Self) -> super::SetDifference<'a, T> {
        super::SetDifference::new(&self.items, &other.items)
    }

    /// Borrow members present in exactly one set, in ascending order.
    ///
    /// Each result comes from the set containing that member. Traversal
    /// allocates nothing, clones no elements, and uses O(n + m) comparisons and
    /// O(1) auxiliary storage.
    pub fn symmetric_difference<'a>(
        &'a self,
        other: &'a Self,
    ) -> super::SetSymmetricDifference<'a, T> {
        super::SetSymmetricDifference::new(&self.items, &other.items)
    }

    /// Number of members present in both sets, without constructing a result.
    ///
    /// Uses O(n + m) comparisons and O(1) auxiliary storage, with no allocation.
    /// Empty sets and disjoint endpoint ranges return immediately.
    pub fn intersection_len(&self, other: &Self) -> usize {
        super::set_ops::intersection_len(&self.items, &other.items)
    }

    /// Number of members present in either set, without constructing a result.
    ///
    /// Uses O(n + m) comparisons, O(1) auxiliary storage, and no allocation.
    ///
    /// # Panics
    /// If the result's cardinality exceeds `usize::MAX`. Overlap is subtracted
    /// before addition, so an overflowing sum of input lengths alone is not an
    /// error.
    pub fn union_len(&self, other: &Self) -> usize {
        super::set_ops::union_len(&self.items, &other.items)
    }

    /// Number of this set's members absent from `other`, without a result set.
    ///
    /// Uses O(n + m) comparisons, O(1) auxiliary storage, and no allocation.
    pub fn difference_len(&self, other: &Self) -> usize {
        super::set_ops::difference_len(&self.items, &other.items)
    }

    /// Number of members present in exactly one set, without a result set.
    ///
    /// Uses O(n + m) comparisons, O(1) auxiliary storage, and no allocation.
    ///
    /// # Panics
    /// If the result's cardinality exceeds `usize::MAX`. Shared members are
    /// subtracted from both lengths before addition.
    pub fn symmetric_difference_len(&self, other: &Self) -> usize {
        super::set_ops::symmetric_difference_len(&self.items, &other.items)
    }

    /// Whether every member of this set is present in `other`.
    ///
    /// Stops at the first missing member. Uses at most O(n + m) comparisons,
    /// O(1) auxiliary storage, and no allocation.
    pub fn is_subset(&self, other: &Self) -> bool {
        super::set_ops::is_subset(&self.items, &other.items)
    }

    /// Whether every member of `other` is present in this set.
    ///
    /// Equivalent to `other.is_subset(self)`; allocates nothing.
    pub fn is_superset(&self, other: &Self) -> bool {
        other.is_subset(self)
    }

    /// Whether the sets have no member in common.
    ///
    /// Stops at the first common member. Uses at most O(n + m) comparisons,
    /// O(1) auxiliary storage, and no allocation. Empty sets and disjoint
    /// endpoint ranges return immediately.
    pub fn is_disjoint(&self, other: &Self) -> bool {
        super::set_ops::is_disjoint(&self.items, &other.items)
    }

    /// Construct a set while rejecting duplicate elements.
    ///
    /// # Errors
    /// Returns the later representative of the smallest duplicated element.
    /// The complete input is consumed; other elements are dropped.
    pub fn try_from_unique<I>(iter: I) -> Result<Self, crate::family::DuplicateElement<T>>
    where
        I: IntoIterator<Item = T>,
    {
        let mut items: Vec<T> = iter.into_iter().collect();
        items.sort();
        if let Some(index) = first_unsorted(&items, T::cmp) {
            return Err(crate::family::DuplicateElement {
                item: items.swap_remove(index),
            });
        }
        Ok(Self {
            items: items.into_boxed_slice(),
        })
    }

    /// Return the paired mutable ordered set, rebuilding its tree.
    #[must_use]
    pub fn thaw(self) -> crate::family::MutableOrderedSet<T> {
        crate::family::MutableOrderedSet::from_set(self.into_btree_set())
    }

    /// Extract an ordered native set; equivalent to `into_btree_set`.
    #[must_use]
    pub fn into_set(self) -> BTreeSet<T> {
        self.into_btree_set()
    }

    /// Validate and adopt a strictly ascending vector. O(n).
    ///
    /// # Errors
    /// Returns the vector unchanged if it is not strictly ascending.
    pub fn try_from_sorted_unique(
        items: Vec<T>,
    ) -> Result<Self, Rejected<Vec<T>, NotSortedUnique>> {
        match first_unsorted(&items, T::cmp) {
            Some(index) => Err(Rejected {
                input: items,
                error: NotSortedUnique { index },
            }),
            None => Ok(Self {
                items: items.into_boxed_slice(),
            }),
        }
    }

    fn search<Q>(&self, value: &Q) -> Result<usize, usize>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.items.binary_search_by(|x| x.borrow().cmp(value))
    }

    /// Membership. O(log n).
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.search(value).is_ok()
    }

    /// Stored representative.
    pub fn get<Q>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.search(value).ok().map(|i| &self.items[i])
    }

    /// Elements in `range`.
    ///
    /// # Panics
    /// As `BTreeSet::range`.
    pub fn range<Q, R>(&self, range: R) -> core::slice::Iter<'_, T>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
    {
        let (s, e) = range_indices(&self.items, |x| x.borrow(), &range);
        self.items[s..e].iter()
    }

    /// Interop: a std `BTreeSet`. O(n) bulk build.
    #[must_use]
    pub fn into_btree_set(self) -> BTreeSet<T> {
        self.items.into_vec().into_iter().collect()
    }
}

/// First representative wins (stable sort + `dedup`, which keeps the first).
impl<T: Ord> FromIterator<T> for FrozenOrderedSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut v: Vec<T> = iter.into_iter().collect();
        v.sort();
        v.dedup_by(|later, first| Ord::cmp(&*later, &*first) == Ordering::Equal); // keeps the first
        Self {
            items: v.into_boxed_slice(),
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for FrozenOrderedSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.items.iter()).finish()
    }
}

impl<T> From<BTreeSet<T>> for FrozenOrderedSet<T> {
    fn from(set: BTreeSet<T>) -> Self {
        Self::from_btree(set)
    }
}

impl<T> IntoIterator for FrozenOrderedSet<T> {
    type Item = T;
    type IntoIter = SortedSetIntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        SortedSetIntoIter(self.items.into_vec().into_iter())
    }
}

impl<'a, T> IntoIterator for &'a FrozenOrderedSet<T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<T> Len for FrozenOrderedSet<T> {
    fn len(&self) -> usize {
        self.items.len()
    }
}
impl<T> ElementType for FrozenOrderedSet<T> {
    type Item = T;
}

impl<T> Elements for FrozenOrderedSet<T> {
    type Iter<'a>
        = core::slice::Iter<'a, T>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.items.iter()
    }
}
impl<T, Q> SetContains<Q> for FrozenOrderedSet<T>
where
    T: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    fn contains(&self, query: &Q) -> bool {
        Self::contains(self, query)
    }
}
impl<T, Q> SetGet<Q> for FrozenOrderedSet<T>
where
    T: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    fn get(&self, query: &Q) -> Option<&T> {
        Self::get(self, query)
    }
}
impl<T> OrderedElements for FrozenOrderedSet<T> {
    fn first_element(&self) -> Option<&T> {
        self.first()
    }
    fn last_element(&self) -> Option<&T> {
        self.last()
    }
}
impl<T, Q> RangeElements<Q> for FrozenOrderedSet<T>
where
    T: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    type Range<'a>
        = core::slice::Iter<'a, T>
    where
        Self: 'a;
    fn range_elements<R: RangeBounds<Q>>(&self, range: R) -> Self::Range<'_> {
        self.range(range)
    }
}

//! `AdaptiveOrderedMap`: sorted `Vec<(K, V)>` (binary search, contiguous
//! scans) promoted to `BTreeMap` when it outgrows the policy threshold.

use alloc::collections::{BTreeMap, btree_map};
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::iter::FusedIterator;
use core::ops::RangeBounds;

use crate::FreezeWith;
use crate::family::ordered::range_indices;
use crate::family::{FrozenOrderedMap, MutableOrderedMap};
use crate::iter::{Keys, Values};
use crate::read::{Entries, Len, LookupRef, MapTypes, OrderedEntries, RangeEntries};
use crate::write::{MapInsert, MapRemove};

/// Growth policy for [`AdaptiveOrderedMap`].
///
/// `sorted_max` is the largest length kept as a sorted vector (inserts there
/// are O(n) shifts). The default (32) is a placeholder, not a measured
/// crossover.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct AdaptiveOrdered {
    /// Largest length stored as a sorted vector.
    pub sorted_max: usize,
}

impl AdaptiveOrdered {
    /// Policy with an explicit threshold.
    pub const fn sorted_up_to(sorted_max: usize) -> Self {
        Self { sorted_max }
    }
}

impl Default for AdaptiveOrdered {
    fn default() -> Self {
        Self::sorted_up_to(32)
    }
}

/// Selected representation (diagnostics only).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OrderedRepr {
    /// Strictly ascending `Vec<(K, V)>`.
    Sorted,
    /// `BTreeMap<K, V>`.
    Tree,
}

pub(crate) enum Repr<K, V> {
    // Invariant: strictly ascending by `K: Ord`.
    Sorted(Vec<(K, V)>),
    Tree(BTreeMap<K, V>),
}

/// Mutable ordered map that stays a sorted vector while small.
///
/// Promotion rebuilds a tree and can compare keys. If user ordering code
/// panics, the map remains valid, but entries moved into the unfinished tree
/// can be dropped; promotion does not promise transactional rollback.
///
/// ```
/// use frost::prelude::*;
/// use frost::adaptive::OrderedRepr;
/// let mut m = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(2));
/// m.insert(2, 'b');
/// m.insert(1, 'a');
/// assert_eq!(m.representation(), OrderedRepr::Sorted);
/// m.insert(3, 'c');                                    // promotes
/// assert_eq!(m.representation(), OrderedRepr::Tree);
/// assert_eq!(m.range(2..).map(|(k, _)| *k).collect::<Vec<_>>(), [2, 3]);
/// let f: FrozenOrderedMap<i32, char> = m.freeze();
/// assert_eq!(f.first_key_value(), Some((&1, &'a')));
/// ```
pub struct AdaptiveOrderedMap<K, V> {
    pub(crate) repr: Repr<K, V>,
    pub(crate) policy: AdaptiveOrdered,
}

impl<K, V> Default for AdaptiveOrderedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> AdaptiveOrderedMap<K, V> {
    /// Empty, default policy; allocates nothing.
    pub const fn new() -> Self {
        Self::with_policy(AdaptiveOrdered::sorted_up_to(32))
    }
    /// Empty with `policy`.
    pub const fn with_policy(policy: AdaptiveOrdered) -> Self {
        Self {
            repr: Repr::Sorted(Vec::new()),
            policy,
        }
    }
    /// Growth policy.
    pub fn policy(&self) -> AdaptiveOrdered {
        self.policy
    }
    /// Selected representation.
    pub fn representation(&self) -> OrderedRepr {
        match self.repr {
            Repr::Sorted(_) => OrderedRepr::Sorted,
            Repr::Tree(_) => OrderedRepr::Tree,
        }
    }
    /// Entry count.
    pub fn len(&self) -> usize {
        match &self.repr {
            Repr::Sorted(v) => v.len(),
            Repr::Tree(t) => t.len(),
        }
    }
    /// `true` when empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Remove all entries; keeps the current representation.
    pub fn clear(&mut self) {
        match &mut self.repr {
            Repr::Sorted(v) => v.clear(),
            Repr::Tree(t) => t.clear(),
        }
    }
    /// Ascending entries.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter(match &self.repr {
            Repr::Sorted(v) => IterRepr::Sorted(v.iter()),
            Repr::Tree(t) => IterRepr::Tree(t.iter()),
        })
    }
    /// Ascending entries with mutable values.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut(match &mut self.repr {
            Repr::Sorted(v) => IterMutRepr::Sorted(v.iter_mut()),
            Repr::Tree(t) => IterMutRepr::Tree(t.iter_mut()),
        })
    }
    /// Ascending keys.
    pub fn keys(&self) -> Keys<Iter<'_, K, V>> {
        Keys(self.iter())
    }
    /// Values in key order.
    pub fn values(&self) -> Values<Iter<'_, K, V>> {
        Values(self.iter())
    }
    /// Mutable values in key order.
    pub fn values_mut(&mut self) -> Values<IterMut<'_, K, V>> {
        Values(self.iter_mut())
    }
    /// Least entry.
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        self.iter().next()
    }
    /// Greatest entry.
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        self.iter().next_back()
    }

    /// Freeze into [`FrozenOrderedMap`]. Sorted: no reallocation when
    /// `len == capacity`; otherwise the allocation may shrink or move.
    /// Tree: O(n) flatten with no key comparisons.
    #[must_use]
    pub fn freeze(self) -> FrozenOrderedMap<K, V> {
        match self.repr {
            Repr::Sorted(v) => FrozenOrderedMap::from_sorted_unique_unchecked(v),
            Repr::Tree(t) => FrozenOrderedMap::from(t),
        }
    }
}

impl<K: Ord, V> AdaptiveOrderedMap<K, V> {
    /// Keep entries for which `f` returns `true` (order preserved).
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut f: F) {
        match &mut self.repr {
            Repr::Sorted(v) => v.retain_mut(|(k, val)| f(k, val)),
            Repr::Tree(t) => t.retain(f),
        }
    }

    fn promote(&mut self) -> &mut BTreeMap<K, V> {
        if let Repr::Sorted(v) = &mut self.repr {
            // Bulk build from strictly ascending input: no duplicate handling.
            let t: BTreeMap<K, V> = core::mem::take(v).into_iter().collect();
            self.repr = Repr::Tree(t);
        }
        match &mut self.repr {
            Repr::Tree(t) => t,
            Repr::Sorted(_) => unreachable!("promoted above"),
        }
    }

    fn search<Q: Ord + ?Sized>(v: &[(K, V)], key: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
    {
        v.binary_search_by(|(k, _)| k.borrow().cmp(key))
    }

    /// Insert; an existing key keeps its stored representative. Sorted
    /// variant: O(n) shift; promotes when a new key would exceed the policy.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let max = self.policy.sorted_max;
        match &mut self.repr {
            Repr::Sorted(v) => match Self::search(v, &key) {
                Ok(i) => Some(core::mem::replace(&mut v[i].1, value)),
                Err(i) if v.len() < max => {
                    v.insert(i, (key, value));
                    None
                }
                Err(_) => self.promote().insert(key, value),
            },
            Repr::Tree(t) => t.insert(key, value),
        }
    }

    /// Stored key and value.
    pub fn get_key_value<Q: Ord + ?Sized>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
    {
        match &self.repr {
            Repr::Sorted(v) => Self::search(v, key).ok().map(|i| {
                let (k, val) = &v[i];
                (k, val)
            }),
            Repr::Tree(t) => t.get_key_value(key),
        }
    }
    /// Value for `key`.
    pub fn get<Q: Ord + ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.get_key_value(key).map(|(_, v)| v)
    }
    /// Mutable value for `key`.
    pub fn get_mut<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        match &mut self.repr {
            Repr::Sorted(v) => match Self::search(v, key) {
                Ok(i) => Some(&mut v[i].1),
                Err(_) => None,
            },
            Repr::Tree(t) => t.get_mut(key),
        }
    }
    /// `true` if present.
    pub fn contains_key<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.get_key_value(key).is_some()
    }
    /// Remove, returning the stored key and value (order preserved).
    pub fn remove_entry<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
    {
        match &mut self.repr {
            Repr::Sorted(v) => Self::search(v, key).ok().map(|i| v.remove(i)),
            Repr::Tree(t) => t.remove_entry(key),
        }
    }
    /// Remove, returning the value.
    pub fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        self.remove_entry(key).map(|(_, v)| v)
    }
    /// Remove the least entry.
    ///
    /// In the sorted representation this shifts the remaining entries, taking
    /// O(n) time. A complete ascending drain is linear with consuming
    /// [`IntoIterator::into_iter`]; repeated `pop_first` calls can be quadratic
    /// when the policy deliberately retains a large sorted representation.
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        match &mut self.repr {
            Repr::Sorted(v) => (!v.is_empty()).then(|| v.remove(0)),
            Repr::Tree(t) => t.pop_first(),
        }
    }
    /// Remove the greatest entry.
    ///
    /// The sorted representation removes its last vector entry in O(1) time.
    pub fn pop_last(&mut self) -> Option<(K, V)> {
        match &mut self.repr {
            Repr::Sorted(v) => v.pop(),
            Repr::Tree(t) => t.pop_last(),
        }
    }

    /// Entries with keys in `range`, ascending.
    ///
    /// # Panics
    /// Exactly when `BTreeMap::range` panics (never on an empty map).
    pub fn range<Q, R>(&self, range: R) -> Range<'_, K, V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
    {
        Range(match &self.repr {
            Repr::Sorted(v) => {
                let (s, e) = range_indices(v, |(k, _)| k.borrow(), &range);
                IterRepr::Sorted(v[s..e].iter())
            }
            Repr::Tree(t) => IterRepr::TreeRange(t.range(range)),
        })
    }

    /// Value for `key`, inserting `f()` if absent (may promote).
    pub fn get_or_insert_with<F: FnOnce() -> V>(&mut self, key: K, f: F) -> &mut V {
        let max = self.policy.sorted_max;
        // Reuse the binary search, including its insertion position, when
        // the sorted representation is retained. Promotion discards this
        // position and uses the tree's entry API.
        let position = match &self.repr {
            Repr::Sorted(v) => Self::search(v, &key),
            Repr::Tree(_) => Err(0),
        };
        if let Repr::Sorted(v) = &self.repr
            && position.is_err()
            && v.len() >= max
        {
            self.promote();
        }
        match &mut self.repr {
            Repr::Sorted(v) => {
                let i = match position {
                    Ok(i) => i,
                    Err(i) => {
                        v.insert(i, (key, f()));
                        i
                    }
                };
                &mut v[i].1
            }
            Repr::Tree(t) => t.entry(key).or_insert_with(f),
        }
    }

    /// Entries in `range` with mutable values.
    ///
    /// # Panics
    /// Exactly when `BTreeMap::range_mut` panics (never on an empty map).
    pub fn range_mut<Q, R>(&mut self, range: R) -> RangeMut<'_, K, V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
    {
        RangeMut(match &mut self.repr {
            Repr::Sorted(v) => {
                let (s, e) = range_indices(v, |(k, _)| k.borrow(), &range);
                RangeMutRepr::Sorted(v[s..e].iter_mut())
            }
            Repr::Tree(t) => RangeMutRepr::Tree(t.range_mut(range)),
        })
    }

    /// Release excess capacity; a tree with `len <= sorted_max` is demoted to
    /// a sorted vector (the only demotion path).
    pub fn shrink_to_fit(&mut self) {
        match &mut self.repr {
            Repr::Sorted(v) => v.shrink_to_fit(),
            Repr::Tree(t) if t.len() <= self.policy.sorted_max => {
                let v: Vec<(K, V)> = core::mem::take(t).into_iter().collect();
                self.repr = Repr::Sorted(v);
            }
            Repr::Tree(_) => {}
        }
    }

    /// Thaw a frozen ordered map into this profile under `policy`.
    pub fn from_frozen(frozen: FrozenOrderedMap<K, V>, policy: AdaptiveOrdered) -> Self {
        let v = frozen.into_sorted_vec();
        let repr = if v.len() <= policy.sorted_max {
            Repr::Sorted(v)
        } else {
            Repr::Tree(v.into_iter().collect())
        };
        Self { repr, policy }
    }
}

impl<K, V> FreezeWith<crate::Compact> for AdaptiveOrderedMap<K, V> {
    type Output = FrozenOrderedMap<K, V>;
    fn freeze_with(self, _: crate::Compact) -> FrozenOrderedMap<K, V> {
        self.freeze()
    }
}

impl<K: Ord, V, const N: usize> From<[(K, V); N]> for AdaptiveOrderedMap<K, V> {
    fn from(a: [(K, V); N]) -> Self {
        a.into_iter().collect()
    }
}

impl<K: Borrow<Q> + Ord, V, Q: Ord + ?Sized> core::ops::Index<&Q> for AdaptiveOrderedMap<K, V> {
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.get(key)
            .expect("key not present in AdaptiveOrderedMap")
    }
}

impl<K, V> From<MutableOrderedMap<K, V>> for AdaptiveOrderedMap<K, V> {
    /// Keeps the tree; [`shrink_to_fit`](Self::shrink_to_fit) may demote it.
    fn from(m: MutableOrderedMap<K, V>) -> Self {
        Self {
            repr: Repr::Tree(m.into()),
            policy: AdaptiveOrdered::default(),
        }
    }
}

/// Family policy (first key, last value), via the frozen builder, then a
/// representation chosen by length under the default policy.
impl<K: Ord, V> FromIterator<(K, V)> for AdaptiveOrderedMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::from_frozen(iter.into_iter().collect(), AdaptiveOrdered::default())
    }
}
impl<K: Ord, V> Extend<(K, V)> for AdaptiveOrderedMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<K: Clone, V: Clone> Clone for AdaptiveOrderedMap<K, V> {
    fn clone(&self) -> Self {
        Self {
            repr: match &self.repr {
                Repr::Sorted(v) => Repr::Sorted(v.clone()),
                Repr::Tree(t) => Repr::Tree(t.clone()),
            },
            policy: self.policy,
        }
    }
}

/// Logical: both variants traverse ascending, so sequence equality is map
/// equality. The policy is not compared.
impl<K: PartialEq, V: PartialEq> PartialEq for AdaptiveOrderedMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}
impl<K: Eq, V: Eq> Eq for AdaptiveOrderedMap<K, V> {}
impl<K: PartialOrd, V: PartialOrd> PartialOrd for AdaptiveOrderedMap<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}
impl<K: Ord, V: Ord> Ord for AdaptiveOrderedMap<K, V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other.iter())
    }
}
/// Length prefix + ascending entries: identical for both variants.
impl<K: Hash, V: Hash> Hash for AdaptiveOrderedMap<K, V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.len());
        for (k, v) in self {
            k.hash(state);
            v.hash(state);
        }
    }
}
impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for AdaptiveOrderedMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

// --- iterators -------------------------------------------------------------

enum IterRepr<'a, K, V> {
    Sorted(core::slice::Iter<'a, (K, V)>),
    Tree(btree_map::Iter<'a, K, V>),
    TreeRange(btree_map::Range<'a, K, V>),
}

macro_rules! dispatch {
    ($self:expr, $it:ident => $e:expr, $map:expr) => {
        match $self {
            IterRepr::Sorted($it) => $map($e),
            IterRepr::Tree($it) => $e,
            IterRepr::TreeRange($it) => $e,
        }
    };
}

fn pair<K, V>((k, v): &(K, V)) -> (&K, &V) {
    (k, v)
}

macro_rules! ordered_iter {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        pub struct $name<'a, K, V>(IterRepr<'a, K, V>);

        impl<'a, K, V> Iterator for $name<'a, K, V> {
            type Item = (&'a K, &'a V);
            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                dispatch!(&mut self.0, it => it.next(), |o: Option<&'a (K, V)>| o.map(pair))
            }
            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                dispatch!(&self.0, it => it.size_hint(), |s| s)
            }
            fn fold<B, F: FnMut(B, Self::Item) -> B>(self, init: B, mut f: F) -> B {
                match self.0 {
                    IterRepr::Sorted(it) => it.fold(init, |acc, e| f(acc, pair(e))),
                    IterRepr::Tree(it) => it.fold(init, f),
                    IterRepr::TreeRange(it) => it.fold(init, f),
                }
            }
        }
        impl<'a, K, V> DoubleEndedIterator for $name<'a, K, V> {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                dispatch!(&mut self.0, it => it.next_back(), |o: Option<&'a (K, V)>| o.map(pair))
            }
        }
        impl<K, V> FusedIterator for $name<'_, K, V> {}
        impl<K, V> Clone for $name<'_, K, V> {
            fn clone(&self) -> Self {
                Self(match &self.0 {
                    IterRepr::Sorted(it) => IterRepr::Sorted(it.clone()),
                    IterRepr::Tree(it) => IterRepr::Tree(it.clone()),
                    IterRepr::TreeRange(it) => IterRepr::TreeRange(it.clone()),
                })
            }
        }
        impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for $name<'_, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.clone()).finish()
            }
        }
    };
}

ordered_iter! {
    /// Ascending entries of [`AdaptiveOrderedMap`].
    Iter
}
ordered_iter! {
    /// Ascending entries within a range of [`AdaptiveOrderedMap`].
    Range
}

/// Exact for whole-map traversal (`btree_map::Range` is not `ExactSizeIterator`).
impl<K, V> ExactSizeIterator for Iter<'_, K, V> {}

enum IterMutRepr<'a, K, V> {
    Sorted(core::slice::IterMut<'a, (K, V)>),
    Tree(btree_map::IterMut<'a, K, V>),
}

/// Ascending entries with mutable values of [`AdaptiveOrderedMap`].
pub struct IterMut<'a, K, V>(IterMutRepr<'a, K, V>);

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            IterMutRepr::Sorted(it) => it.next().map(|(k, v)| (&*k, v)),
            IterMutRepr::Tree(it) => it.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            IterMutRepr::Sorted(it) => it.size_hint(),
            IterMutRepr::Tree(it) => it.size_hint(),
        }
    }
}
impl<K, V> DoubleEndedIterator for IterMut<'_, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            IterMutRepr::Sorted(it) => it.next_back().map(|(k, v)| (&*k, v)),
            IterMutRepr::Tree(it) => it.next_back(),
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

impl<'a, K, V> IntoIterator for &'a AdaptiveOrderedMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}
impl<'a, K, V> IntoIterator for &'a mut AdaptiveOrderedMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}
enum IntoIterRepr<K, V> {
    Sorted(alloc::vec::IntoIter<(K, V)>),
    Tree(btree_map::IntoIter<K, V>),
}

/// Ascending owning entries of [`AdaptiveOrderedMap`]; no extra allocation
/// for either variant.
pub struct IntoIter<K, V>(IntoIterRepr<K, V>);

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<(K, V)> {
        match &mut self.0 {
            IntoIterRepr::Sorted(it) => it.next(),
            IntoIterRepr::Tree(it) => it.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            IntoIterRepr::Sorted(it) => it.size_hint(),
            IntoIterRepr::Tree(it) => it.size_hint(),
        }
    }
    fn fold<B, F: FnMut(B, (K, V)) -> B>(self, init: B, f: F) -> B {
        match self.0 {
            IntoIterRepr::Sorted(it) => it.fold(init, f),
            IntoIterRepr::Tree(it) => it.fold(init, f),
        }
    }
}
impl<K, V> DoubleEndedIterator for IntoIter<K, V> {
    fn next_back(&mut self) -> Option<(K, V)> {
        match &mut self.0 {
            IntoIterRepr::Sorted(it) => it.next_back(),
            IntoIterRepr::Tree(it) => it.next_back(),
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

impl<K, V> IntoIterator for AdaptiveOrderedMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter(match self.repr {
            Repr::Sorted(v) => IntoIterRepr::Sorted(v.into_iter()),
            Repr::Tree(t) => IntoIterRepr::Tree(t.into_iter()),
        })
    }
}

enum RangeMutRepr<'a, K, V> {
    Sorted(core::slice::IterMut<'a, (K, V)>),
    Tree(btree_map::RangeMut<'a, K, V>),
}

/// Entries in a key range with mutable values, of [`AdaptiveOrderedMap`].
pub struct RangeMut<'a, K, V>(RangeMutRepr<'a, K, V>);

impl<'a, K, V> Iterator for RangeMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            RangeMutRepr::Sorted(it) => it.next().map(|(k, v)| (&*k, v)),
            RangeMutRepr::Tree(it) => it.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            RangeMutRepr::Sorted(it) => it.size_hint(),
            RangeMutRepr::Tree(it) => it.size_hint(),
        }
    }
}
impl<K, V> DoubleEndedIterator for RangeMut<'_, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            RangeMutRepr::Sorted(it) => it.next_back().map(|(k, v)| (&*k, v)),
            RangeMutRepr::Tree(it) => it.next_back(),
        }
    }
}
impl<K, V> FusedIterator for RangeMut<'_, K, V> {}
impl<K, V> fmt::Debug for RangeMut<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RangeMut").finish_non_exhaustive()
    }
}

// --- capabilities ----------------------------------------------------------

impl<K, V> Len for AdaptiveOrderedMap<K, V> {
    fn len(&self) -> usize {
        Self::len(self)
    }
}
impl<K, V> MapTypes for AdaptiveOrderedMap<K, V> {
    type Key = K;
    type Value = V;
}
impl<K: Borrow<Q> + Ord, V, Q: Ord + ?Sized> LookupRef<Q> for AdaptiveOrderedMap<K, V> {
    fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> {
        Self::get_key_value(self, query)
    }
}
impl<K, V> Entries for AdaptiveOrderedMap<K, V> {
    type Iter<'a>
        = Iter<'a, K, V>
    where
        Self: 'a;
    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}
impl<K, V> OrderedEntries for AdaptiveOrderedMap<K, V> {
    fn first_key_value(&self) -> Option<(&K, &V)> {
        Self::first_key_value(self)
    }
    fn last_key_value(&self) -> Option<(&K, &V)> {
        Self::last_key_value(self)
    }
}
impl<K: Borrow<Q> + Ord, V, Q: Ord + ?Sized> RangeEntries<Q> for AdaptiveOrderedMap<K, V> {
    type Range<'a>
        = Range<'a, K, V>
    where
        Self: 'a;
    fn range_entries<R: RangeBounds<Q>>(&self, range: R) -> Self::Range<'_> {
        self.range(range)
    }
}
impl<K: Ord, V> MapInsert for AdaptiveOrderedMap<K, V> {
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        Self::insert(self, key, value)
    }
}
impl<K: Borrow<Q> + Ord, V, Q: Ord + ?Sized> MapRemove<Q> for AdaptiveOrderedMap<K, V> {
    fn remove_entry(&mut self, query: &Q) -> Option<(K, V)> {
        Self::remove_entry(self, query)
    }
}

//! Paired hash-set families with a preserved hashing policy.
//!
//! Frozen set algebra borrows its operands and does not allocate or clone
//! elements or hashers. Union and intersection retain the left operand's
//! representative for equivalent members. Membership is commutative for those
//! operations; the selected representatives need not be.
//!
//! Traversing a standard hash table currently costs O(capacity), including
//! empty buckets, rather than O(len). Intersection and disjointness scan the
//! operand with fewer members to reduce membership probes. Hash lookup costs
//! also depend on the hashing policy and collision behavior.

#[path = "hash_set_ops.rs"]
mod ops;

pub use ops::{SetDifference, SetIntersection, SetSymmetricDifference, SetUnion};

use core::borrow::Borrow;
use core::fmt;
use core::hash::{BuildHasher, Hash};
use std::collections::HashSet;
use std::hash::RandomState;

use super::DuplicateElement;
pub use crate::family::sets::{HashSetIntoIter, HashSetIter, MutableHashSet};
use crate::read::{ElementType, Elements, Len, SetContains, SetGet};

/// Frozen hash set with an explicit, preserved hashing policy.
///
/// Construction keeps the first representative of each equivalence class.
/// Iteration order is unspecified, and equality compares membership. The
/// private representation is a shrunk `HashSet`; shrinking is a best effort,
/// not a promise that bucket capacity equals element count.
///
/// ```
/// use frost::family::FrozenHashSet;
///
/// let allowed: FrozenHashSet<String> = ["read", "write", "read"]
///     .into_iter().map(String::from).collect();
/// assert_eq!(allowed.len(), 2);
/// assert!(allowed.contains("read"));
/// assert_eq!(allowed.get("write").map(String::as_str), Some("write"));
/// let mut mutable = allowed.thaw();
/// mutable.insert(String::from("execute"));
/// assert_eq!(mutable.len(), 3);
/// ```
///
/// ```compile_fail,E0599
/// use frost::family::FrozenHashSet;
/// let mut allowed: FrozenHashSet<u32> = [1, 2].into_iter().collect();
/// allowed.insert(3);
/// ```
#[derive(Clone)]
pub struct FrozenHashSet<T, S = RandomState> {
    pub(crate) set: HashSet<T, S>,
}

impl<T, S: Default> Default for FrozenHashSet<T, S> {
    fn default() -> Self {
        Self {
            set: HashSet::default(),
        }
    }
}

impl<T, S> FrozenHashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    pub(crate) fn from_set(mut set: HashSet<T, S>) -> Self {
        set.shrink_to_fit();
        Self { set }
    }

    /// Build with an explicit hasher, preserving the first representative.
    pub fn from_iter_with_hasher<I>(iter: I, hasher: S) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let iter = iter.into_iter();
        let mut set = HashSet::with_capacity_and_hasher(iter.size_hint().0, hasher);
        for item in iter {
            set.insert(item);
        }
        Self::from_set(set)
    }

    /// Strict construction using `S::default()`.
    ///
    /// # Errors
    /// Returns the first duplicate in input order. Other consumed elements
    /// are dropped; the error does not return the complete input.
    pub fn try_from_unique<I>(iter: I) -> Result<Self, DuplicateElement<T>>
    where
        I: IntoIterator<Item = T>,
        S: Default,
    {
        Self::try_from_unique_with_hasher(iter, S::default())
    }

    /// Strict construction using an explicit hasher.
    ///
    /// # Errors
    /// Returns the first duplicate in input order. Other consumed elements
    /// are dropped; the error does not return the complete input.
    pub fn try_from_unique_with_hasher<I>(iter: I, hasher: S) -> Result<Self, DuplicateElement<T>>
    where
        I: IntoIterator<Item = T>,
    {
        let iter = iter.into_iter();
        let mut set = HashSet::with_capacity_and_hasher(iter.size_hint().0, hasher);
        for item in iter {
            // A separate membership check preserves ownership of the
            // rejected representative for the caller.
            if set.contains(&item) {
                return Err(DuplicateElement { item });
            }
            set.insert(item);
        }
        Ok(Self::from_set(set))
    }

    /// Whether an equivalent element is present.
    pub fn contains<Q>(&self, query: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.set.contains(query)
    }

    /// The stored representative equivalent to `query`.
    pub fn get<Q>(&self, query: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.set.get(query)
    }

    /// Whether every element of this set occurs in `other`.
    pub fn is_subset<S2: BuildHasher>(&self, other: &FrozenHashSet<T, S2>) -> bool {
        self.len() <= other.len() && self.iter().all(|item| other.contains(item))
    }

    /// Whether every element of `other` occurs in this set.
    pub fn is_superset<S2: BuildHasher>(&self, other: &FrozenHashSet<T, S2>) -> bool {
        other.is_subset(self)
    }

    /// Whether the sets have no element in common.
    pub fn is_disjoint<S2: BuildHasher>(&self, other: &FrozenHashSet<T, S2>) -> bool {
        if self.len() <= other.len() {
            self.iter().all(|item| !other.contains(item))
        } else {
            other.iter().all(|item| !self.contains(item))
        }
    }
}

impl<T: Eq + Hash, S> FrozenHashSet<T, S> {
    /// Borrow each member present in either set, without allocation or cloning.
    ///
    /// Equivalent members are yielded once, using this set's representative.
    /// Traversal order is unspecified. Different hashing policies are supported.
    pub fn union<'a, S2>(&'a self, other: &'a FrozenHashSet<T, S2>) -> SetUnion<'a, T, S>
    where
        S: BuildHasher,
    {
        SetUnion::new(self, other)
    }

    /// Borrow common members, always using this set's stored representatives.
    ///
    /// Scans the operand with fewer members, even when it is `other`; matches
    /// then borrow the representative stored in `self`. Traversal order is
    /// unspecified. Does not allocate or clone elements or hashers.
    pub fn intersection<'a, S2: BuildHasher>(
        &'a self,
        other: &'a FrozenHashSet<T, S2>,
    ) -> SetIntersection<'a, T, S, S2>
    where
        S: BuildHasher,
    {
        SetIntersection::new(self, other)
    }

    /// Borrow members of this set that are absent from `other`.
    ///
    /// Traversal order is unspecified. Does not allocate or clone elements or
    /// hashers; the operands may use different hashing policies.
    pub fn difference<'a, S2: BuildHasher>(
        &'a self,
        other: &'a FrozenHashSet<T, S2>,
    ) -> SetDifference<'a, T, S2> {
        SetDifference::new(self, other)
    }

    /// Borrow members present in exactly one of the two sets.
    ///
    /// Each representative comes from the sole operand containing its member.
    /// Traversal order is unspecified. Does not allocate or clone elements or
    /// hashers; the operands may use different hashing policies.
    pub fn symmetric_difference<'a, S2: BuildHasher>(
        &'a self,
        other: &'a FrozenHashSet<T, S2>,
    ) -> SetSymmetricDifference<'a, T, S, S2>
    where
        S: BuildHasher,
    {
        SetSymmetricDifference::new(self, other)
    }

    /// Count common members without constructing a result set.
    ///
    /// Scans the operand with fewer members. Does not allocate or clone.
    pub fn intersection_len<S2: BuildHasher>(&self, other: &FrozenHashSet<T, S2>) -> usize
    where
        S: BuildHasher,
    {
        self.intersection(other).count()
    }

    /// Count members of this set that are absent from `other`, without allocation.
    ///
    /// Counts the intersection by scanning the operand with fewer members.
    pub fn difference_len<S2: BuildHasher>(&self, other: &FrozenHashSet<T, S2>) -> usize
    where
        S: BuildHasher,
    {
        self.len() - self.intersection_len(other)
    }

    /// Count members present in either set without allocating a result set.
    ///
    /// Counts the intersection by scanning the operand with fewer members.
    ///
    /// # Panics
    /// Panics if the result's cardinality exceeds `usize::MAX`.
    pub fn union_len<S2: BuildHasher>(&self, other: &FrozenHashSet<T, S2>) -> usize
    where
        S: BuildHasher,
    {
        let common = self.intersection_len(other);
        (self.len() - common)
            .checked_add(other.len())
            .expect("set union cardinality exceeds usize::MAX")
    }

    /// Count members present in exactly one operand without allocating a result.
    ///
    /// Counts the intersection by scanning the operand with fewer members.
    ///
    /// # Panics
    /// Panics if the result's cardinality exceeds `usize::MAX`.
    pub fn symmetric_difference_len<S2: BuildHasher>(&self, other: &FrozenHashSet<T, S2>) -> usize
    where
        S: BuildHasher,
    {
        let common = self.intersection_len(other);
        (self.len() - common)
            .checked_add(other.len() - common)
            .expect("set symmetric difference cardinality exceeds usize::MAX")
    }
}

impl<T, S> FrozenHashSet<T, S> {
    /// Number of distinct elements.
    pub fn len(&self) -> usize {
        self.set.len()
    }

    /// Whether no elements are stored.
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    /// Elements in unspecified order.
    pub fn iter(&self) -> HashSetIter<'_, T> {
        HashSetIter::new(self.set.iter())
    }

    /// The preserved hashing policy.
    pub fn hasher(&self) -> &S {
        self.set.hasher()
    }

    /// Return the paired mutable family, retaining its table and hasher.
    #[must_use]
    pub fn thaw(self) -> MutableHashSet<T, S> {
        MutableHashSet::from_set(self.set)
    }

    /// Extract the native set without rebuilding or changing its hasher.
    #[must_use]
    pub fn into_set(self) -> HashSet<T, S> {
        self.set
    }

    /// Extract the native hash set, retaining the stored hashing policy.
    #[must_use]
    pub fn into_hash_set(self) -> HashSet<T, S> {
        self.set
    }
}

impl<T: Eq + Hash, S: BuildHasher> From<HashSet<T, S>> for FrozenHashSet<T, S> {
    fn from(set: HashSet<T, S>) -> Self {
        Self::from_set(set)
    }
}

impl<T: Eq + Hash, S: BuildHasher + Default> FromIterator<T> for FrozenHashSet<T, S> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_iter_with_hasher(iter, S::default())
    }
}

impl<T, S1, S2> PartialEq<FrozenHashSet<T, S2>> for FrozenHashSet<T, S1>
where
    T: Eq + Hash,
    S1: BuildHasher,
    S2: BuildHasher,
{
    fn eq(&self, other: &FrozenHashSet<T, S2>) -> bool {
        self.len() == other.len() && self.is_subset(other)
    }
}

impl<T: Eq + Hash, S: BuildHasher> Eq for FrozenHashSet<T, S> {}

impl<T: fmt::Debug, S> fmt::Debug for FrozenHashSet<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T, S> IntoIterator for FrozenHashSet<T, S> {
    type Item = T;
    type IntoIter = HashSetIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        HashSetIntoIter::new(self.set.into_iter())
    }
}

impl<'a, T, S> IntoIterator for &'a FrozenHashSet<T, S> {
    type Item = &'a T;
    type IntoIter = HashSetIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T, S> Len for FrozenHashSet<T, S> {
    fn len(&self) -> usize {
        self.len()
    }
}

impl<T, S> ElementType for FrozenHashSet<T, S> {
    type Item = T;
}

impl<T, S> Elements for FrozenHashSet<T, S> {
    type Iter<'a>
        = HashSetIter<'a, T>
    where
        Self: 'a;

    fn elements(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

impl<T, S, Q> SetContains<Q> for FrozenHashSet<T, S>
where
    T: Borrow<Q> + Eq + Hash,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    fn contains(&self, query: &Q) -> bool {
        self.contains(query)
    }
}

impl<T, S, Q> SetGet<Q> for FrozenHashSet<T, S>
where
    T: Borrow<Q> + Eq + Hash,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    fn get(&self, query: &Q) -> Option<&T> {
        self.get(query)
    }
}

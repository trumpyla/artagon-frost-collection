//! Mutable set families with explicit frozen partners.
use alloc::collections::{BTreeSet, btree_set};
use core::borrow::Borrow;

use crate::family::FrozenOrderedSet;
use crate::read::{
    ElementType, Elements, Len, OrderedElements, RangeElements, SetContains, SetGet,
};
use crate::write::{SetInsert, SetRemove};
use crate::{Compact, FreezeExt, FreezeWith};

macro_rules! borrowed_iter {
    ($name:ident, $inner:ty, $doc:literal) => {
        #[doc = $doc]
        pub struct $name<'a, T> {
            inner: $inner,
        }
        impl<'a, T> $name<'a, T> {
            pub(crate) fn new(inner: $inner) -> Self {
                Self { inner }
            }
        }
        impl<T> Clone for $name<'_, T> {
            fn clone(&self) -> Self {
                Self {
                    inner: self.inner.clone(),
                }
            }
        }
        impl<T: core::fmt::Debug> core::fmt::Debug for $name<'_, T> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.inner.fmt(f)
            }
        }
        impl<'a, T> Iterator for $name<'a, T> {
            type Item = &'a T;
            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next()
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }
            #[inline]
            fn fold<B, F: FnMut(B, Self::Item) -> B>(self, init: B, f: F) -> B {
                self.inner.fold(init, f)
            }
            #[inline]
            fn count(self) -> usize {
                self.inner.count()
            }
        }
        impl<T> core::iter::FusedIterator for $name<'_, T> {}
    };
}
macro_rules! owned_iter {
    ($name:ident, $inner:ty, $doc:literal) => {
        #[doc = $doc]
        pub struct $name<T> {
            inner: $inner,
        }
        impl<T> $name<T> {
            pub(crate) fn new(inner: $inner) -> Self {
                Self { inner }
            }
        }
        impl<T: core::fmt::Debug> core::fmt::Debug for $name<T> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.inner.fmt(f)
            }
        }
        impl<T> Iterator for $name<T> {
            type Item = T;
            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next()
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }
            #[inline]
            fn fold<B, F: FnMut(B, Self::Item) -> B>(self, init: B, f: F) -> B {
                self.inner.fold(init, f)
            }
            #[inline]
            fn count(self) -> usize {
                self.inner.count()
            }
        }
        impl<T> ExactSizeIterator for $name<T> {}
        impl<T> core::iter::FusedIterator for $name<T> {}
    };
}
borrowed_iter!(
    OrderedSetIter,
    btree_set::Iter<'a, T>,
    "Borrowed ascending traversal with a backend-independent type."
);
borrowed_iter!(
    OrderedSetRange,
    btree_set::Range<'a, T>,
    "Borrowed ascending range traversal with a backend-independent type."
);
owned_iter!(
    OrderedSetIntoIter,
    btree_set::IntoIter<T>,
    "Consuming ascending traversal with a backend-independent type."
);
impl<T> ExactSizeIterator for OrderedSetIter<'_, T> {}
impl<T> DoubleEndedIterator for OrderedSetIter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
    fn rfold<B, F: FnMut(B, Self::Item) -> B>(self, init: B, f: F) -> B {
        self.inner.rfold(init, f)
    }
}
impl<T> DoubleEndedIterator for OrderedSetRange<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
    fn rfold<B, F: FnMut(B, Self::Item) -> B>(self, init: B, f: F) -> B {
        self.inner.rfold(init, f)
    }
}
impl<T> DoubleEndedIterator for OrderedSetIntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
    fn rfold<B, F: FnMut(B, Self::Item) -> B>(self, init: B, f: F) -> B {
        self.inner.rfold(init, f)
    }
}

/// A set with ascending iteration and a private ordered representation.
///
/// Equivalent insertions keep the first stored representative. `freeze`
/// consumes this builder and creates its sole frozen partner.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MutableOrderedSet<T> {
    pub(crate) inner: BTreeSet<T>,
}

impl<T> MutableOrderedSet<T> {
    /// Create an empty set without allocation.
    pub const fn new() -> Self {
        Self {
            inner: BTreeSet::new(),
        }
    }
    /// Adopt a native set, preserving its stored representatives.
    pub fn from_set(inner: BTreeSet<T>) -> Self {
        Self { inner }
    }
    /// Consume the family and return its native mutable representation.
    pub fn into_set(self) -> BTreeSet<T> {
        self.inner
    }
    /// Extract the native tree without rebuilding it.
    #[must_use]
    pub fn into_btree_set(self) -> BTreeSet<T> {
        self.inner
    }
    /// Number of members.
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    /// Whether there are no members.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    /// Remove every member.
    pub fn clear(&mut self) {
        self.inner.clear();
    }
    /// Iterate members in ascending order.
    pub fn iter(&self) -> OrderedSetIter<'_, T> {
        OrderedSetIter::new(self.inner.iter())
    }

    /// Consume the already ordered builder into a compact frozen set; O(n).
    pub fn freeze(self) -> FrozenOrderedSet<T> {
        self.inner.compact()
    }
}

impl<T: Ord> MutableOrderedSet<T> {
    /// Construct a set while rejecting duplicate elements.
    ///
    /// # Errors
    /// Returns the later representative of the smallest duplicated element.
    pub fn try_from_unique<I>(iter: I) -> Result<Self, crate::family::DuplicateElement<T>>
    where
        I: IntoIterator<Item = T>,
    {
        FrozenOrderedSet::try_from_unique(iter).map(FrozenOrderedSet::thaw)
    }

    /// Least member, if present.
    pub fn first(&self) -> Option<&T> {
        self.inner.first()
    }

    /// Greatest member, if present.
    pub fn last(&self) -> Option<&T> {
        self.inner.last()
    }

    /// Retain members selected by the callback, preserving their order.
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, keep: F) {
        self.inner.retain(keep);
    }

    /// Remove and return the least member.
    pub fn pop_first(&mut self) -> Option<T> {
        self.inner.pop_first()
    }

    /// Remove and return the greatest member.
    pub fn pop_last(&mut self) -> Option<T> {
        self.inner.pop_last()
    }

    /// Traverse the ascending members within a borrowed range.
    ///
    /// # Panics
    /// Panics for invalid bounds under the same rules as `BTreeSet::range`.
    pub fn range<Q, R>(&self, range: R) -> OrderedSetRange<'_, T>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
        R: core::ops::RangeBounds<Q>,
    {
        OrderedSetRange::new(self.inner.range(range))
    }

    /// Insert a new member, retaining the first equivalent representative.
    pub fn insert(&mut self, value: T) -> bool {
        self.inner.insert(value)
    }
    /// Check membership with a borrowed query.
    pub fn contains<Q: Ord + ?Sized>(&self, query: &Q) -> bool
    where
        T: Borrow<Q>,
    {
        self.inner.contains(query)
    }
    /// Borrow the stored representative.
    pub fn get<Q: Ord + ?Sized>(&self, query: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
    {
        self.inner.get(query)
    }
    /// Remove a member using a borrowed query.
    pub fn remove<Q: Ord + ?Sized>(&mut self, query: &Q) -> bool
    where
        T: Borrow<Q>,
    {
        self.inner.remove(query)
    }
    /// Remove and return the stored representative.
    pub fn take<Q: Ord + ?Sized>(&mut self, query: &Q) -> Option<T>
    where
        T: Borrow<Q>,
    {
        self.inner.take(query)
    }
}

impl<T> Default for MutableOrderedSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> From<BTreeSet<T>> for MutableOrderedSet<T> {
    fn from(value: BTreeSet<T>) -> Self {
        Self::from_set(value)
    }
}
/// First representative. The elements are staged, sorted stably, and reduced
/// by the frozen builder; the tree is then built in bulk from strictly
/// ascending elements, at the cost of O(n) temporary staging.
impl<T: Ord> FromIterator<T> for MutableOrderedSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        crate::family::FrozenOrderedSet::from_iter(iter).thaw()
    }
}
impl<T: Ord> Extend<T> for MutableOrderedSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for value in iter {
            self.insert(value);
        }
    }
}
impl<T> IntoIterator for MutableOrderedSet<T> {
    type Item = T;
    type IntoIter = OrderedSetIntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        OrderedSetIntoIter::new(self.inner.into_iter())
    }
}
impl<'a, T> IntoIterator for &'a MutableOrderedSet<T> {
    type Item = &'a T;
    type IntoIter = OrderedSetIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<T> Len for MutableOrderedSet<T> {
    fn len(&self) -> usize {
        self.len()
    }
}
impl<T> ElementType for MutableOrderedSet<T> {
    type Item = T;
}
impl<T> Elements for MutableOrderedSet<T> {
    type Iter<'a>
        = OrderedSetIter<'a, T>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.iter()
    }
}
impl<T: Ord, Q: Ord + ?Sized> SetContains<Q> for MutableOrderedSet<T>
where
    T: Borrow<Q>,
{
    fn contains(&self, query: &Q) -> bool {
        self.contains(query)
    }
}
impl<T: Ord, Q: Ord + ?Sized> SetGet<Q> for MutableOrderedSet<T>
where
    T: Borrow<Q>,
{
    fn get(&self, query: &Q) -> Option<&T> {
        self.get(query)
    }
}
impl<T: Ord> OrderedElements for MutableOrderedSet<T> {
    fn first_element(&self) -> Option<&T> {
        self.inner.first()
    }
    fn last_element(&self) -> Option<&T> {
        self.inner.last()
    }
}
impl<T: Ord, Q: Ord + ?Sized> RangeElements<Q> for MutableOrderedSet<T>
where
    T: Borrow<Q>,
{
    type Range<'a>
        = OrderedSetRange<'a, T>
    where
        Self: 'a;
    fn range_elements<R: core::ops::RangeBounds<Q>>(&self, range: R) -> Self::Range<'_> {
        OrderedSetRange::new(self.inner.range(range))
    }
}
impl<T: Ord> SetInsert for MutableOrderedSet<T> {
    fn insert(&mut self, value: T) -> bool {
        self.insert(value)
    }
}
impl<T: Ord, Q: Ord + ?Sized> SetRemove<Q> for MutableOrderedSet<T>
where
    T: Borrow<Q>,
{
    fn remove(&mut self, query: &Q) -> bool {
        self.remove(query)
    }
}
impl<T> FreezeWith<Compact> for MutableOrderedSet<T> {
    type Output = FrozenOrderedSet<T>;
    fn freeze_with(self, _: Compact) -> Self::Output {
        self.freeze()
    }
}

#[cfg(feature = "std")]
mod hashed {
    use crate::family::FrozenHashSet;
    use crate::read::{ElementType, Elements, Len, SetContains, SetGet};
    use crate::write::{SetInsert, SetRemove};
    use crate::{Compact, FreezeExt, FreezeWith};
    use core::borrow::Borrow;
    use core::hash::{BuildHasher, Hash};
    use std::collections::{HashSet, hash_map::RandomState, hash_set};

    borrowed_iter!(
        HashSetIter,
        hash_set::Iter<'a, T>,
        "Borrowed hash-set traversal in unspecified order."
    );
    owned_iter!(
        HashSetIntoIter,
        hash_set::IntoIter<T>,
        "Consuming hash-set traversal in unspecified order."
    );
    impl<T> ExactSizeIterator for HashSetIter<'_, T> {}

    /// A hash-set family that preserves its hasher and first representatives.
    #[derive(Clone)]
    pub struct MutableHashSet<T, S = RandomState> {
        pub(crate) inner: HashSet<T, S>,
    }

    impl<T: core::fmt::Debug, S> core::fmt::Debug for MutableHashSet<T, S> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_set().entries(self.iter()).finish()
        }
    }

    impl<T> MutableHashSet<T> {
        /// Empty set with the standard randomized hasher.
        pub fn new() -> Self {
            Self::from_set(HashSet::new())
        }
        /// Empty set with capacity for at least `capacity` members.
        pub fn with_capacity(capacity: usize) -> Self {
            Self::from_set(HashSet::with_capacity(capacity))
        }
    }
    impl<T, S> MutableHashSet<T, S> {
        /// Adopt a native set, preserving the hasher and representatives.
        pub fn from_set(inner: HashSet<T, S>) -> Self {
            Self { inner }
        }
        /// Create an empty set with an explicitly selected hasher.
        pub const fn with_hasher(hasher: S) -> Self {
            Self {
                inner: HashSet::with_hasher(hasher),
            }
        }
        /// Consume this family and return its native representation.
        pub fn into_set(self) -> HashSet<T, S> {
            self.inner
        }
        /// Extract the native hash set without rebuilding it.
        #[must_use]
        pub fn into_hash_set(self) -> HashSet<T, S> {
            self.inner
        }
        /// Number of members.
        pub fn len(&self) -> usize {
            self.inner.len()
        }
        /// Whether there are no members.
        pub fn is_empty(&self) -> bool {
            self.inner.is_empty()
        }
        /// Member capacity before reallocation is required.
        pub fn capacity(&self) -> usize {
            self.inner.capacity()
        }
        /// The chosen hash builder.
        pub fn hasher(&self) -> &S {
            self.inner.hasher()
        }
        /// Iterate in unspecified hash-table order.
        pub fn iter(&self) -> HashSetIter<'_, T> {
            HashSetIter::new(self.inner.iter())
        }
        /// Remove all members, retaining allocated capacity.
        pub fn clear(&mut self) {
            self.inner.clear();
        }
    }
    impl<T: Eq + Hash, S: BuildHasher> MutableHashSet<T, S> {
        /// Construct with an explicit hasher and first-representative retention.
        pub fn from_iter_with_hasher<I: IntoIterator<Item = T>>(iter: I, hasher: S) -> Self {
            let mut set = Self::with_hasher(hasher);
            set.extend(iter);
            set
        }

        /// Construct with an explicit hasher while rejecting duplicates.
        ///
        /// # Errors
        /// Returns the first rejected duplicate; other consumed elements drop.
        pub fn try_from_unique_with_hasher<I: IntoIterator<Item = T>>(
            iter: I,
            hasher: S,
        ) -> Result<Self, crate::family::DuplicateElement<T>> {
            FrozenHashSet::try_from_unique_with_hasher(iter, hasher).map(FrozenHashSet::thaw)
        }

        /// Construct with a default hasher while rejecting duplicates.
        ///
        /// # Errors
        /// Returns the first rejected duplicate; other consumed elements drop.
        pub fn try_from_unique<I: IntoIterator<Item = T>>(
            iter: I,
        ) -> Result<Self, crate::family::DuplicateElement<T>>
        where
            S: Default,
        {
            Self::try_from_unique_with_hasher(iter, S::default())
        }

        /// Retain members selected by the callback.
        pub fn retain<F: FnMut(&T) -> bool>(&mut self, keep: F) {
            self.inner.retain(keep);
        }

        /// Reserve without changing membership when allocation is refused.
        ///
        /// # Errors
        /// Reports allocation failure or capacity overflow.
        pub fn try_reserve(
            &mut self,
            additional: usize,
        ) -> Result<(), std::collections::TryReserveError> {
            self.inner.try_reserve(additional)
        }

        /// Reduce excess capacity while preserving members and hasher.
        pub fn shrink_to_fit(&mut self) {
            self.inner.shrink_to_fit();
        }

        /// Insert a member, keeping an existing equivalent representative.
        pub fn insert(&mut self, value: T) -> bool {
            self.inner.insert(value)
        }
        /// Membership by borrowed query.
        pub fn contains<Q: Eq + Hash + ?Sized>(&self, query: &Q) -> bool
        where
            T: Borrow<Q>,
        {
            self.inner.contains(query)
        }
        /// Borrow the stored representative.
        pub fn get<Q: Eq + Hash + ?Sized>(&self, query: &Q) -> Option<&T>
        where
            T: Borrow<Q>,
        {
            self.inner.get(query)
        }
        /// Remove a member by borrowed query.
        pub fn remove<Q: Eq + Hash + ?Sized>(&mut self, query: &Q) -> bool
        where
            T: Borrow<Q>,
        {
            self.inner.remove(query)
        }
        /// Remove and return the stored representative.
        pub fn take<Q: Eq + Hash + ?Sized>(&mut self, query: &Q) -> Option<T>
        where
            T: Borrow<Q>,
        {
            self.inner.take(query)
        }
        /// Reserve space for additional members; may panic on capacity overflow.
        pub fn reserve(&mut self, additional: usize) {
            self.inner.reserve(additional);
        }
        /// Shrink and freeze into the paired hash-set family; may rehash.
        pub fn freeze(self) -> FrozenHashSet<T, S> {
            self.inner.compact()
        }
    }
    impl<T, S: Default> Default for MutableHashSet<T, S> {
        fn default() -> Self {
            Self::with_hasher(S::default())
        }
    }
    impl<T, S> From<HashSet<T, S>> for MutableHashSet<T, S> {
        fn from(value: HashSet<T, S>) -> Self {
            Self::from_set(value)
        }
    }
    impl<T: Eq + Hash, S: BuildHasher + Default> FromIterator<T> for MutableHashSet<T, S> {
        fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
            let mut output = Self::default();
            output.extend(iter);
            output
        }
    }
    impl<T: Eq + Hash, S: BuildHasher> Extend<T> for MutableHashSet<T, S> {
        fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
            let iter = iter.into_iter();
            // Reserve from the lower size hint, as the paired map does, so a
            // large input does not rehash at every doubling.
            self.inner.reserve(iter.size_hint().0);
            for value in iter {
                self.insert(value);
            }
        }
    }
    impl<T, S> IntoIterator for MutableHashSet<T, S> {
        type Item = T;
        type IntoIter = HashSetIntoIter<T>;
        fn into_iter(self) -> Self::IntoIter {
            HashSetIntoIter::new(self.inner.into_iter())
        }
    }
    impl<'a, T, S> IntoIterator for &'a MutableHashSet<T, S> {
        type Item = &'a T;
        type IntoIter = HashSetIter<'a, T>;
        fn into_iter(self) -> Self::IntoIter {
            self.iter()
        }
    }
    impl<T: Eq + Hash, S1: BuildHasher, S2: BuildHasher> PartialEq<MutableHashSet<T, S2>>
        for MutableHashSet<T, S1>
    {
        fn eq(&self, other: &MutableHashSet<T, S2>) -> bool {
            self.len() == other.len() && self.iter().all(|value| other.contains(value))
        }
    }
    impl<T: Eq + Hash, S: BuildHasher> Eq for MutableHashSet<T, S> {}
    impl<T, S> Len for MutableHashSet<T, S> {
        fn len(&self) -> usize {
            self.len()
        }
    }
    impl<T, S> ElementType for MutableHashSet<T, S> {
        type Item = T;
    }
    impl<T, S> Elements for MutableHashSet<T, S> {
        type Iter<'a>
            = HashSetIter<'a, T>
        where
            Self: 'a;
        fn elements(&self) -> Self::Iter<'_> {
            self.iter()
        }
    }
    impl<T: Eq + Hash, S: BuildHasher, Q: Eq + Hash + ?Sized> SetContains<Q> for MutableHashSet<T, S>
    where
        T: Borrow<Q>,
    {
        fn contains(&self, query: &Q) -> bool {
            self.contains(query)
        }
    }
    impl<T: Eq + Hash, S: BuildHasher, Q: Eq + Hash + ?Sized> SetGet<Q> for MutableHashSet<T, S>
    where
        T: Borrow<Q>,
    {
        fn get(&self, query: &Q) -> Option<&T> {
            self.get(query)
        }
    }
    impl<T: Eq + Hash, S: BuildHasher> SetInsert for MutableHashSet<T, S> {
        fn insert(&mut self, value: T) -> bool {
            self.insert(value)
        }
    }
    impl<T: Eq + Hash, S: BuildHasher, Q: Eq + Hash + ?Sized> SetRemove<Q> for MutableHashSet<T, S>
    where
        T: Borrow<Q>,
    {
        fn remove(&mut self, query: &Q) -> bool {
            self.remove(query)
        }
    }
    impl<T: Eq + Hash, S: BuildHasher> FreezeWith<Compact> for MutableHashSet<T, S> {
        type Output = FrozenHashSet<T, S>;
        fn freeze_with(self, _: Compact) -> Self::Output {
            self.freeze()
        }
    }
}

#[cfg(feature = "std")]
pub use hashed::{HashSetIntoIter, HashSetIter, MutableHashSet};

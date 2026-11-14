//! Core of the Frost collection library: capability traits, freeze plans,
//! `Frozen` and its seal, the mutable and frozen families with their set
//! algebra, adaptive profiles, native backends, and framework integrations.
//!
//! This package is internal. Depend on `frost-collections`, whose library
//! `frost` re-exports everything here at the same paths, and read its
//! documentation. The examples in this package's documentation are written
//! against `frost`.
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod frozen;
#[cfg(feature = "alloc")]
pub mod iter;
pub mod plan;
pub mod read;
pub mod write;

pub mod backend;
mod backends;
#[cfg(feature = "alloc")]
pub mod mutable;

pub mod adapters;

#[cfg(feature = "alloc")]
pub mod family;

#[cfg(feature = "alloc")]
pub mod adaptive;

pub use frozen::{Eligible, Frozen};
pub use plan::{Compact, ExactLen, Retain};
pub use plan::{FreezeExt, FreezeWith, Rejected, TryFreezeWith};

/// Glob-importable names for everyday use.
pub mod prelude {
    pub use crate::Frozen;
    pub use crate::plan::{Compact, ExactLen, FreezeExt, FreezeWith, Retain, TryFreezeWith};
    pub use crate::read::{
        Cardinality, DequeRead, ElementType, Elements, Entries, IndexOf, IndexedEntries, Len,
        LookupRef, LookupValue, MapTypes, OrderedElements, OrderedEntries, PriorityRead,
        RangeElements, RangeEntries, SetContains, SetGet, TwoSlices, ValueElements, ValueRef,
    };

    #[cfg(feature = "alloc")]
    pub use crate::adaptive::{
        AdaptiveOrdered, AdaptiveOrderedMap, Inline, InlineFrozenVec, InlineVec,
    };
    #[cfg(feature = "alloc")]
    pub use crate::family::{
        FrozenOrderedMap, FrozenOrderedSet, FrozenVec, MutableOrderedMap, MutableOrderedSet,
        MutableVec,
    };
    #[cfg(feature = "alloc")]
    pub use crate::plan::Sorted;

    #[cfg(feature = "std")]
    pub use crate::adaptive::{AdaptiveFrozenHashMap, AdaptiveHash, AdaptiveHashMap};
    #[cfg(feature = "std")]
    pub use crate::family::{FrozenHashMap, FrozenHashSet, MutableHashMap, MutableHashSet};
}

/// Items other Frost packages need that are not public API.
///
/// Nothing here is covered by any compatibility or documentation promise.
/// Packages in this workspace depend on each other at an exact version.
#[doc(hidden)]
pub mod __private {
    use crate::plan::Rejected;

    /// Builds a [`Rejected`], which is `#[non_exhaustive]` outside this crate.
    #[inline]
    pub fn rejected<I, E>(input: I, error: E) -> Rejected<I, E> {
        Rejected { input, error }
    }

    /// Sorted-slice kernels of the ordered families.
    ///
    /// Every slice argument must be strictly ascending. Other input gives
    /// wrong results, not undefined behaviour.
    #[cfg(feature = "alloc")]
    pub mod set_ops {
        use crate::family::ordered::set_ops as kernels;
        use kernels::{SetDifference, SetIntersection, SetSymmetricDifference, SetUnion};

        /// Lazy union of two strictly ascending slices.
        #[inline]
        pub fn union<'a, T>(left: &'a [T], right: &'a [T]) -> SetUnion<'a, T> {
            SetUnion::new(left, right)
        }

        /// Lazy intersection of two strictly ascending slices.
        #[inline]
        pub fn intersection<'a, T>(left: &'a [T], right: &'a [T]) -> SetIntersection<'a, T> {
            SetIntersection::new(left, right)
        }

        /// Lazy difference of two strictly ascending slices.
        #[inline]
        pub fn difference<'a, T>(left: &'a [T], right: &'a [T]) -> SetDifference<'a, T> {
            SetDifference::new(left, right)
        }

        /// Lazy symmetric difference of two strictly ascending slices.
        #[inline]
        pub fn symmetric_difference<'a, T>(
            left: &'a [T],
            right: &'a [T],
        ) -> SetSymmetricDifference<'a, T> {
            SetSymmetricDifference::new(left, right)
        }

        /// Size of the intersection of two strictly ascending slices.
        #[inline]
        pub fn intersection_len<T: Ord>(left: &[T], right: &[T]) -> usize {
            kernels::intersection_len(left, right)
        }

        /// Whether two strictly ascending slices share no element.
        #[inline]
        pub fn is_disjoint<T: Ord>(left: &[T], right: &[T]) -> bool {
            kernels::is_disjoint(left, right)
        }

        /// Whether every element of `left` is in `right`.
        #[inline]
        pub fn is_subset<T: Ord>(left: &[T], right: &[T]) -> bool {
            kernels::is_subset(left, right)
        }
    }
}

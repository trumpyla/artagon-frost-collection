#![doc = include_str!("../README.md")]
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
#[cfg(feature = "std")]
pub mod concurrent;
#[cfg(feature = "alloc")]
pub mod mutable;

pub mod adapters;

#[cfg(feature = "int-set")]
pub mod integer;

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

    #[cfg(feature = "int-set")]
    pub use crate::integer::{FrozenIntSet, IntKey, IntSetPolicy};
}

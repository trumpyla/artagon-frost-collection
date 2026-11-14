//! *Ordered-map* sub-category profile: a sorted vector while small, a B-tree
//! when large.
//!
//! One enum (`Sorted`, `Tree`), one contract: `K: Borrow<Q> + Ord` lookup,
//! ascending traversal, `BTreeMap`-identical `range` (panics included).
//!
//! No frozen enum is needed here: [`FrozenOrderedMap`](crate::family::FrozenOrderedMap)
//! (sorted boxed entries, binary search) is already the compact form for every
//! size, so [`AdaptiveOrderedMap::freeze`] targets it directly.

mod mutable;

pub use mutable::{
    AdaptiveOrdered, AdaptiveOrderedMap, IntoIter, Iter, IterMut, OrderedRepr, Range, RangeMut,
};

#[cfg(feature = "rayon-1")]
pub(crate) use mutable::Repr;

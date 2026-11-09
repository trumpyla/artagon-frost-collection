//! Ordered families: [`MutableOrderedMap`] ⇄ [`FrozenOrderedMap`], and
//! [`FrozenOrderedSet`]. Mutable iterators wrap the tree; frozen ones
//! (`Sorted*`) wrap the sorted slice.

mod frozen;
mod mutable;
pub(crate) mod set_ops;

pub use crate::family::sets::{
    MutableOrderedSet, OrderedSetIntoIter, OrderedSetIter, OrderedSetRange,
};
pub(crate) use frozen::range_indices;
pub use frozen::{
    FrozenOrderedMap, FrozenOrderedSet, NotSortedUnique, SortedIntoIter, SortedIter,
    SortedSetIntoIter,
};
pub use mutable::{
    Entry, IntoIter, Iter, IterMut, Keys, MutableOrderedMap, OccupiedEntry, Range, RangeMut,
    VacantEntry, Values, ValuesMut,
};
pub use set_ops::{SetDifference, SetIntersection, SetSymmetricDifference, SetUnion};

//! Family types: stable semantic names with a hidden representation and a
//! crate-defined contract (duplicate policy, equality, order).
//!
//! Default construction policy for every family built from entries:
//! **first stored key representative wins, last value wins**; sets keep the
//! first representative. Strict construction is `try_from_unique`.

//!
//! Families come in mutable/frozen pairs per sub-category, each with a fixed,
//! documented transition (`freeze` / `thaw`) and crate-owned iterator and
//! entry types in the sub-category module:
//!
//! | Sub-category | Mutable | Frozen | Module |
//! |---|---|---|---|
//! | Contiguous sequence | [`MutableVec`] | [`FrozenVec`] | [`vec`] |
//! | Hash map (`std`) | `MutableHashMap` | `FrozenHashMap` | `hash_map` |
//! | Ordered map | [`MutableOrderedMap`] | [`FrozenOrderedMap`] | [`ordered`] |
//! | Ordered set | [`MutableOrderedSet`] | [`FrozenOrderedSet`] | [`ordered`] |
//! | Hash set | `MutableHashSet` | `FrozenHashSet` | `hash_set` (`std`) |

#[cfg(feature = "std")]
pub mod hash_set;
pub mod ordered;
pub mod sets;
pub mod vec;

#[cfg(feature = "std")]
pub mod hash_map;

#[cfg(feature = "std")]
pub use hash_set::{FrozenHashSet, MutableHashSet};
pub use ordered::{FrozenOrderedMap, FrozenOrderedSet, MutableOrderedMap, NotSortedUnique};
pub use sets::MutableOrderedSet;
pub use vec::{FrozenVec, MutableVec};

#[cfg(feature = "std")]
pub use hash_map::{FrozenHashMap, MutableHashMap};

/// Entries to reserve before extending a hash table from an iterator with the
/// lower size hint `hint`.
///
/// An empty table receives the whole hint. A nonempty table may already hold
/// the incoming keys, so it receives half, as the standard collections do:
/// reserving for all of them would double the table, and hash every stored
/// key again, when the input only replaces values.
#[cfg(feature = "std")]
pub(crate) const fn extend_reservation(is_empty: bool, hint: usize) -> usize {
    if is_empty { hint } else { hint.div_ceil(2) }
}

/// A duplicate key found by a strict (`try_from_unique`) builder.
///
/// Carries only the rejected entry. Other consumed entries are dropped with
/// the partially built collection. The constructor documents whether it
/// validates incrementally or first consumes and sorts the complete input.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DuplicateKey<K, V> {
    /// The later entry whose key was already present.
    pub key: K,
    /// Its value.
    pub value: V,
}

impl<K, V> core::fmt::Debug for DuplicateKey<K, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DuplicateKey").finish_non_exhaustive()
    }
}

impl<K, V> core::fmt::Display for DuplicateKey<K, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("duplicate key in strict construction")
    }
}

impl<K, V> core::error::Error for DuplicateKey<K, V> {}

/// A duplicate element returned by a strict set builder.
///
/// This error returns only the rejected duplicate. Other consumed elements
/// are dropped. Ordered builders consume the entire input before validation.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DuplicateElement<T> {
    /// The later element equivalent to one already stored.
    pub item: T,
}

impl<T> core::fmt::Debug for DuplicateElement<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DuplicateElement").finish_non_exhaustive()
    }
}

impl<T> core::fmt::Display for DuplicateElement<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("duplicate element in strict set construction")
    }
}

impl<T> core::error::Error for DuplicateElement<T> {}

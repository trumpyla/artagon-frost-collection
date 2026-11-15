//! Mutation capabilities for native mutable backends and mutable families.
//! Frozen wrappers and frozen families do not implement these traits.
//!
//! Fallible variants exist only where the backend can actually report failure
//! before committing. An infallible operation may still abort the process on
//! allocation failure; that is not converted into a `Result`.

use core::fmt;

use crate::read::{ElementType, MapTypes};

/// Append to the logical end of a sequence (infallible API; may abort on OOM).
pub trait Push {
    /// Element type.
    type Item;
    /// Append `value`.
    fn push(&mut self, value: Self::Item);
}

/// A push that was refused. The value is returned to the caller.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PushRejected<T, E> {
    /// The value that was not stored.
    pub value: T,
    /// Why it was refused.
    pub error: E,
}

impl<T, E: fmt::Debug> fmt::Debug for PushRejected<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PushRejected")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<T, E: fmt::Display> fmt::Display for PushRejected<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "push rejected: {}", self.error)
    }
}

impl<T, E: core::error::Error + 'static> core::error::Error for PushRejected<T, E> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Append with recoverable failure (capacity exhaustion or a reservation
/// failure detected *before* the value is moved into storage).
pub trait TryPush {
    /// Element type.
    type Item;
    /// Failure reason.
    type Error;
    /// Append `value`, or hand it back with the reason.
    ///
    /// # Errors
    /// The backend could not store another element; nothing was modified.
    fn try_push(&mut self, value: Self::Item) -> Result<(), PushRejected<Self::Item, Self::Error>>;
}

/// Insert-or-replace for maps.
///
/// Contract (matches `std` and `indexmap`): if an equivalent key is present,
/// the **stored key is kept**, its value is replaced and the old value is
/// returned; an insertion-ordered map keeps the entry's position. The passed-in
/// key is dropped in that case.
pub trait MapInsert: MapTypes {
    /// Insert `value` under `key`, returning the replaced value.
    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value>;
}

/// Removal by a borrowed query, for maps whose removal has one meaning.
///
/// Deliberately **not** implemented for `IndexMap`: there removal is either
/// `swap_remove` (O(1), reorders) or `shift_remove` (O(n), keeps order), and a
/// generic `remove` would hide that choice.
pub trait MapRemove<Q: ?Sized>: MapTypes {
    /// Remove, returning the stored key and value.
    fn remove_entry(&mut self, query: &Q) -> Option<(Self::Key, Self::Value)>;
    /// Remove, returning the value.
    fn remove(&mut self, query: &Q) -> Option<Self::Value> {
        self.remove_entry(query).map(|(_, v)| v)
    }
}

/// Set insertion with first-representative retention.
///
/// Returns `true` for a newly inserted member. When an equivalent member
/// already exists, its stored representative is retained and `value` is dropped.
pub trait SetInsert: ElementType {
    /// Insert `value`, reporting whether membership changed.
    fn insert(&mut self, value: Self::Item) -> bool;
}

/// Remove a set member by a borrowed query.
///
/// Insertion-ordered sets deliberately retain their explicit native
/// `swap_remove` versus `shift_remove` choice instead of implementing this trait.
pub trait SetRemove<Q: ?Sized> {
    /// Remove the member if present, reporting whether membership changed.
    fn remove(&mut self, query: &Q) -> bool;
}

#[cfg(feature = "hashbrown-0_17")]
impl<T, S> SetInsert for hashbrown::HashSet<T, S>
where
    T: Eq + core::hash::Hash,
    S: core::hash::BuildHasher,
{
    fn insert(&mut self, value: T) -> bool {
        Self::insert(self, value)
    }
}

#[cfg(feature = "hashbrown-0_17")]
impl<T, Q, S> SetRemove<Q> for hashbrown::HashSet<T, S>
where
    T: Eq + core::hash::Hash,
    Q: core::hash::Hash + hashbrown::Equivalent<T> + ?Sized,
    S: core::hash::BuildHasher,
{
    fn remove(&mut self, query: &Q) -> bool {
        Self::remove(self, query)
    }
}

#[cfg(feature = "indexmap-2")]
impl<T, S> SetInsert for indexmap::IndexSet<T, S>
where
    T: Eq + core::hash::Hash,
    S: core::hash::BuildHasher,
{
    fn insert(&mut self, value: T) -> bool {
        Self::insert(self, value)
    }
}

#[cfg(feature = "alloc")]
impl<T: Ord> SetInsert for alloc::collections::BTreeSet<T> {
    fn insert(&mut self, value: T) -> bool {
        Self::insert(self, value)
    }
}

#[cfg(feature = "alloc")]
impl<T, Q> SetRemove<Q> for alloc::collections::BTreeSet<T>
where
    T: core::borrow::Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    fn remove(&mut self, query: &Q) -> bool {
        Self::remove(self, query)
    }
}

#[cfg(feature = "std")]
impl<T, S> SetInsert for std::collections::HashSet<T, S>
where
    T: Eq + core::hash::Hash,
    S: core::hash::BuildHasher,
{
    fn insert(&mut self, value: T) -> bool {
        Self::insert(self, value)
    }
}

#[cfg(feature = "std")]
impl<T, Q, S> SetRemove<Q> for std::collections::HashSet<T, S>
where
    T: core::borrow::Borrow<Q> + Eq + core::hash::Hash,
    Q: Eq + core::hash::Hash + ?Sized,
    S: core::hash::BuildHasher,
{
    fn remove(&mut self, query: &Q) -> bool {
        Self::remove(self, query)
    }
}

#[cfg(feature = "alloc")]
mod alloc_impls {
    use super::{MapInsert, MapRemove, Push, PushRejected, TryPush};
    use alloc::collections::{BTreeMap, BinaryHeap, LinkedList, TryReserveError, VecDeque};
    use alloc::vec::Vec;
    use core::borrow::Borrow;

    impl<T> Push for Vec<T> {
        type Item = T;
        fn push(&mut self, value: T) {
            Vec::push(self, value);
        }
    }

    impl<T> TryPush for Vec<T> {
        type Item = T;
        type Error = TryReserveError;
        fn try_push(&mut self, value: T) -> Result<(), PushRejected<T, TryReserveError>> {
            match self.try_reserve(1) {
                Ok(()) => {
                    // Cannot reallocate: capacity was reserved above.
                    Vec::push(self, value);
                    Ok(())
                }
                Err(error) => Err(PushRejected { value, error }),
            }
        }
    }

    impl<T> Push for VecDeque<T> {
        type Item = T;
        fn push(&mut self, value: T) {
            self.push_back(value);
        }
    }

    impl<T> TryPush for VecDeque<T> {
        type Item = T;
        type Error = TryReserveError;
        fn try_push(&mut self, value: T) -> Result<(), PushRejected<T, TryReserveError>> {
            match self.try_reserve(1) {
                Ok(()) => {
                    self.push_back(value);
                    Ok(())
                }
                Err(error) => Err(PushRejected { value, error }),
            }
        }
    }

    impl<T> Push for LinkedList<T> {
        type Item = T;
        fn push(&mut self, value: T) {
            self.push_back(value);
        }
    }

    impl<T: Ord> Push for BinaryHeap<T> {
        type Item = T;
        fn push(&mut self, value: T) {
            BinaryHeap::push(self, value);
        }
    }

    impl<K: Ord, V> MapInsert for BTreeMap<K, V> {
        fn insert(&mut self, key: K, value: V) -> Option<V> {
            BTreeMap::insert(self, key, value)
        }
    }

    impl<K: Borrow<Q> + Ord, V, Q: Ord + ?Sized> MapRemove<Q> for BTreeMap<K, V> {
        fn remove_entry(&mut self, query: &Q) -> Option<(K, V)> {
            BTreeMap::remove_entry(self, query)
        }
    }

    #[cfg(feature = "std")]
    impl<K, V, S, Q> MapRemove<Q> for std::collections::HashMap<K, V, S>
    where
        K: Borrow<Q> + Eq + core::hash::Hash,
        Q: Eq + core::hash::Hash + ?Sized,
        S: core::hash::BuildHasher,
    {
        fn remove_entry(&mut self, query: &Q) -> Option<(K, V)> {
            std::collections::HashMap::remove_entry(self, query)
        }
    }

    #[cfg(feature = "hashbrown-0_17")]
    impl<K, V, S, Q> MapRemove<Q> for hashbrown::HashMap<K, V, S>
    where
        K: Eq + core::hash::Hash,
        Q: core::hash::Hash + hashbrown::Equivalent<K> + ?Sized,
        S: core::hash::BuildHasher,
    {
        fn remove_entry(&mut self, query: &Q) -> Option<(K, V)> {
            hashbrown::HashMap::remove_entry(self, query)
        }
    }

    #[cfg(feature = "std")]
    impl<K, V, S> MapInsert for std::collections::HashMap<K, V, S>
    where
        K: Eq + core::hash::Hash,
        S: core::hash::BuildHasher,
    {
        fn insert(&mut self, key: K, value: V) -> Option<V> {
            std::collections::HashMap::insert(self, key, value)
        }
    }

    #[cfg(feature = "indexmap-2")]
    impl<K, V, S> MapInsert for indexmap::IndexMap<K, V, S>
    where
        K: Eq + core::hash::Hash,
        S: core::hash::BuildHasher,
    {
        fn insert(&mut self, key: K, value: V) -> Option<V> {
            indexmap::IndexMap::insert(self, key, value)
        }
    }

    #[cfg(feature = "hashbrown-0_17")]
    impl<K, V, S> MapInsert for hashbrown::HashMap<K, V, S>
    where
        K: Eq + core::hash::Hash,
        S: core::hash::BuildHasher,
    {
        fn insert(&mut self, key: K, value: V) -> Option<V> {
            hashbrown::HashMap::insert(self, key, value)
        }
    }
}

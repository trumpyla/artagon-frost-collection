use super::{ConcurrentLookup, ConcurrentSetLookup, ConcurrentUpdate};
use core::hash::{BuildHasher, Hash};
use scc::{Equivalent, HashMap, HashSet};

impl<K, V, S, Q: ?Sized> ConcurrentLookup<Q> for HashMap<K, V, S>
where
    K: Eq + Hash,
    Q: Equivalent<K> + Hash,
    S: BuildHasher,
{
    fn with_entry<R>(&self, key: &Q, read: impl FnOnce(&K, &V) -> R) -> Option<R> {
        self.read_sync(key, read)
    }
}

impl<K, V, S, Q: ?Sized> ConcurrentUpdate<Q> for HashMap<K, V, S>
where
    K: Eq + Hash,
    Q: Equivalent<K> + Hash,
    S: BuildHasher,
{
    fn with_entry_mut<R>(&self, key: &Q, update: impl FnOnce(&K, &mut V) -> R) -> Option<R> {
        self.update_sync(key, update)
    }
}

impl<K, S, Q: ?Sized> ConcurrentSetLookup<Q> for HashSet<K, S>
where
    K: Eq + Hash,
    Q: Equivalent<K> + Hash,
    S: BuildHasher,
{
    fn with_value<R>(&self, key: &Q, read: impl FnOnce(&K) -> R) -> Option<R> {
        self.read_sync(key, read)
    }
}

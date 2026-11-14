use super::{ConcurrentLookup, ConcurrentSetLookup};
use core::borrow::Borrow;
use crossbeam_skiplist::{SkipMap, SkipSet};

impl<K, V, Q: ?Sized> ConcurrentLookup<Q> for SkipMap<K, V>
where
    K: Ord + Borrow<Q>,
    Q: Ord,
{
    fn with_entry<R>(&self, key: &Q, read: impl FnOnce(&K, &V) -> R) -> Option<R> {
        self.get(key).map(|entry| read(entry.key(), entry.value()))
    }
}

impl<K, Q: ?Sized> ConcurrentSetLookup<Q> for SkipSet<K>
where
    K: Ord + Borrow<Q>,
    Q: Ord,
{
    fn with_value<R>(&self, key: &Q, read: impl FnOnce(&K) -> R) -> Option<R> {
        self.get(key).map(|entry| read(entry.value()))
    }
}

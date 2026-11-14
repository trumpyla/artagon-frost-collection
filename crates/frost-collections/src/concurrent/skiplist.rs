use super::{ConcurrentLookup, ConcurrentSetLookup};
use crate::read::{ElementType, MapTypes, SetContains};
use core::borrow::Borrow;
use crossbeam_skiplist::{SkipMap, SkipSet};

impl<K, V> MapTypes for SkipMap<K, V> {
    type Key = K;
    type Value = V;
}

impl<K, V, Q: ?Sized> ConcurrentLookup<Q> for SkipMap<K, V>
where
    K: Ord + Borrow<Q>,
    Q: Ord,
{
    fn with_entry<R>(&self, key: &Q, read: impl FnOnce(&K, &V) -> R) -> Option<R> {
        self.get(key).map(|entry| read(entry.key(), entry.value()))
    }
}

impl<K> ElementType for SkipSet<K> {
    type Item = K;
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

impl<K, Q: ?Sized> SetContains<Q> for SkipSet<K>
where
    K: Ord + Borrow<Q>,
    Q: Ord,
{
    fn contains(&self, value: &Q) -> bool {
        SkipSet::contains(self, value)
    }
}

use super::{ConcurrentLookup, ConcurrentSetLookup, ConcurrentUpdate};
use crate::read::{ElementType, MapTypes, SetContains};
use core::borrow::Borrow;
use core::hash::{BuildHasher, Hash};
use dashmap::{DashMap, DashSet};

impl<K, V, S> MapTypes for DashMap<K, V, S> {
    type Key = K;
    type Value = V;
}

impl<K, V, S, Q: ?Sized> ConcurrentLookup<Q> for DashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash,
    S: BuildHasher + Clone,
{
    fn with_entry<R>(&self, key: &Q, read: impl FnOnce(&K, &V) -> R) -> Option<R> {
        self.get(key).map(|entry| {
            let (key, value) = entry.pair();
            read(key, value)
        })
    }
}

impl<K, V, S, Q: ?Sized> ConcurrentUpdate<Q> for DashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash,
    S: BuildHasher + Clone,
{
    fn with_entry_mut<R>(&self, key: &Q, update: impl FnOnce(&K, &mut V) -> R) -> Option<R> {
        self.get_mut(key).map(|mut entry| {
            let (key, value) = entry.pair_mut();
            update(key, value)
        })
    }
}

impl<K, S> ElementType for DashSet<K, S> {
    type Item = K;
}

impl<K, S, Q: ?Sized> ConcurrentSetLookup<Q> for DashSet<K, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash,
    S: BuildHasher + Clone,
{
    fn with_value<R>(&self, key: &Q, read: impl FnOnce(&K) -> R) -> Option<R> {
        self.get(key).map(|entry| read(entry.key()))
    }
}

impl<K, S, Q: ?Sized> SetContains<Q> for DashSet<K, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash,
    S: BuildHasher + Clone,
{
    fn contains(&self, value: &Q) -> bool {
        DashSet::contains(self, value)
    }
}

use crate::read::{ElementType, MapTypes, SetContains};
use core::borrow::Borrow;
use core::hash::{BuildHasher, Hash};
use dashmap::{DashMap, DashSet};

impl<K, V, S> MapTypes for DashMap<K, V, S> {
    type Key = K;
    type Value = V;
}

impl<K, S> ElementType for DashSet<K, S> {
    type Item = K;
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

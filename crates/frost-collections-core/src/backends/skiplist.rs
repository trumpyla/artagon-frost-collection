use crate::read::{ElementType, MapTypes, SetContains};
use core::borrow::Borrow;
use crossbeam_skiplist::{SkipMap, SkipSet};

impl<K, V> MapTypes for SkipMap<K, V> {
    type Key = K;
    type Value = V;
}

impl<K> ElementType for SkipSet<K> {
    type Item = K;
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

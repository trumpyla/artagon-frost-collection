use crate::read::{ElementType, MapTypes, SetContains};
use core::hash::{BuildHasher, Hash};
use scc::{Equivalent, HashMap, HashSet};

impl<K, V, S: BuildHasher> MapTypes for HashMap<K, V, S> {
    type Key = K;
    type Value = V;
}

impl<K, S: BuildHasher> ElementType for HashSet<K, S> {
    type Item = K;
}

impl<K, S, Q: ?Sized> SetContains<Q> for HashSet<K, S>
where
    K: Eq + Hash,
    Q: Equivalent<K> + Hash,
    S: BuildHasher,
{
    fn contains(&self, value: &Q) -> bool {
        self.contains_sync(value)
    }
}

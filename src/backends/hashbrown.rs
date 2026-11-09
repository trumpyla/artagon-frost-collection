//! `hashbrown::{HashMap, HashSet}` (feature `hashbrown-0_17`).
//!
//! Query relation is hashbrown's own: `Q: Hash + Equivalent<K>`, which is
//! strictly more permissive than `K: Borrow<Q>`. The adapter does not narrow
//! it to the std relation, and no plan changes the hasher.

use core::hash::{BuildHasher, Hash};

use hashbrown::{Equivalent, HashMap, HashSet, hash_map, hash_set};

use crate::frozen::sealed::Sealed;
use crate::read::{ElementType, Elements, Entries, Len, LookupRef, MapTypes, SetContains, SetGet};
use crate::{Compact, Eligible, FreezeWith, Frozen, Retain};

impl<K, V, S> Sealed for HashMap<K, V, S> {}
impl<K, V, S> Eligible for HashMap<K, V, S> {}
impl<T, S> Sealed for HashSet<T, S> {}
impl<T, S> Eligible for HashSet<T, S> {}

impl<K, V, S> Len for HashMap<K, V, S> {
    fn len(&self) -> usize {
        HashMap::len(self)
    }
}

impl<K, V, S> MapTypes for HashMap<K, V, S> {
    type Key = K;
    type Value = V;
}

impl<K, V, S, Q> LookupRef<Q> for HashMap<K, V, S>
where
    K: Eq + Hash,
    Q: Hash + Equivalent<K> + ?Sized,
    S: BuildHasher,
{
    fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> {
        HashMap::get_key_value(self, query)
    }
    fn get(&self, query: &Q) -> Option<&V> {
        HashMap::get(self, query)
    }
}

impl<K, V, S> Entries for HashMap<K, V, S> {
    type Iter<'a>
        = hash_map::Iter<'a, K, V>
    where
        Self: 'a;
    fn entries(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

impl<K, V, S> FreezeWith<Retain> for HashMap<K, V, S> {
    type Output = Frozen<Self>;
    fn freeze_with(self, _: Retain) -> Frozen<Self> {
        Frozen::new_unchecked(self)
    }
}

/// Shrinks in place; stays a hashbrown table with the same hasher.
impl<K, V, S> FreezeWith<Compact> for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    type Output = Frozen<Self>;
    fn freeze_with(mut self, _: Compact) -> Frozen<Self> {
        self.shrink_to_fit();
        Frozen::new_unchecked(self)
    }
}

impl<K, V, S> Frozen<HashMap<K, V, S>> {
    /// Value for `key`.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Eq + Hash,
        Q: Hash + Equivalent<K> + ?Sized,
        S: BuildHasher,
    {
        self.backend().get(key)
    }
    /// Stored key representative and value.
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Eq + Hash,
        Q: Hash + Equivalent<K> + ?Sized,
        S: BuildHasher,
    {
        self.backend().get_key_value(key)
    }
    /// `true` if present.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Eq + Hash,
        Q: Hash + Equivalent<K> + ?Sized,
        S: BuildHasher,
    {
        self.backend().contains_key(key)
    }
    /// Entries in unspecified order.
    pub fn iter(&self) -> hash_map::Iter<'_, K, V> {
        self.backend().iter()
    }
    /// Keys in unspecified order.
    pub fn keys(&self) -> hash_map::Keys<'_, K, V> {
        self.backend().keys()
    }
    /// Values in unspecified order.
    pub fn values(&self) -> hash_map::Values<'_, K, V> {
        self.backend().values()
    }
    /// The map's hasher.
    pub fn hasher(&self) -> &S {
        self.backend().hasher()
    }
}

impl<T, S> Len for HashSet<T, S> {
    fn len(&self) -> usize {
        HashSet::len(self)
    }
}

impl<T, S> ElementType for HashSet<T, S> {
    type Item = T;
}

impl<T, S> Elements for HashSet<T, S> {
    type Iter<'a>
        = hash_set::Iter<'a, T>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

impl<T, S, Q> SetContains<Q> for HashSet<T, S>
where
    T: Eq + Hash,
    Q: Hash + Equivalent<T> + ?Sized,
    S: BuildHasher,
{
    fn contains(&self, query: &Q) -> bool {
        HashSet::contains(self, query)
    }
}

impl<T, S, Q> SetGet<Q> for HashSet<T, S>
where
    T: Eq + Hash,
    Q: Hash + Equivalent<T> + ?Sized,
    S: BuildHasher,
{
    fn get(&self, query: &Q) -> Option<&T> {
        HashSet::get(self, query)
    }
}

impl<T, S> FreezeWith<Retain> for HashSet<T, S> {
    type Output = Frozen<Self>;
    fn freeze_with(self, _: Retain) -> Frozen<Self> {
        Frozen::new_unchecked(self)
    }
}

impl<T: Eq + Hash, S: BuildHasher> FreezeWith<Compact> for HashSet<T, S> {
    type Output = Frozen<Self>;
    fn freeze_with(mut self, _: Compact) -> Self::Output {
        self.shrink_to_fit();
        Frozen::new_unchecked(self)
    }
}

impl<T, S> Frozen<HashSet<T, S>> {
    /// Stored representative of an equivalent query.
    pub fn get<Q>(&self, query: &Q) -> Option<&T>
    where
        T: Eq + Hash,
        Q: Hash + Equivalent<T> + ?Sized,
        S: BuildHasher,
    {
        self.backend().get(query)
    }
    /// Membership.
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Eq + Hash,
        Q: Hash + Equivalent<T> + ?Sized,
        S: BuildHasher,
    {
        self.backend().contains(value)
    }
    /// Elements in unspecified order.
    pub fn iter(&self) -> hash_set::Iter<'_, T> {
        self.backend().iter()
    }
}

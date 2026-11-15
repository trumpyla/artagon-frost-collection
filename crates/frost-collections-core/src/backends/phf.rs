//! Compile-time perfect-hash maps and sets. Declaration order is not key order.

use crate::frozen::sealed::Sealed;
use crate::read::{
    ElementType, Elements, Entries, IndexOf, IndexedEntries, Len, LookupRef, MapTypes, SetContains,
    SetGet,
};
use crate::{Eligible, FreezeWith, Frozen, Retain};
use phf::{PhfEq, PhfHash};

macro_rules! maps {
    ($($name:ident, $module:ident);* $(;)?) => {$(
        impl<K, V> Sealed for phf::$name<K, V> {}
        impl<K, V> Eligible for phf::$name<K, V> {}
        impl<K, V> Len for phf::$name<K, V> {
            fn len(&self) -> usize { phf::$name::len(self) }
        }
        impl<K, V> MapTypes for phf::$name<K, V> { type Key = K; type Value = V; }
        impl<K, V, Q: ?Sized + Eq + PhfHash> LookupRef<Q> for phf::$name<K, V>
        where K: PhfEq<Q> {
            fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> { self.get_entry(query) }
        }
        impl<K, V> Entries for phf::$name<K, V> {
            type Iter<'a> = phf::$module::Entries<'a, K, V> where Self: 'a;
            fn entries(&self) -> Self::Iter<'_> { phf::$name::entries(self) }
        }
        impl<K, V> FreezeWith<Retain> for phf::$name<K, V> {
            type Output = Frozen<Self>;
            fn freeze_with(self, _: Retain) -> Self::Output { Frozen::new_unchecked(self) }
        }
        impl<K, V> Frozen<phf::$name<K, V>> {
            /// Borrow the value associated with the query.
            pub fn get<Q: ?Sized + Eq + PhfHash>(&self, query: &Q) -> Option<&V>
            where K: PhfEq<Q> { self.backend().get(query) }
            /// Borrow the stored key representative and its value.
            pub fn get_key_value<Q: ?Sized + Eq + PhfHash>(&self, query: &Q) -> Option<(&K, &V)>
            where K: PhfEq<Q> { self.backend().get_entry(query) }
            /// Traverse the entries in the backend's order.
            pub fn iter(&self) -> phf::$module::Entries<'_, K, V> { self.backend().entries() }
        }
    )*};
}
maps! { Map, map; OrderedMap, ordered_map; }

impl<K, V> IndexedEntries for phf::OrderedMap<K, V> {
    fn get_index(&self, index: usize) -> Option<(&K, &V)> {
        self.index(index)
    }
}
impl<K, V, Q: ?Sized + Eq + PhfHash> IndexOf<Q> for phf::OrderedMap<K, V>
where
    K: PhfEq<Q>,
{
    fn get_index_of(&self, query: &Q) -> Option<usize> {
        self.get_index(query)
    }
}

macro_rules! sets {
    ($($name:ident, $module:ident);* $(;)?) => {$(
        impl<T> Sealed for phf::$name<T> {}
        impl<T> Eligible for phf::$name<T> {}
        impl<T> Len for phf::$name<T> {
            fn len(&self) -> usize { phf::$name::len(self) }
        }
        impl<T> ElementType for phf::$name<T> { type Item = T; }
        impl<T> Elements for phf::$name<T> {
            type Iter<'a> = phf::$module::Iter<'a, T> where Self: 'a;
            fn elements(&self) -> Self::Iter<'_> { self.iter() }
        }
        impl<T, Q: ?Sized + Eq + PhfHash> SetContains<Q> for phf::$name<T>
        where T: PhfEq<Q> {
            fn contains(&self, query: &Q) -> bool { phf::$name::contains(self, query) }
        }
        impl<T, Q: ?Sized + Eq + PhfHash> SetGet<Q> for phf::$name<T>
        where T: PhfEq<Q> {
            fn get(&self, query: &Q) -> Option<&T> { self.get_key(query) }
        }
        impl<T> FreezeWith<Retain> for phf::$name<T> {
            type Output = Frozen<Self>;
            fn freeze_with(self, _: Retain) -> Self::Output { Frozen::new_unchecked(self) }
        }
        impl<T> Frozen<phf::$name<T>> {
            /// Test membership using the backend's equality relation.
            pub fn contains<Q: ?Sized + Eq + PhfHash>(&self, query: &Q) -> bool
            where T: PhfEq<Q> { self.backend().contains(query) }
            /// Borrow the stored representative.
            pub fn get<Q: ?Sized + Eq + PhfHash>(&self, query: &Q) -> Option<&T>
            where T: PhfEq<Q> { self.backend().get_key(query) }
            /// Traverse elements in the backend's order.
            pub fn iter(&self) -> phf::$module::Iter<'_, T> { self.backend().iter() }
        }
    )*};
}
sets! { Set, set; OrderedSet, ordered_set; }

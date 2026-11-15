//! Adapters for persistent collections from `imbl`.
//!
//! These collections share immutable nodes. A mutation of an independently owned
//! clone uses copy-on-write, so retaining that clone cannot mutate frozen shape.

use core::borrow::Borrow;
use core::hash::Hash;
use imbl::shared_ptr::DefaultSharedPtr;

use crate::frozen::sealed::Sealed;
use crate::read::{
    DequeRead, ElementType, Elements, Entries, Len, LookupRef, MapTypes, OrderedElements,
    OrderedEntries, SetContains, SetGet, ValueRef,
};
use crate::write::{MapInsert, MapRemove, Push, SetInsert, SetRemove};
use crate::{Eligible, FreezeWith, Frozen, Retain};

macro_rules! map_adapter {
    ($map:ident) => {
        impl<K, V> Sealed for imbl::$map<K, V> {}
        impl<K, V> Eligible for imbl::$map<K, V> {}

        impl<K, V> FreezeWith<Retain> for imbl::$map<K, V> {
            type Output = Frozen<Self>;

            fn freeze_with(self, _: Retain) -> Self::Output {
                Frozen::new_unchecked(self)
            }
        }

        impl<K, V> Len for imbl::$map<K, V> {
            fn len(&self) -> usize {
                imbl::$map::len(self)
            }
        }

        impl<K, V> MapTypes for imbl::$map<K, V> {
            type Key = K;
            type Value = V;
        }

        impl<K, V> Frozen<imbl::$map<K, V>> {
            /// Returns the value associated with a borrowed key.
            pub fn get<Q: ?Sized>(&self, query: &Q) -> Option<&V>
            where
                imbl::$map<K, V>: LookupRef<Q, Key = K, Value = V>,
            {
                LookupRef::get(self.backend(), query)
            }

            /// Returns the stored key and its value for a borrowed key.
            pub fn get_key_value<Q: ?Sized>(&self, query: &Q) -> Option<(&K, &V)>
            where
                imbl::$map<K, V>: LookupRef<Q, Key = K, Value = V>,
            {
                LookupRef::get_key_value(self.backend(), query)
            }

            /// Reports whether a borrowed key is present.
            pub fn contains_key<Q: ?Sized>(&self, query: &Q) -> bool
            where
                imbl::$map<K, V>: LookupRef<Q, Key = K, Value = V>,
            {
                LookupRef::contains_key(self.backend(), query)
            }
        }
    };
}

map_adapter!(HashMap);
map_adapter!(OrdMap);

impl<K, V> Frozen<imbl::HashMap<K, V>> {
    /// Iterates over stored keys and values in the backend's native order.
    pub fn iter(&self) -> imbl::hashmap::Iter<'_, K, V, DefaultSharedPtr> {
        self.backend().iter()
    }
}

impl<K: Ord, V> Frozen<imbl::OrdMap<K, V>> {
    /// Iterates over stored keys and values in ascending key order.
    pub fn iter(&self) -> imbl::ordmap::Iter<'_, K, V, DefaultSharedPtr> {
        self.backend().iter()
    }
}

impl<K, V> Entries for imbl::HashMap<K, V> {
    type Iter<'a>
        = imbl::hashmap::Iter<'a, K, V, DefaultSharedPtr>
    where
        Self: 'a;

    fn entries(&self) -> Self::Iter<'_> {
        imbl::HashMap::iter(self)
    }
}

impl<K: Ord, V> Entries for imbl::OrdMap<K, V> {
    type Iter<'a>
        = imbl::ordmap::Iter<'a, K, V, DefaultSharedPtr>
    where
        Self: 'a;

    fn entries(&self) -> Self::Iter<'_> {
        imbl::OrdMap::iter(self)
    }
}

impl<K, V, Q: ?Sized> LookupRef<Q> for imbl::HashMap<K, V>
where
    K: Borrow<Q> + Eq + Hash,
    Q: Eq + Hash,
{
    fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> {
        imbl::HashMap::get_key_value(self, query)
    }
}

impl<K, V, Q: ?Sized> LookupRef<Q> for imbl::OrdMap<K, V>
where
    K: Borrow<Q> + Ord,
    Q: Ord,
{
    fn get_key_value(&self, query: &Q) -> Option<(&K, &V)> {
        imbl::OrdMap::get_key_value(self, query)
    }
}

impl<K: Clone + Eq + Hash, V: Clone> MapInsert for imbl::HashMap<K, V> {
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        // Native imbl insertion replaces the stored key as well. The capability
        // contract retains it, so an occupied entry must update only its value.
        if let Some(stored) = self.get_mut(&key) {
            Some(core::mem::replace(stored, value))
        } else {
            imbl::HashMap::insert(self, key, value)
        }
    }
}

impl<K: Clone + Ord, V: Clone> MapInsert for imbl::OrdMap<K, V> {
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(stored) = self.get_mut(&key) {
            Some(core::mem::replace(stored, value))
        } else {
            imbl::OrdMap::insert(self, key, value)
        }
    }
}

impl<K, V, Q: ?Sized> MapRemove<Q> for imbl::HashMap<K, V>
where
    K: Borrow<Q> + Clone + Eq + Hash,
    V: Clone,
    Q: Eq + Hash,
{
    fn remove_entry(&mut self, query: &Q) -> Option<(K, V)> {
        self.remove_with_key(query)
    }
}

impl<K, V, Q: ?Sized> MapRemove<Q> for imbl::OrdMap<K, V>
where
    K: Borrow<Q> + Clone + Ord,
    V: Clone,
    Q: Ord,
{
    fn remove_entry(&mut self, query: &Q) -> Option<(K, V)> {
        self.remove_with_key(query)
    }
}

impl<K: Ord, V> OrderedEntries for imbl::OrdMap<K, V> {
    fn first_key_value(&self) -> Option<(&K, &V)> {
        self.get_min().map(|(key, value)| (key, value))
    }

    fn last_key_value(&self) -> Option<(&K, &V)> {
        self.get_max().map(|(key, value)| (key, value))
    }
}

macro_rules! set_adapter {
    ($set:ident) => {
        impl<T> Sealed for imbl::$set<T> {}
        impl<T> Eligible for imbl::$set<T> {}

        impl<T> FreezeWith<Retain> for imbl::$set<T> {
            type Output = Frozen<Self>;

            fn freeze_with(self, _: Retain) -> Self::Output {
                Frozen::new_unchecked(self)
            }
        }

        impl<T> Len for imbl::$set<T> {
            fn len(&self) -> usize {
                imbl::$set::len(self)
            }
        }

        impl<T> ElementType for imbl::$set<T> {
            type Item = T;
        }

        impl<T> Frozen<imbl::$set<T>> {
            /// Reports whether a borrowed value is present.
            pub fn contains<Q: ?Sized>(&self, query: &Q) -> bool
            where
                imbl::$set<T>: SetContains<Q>,
            {
                SetContains::contains(self.backend(), query)
            }
        }
    };
}

set_adapter!(HashSet);
set_adapter!(OrdSet);

impl<T: Clone + Eq + Hash> SetInsert for imbl::HashSet<T> {
    fn insert(&mut self, value: T) -> bool {
        // Native insertion replaces the equivalent representative. Preserve
        // the first representative required by Frost's mutation capability.
        if self.contains(&value) {
            false
        } else {
            imbl::HashSet::insert(self, value);
            true
        }
    }
}

impl<T: Clone + Ord> SetInsert for imbl::OrdSet<T> {
    fn insert(&mut self, value: T) -> bool {
        if self.contains(&value) {
            false
        } else {
            imbl::OrdSet::insert(self, value);
            true
        }
    }
}

impl<T, Q: ?Sized> SetRemove<Q> for imbl::HashSet<T>
where
    T: Borrow<Q> + Clone + Eq + Hash,
    Q: Eq + Hash,
{
    fn remove(&mut self, query: &Q) -> bool {
        imbl::HashSet::remove(self, query).is_some()
    }
}

impl<T, Q: ?Sized> SetRemove<Q> for imbl::OrdSet<T>
where
    T: Borrow<Q> + Clone + Ord,
    Q: Ord,
{
    fn remove(&mut self, query: &Q) -> bool {
        imbl::OrdSet::remove(self, query).is_some()
    }
}

impl<T> Frozen<imbl::HashSet<T>> {
    /// Iterates over stored values in the backend's native order.
    #[expect(
        clippy::iter_without_into_iter,
        reason = "native iteration needs no element bounds; borrowed IntoIterator requires Hash + Eq"
    )]
    pub fn iter(&self) -> imbl::hashset::Iter<'_, T, DefaultSharedPtr> {
        self.backend().iter()
    }
}

impl<T: Ord> Frozen<imbl::OrdSet<T>> {
    /// Iterates over stored values in ascending order.
    pub fn iter(&self) -> imbl::ordset::Iter<'_, T, DefaultSharedPtr> {
        self.backend().iter()
    }
}

// Native HashSet::iter has weaker bounds than its borrowed IntoIterator impl;
// using the named iterator preserves those native read capabilities.
impl<T> Elements for imbl::HashSet<T> {
    type Iter<'a>
        = imbl::hashset::Iter<'a, T, DefaultSharedPtr>
    where
        Self: 'a;

    fn elements(&self) -> Self::Iter<'_> {
        imbl::HashSet::iter(self)
    }
}

impl<T: Ord> Elements for imbl::OrdSet<T> {
    type Iter<'a>
        = imbl::ordset::Iter<'a, T, DefaultSharedPtr>
    where
        Self: 'a;

    fn elements(&self) -> Self::Iter<'_> {
        imbl::OrdSet::iter(self)
    }
}

impl<T, Q: ?Sized> SetContains<Q> for imbl::HashSet<T>
where
    T: Borrow<Q> + Eq + Hash,
    Q: Eq + Hash,
{
    fn contains(&self, query: &Q) -> bool {
        imbl::HashSet::contains(self, query)
    }
}

// imbl 7 exposes HashSet::contains, but no stored-representative lookup.
// Deliberately omit SetGet instead of hiding a linear scan in a hash lookup.

impl<T, Q: ?Sized> SetContains<Q> for imbl::OrdSet<T>
where
    T: Borrow<Q> + Ord,
    Q: Ord,
{
    fn contains(&self, query: &Q) -> bool {
        imbl::OrdSet::contains(self, query)
    }
}

impl<T, Q: ?Sized> SetGet<Q> for imbl::OrdSet<T>
where
    T: Borrow<Q> + Ord,
    Q: Ord,
{
    fn get(&self, query: &Q) -> Option<&T> {
        imbl::OrdSet::get(self, query)
    }
}

impl<T: Ord> OrderedElements for imbl::OrdSet<T> {
    fn first_element(&self) -> Option<&T> {
        self.get_min()
    }

    fn last_element(&self) -> Option<&T> {
        self.get_max()
    }
}

impl<T> Frozen<imbl::OrdSet<T>> {
    /// Returns the stored value equivalent to the borrowed value.
    pub fn get<Q: ?Sized>(&self, query: &Q) -> Option<&T>
    where
        imbl::OrdSet<T>: SetGet<Q, Item = T>,
    {
        SetGet::get(self.backend(), query)
    }
}

impl<T> Sealed for imbl::Vector<T> {}
impl<T> Eligible for imbl::Vector<T> {}

impl<T> FreezeWith<Retain> for imbl::Vector<T> {
    type Output = Frozen<Self>;

    fn freeze_with(self, _: Retain) -> Self::Output {
        Frozen::new_unchecked(self)
    }
}

impl<T> Len for imbl::Vector<T> {
    fn len(&self) -> usize {
        imbl::Vector::len(self)
    }
}

impl<T> ElementType for imbl::Vector<T> {
    type Item = T;
}

impl<T> Elements for imbl::Vector<T> {
    type Iter<'a>
        = <&'a Self as IntoIterator>::IntoIter
    where
        Self: 'a;

    fn elements(&self) -> Self::Iter<'_> {
        self.into_iter()
    }
}

impl<T> ValueRef<usize> for imbl::Vector<T> {
    type Value = T;

    fn get_ref(&self, index: &usize) -> Option<&T> {
        self.get(*index)
    }
}

impl<T> DequeRead for imbl::Vector<T> {
    fn front(&self) -> Option<&T> {
        imbl::Vector::front(self)
    }

    fn back(&self) -> Option<&T> {
        imbl::Vector::back(self)
    }
}

impl<T: Clone> Push for imbl::Vector<T> {
    type Item = T;

    fn push(&mut self, item: T) {
        self.push_back(item);
    }
}

impl<T> Frozen<imbl::Vector<T>> {
    /// Returns the element at an index, if present.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.backend().get(index)
    }

    /// Iterates over elements in sequence order.
    pub fn iter(&self) -> <imbl::Vector<T> as Elements>::Iter<'_> {
        self.backend().elements()
    }

    /// Returns the first element, if present.
    pub fn front(&self) -> Option<&T> {
        self.backend().front()
    }

    /// Returns the last element, if present.
    pub fn back(&self) -> Option<&T> {
        self.backend().back()
    }
}

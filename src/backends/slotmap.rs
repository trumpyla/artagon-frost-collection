//! Generational maps use copied keys rather than references to stored keys.
//!
//! `ValueRef` preserves each map's own generation checks. The reference-key map
//! traits intentionally do not apply. Creating keys remains a native backend
//! operation. A `SecondaryMap` has independent membership: removing a key from
//! its originating primary map does not remove the secondary value or invalidate
//! that key in the secondary map.

use crate::frozen::sealed::Sealed;
use crate::read::{Len, MapTypes, ValueRef};
use crate::{Eligible, FreezeWith, Frozen, Retain};

macro_rules! slotmap_adapter {
    ($map:ident) => {
        impl<K: slotmap::Key, V> Sealed for slotmap::$map<K, V> {}
        impl<K: slotmap::Key, V> Eligible for slotmap::$map<K, V> {}

        impl<K: slotmap::Key, V> FreezeWith<Retain> for slotmap::$map<K, V> {
            type Output = Frozen<Self>;

            fn freeze_with(self, _: Retain) -> Self::Output {
                Frozen::new_unchecked(self)
            }
        }

        impl<K: slotmap::Key, V> Len for slotmap::$map<K, V> {
            fn len(&self) -> usize {
                slotmap::$map::len(self)
            }
        }

        impl<K: slotmap::Key, V> MapTypes for slotmap::$map<K, V> {
            type Key = K;
            type Value = V;
        }

        impl<K: slotmap::Key, V> ValueRef<K> for slotmap::$map<K, V> {
            type Value = V;

            fn get_ref(&self, key: &K) -> Option<&V> {
                slotmap::$map::get(self, *key)
            }
        }

        impl<K: slotmap::Key, V> Frozen<slotmap::$map<K, V>> {
            /// Returns a value if the key matches a generation stored in this map.
            ///
            /// Secondary maps check their own membership; they do not track
            /// whether the key still exists in its originating primary map.
            pub fn get(&self, key: K) -> Option<&V> {
                self.backend().get(key)
            }

            /// Reports whether the key matches a generation stored in this map.
            ///
            /// Secondary membership is independent of the originating primary map.
            pub fn contains_key(&self, key: K) -> bool {
                self.backend().contains_key(key)
            }

            /// Iterates over copied generational keys and borrowed values.
            pub fn iter(&self) -> <&slotmap::$map<K, V> as IntoIterator>::IntoIter {
                self.backend().into_iter()
            }
        }
    };
}

slotmap_adapter!(SlotMap);
slotmap_adapter!(DenseSlotMap);
slotmap_adapter!(SecondaryMap);

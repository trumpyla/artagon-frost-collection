//! `serde` 1.x (feature `serde-1`).
//!
//! * Sequences encode as sequences; maps as maps in the family's traversal
//!   order; sets as sequences. Ordered families therefore encode
//!   identically for every representation. Hash families and profiles encode
//!   in **unspecified** order (as `HashMap` does), which may differ between
//!   two equal maps; `borsh-1` provides canonical member/key ordering.
//! * Default decoding uses the family builder: duplicate keys keep the first
//!   key representative and the last value (what serde's own `HashMap` /
//!   `BTreeMap` impls produce via `insert`).
//! * Decoding reconstructs the type's default policy and, for hash families,
//!   `S::default()`. Runtime hasher state and adaptive thresholds are not encoded.
//! * Decoding stages the complete input before construction or strict
//!   validation. The cautious initial size-hint allocation is not a total
//!   resource limit; callers that need a budget must bound their input.
//! * Strict decoding is opt-in per field:
//!   * [`unique`] rejects duplicate keys;
//!   * [`sorted_unique`] additionally requires canonical (strictly
//!     ascending) input for ordered families and validates in O(n).
//!
//! ```
//! # extern crate frost_collections_core as frost;
//! # use frost::prelude::*;
//! #[derive(serde::Deserialize)]
//! struct Config {
//!     #[serde(deserialize_with = "frost::adapters::serde::unique::deserialize")]
//!     limits: FrozenOrderedMap<String, u32>,
//! }
//! let ok: Config = serde_json::from_str(r#"{"limits":{"b":2,"a":1}}"#).unwrap();
//! assert_eq!(ok.limits.first_key_value(), Some((&"a".to_owned(), &1)));
//! let dup = serde_json::from_str::<Config>(r#"{"limits":{"a":1,"a":2}}"#);
//! assert!(dup.is_err());
//! ```

use alloc::vec::Vec;
use core::fmt;
use core::marker::PhantomData;

use ::serde::de::{self, MapAccess, SeqAccess, Visitor};
use ::serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::adaptive::{AdaptiveOrderedMap, InlineFrozenVec, InlineVec};
use crate::family::{
    DuplicateKey, FrozenOrderedMap, FrozenOrderedSet, FrozenVec, MutableOrderedMap,
    MutableOrderedSet, MutableVec,
};
use crate::{Eligible, Frozen};

/// Cap on pre-allocation from untrusted size hints (bytes).
const MAX_PREALLOC_BYTES: usize = 1 << 20;

fn cautious<T>(hint: Option<usize>) -> usize {
    let per = core::mem::size_of::<T>().max(1);
    hint.unwrap_or(0).min(MAX_PREALLOC_BYTES / per)
}

/// Collect a serde map into `Vec<(K, V)>` preserving input order.
struct EntriesVisitor<K, V>(PhantomData<(K, V)>);

impl<'de, K: Deserialize<'de>, V: Deserialize<'de>> Visitor<'de> for EntriesVisitor<K, V> {
    type Value = Vec<(K, V)>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a map")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut out = Vec::with_capacity(cautious::<(K, V)>(access.size_hint()));
        while let Some(entry) = access.next_entry()? {
            out.push(entry);
        }
        Ok(out)
    }
}

fn entries<'de, D, K, V>(d: D) -> Result<Vec<(K, V)>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de>,
    V: Deserialize<'de>,
{
    d.deserialize_map(EntriesVisitor(PhantomData))
}

/// Collect a serde sequence into `Vec<T>` with a cautious pre-allocation.
struct ElementsVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for ElementsVisitor<T> {
    type Value = Vec<T>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a sequence")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut out = Vec::with_capacity(cautious::<T>(access.size_hint()));
        while let Some(x) = access.next_element()? {
            out.push(x);
        }
        Ok(out)
    }
}

fn elements<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Vec<T>, D::Error> {
    d.deserialize_seq(ElementsVisitor(PhantomData))
}

// ---------------------------------------------------------------------------
// Exact frozen wrapper: transparent, eligibility-gated decoding.
// ---------------------------------------------------------------------------

impl<B: Serialize> Serialize for Frozen<B> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.backend().serialize(s)
    }
}

/// Decodes with the **backend's** semantics (exact-wrapper rule).
impl<'de, B: Eligible + Deserialize<'de>> Deserialize<'de> for Frozen<B> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        B::deserialize(d).map(Frozen::new_unchecked)
    }
}

// ---------------------------------------------------------------------------
// Sequences
// ---------------------------------------------------------------------------

macro_rules! seq_serde {
    ($([$($g:tt)*] $ty:ty),* $(,)?) => {$(
        impl<$($g)*> Serialize for $ty
        where
            T: Serialize,
        {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_seq(self.as_slice())
            }
        }
        impl<'de, $($g)*> Deserialize<'de> for $ty
        where
            T: Deserialize<'de>,
        {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                elements::<D, T>(d).map(Into::into)
            }
        }
    )*};
}

seq_serde! {
    [T] MutableVec<T>,
    [T] FrozenVec<T>,
    [T, const N: usize] InlineVec<T, N>,
}

impl<T: Serialize, const N: usize> Serialize for InlineFrozenVec<T, N> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.as_slice())
    }
}
impl<'de, T: Deserialize<'de>, const N: usize> Deserialize<'de> for InlineFrozenVec<T, N> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        elements::<D, T>(d).map(InlineFrozenVec::from_vec)
    }
}

// ---------------------------------------------------------------------------
// Maps and sets
// ---------------------------------------------------------------------------

macro_rules! map_serde {
    ($([$($g:tt)*] $ty:ty where [$($b:tt)*]),* $(,)?) => {$(
        impl<$($g)*> Serialize for $ty
        where
            K: Serialize,
            V: Serialize,
        {
            fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
                s.collect_map(self.iter())
            }
        }
        /// Family builder: first key representative, last value.
        impl<'de, $($g)*> Deserialize<'de> for $ty
        where
            K: Deserialize<'de>,
            V: Deserialize<'de>,
            $($b)*
        {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                entries::<D, K, V>(d).map(|v| v.into_iter().collect())
            }
        }
    )*};
}

macro_rules! set_serde {
    ($([$($g:tt)*] $ty:ty where [$($b:tt)*]),* $(,)?) => {$(
        impl<$($g)*> Serialize for $ty
        where T: Serialize,
        {
            fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
                serializer.collect_seq(self.iter())
            }
        }
        impl<'de, $($g)*> Deserialize<'de> for $ty
        where T: Deserialize<'de>, $($b)*
        {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                elements::<D, T>(deserializer).map(|values| values.into_iter().collect())
            }
        }
    )*};
}

map_serde! {
    [K, V] MutableOrderedMap<K, V> where [K: Ord,],
    [K, V] FrozenOrderedMap<K, V> where [K: Ord,],
    [K, V] AdaptiveOrderedMap<K, V> where [K: Ord,],
}

#[cfg(feature = "std")]
mod hash_impls {
    use super::{Deserialize, Deserializer, Serialize, Serializer, elements, entries};
    use crate::adaptive::{AdaptiveFrozenHashMap, AdaptiveHashMap};
    use crate::family::{FrozenHashMap, FrozenHashSet, MutableHashMap, MutableHashSet};
    use core::hash::{BuildHasher, Hash};

    map_serde! {
        [K, V, S] MutableHashMap<K, V, S> where [K: Eq + Hash, S: BuildHasher + Default,],
        [K, V, S] FrozenHashMap<K, V, S> where [K: Eq + Hash, S: BuildHasher + Default,],
        [K, V, S] AdaptiveHashMap<K, V, S> where [K: Eq + Hash, S: BuildHasher + Clone + Default,],
        [K, V, S] AdaptiveFrozenHashMap<K, V, S> where [K: Eq + Hash, S: BuildHasher + Clone + Default,],
    }
    set_serde! {
        [T, S] MutableHashSet<T, S> where [T: Eq + Hash, S: BuildHasher + Default,],
        [T, S] FrozenHashSet<T, S> where [T: Eq + Hash, S: BuildHasher + Default,],
    }
}

set_serde! {
    [T] FrozenOrderedSet<T> where [T: Ord,],
    [T] MutableOrderedSet<T> where [T: Ord,],
}

// ---------------------------------------------------------------------------
// Strict decoding
// ---------------------------------------------------------------------------

/// Map families that can be built strictly (rejecting duplicate keys).
///
/// Sealed: implemented for every map family and profile in this crate.
pub trait StrictEntries: Sized + private::Sealed {
    /// Key type.
    type Key;
    /// Value type.
    type Value;
    /// Build, rejecting the first duplicate key.
    ///
    /// # Errors
    /// [`DuplicateKey`] carrying the rejected entry.
    fn try_from_unique_entries(
        entries: Vec<(Self::Key, Self::Value)>,
    ) -> Result<Self, DuplicateKey<Self::Key, Self::Value>>;
}

mod private {
    pub trait Sealed {}
}

macro_rules! strict {
    ($([$($g:tt)*] $ty:ty where [$($b:tt)*] => |$v:ident| $build:expr),* $(,)?) => {$(
        impl<$($g)*> private::Sealed for $ty where $($b)* {}
        impl<$($g)*> StrictEntries for $ty where $($b)* {
            type Key = K;
            type Value = V;
            fn try_from_unique_entries($v: Vec<(K, V)>) -> Result<Self, DuplicateKey<K, V>> {
                $build
            }
        }
    )*};
}

strict! {
    [K, V] FrozenOrderedMap<K, V> where [K: Ord] => |v| FrozenOrderedMap::try_from_unique(v),
    [K, V] MutableOrderedMap<K, V> where [K: Ord] => |v| FrozenOrderedMap::try_from_unique(v).map(FrozenOrderedMap::thaw),
    [K, V] AdaptiveOrderedMap<K, V> where [K: Ord] => |v| FrozenOrderedMap::try_from_unique(v)
        .map(|f| AdaptiveOrderedMap::from_frozen(f, crate::adaptive::AdaptiveOrdered::default())),
}

#[cfg(feature = "std")]
mod strict_hash {
    use super::{DuplicateKey, StrictEntries, Vec, private};
    use crate::adaptive::{AdaptiveFrozenHashMap, AdaptiveHash, AdaptiveHashMap};
    use crate::family::{FrozenHashMap, MutableHashMap};
    use core::hash::{BuildHasher, Hash};

    strict! {
        [K, V, S] FrozenHashMap<K, V, S> where [K: Eq + Hash, S: BuildHasher + Default]
            => |v| FrozenHashMap::try_from_unique(v),
        [K, V, S] MutableHashMap<K, V, S> where [K: Eq + Hash, S: BuildHasher + Default]
            => |v| FrozenHashMap::try_from_unique(v).map(FrozenHashMap::thaw),
        [K, V, S] AdaptiveFrozenHashMap<K, V, S> where [K: Eq + Hash, S: BuildHasher + Clone + Default]
            => |v| FrozenHashMap::<K, V, S>::try_from_unique(v)
                .map(|f| AdaptiveFrozenHashMap::from_map(f.into_hash_map(), AdaptiveHash::default())),
        [K, V, S] AdaptiveHashMap<K, V, S> where [K: Eq + Hash, S: BuildHasher + Clone + Default]
            => |v| FrozenHashMap::<K, V, S>::try_from_unique(v)
                .map(|f| AdaptiveFrozenHashMap::from_map(f.into_hash_map(), AdaptiveHash::default()).thaw()),
    }
}

/// `#[serde(deserialize_with = "frost::adapters::serde::unique::deserialize")]`:
/// reject duplicate keys instead of applying the last-value policy.
pub mod unique {
    use super::{Deserialize, Deserializer, StrictEntries, de, entries};

    /// Strict map decoding for any map family.
    ///
    /// # Errors
    /// A decoding error, or a custom error on the first duplicate key.
    pub fn deserialize<'de, D, M>(d: D) -> Result<M, D::Error>
    where
        D: Deserializer<'de>,
        M: StrictEntries,
        M::Key: Deserialize<'de>,
        M::Value: Deserialize<'de>,
    {
        let v = entries::<D, M::Key, M::Value>(d)?;
        M::try_from_unique_entries(v).map_err(|_| de::Error::custom("duplicate key in map"))
    }
}

/// `#[serde(deserialize_with = "frost::adapters::serde::sorted_unique::deserialize")]`
/// for ordered families: require canonical (strictly ascending) input.
/// Validates in O(n) without sorting.
pub mod sorted_unique {
    use super::{Deserialize, Deserializer, de, elements, entries};
    use crate::family::{FrozenOrderedMap, FrozenOrderedSet};

    /// Ordered families with a canonical (strictly ascending) encoding.
    /// Sealed.
    pub trait Canonical<'de>: Sized + super::private::Sealed {
        #[doc(hidden)]
        fn decode<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error>;
    }

    impl<'de, K: Ord + Deserialize<'de>, V: Deserialize<'de>> Canonical<'de>
        for FrozenOrderedMap<K, V>
    {
        fn decode<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let v = entries::<D, K, V>(d)?;
            FrozenOrderedMap::try_from_sorted_unique(v).map_err(|r| de::Error::custom(r.error))
        }
    }

    impl<T: Ord> super::private::Sealed for FrozenOrderedSet<T> {}
    impl<T: Ord> super::private::Sealed for crate::family::MutableOrderedSet<T> {}

    impl<'de, K: Ord + Deserialize<'de>, V: Deserialize<'de>> Canonical<'de>
        for crate::family::MutableOrderedMap<K, V>
    {
        fn decode<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            FrozenOrderedMap::decode(d).map(FrozenOrderedMap::thaw)
        }
    }

    impl<'de, K: Ord + Deserialize<'de>, V: Deserialize<'de>> Canonical<'de>
        for crate::adaptive::AdaptiveOrderedMap<K, V>
    {
        fn decode<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            FrozenOrderedMap::decode(d).map(|f| {
                crate::adaptive::AdaptiveOrderedMap::from_frozen(
                    f,
                    crate::adaptive::AdaptiveOrdered::default(),
                )
            })
        }
    }

    impl<'de, T: Ord + Deserialize<'de>> Canonical<'de> for FrozenOrderedSet<T> {
        fn decode<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let v = elements::<D, T>(d)?;
            FrozenOrderedSet::try_from_sorted_unique(v).map_err(|r| de::Error::custom(r.error))
        }
    }

    impl<'de, T: Ord + Deserialize<'de>> Canonical<'de> for crate::family::MutableOrderedSet<T> {
        fn decode<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            FrozenOrderedSet::decode(d).map(FrozenOrderedSet::thaw)
        }
    }

    /// Strict canonical decoding.
    ///
    /// # Errors
    /// A decoding error, or a custom error naming the first out-of-order index.
    pub fn deserialize<'de, D: Deserializer<'de>, C: Canonical<'de>>(d: D) -> Result<C, D::Error> {
        C::decode(d)
    }
}

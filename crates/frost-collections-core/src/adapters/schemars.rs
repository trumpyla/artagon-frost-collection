//! `schemars` 1.x (feature `schemars-1`): JSON Schema matching the serde
//! encoding. Every family forwards to the std type with the same logical
//! encoding (`Vec<T>`, `BTreeMap<K, V>`, `BTreeSet<T>`), so schemas never
//! depend on the representation and never change when it does.

use alloc::borrow::Cow;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use ::schemars::{JsonSchema, Schema, SchemaGenerator};

use crate::adaptive::{AdaptiveOrderedMap, InlineFrozenVec, InlineVec};
use crate::family::{
    FrozenOrderedMap, FrozenOrderedSet, FrozenVec, MutableOrderedMap, MutableOrderedSet, MutableVec,
};
use crate::{Eligible, Frozen};

macro_rules! forward_schema {
    ($([$($g:tt)*] $ty:ty => $target:ty),* $(,)?) => {$(
        impl<$($g)*> JsonSchema for $ty {
            fn inline_schema() -> bool {
                <$target as JsonSchema>::inline_schema()
            }
            fn schema_name() -> Cow<'static, str> {
                <$target as JsonSchema>::schema_name()
            }
            fn schema_id() -> Cow<'static, str> {
                <$target as JsonSchema>::schema_id()
            }
            fn json_schema(generator: &mut SchemaGenerator) -> Schema {
                <$target as JsonSchema>::json_schema(generator)
            }
        }
    )*};
}

forward_schema! {
    [B: Eligible + JsonSchema] Frozen<B> => B,
    [T: JsonSchema] MutableVec<T> => Vec<T>,
    [T: JsonSchema] FrozenVec<T> => Vec<T>,
    [T: JsonSchema, const N: usize] InlineVec<T, N> => Vec<T>,
    [T: JsonSchema, const N: usize] InlineFrozenVec<T, N> => Vec<T>,
    [T: JsonSchema] FrozenOrderedSet<T> => BTreeSet<T>,
    [T: JsonSchema] MutableOrderedSet<T> => BTreeSet<T>,
    [K: JsonSchema, V: JsonSchema] MutableOrderedMap<K, V> => BTreeMap<K, V>,
    [K: JsonSchema, V: JsonSchema] FrozenOrderedMap<K, V> => BTreeMap<K, V>,
    [K: JsonSchema, V: JsonSchema] AdaptiveOrderedMap<K, V> => BTreeMap<K, V>,
}

#[cfg(feature = "std")]
mod hash_impls {
    use super::{BTreeMap, BTreeSet, Cow, JsonSchema, Schema, SchemaGenerator};
    use crate::adaptive::{AdaptiveFrozenHashMap, AdaptiveHashMap};
    use crate::family::{FrozenHashMap, FrozenHashSet, MutableHashMap, MutableHashSet};

    forward_schema! {
        [K: JsonSchema, V: JsonSchema, S] MutableHashMap<K, V, S> => BTreeMap<K, V>,
        [K: JsonSchema, V: JsonSchema, S] FrozenHashMap<K, V, S> => BTreeMap<K, V>,
        [K: JsonSchema, V: JsonSchema, S] AdaptiveHashMap<K, V, S> => BTreeMap<K, V>,
        [K: JsonSchema, V: JsonSchema, S] AdaptiveFrozenHashMap<K, V, S> => BTreeMap<K, V>,
        [T: JsonSchema, S] MutableHashSet<T, S> => BTreeSet<T>,
        [T: JsonSchema, S] FrozenHashSet<T, S> => BTreeSet<T>,
    }
}

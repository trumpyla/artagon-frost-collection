//! *Hash-map* sub-category profiles: a linear `(K, V)` table while small, a
//! hash table when large.
//!
//! One enum (`Linear`, `Table`), one contract: `K: Borrow<Q> + Eq + Hash`
//! lookup under the map's `S`, unspecified order, mapping equality, hasher
//! preserved in both variants. Mutable [`AdaptiveHashMap`] and frozen
//! [`AdaptiveFrozenHashMap`] share iterator types.

mod frozen;
mod mutable;

pub use frozen::{AdaptiveFrozenHashMap, AdaptiveHash, HashRepr, Iter};
pub use mutable::{AdaptiveHashMap, IntoIter, IterMut};

#[cfg(feature = "rayon-1")]
pub(crate) use frozen::Repr as FrozenRepr;
#[cfg(feature = "rayon-1")]
pub(crate) use mutable::Repr as MutableRepr;

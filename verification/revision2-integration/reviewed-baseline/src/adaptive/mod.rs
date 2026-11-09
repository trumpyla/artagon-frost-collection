//! Adaptive profiles: one public type, a **fixed** private set of
//! representations chosen at construction, one shared query contract.
//!
//! Rules every profile follows:
//! * The representation set is closed. Enabling a Cargo feature never adds a
//!   variant (and so never changes size, auto traits or bounds).
//! * All variants answer queries under the profile's single contract.
//! * Equality and hashing are logical, never derived from the enum; `Debug`
//!   prints the logical entries (in unspecified order for hash profiles).
//! * Bulk read operations can dispatch once before traversing a representation;
//!   mutable operations may also cross a promotion threshold.
//! * Thresholds are policy *inputs*; the shipped defaults are uncalibrated
//!   placeholders until benchmarked on target workloads.

//!
//! There is **one enum per sub-category**, never one per category: a
//! sequence, a hash map and an ordered map have different bounds, query
//! relations and traversal contracts, so a single `Map` enum would have to
//! demand the union of all bounds and could promise none of the contracts.
//!
//! | Sub-category | Variants | Mutable | Frozen |
//! |---|---|---|---|
//! | [`seq`] | `Inline` / `Heap` | [`InlineVec`] | [`InlineFrozenVec`] |
//! | `hash_map` (requires `std`) | `Linear` / `Table` | `AdaptiveHashMap` | `AdaptiveFrozenHashMap` |
//! | [`ordered_map`] | `Sorted` / `Tree` | [`AdaptiveOrderedMap`] | [`FrozenOrderedMap`](crate::family::FrozenOrderedMap) (no enum needed) |

pub mod ordered_map;
pub mod seq;

#[cfg(feature = "std")]
pub mod hash_map;

pub use ordered_map::{AdaptiveOrdered, AdaptiveOrderedMap, OrderedRepr};
pub use seq::{Inline, InlineFrozenVec, InlineVec};

#[cfg(feature = "std")]
pub use hash_map::{AdaptiveFrozenHashMap, AdaptiveHash, AdaptiveHashMap, HashRepr};

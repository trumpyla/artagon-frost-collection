//! Frozen integer sets with domain types and private, distribution-aware storage.
//!
//! Enable `int-set` for the complete profile: sorted codes, bounded dense
//! windows, the existing Rust Roaring backend, and explicitly selected chunked
//! storage with shared containers. This feature supports `no_std + alloc`;
//! enabling `std` also enables the dependency's supported runtime CPU dispatch.
//! Shared storage uses `Arc` on targets with pointer-width atomics and `Rc` on
//! targets without them. It does not alter the ordinary ordered family's
//! freeze/thaw partner.
//!
//! ```
//! use frost::integer::{FrozenIntSet, IntSetPolicy};
//! let left = FrozenIntSet::try_from_iter([-3_i64, -1, 0, 1], IntSetPolicy::auto()).unwrap();
//! let right: FrozenIntSet<i64> = [-1, 1, 7].into_iter().collect();
//! assert_eq!(left.intersection(&right).iter().collect::<Vec<_>>(), [-1, 1]);
//! assert_eq!(left.intersection_len(&right), 2);
//! assert_eq!(left.bounds(), Some((-3, 1)));
//! ```
//!
//! Domain types do not mix implicitly, even when their encoded words match:
//!
//! ```compile_fail,E0308
//! use frost::integer::{FrozenIntSet, IntKey};
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
//! struct UserId(u64);
//! impl IntKey for UserId {
//!     type Word = u64;
//!     fn encode(self) -> u64 { self.0 }
//!     fn decode(word: u64) -> Option<Self> { Some(Self(word)) }
//! }
//! let users: FrozenIntSet<UserId> = [UserId(1)].into_iter().collect();
//! let numbers: FrozenIntSet<u64> = [1].into_iter().collect();
//! let invalid = users.union(&numbers);
//! ```
//!
//! The set is structurally frozen and has no mutable element view:
//!
//! ```compile_fail,E0599
//! use frost::integer::FrozenIntSet;
//! let mut set: FrozenIntSet<u32> = [1, 2].into_iter().collect();
//! set.insert(3);
//! ```

// Keep the allocating profile available on targets without atomic pointers.
// No manual auto-trait implementations: the chosen pointer's traits apply.
#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Shared;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc as Shared;

mod chunked;
mod dense;
mod key;
mod policy;
mod set;
mod storage;

pub use key::{IntKey, IntWord};
pub use policy::{
    IntSetBackend, IntSetError, IntSetLimits, IntSetPolicy, IntSetRepr, IntSetStatistics,
};
pub use set::{EncodedValues, FrozenIntSet, IntSetIter};

//! Value domains with order-preserving encodings, and interval sets and maps
//! over them. The base trait is [`Domain`].
//!
//! Three ideas carry the design:
//!
//! 1. **The domain is the type.** A value type declares its extrema, density,
//!    and encoding as associated items. Collections hold no domain data.
//! 2. **Collections work on words.** Every ordered domain has an order-preserving
//!    encoding into a word type (`u64` for `i64`, `NotNanF64`, a newtype of
//!    `u64`, ...). The set and map kernels in [`kernel`] see sorted words only,
//!    so all domains with the same word share one copy of the kernel code.
//! 3. **Capabilities are traits.** `count()` needs [`Ranked`]; member
//!    iteration needs [`DiscreteDomain`]. The cost of a method is known from
//!    its bounds.
//!
//! The design record is `docs/design/proposals/frozen-box.md` in the repository.
//!
//! ```
//! use frost_interval::{FrozenIntervalSet, Interval, OrderedDomainExt, Cut};
//! let q: FrozenIntervalSet<u8> = FrozenIntervalSet::of(1..=3);
//! // Complement stays inside the domain of the type, 0..=255.
//! assert_eq!(q.complement().closed_intervals(), vec![(0, 0), (4, 255)]);
//! assert_eq!(q.complement().count(), 253);
//! // Every interval is canonical, so equal intervals compare equal.
//! assert_eq!(Interval::closed(2u8, 4), Interval::open(1u8, 5));
//! // The set stores words, not values.
//! assert_eq!(q.words(), &[1u32, 4]);
//! assert_eq!(u8::min_cut(), Cut::Below(0));
//! ```
//!
//! Domains with equal words do not mix:
//!
//! ```compile_fail,E0308
//! use frost_interval::*;
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
//! struct UserId(u64);
//! newtype_domain!(UserId(u64): discrete, cover_preserving);
//! let users: FrozenIntervalSet<UserId> = FrozenIntervalSet::of(UserId(10)..);
//! let numbers: FrozenIntervalSet<u64> = FrozenIntervalSet::of(10..);
//! let _ = users.union(&numbers);
//! ```
//!
//! A domain with no order claim has no intervals:
//!
//! ```compile_fail,E0599
//! use frost_interval::{Domain, OrderedDomainExt};
//! #[derive(Clone, PartialEq, Eq)]
//! enum Color { Red, Blue }
//! impl Domain for Color {}
//! let _ = Color::closed(Color::Red, Color::Blue);
//! ```
//!
//! A discrete domain cannot forget `succ`: `Density = Discrete` requires a
//! `DiscreteDomain` impl.
//!
//! ```compile_fail,E0277
//! use frost_interval::*;
//! #[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
//! struct Forgot(u32);
//! impl Domain for Forgot {}
//! impl OrderedDomain for Forgot {
//!     type Density = Discrete;
//!     identity_encoding!();
//! }
//! ```
//!
//! Counting needs a rank. A discrete domain without one has no `count()`,
//! so no call can walk every member by accident:
//!
//! ```compile_fail,E0599
//! use frost_interval::FrozenIntervalSet;
//! let s: FrozenIntervalSet<i128> = FrozenIntervalSet::of(0..10);
//! let _ = s.count();
//! ```
//!
//! A member query on a domain with a missing extremum fails at build time:
//!
//! ```compile_fail,E0080
//! use frost_interval::*;
//! #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
//! struct NoMax(u32);
//! impl Domain for NoMax {}
//! impl OrderedDomain for NoMax {
//!     type Density = Discrete;
//!     const MIN: Option<Self> = Some(NoMax(0));
//!     identity_encoding!();
//! }
//! impl DiscreteDomain for NoMax {
//!     fn succ(&self) -> Option<Self> { self.0.checked_add(1).map(NoMax) }
//!     fn pred(&self) -> Option<Self> { self.0.checked_sub(1).map(NoMax) }
//! }
//! let s: FrozenIntervalSet<NoMax> = FrozenIntervalSet::of(NoMax(5)..);
//! let _ = s.closed_intervals();
//! ```

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod cut;
pub mod domain;
#[cfg(feature = "chrono-0_4")]
mod ext_chrono;
#[cfg(feature = "jiff-0_2")]
mod ext_jiff;
#[cfg(feature = "time-0_3")]
mod ext_time;
pub mod finite;
pub mod float;
pub mod interval;
pub mod interval_set;
pub mod kernel;
pub mod map;
pub mod order;
pub mod region;
pub mod set;
mod std_domains;
mod temporal;
pub mod tree;

pub use cut::Cut;
pub use domain::{
    BoundariesOf, BoundedLattice, CoverPreserving, CyclicDomain, Dense, Density, Discrete,
    DiscreteDomain, Domain, Extrema, FiniteDomain, Lattice, MachineWord, OrderedDomain,
    OrderedDomainExt, Poset, PrefixDomain, Ranked,
};
pub use finite::FiniteSet;
pub use float::{NanError, NotNanF32, NotNanF64};
pub use interval::{Interval, IntoInterval};
pub use interval_set::IntervalSet;
pub use map::FrozenIntervalMap;
pub use order::DownSet;
pub use region::{BooleanRegion, Convex, CyclicInterval, LatticeInterval, ProductRegion, Region};
pub use set::FrozenIntervalSet;

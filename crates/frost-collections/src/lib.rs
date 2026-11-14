#![doc = include_str!("../README.md")]
//!
//! # Internal packages
//!
//! `frost` re-exports the internal packages `frost-collections-core`,
//! `frost-collections-integer` and `frost-collections-concurrent`. Depend on
//! `frost-collections` only. The hidden interface those packages share is not
//! reachable from here:
//!
//! ```compile_fail,E0432
//! use frost::__private;
//! ```
#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// The facade holds no algorithm. It names every re-export and does not
// re-export the core package's hidden `__private` module.
pub use frost_collections_core::{adapters, plan, read, write};
#[cfg(feature = "alloc")]
pub use frost_collections_core::{adaptive, family, iter, mutable};

#[cfg(feature = "std")]
pub use frost_collections_concurrent::concurrent;

#[cfg(feature = "int-set")]
pub use frost_collections_integer::integer;

pub use frost_collections_core::{Compact, ExactLen, Retain};
pub use frost_collections_core::{Eligible, Frozen};
pub use frost_collections_core::{FreezeExt, FreezeWith, Rejected, TryFreezeWith};

/// Public backend and framework dependencies, gated by versioned features.
///
/// Use these reexports to ensure that the native collection and its Frost
/// adapter use the same dependency version. Frozen bitmap adapters also expose
/// set algebra and result-free count queries. Specialized operations, such as
/// FST automata and native concurrent entry APIs, remain available on the native
/// backend. Their synchronization and ordering
/// contracts are documented by the corresponding upstream crate.
pub mod backend {
    #[cfg(feature = "arc-swap-1")]
    pub use frost_collections_concurrent::backend::arc_swap;
    #[doc(inline)]
    #[allow(
        unused_imports,
        unreachable_pub,
        reason = "the glob is empty when no backend or framework feature is enabled"
    )]
    pub use frost_collections_core::backend::*;
}

/// Glob-importable names for everyday use.
pub mod prelude {
    #[doc(inline)]
    pub use frost_collections_core::prelude::*;

    #[cfg(feature = "int-set")]
    pub use crate::integer::{FrozenIntSet, IntKey, IntSetPolicy};
}

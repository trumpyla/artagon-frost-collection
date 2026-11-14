//! Concurrent access for the Frost collection library: the [`concurrent`]
//! module with live guarded lookups and whole-snapshot publication.
//!
//! This package is internal. Depend on `frost-collections`, whose library
//! `frost` re-exports [`concurrent`] at the same path, and read its
//! documentation. The examples in this package's documentation are written
//! against `frost`.
//!
//! The root mirrors the paths of `frost`: it re-exports the core package, the
//! integer profile under `int-set`, and adds the `concurrent` module, so code
//! and tests here use the same paths as users of the facade.
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Only the snapshot store allocates.
#[cfg(feature = "arc-swap-1")]
extern crate alloc;

#[doc(no_inline)]
pub use frost_collections_core::*;

#[cfg(feature = "int-set")]
#[doc(no_inline)]
pub use frost_collections_integer::integer;

pub mod concurrent;

/// Public backend and framework dependencies, gated by versioned features:
/// the core package's re-exports and `arc_swap`.
pub mod backend {
    #[cfg(feature = "arc-swap-1")]
    pub use arc_swap;
    #[doc(no_inline)]
    #[allow(
        unused_imports,
        unreachable_pub,
        reason = "the glob is empty when no backend or framework feature is enabled"
    )]
    pub use frost_collections_core::backend::*;
}

/// Glob-importable names for everyday use: the core prelude and, under
/// `int-set`, the integer profile's main types.
pub mod prelude {
    #[doc(no_inline)]
    pub use frost_collections_core::prelude::*;

    #[cfg(feature = "int-set")]
    pub use crate::integer::{FrozenIntSet, IntKey, IntSetPolicy};
}

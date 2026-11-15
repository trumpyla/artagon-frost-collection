//! The typed integer-set profile of the Frost collection library: the
//! [`integer`] module with its key codec, policies and private storage.
//!
//! This package is internal. Depend on `frost-collections`, whose library
//! `frost` re-exports [`integer`] at the same path, and read its
//! documentation. The examples in this package's documentation are written
//! against `frost`.
//!
//! The root mirrors the paths of `frost`: it re-exports the core package and
//! adds the `integer` module, so code and tests here use the same paths as
//! users of the facade.
#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

#[doc(no_inline)]
pub use frost_collections_core::*;

pub mod integer;

/// Glob-importable names for everyday use: the core prelude and the integer
/// profile's main types.
pub mod prelude {
    #[doc(no_inline)]
    pub use frost_collections_core::prelude::*;

    pub use crate::integer::{FrozenIntSet, IntKey, IntSetPolicy};
}

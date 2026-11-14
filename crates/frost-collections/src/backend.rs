//! Public backend and framework dependencies, gated by versioned features.
//!
//! Use these reexports to ensure that the native collection and its Frost
//! adapter use the same dependency version. Frozen bitmap adapters also expose
//! set algebra and result-free count queries. Specialized operations, such as
//! FST automata and native concurrent entry APIs, remain available on the native
//! backend. Their synchronization and ordering
//! contracts are documented by the corresponding upstream crate.

#[cfg(feature = "fixedbitset-0_5")]
pub use crate::backends::FixedBitSetValues;

#[cfg(feature = "arbitrary-1")]
pub use arbitrary;
#[cfg(feature = "arc-swap-1")]
pub use arc_swap;
#[cfg(feature = "arrayvec-0_7")]
pub use arrayvec;
#[cfg(feature = "borsh-1")]
pub use borsh;
#[cfg(feature = "crossbeam-skiplist-0_1")]
pub use crossbeam_skiplist;
#[cfg(feature = "dashmap-6")]
pub use dashmap;
#[cfg(feature = "fixedbitset-0_5")]
pub use fixedbitset;
#[cfg(feature = "fst-0_4")]
pub use fst;
#[cfg(feature = "hashbrown-0_17")]
pub use hashbrown;
#[cfg(feature = "imbl-7")]
pub use imbl;
#[cfg(feature = "indexmap-2")]
pub use indexmap;
#[cfg(feature = "phf-0_14")]
pub use phf;
#[cfg(feature = "rayon-1")]
pub use rayon;
#[cfg(any(feature = "roaring-0_12", feature = "int-set"))]
pub use roaring;
#[cfg(feature = "scc-3")]
pub use scc;
#[cfg(feature = "schemars-1")]
pub use schemars;
#[cfg(feature = "serde-1")]
pub use serde;
#[cfg(feature = "slotmap-1")]
pub use slotmap;
#[cfg(feature = "smallvec-1")]
pub use smallvec;

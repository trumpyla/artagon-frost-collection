//! Audited backend adapters. Each module:
//! 1. marks the backend [`Eligible`](crate::Eligible),
//! 2. implements the read capabilities with the backend's *own* query bounds,
//! 3. implements the freeze plans that make sense for it, and
//! 4. adds inherent read methods on `Frozen<Backend>`.

mod array;

#[cfg(feature = "alloc")]
mod std_seq;

#[cfg(feature = "alloc")]
mod std_btree;

#[cfg(feature = "std")]
mod std_hash;

#[cfg(feature = "hashbrown-0_17")]
mod hashbrown;

#[cfg(feature = "indexmap-2")]
mod indexmap;

#[cfg(feature = "arrayvec-0_7")]
mod arrayvec;
#[cfg(feature = "fst-0_4")]
mod fst;
#[cfg(feature = "phf-0_14")]
mod phf;
#[cfg(feature = "smallvec-1")]
mod smallvec;

#[cfg(any(
    feature = "roaring-0_12",
    feature = "int-set",
    feature = "fixedbitset-0_5"
))]
mod bitmap;
#[cfg(feature = "imbl-7")]
mod persistent;
#[cfg(feature = "fixedbitset-0_5")]
pub use bitmap::FixedBitSetValues;
#[cfg(feature = "slotmap-1")]
mod slotmap;

// Capability traits for the live concurrent backends. Their guarded access
// traits and snapshot publication are in the concurrent package; the
// implementations of this crate's traits for those foreign types must be here.
#[cfg(feature = "dashmap-6")]
mod dashmap;
#[cfg(feature = "scc-3")]
mod scc;
#[cfg(feature = "crossbeam-skiplist-0_1")]
mod skiplist;

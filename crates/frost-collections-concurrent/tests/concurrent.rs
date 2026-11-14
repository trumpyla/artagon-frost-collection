//! Live backend contracts and frozen snapshot publication tests.

// The package under test, under the name its users import.
extern crate frost_collections_concurrent as frost;

#[path = "concurrent/live.rs"]
mod live;

#[cfg(feature = "arc-swap-1")]
#[path = "concurrent/snapshot.rs"]
mod snapshot;

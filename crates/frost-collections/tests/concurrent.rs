//! Live backend contracts and frozen snapshot publication tests.

#[path = "concurrent/live.rs"]
mod live;

#[cfg(feature = "arc-swap-1")]
#[path = "concurrent/snapshot.rs"]
mod snapshot;

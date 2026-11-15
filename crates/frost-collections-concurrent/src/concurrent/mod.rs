//! Live concurrent collections and atomic publication of frozen snapshots.
//!
//! Concurrent maps expose [`ConcurrentLookup`] and, where the backend can lend
//! an exclusive value reference, [`ConcurrentUpdate`]. Concurrent sets expose
//! [`ConcurrentSetLookup`] and [`crate::read::SetContains`]. These capabilities
//! observe one entry at a time. They do not promise a consistent traversal or a
//! transaction across separate calls.
//!
//! Keep live-entry callbacks short. In particular, do not reenter the same
//! collection, wait for another thread that might use it, or perform blocking
//! work inside a callback. Hash-based backends may hold a shard or bucket lock
//! until the callback returns. A panic releases the guard during unwinding,
//! but does not roll back changes already made to a value.
//!
//! Insertion, removal, async access, and traversal remain native backend APIs;
//! their contracts differ too much for a common replacement operation. Live
//! backends deliberately do not implement `LookupRef`, `Entries`, or `Eligible`.
//! With `arc-swap-1`, `SnapshotStore` publishes complete frozen collections and
//! lets readers retain one guarded or owned version across multiple queries.

mod live;

pub use live::{ConcurrentLookup, ConcurrentSetLookup, ConcurrentUpdate};

#[cfg(feature = "dashmap-6")]
mod dashmap;
#[cfg(feature = "scc-3")]
mod scc;
#[cfg(feature = "crossbeam-skiplist-0_1")]
mod skiplist;

#[cfg(feature = "arc-swap-1")]
mod publication;
#[cfg(feature = "arc-swap-1")]
mod snapshot;
#[cfg(feature = "arc-swap-1")]
pub use snapshot::{SnapshotGuard, SnapshotStore, SnapshotValue};

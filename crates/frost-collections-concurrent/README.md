# frost-collections-concurrent

frost-collections-concurrent provides live guarded access to concurrent
collections, and whole-snapshot publication, for Frost. The package always
needs `std`, and it forbids unsafe code.

This package is internal. Users depend on
[frost-collections](../frost-collections/README.md) only.

## Contents

- Live access traits: `ConcurrentLookup`, `ConcurrentUpdate` and
  `ConcurrentSetLookup`. They have implementations for `dashmap`, `scc` and
  `crossbeam-skiplist` types. Each backend has its own feature.
- Snapshot publication: `SnapshotStore`, `SnapshotGuard` and the sealed
  `SnapshotValue`. These need the `arc-swap-1` feature.
- The capability-trait implementations (`MapTypes`, `ElementType` and
  `SetContains`) for the backend types are in `frost-collections-core`.
  Rust's orphan rule requires that placement.

## Features

The `alloc` and `std` features are empty. The default feature is `std`.

- `dashmap-6`, `scc-3` and `crossbeam-skiplist-0_1` enable one backend
  each, and the matching feature of `frost-collections-core`.
- `arc-swap-1` enables snapshot publication.
- `int-set` enables `frost-collections-core/int-set` and
  `frost-collections-integer`.
- The other features forward to `frost-collections-core` under the same
  names: `hashbrown-0_17`, `indexmap-2`, `serde-1`, `rayon-1`, `arbitrary-1`,
  `borsh-1`, `schemars-1`, `smallvec-1`, `arrayvec-0_7`, `imbl-7`,
  `roaring-0_12`, `fixedbitset-0_5`, `slotmap-1`, `phf-0_14` and `fst-0_4`.

## Tests, benches and harnesses

- `tests/` holds the tests. `publication_model.rs` includes
  `src/concurrent/publication.rs` by path.
- `benches/snapshots.rs` is the snapshot benchmark. It requires
  `arc-swap-1`.
- `verification/loom` is the Loom package. `make loom` runs it.
- The test catalog is `verification/metadata/catalog.json`. The bench
  inventory is `verification/metadata/bench-inventory.json`.

Run the package tests with:

    cargo test -p frost-collections-concurrent --all-features --locked

The root `Makefile` is the only build and verification entry point. Run
`make help` to list its targets.

## Documents

- [snapshots.md](docs/snapshots.md): Snapshot publication contract

Repository documents:

- [CRATES.md](../../CRATES.md): Repository crate rules
- [README.md](../../docs/README.md): Documentation index
- [crate-split-plan.md](../../docs/design/crate-split-plan.md): Crate split
  plan

# frost-collections-core

frost-collections-core is the core package of Frost. It holds the
capability traits, freeze plans, the mutable and frozen collection
families and their set algebra, adaptive profiles, native backends and
framework adapters. The package is `no_std`.

This package is internal. Users depend on
[frost-collections](../frost-collections/README.md) only.

## Contents

- `read` and `write`: capability traits.
- `plan`: freeze plans.
- `frozen`: `Frozen` and its sealed `Eligible` trait.
- `family`: the five mutable and frozen collection families and their set
  algebra.
- `adaptive`: adaptive profiles and inline sequences.
- `backend`: native backends.
- `adapters`: framework integrations for serde, borsh, rayon, arbitrary and
  schemars.

The hidden module `__private` is used by the other Frost packages. It is
not public API.

The workspace denies `unsafe_code`. The only exception is
`src/adaptive/seq/buf.rs`.

## Features

The default feature is `std`. The other features are:

`alloc`, `std`, `hashbrown-0_17`, `indexmap-2`, `serde-1`, `rayon-1`,
`arbitrary-1`, `borsh-1`, `schemars-1`, `smallvec-1`, `arrayvec-0_7`,
`imbl-7`, `roaring-0_12`, `int-set`, `fixedbitset-0_5`, `slotmap-1`,
`phf-0_14`, `fst-0_4`, `dashmap-6`, `scc-3` and `crossbeam-skiplist-0_1`.

Features are additive. Enabling one never changes the representation or
the bounds of an existing type. The `int-set` feature keeps the Roaring
adapter available under that name. The integer profile itself lives in
`frost-collections-integer`.

## Tests, benches and harnesses

- `tests/` holds the tests. The finite formal models are in
  `tests/formal_models/`.
- `benches/` holds four benchmark targets: `adaptive_maps`,
  `ordered_sets`, `performance_review` and `set_operations`.
- `verification/kani` is the Kani proof package. `make bounded` runs its
  native helper tests. `make kani` runs the solver.
- `verification/verus` holds the Verus proofs. `make verus` runs them.
- The test catalog is `verification/metadata/catalog.json`. The bench
  inventory is `verification/metadata/bench-inventory.json`.

Run the package tests with:

    cargo test -p frost-collections-core --all-features --locked

The root `Makefile` is the only build and verification entry point. Run
`make help` to list its targets.

## Documents

- [frozen-api.md](docs/frozen-api.md): Frozen API guide
- [set-algebra.md](docs/set-algebra.md): Set algebra across all Frost
  set families
- [set-operations.md](docs/set-operations.md): Frozen set operations

Repository documents:

- [CRATES.md](../../CRATES.md): Repository crate rules
- [README.md](../../docs/README.md): Documentation index
- [crate-split-plan.md](../../docs/design/crate-split-plan.md): Crate split
  plan

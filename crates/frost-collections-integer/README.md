# frost-collections-integer

frost-collections-integer provides the typed integer-set profile of Frost.
It holds `FrozenIntSet`, the `IntKey` codec, `IntSetPolicy` and its limits,
and private storage. The storage has sorted, dense, Roaring and explicit
chunked policies. The package is `no_std`, always uses `alloc`, and forbids
unsafe code.

This package is internal. Users depend on
[frost-collections](../frost-collections/README.md) only.

## Contents

- The `integer` module: `FrozenIntSet`, the `IntKey` codec, `IntSetPolicy`
  and its limits.
- Private storage for the sorted, dense, Roaring and explicit chunked
  policies.
- The crate root re-exports `frost-collections-core`. So
  `frost_collections_integer::family` and `frost::family` name the same
  items. Tests and doctests alias this package as `frost`.

## Features

The package always uses `alloc` and always provides the integer profile.
The `alloc` and `int-set` features are empty, so that other packages can
name them. The `std` feature forwards to `frost-collections-core` and
`roaring`. The other features forward to `frost-collections-core` under the
same names:

`hashbrown-0_17`, `indexmap-2`, `serde-1`, `rayon-1`, `arbitrary-1`,
`borsh-1`, `schemars-1`, `smallvec-1`, `arrayvec-0_7`, `imbl-7`,
`roaring-0_12`, `fixedbitset-0_5`, `slotmap-1`, `phf-0_14`, `fst-0_4`,
`dashmap-6`, `scc-3` and `crossbeam-skiplist-0_1`.

The default feature is `std`.

## Tests, benches and harnesses

- `tests/` holds the tests.
- `benches/` holds the `integer_sets` and `integer_output` targets. The
  `integer_output` target requires `std`.
- `fuzz/` is the fuzz package. It has two targets, `ordered_set_algebra`
  and `integer_set_algebra`. `make fuzz-check` runs its library tests.
  `make fuzz-smoke` runs libFuzzer.
- The test catalog is `verification/metadata/catalog.json`. The bench
  inventory is `verification/metadata/bench-inventory.json`.

Run the package tests with:

    cargo test -p frost-collections-integer --all-features --locked

The root `Makefile` is the only build and verification entry point. Run
`make help` to list its targets.

## Documents

- [integer-benchmarks.md](docs/integer-benchmarks.md): Integer-profile
  benchmark evidence
- [integer-optimization-validation.md](docs/integer-optimization-validation.md):
  Integer and family optimizations: validation and measurements
- [integer-output-benchmarks.md](docs/integer-output-benchmarks.md): Integer
  output benchmark comparison
- [integer-output-validation.md](docs/integer-output-validation.md): Integer
  output validation
- [integer-roadmap.md](docs/integer-roadmap.md): Integer collection roadmap
  and admission criteria
- [integer-sets.md](docs/integer-sets.md): Frozen integer profile
- [integer-validation.md](docs/integer-validation.md): Integer profile:
  implementation and validation
- [integer-views.md](docs/integer-views.md): Borrowed integer expressions
- [persistent-integer-storage.md](docs/persistent-integer-storage.md):
  Persistent integer storage: an adversarial design

Repository documents:

- [CRATES.md](../../CRATES.md): Repository crate rules
- [README.md](../../docs/README.md): Documentation index
- [crate-split-plan.md](../../docs/design/crate-split-plan.md): Crate split
  plan

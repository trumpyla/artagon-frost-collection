# frost-collections-test-support

frost-collections-test-support holds support code shared by the tests and
benches of the other Frost packages. It has no dependencies, and it depends
on no Frost package. It is only used as a dev-dependency. The package
forbids unsafe code.

This package is internal. Users depend on
[frost-collections](../frost-collections/README.md) only.

## Contents

- `workload`: deterministic workload factories. They produce plain data
  with independent oracles.
- `bench`: dependency-free timing and listing helpers. Every benchmark
  target uses them.

## Tests, benches and harnesses

The package has no tests of its own and no benches. Its test catalog is
`verification/metadata/catalog.json`.

The rule that keeps this package a dev-dependency, and free of Frost
dependencies, is CR-BOUND-1 in [CRATES.md](../../CRATES.md).

## Documents

The package has no `docs/` directory.

Repository documents:

- [CRATES.md](../../CRATES.md): Repository crate rules
- [README.md](../../docs/README.md): Documentation index
- [crate-split-plan.md](../../docs/design/crate-split-plan.md): Crate split
  plan

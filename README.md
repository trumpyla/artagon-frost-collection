# Frost

Frost gives Rust applications dependable collection semantics as their storage
and execution needs change: mutable and frozen collection families, frozen set
algebra, typed integer sets, adaptive storage, and snapshot publication, with
explicit rules for representatives, traversal, ownership, and cost.

This repository is a Cargo workspace. The library is the package
`frost-collections`, imported as `frost`. Its
[README](crates/frost-collections/README.md) is the crate documentation:
start there for what Frost is, the contracts it keeps, the feature catalog,
and code examples.

**Target:** Rust 1.99.1, edition 2024, pinned by `rust-toolchain.toml`.
Nothing is published to a registry.

## Packages

| Package | Directory | Role |
| --- | --- | --- |
| `frost-collections` | [`crates/frost-collections`](crates/frost-collections/README.md) | The library and the only supported entry point. It re-exports the packages below and still holds the concurrent module |
| `frost-collections-core` | `crates/frost-collections-core` | Internal. Capability traits, plans, families, adaptive profiles, native backends, framework integrations |
| `frost-collections-integer` | `crates/frost-collections-integer` | Internal. The typed integer-set profile |

The library is being split into internal packages behind this one under
tracker epic `frost-wlt`; `frost-collections-concurrent` is still to come. The [crate split plan](docs/design/crate-split-plan.md) says what
moves where and when; this table lists a package once it exists. Depend on
`frost-collections` only: the other packages are internal.

## Working in the repository

| Read | For |
| --- | --- |
| [Documentation index](docs/README.md) | Design, contracts, verification, research |
| [CRATES.md](CRATES.md) | Package layout, dependency, feature, test and benchmark rules |
| [AGENTS.md](AGENTS.md) and [implementation contracts](docs/maintainers/implementation-contracts.md) | Rules for every change |
| [TOOLS.md](TOOLS.md) | Search and verification tools |
| [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md) | Contributions and reports |
| [CHANGELOG.md](CHANGELOG.md) | Changes |

The root `Makefile` is the only build and verification entry point:

```sh
make help     # list targets; runs nothing
make check    # formatting, links, Clippy, rustdoc, tests, examples, catalog
make verify   # check plus profiles, isolated features, integer lanes, examples, package test
```

## License

MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).

# Contributing

Install Rust 1.99.1 with Clippy and rustfmt. Python 3 and Bash support the
repository verification scripts; they are not library dependencies.

```sh
rustup toolchain install 1.99.1 --profile minimal --component clippy,rustfmt
./scripts/verify.sh check
```

Read [the design philosophy](docs/design/philosophy.md) before changing a public
family or feature. Use [the maintainer guide](docs/maintainers/guide.md) to find
the relevant implementation, invariants, and tests.

## Changes that are easy to review

A pull request should explain the concrete problem, the changed behavior, the
contracts retained, and the checks executed. Include a regression test for a
bug or a new public guarantee. Keep unrelated formatting and dependency updates
separate from semantic changes. There is no requirement to add tests for prose
edits or tests that only repeat an implementation.

Use the existing naming and capability traits. Prefer a private kernel over a
new public abstraction when only an implementation changes. New public methods
need rustdoc with ownership, panic, complexity, feature, and concurrency details
where relevant. Compile-fail examples are part of the API contract.

For a performance change, provide correct answers and a fair native baseline,
then raw measurements with the compiler, CPU, flags, workload shape, and what is
timed. Preserve regressions. See [benchmark guidance](docs/maintainers/benchmarks.md).

## Verification scopes

| Command | Purpose |
| --- | --- |
| `./scripts/verify.sh check` | Formatting, lints, docs, all-feature tests and examples |
| `./scripts/verify.sh profiles` | Core-only, alloc-only, and default profiles |
| `./scripts/verify.sh features` | Each optional integration independently |
| `./scripts/verify.sh integers` | Integer profile in debug/release and a separate downstream codec crate |
| `./scripts/verify.sh embedded` | Core and alloc-compatible bare-metal compilation |
| `./scripts/verify.sh miri` | Pinned-nightly ownership and aliasing diagnostics |
| `./scripts/verify.sh package` | Build and test the actual Cargo source package |
| `./scripts/verify-formal.sh bounded` | Exhaustive finite collections and retry model |
| `./scripts/verify-formal.sh loom` | Instrumented publication protocol model |
| `./scripts/verify-formal.sh kani` | Bounded symbolic proof harnesses; requires Kani |

CI results and local results are separate evidence. Do not write “proved” or
“production ready” merely because a test suite is green. See
[verification scope](docs/verification.md) and [release gates](docs/maintainers/releasing.md).

Contributions are provided under the existing MIT OR Apache-2.0 project license.
Security reports follow [SECURITY.md](SECURITY.md).

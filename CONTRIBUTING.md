# Contributing

Install Rust 1.99.1 with Clippy and rustfmt, GNU Make 3.81+, and Bash 3.2+.
Feature, example, and package discovery uses `jq` 1.6+. These are development
tools, separate from the library dependency graph; normal builds and checks
do not require Python.

```sh
rustup toolchain install 1.99.1 --profile minimal --component clippy,rustfmt
make check
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
| `make` or `make build` | Build the default library profile |
| `make check` | Formatting, lints, docs, all-feature tests and examples |
| `make verify` | Complete host checks, profiles, features, integers, examples, and package |
| `make profiles` | Core-only, alloc-only, and default profiles |
| `make features` | Each optional integration independently |
| `make integers` | Integer profile in debug/release and a separate downstream codec crate |
| `make examples` | Execute every example program |
| `make embedded` | Core and alloc-compatible bare-metal compilation, including integer storage with and without pointer atomics |
| `make miri` | Pinned-nightly ownership and aliasing diagnostics |
| `make package` | Build and test the actual Cargo source package |
| `make bounded` | Exhaustive finite collections and retry model |
| `make loom` | Instrumented publication protocol model using its committed lockfile |
| `make kani` | Bounded symbolic proof harnesses; requires Kani |
| `make shell-check` | ShellCheck, shfmt, and Bats behavior tests for shell/Make changes |
| `make tools-check` | Optional ast-grep rule tests and production scan |
| `make search-index` | Refresh the ignored local tgrep index |

`make help` is read-only and runs no Cargo commands. `all` is the default build
target; the broader release-review entry point is `verify`. Miri, embedded,
and formal tools are explicit targets and fail when prerequisites are absent.
`make -j check` may run independent checks concurrently; Cargo manages its
artifact locks. Keep benchmark measurements serial.

The Makefile delegates Rust freshness and compilation to Cargo. Command targets
are phony, metadata discovery happens only inside recipes, and compound recipes
stop on failures even with GNU Make 3.81. Override executable paths with
`CARGO=...`, `JQ=...`, or `SHELL=...`; set `MIRI_TOOLCHAIN=...` and
`EMBEDDED_TARGET=...` and `EMBEDDED_NO_ATOMICS_TARGET=...` explicitly for
diagnostic variants. The default embedded scope requires both
`thumbv7em-none-eabihf` and `thumbv6m-none-eabi`; install them with
`rustup target add thumbv7em-none-eabihf thumbv6m-none-eabi` before that gate.
`make clean` calls Cargo clean and does not remove verification evidence.

Shell changes use the Artagon shell-authoring, shell-testing, and shell-linting
guidance. The link checker is an executed Bash script with BSD/GNU `find`,
`mktemp`, and `rm`; it preserves filenames with spaces/newlines, checks percent
encoded local paths, and propagates discovery/read failures. Its supported
Markdown subset is inline links with a label and parenthesized path, matching
the prior checker.
Reference-style links, titles with whitespace, and anchor existence are outside
that checker's scope.

For shell/Make changes, install Bats, ShellCheck, and shfmt, then run:

```sh
make shell-check
make shell-test SHELL=/bin/bash  # exercises the system Bash 3.2 on macOS
```

The Bats tests use inert Cargo stubs to test Make control flow; they do not
replace `make check` against real Rust. Test both minimum/current interpreters
and Make versions when available, and report missing OS coverage explicitly.
Historical analysis scripts and their recorded commands remain evidence; the
current build/verification entry points are the Make targets above.

CI results and local results are separate evidence. Do not write “proved” or
“production ready” merely because a test suite is green. See
[verification scope](docs/verification.md) and [release gates](docs/maintainers/releasing.md).

Contributions are provided under the existing MIT OR Apache-2.0 project license.
Security reports follow [SECURITY.md](SECURITY.md).

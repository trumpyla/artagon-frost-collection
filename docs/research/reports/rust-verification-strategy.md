---
id: frost-research-report-rust-verification-strategy
kind: report
level: deep-dive
research_status: curated
implementation_status: mixed
source_validation: verified
input_files: [verification/crate-standards-validation/baseline-coverage.json, verification/crate-standards-validation/api-contracts.md]
input_members: []
input_sha256: [8814a2f7e58c8fd1a6ab253b2df6d5cc9bf1312dbbd3ffbe220994f150ca9990, 52cd0ef1c3dc12cbdcbf645e62067db9486534117fd89b372fd9c4943e1cf620]
origin_created_at: 2026-11-09
imported_at: 2026-11-09
repo_baseline: 6ab0918dbbf655894d8d3176b81091bd28899c5f
parent_ids: [frost-research-synthesis-foundations-and-engine-roadmap]
related_ids: [frost-research-report-collection-algebra-implementation-notes]
supersedes: []
open_questions: [Q-FROZEN-1, Q-ALG-1]
---

# Rust verification, coverage, and mutation strategy for Frost

## Executive decision brief

Frost's verification architecture under epic `frost-wou` combines seven
complementary evidence lanes:

1. **Deterministic contract, differential, and exhaustive finite-domain suites**
   executed via `cargo test` and `cargo-nextest` ([`.config/nextest.toml`](../../../.config/nextest.toml),
   `make check`, `make nextest`, `make bounded`).
2. **Seeded property-based testing** with `proptest 1.12` and committed
   `.proptest-regressions` seed persistence (`SRC-VERIFY-PROPTEST`).
3. **Isolated structure-aware differential fuzzing** in
   [`verification/fuzz`](../../../verification/fuzz/README.md) (`cargo-fuzz 0.13.2`,
   `libfuzzer-sys 0.4.13`, `arbitrary 1.5.0`) with a stable-toolchain corpus replay
   gate (`make fuzz-check`) and bounded nightly `libFuzzer` exploration
   (`make fuzz-smoke`, `SRC-VERIFY-CARGO-FUZZ`).
4. **Source-only LLVM coverage** via `cargo-llvm-cov 0.9.1` (`make coverage`,
   `make coverage-gate`, `make coverage-branch`, `SRC-VERIFY-LLVM-COV`) with a
   portable integer adoption floor (`77 / 70 / 77`) and a scoped exact
   non-regression ratchet anchored to the observed `aarch64-apple-darwin`
   baseline at commit `6ab0918dbbf655894d8d3176b81091bd28899c5f` (`77.197%`
   lines, `70.908%` functions, `77.772%` regions; ~`79.16% / 73.76% / 79.43%`
   with `tests/api_contracts.rs`), plus explicit follow-up Beads tasks
   (`frost-wou.6` through `frost-wou.10`) toward the [`CRATES.md`](../../../CRATES.md)
   §5 target (`>= 90%` lines, `>= 85%` functions, `>= 90%` regions;
   `instantiations` report-only; `branches` provisional until first measurement).
5. **Mutation testing and survivor triage** via `cargo-mutants`
   ([`.config/mutants.toml`](../../../.config/mutants.toml), `make mutants`,
   [`verification/mutation/README.md`](../../../verification/mutation/README.md),
   `SRC-VERIFY-CARGO-MUTANTS`).
6. **Dynamic UB and aliasing verification** via Miri (`nightly-2026-11-08`,
   Stacked Borrows and Tree Borrows with strict provenance, plus 32-bit
   `i686-unknown-linux-gnu` integer storage checks in `make miri`,
   `SRC-VERIFY-MIRI`).
7. **Concurrency schedule exploration and bounded symbolic proofs** via the
   shared production retry helper in [`tests/publication_model.rs`](../../../tests/publication_model.rs),
   isolated [`verification/loom`](../../../verification/loom/src/lib.rs)
   (`loom 0.7.3`, `SRC-VERIFY-LOOM`), and isolated
   [`verification/kani`](../../../verification/kani/src/lib.rs) (`Kani 0.68.0`,
   `SRC-VERIFY-KANI`). Deductive verifiers (`Verus`, `Creusot`) are evaluated
   and deferred until post-`frost-wlt` stabilization (`SRC-VERIFY-VERUS`,
   `SRC-VERIFY-CREUSOT`).

All optional tools stay outside the root [`Cargo.toml`](../../../Cargo.toml)
dependency graph, preserve `#![no_std]` and additive feature semantics, and
fail closed with a non-zero exit code when an optional binary is missing.

## Research question and inherited context

- **Epic linkage**: Standards and testing epic `frost-wou` defines the
  verification gates and crate-local target layout required before the
  workspace split in `frost-wlt`. No package migration is performed in
  `frost-wou`.
- **Baseline commit**: `6ab0918dbbf655894d8d3176b81091bd28899c5f` on branch
  `feat/crate-verification-standards`.
- **Host environment**: `aarch64-apple-darwin`, pinned Rust `1.99.1` (edition
  2024), diagnostic nightly `nightly-2026-11-08` with `llvm-tools-preview`,
  `cargo-llvm-cov 0.9.1`, `cargo-nextest 0.9.151`, and `cargo-fuzz 0.13.2`.
  `cargo-mutants` and `kani-verifier` are not installed on the local validation
  host and are reported honestly as unavailable local gates.

## Tool evaluation and applicability matrix

| Tool | Evaluated version | Role in Frost | Production linkage | Scope and limits | Disposition |
| --- | --- | --- | --- | --- | --- |
| `cargo-nextest` | `0.9.151` (`SRC-VERIFY-NEXTEST`) | Process-per-test runner with deterministic `retries = 0`, slow-timeout enforcement, and JUnit XML output | Executes root `Cargo.toml` unit, integration, and example targets | Does not run doctests (`cargo test --doc` remains separate in `make check` and `make miri`) | **Adopted** ([`.config/nextest.toml`](../../../.config/nextest.toml), `make nextest`) |
| `cargo-llvm-cov` | `0.9.1` (`SRC-VERIFY-LLVM-COV`) | Source-based LLVM coverage (`-C instrument-coverage`) over `src/**/*.rs` | Compiles `frost-collections` with `--all-features --locked` | Stable `1.99.1` measures lines, functions, regions, and instantiations; `--branch` and `--doctests` require nightly | **Adopted** (`make coverage`, `make coverage-gate`, `make coverage-branch`) |
| `cargo-mutants` | `25.3.1` CLI contract (`SRC-VERIFY-CARGO-MUTANTS`) | AST-driven fault injection in `src/**/*.rs` to audit assertion strength | Mutates `src/**/*.rs` against `--all-features --locked` test runs | Cannot prove equivalence of surviving fast-path mutants; requires human survivor triage | **Adopted** ([`.config/mutants.toml`](../../../.config/mutants.toml), `make mutants`) |
| `cargo-fuzz` / `libfuzzer-sys` | `0.13.2` / `0.4.13` (`SRC-VERIFY-CARGO-FUZZ`) | Structure-aware coverage-guided differential fuzzing with `arbitrary 1.5.0` | Isolated `[workspace]` in `verification/fuzz` depending on `frost-collections` by path | Sanitizer instrumentation requires nightly; `make fuzz-check` replays corpora on stable `1.99.1` | **Adopted** ([`verification/fuzz`](../../../verification/fuzz/README.md), `make fuzz-check`, `make fuzz-smoke`) |
| `proptest` | `1.12` (`SRC-VERIFY-PROPTEST`) | Property-based testing with automatic shrinking and `.proptest-regressions` seed persistence | Dev-dependency in root `Cargo.toml` | Default 256 cases sample the input space; does not replace finite exhaustive domains | **Retained & standardized** (`PROPTEST_CASES` scaling in nightly CI) |
| `Miri` | `nightly-2026-11-08` (`SRC-VERIFY-MIRI`) | Undefined-behavior, provenance, aliasing (Stacked + Tree Borrows), and 32-bit layout checker | Executes `src/adaptive/seq/buf.rs` and `src/integer/**` through public and unit tests | Interprets executed paths only; not a symbolic proof over all inputs | **Retained** (`make miri`) |
| `Loom` | `0.7.3` (`SRC-VERIFY-LOOM`) | Exhaustive C11-style schedule exploration of `publication::update` | Isolated `[workspace]` in `verification/loom` including `src/concurrent/publication.rs` via `#[path]` | Models retry protocol against a trusted `loom::sync::Mutex` store; does not verify `arc-swap` internals | **Retained** (`make loom`) |
| `Kani` | `0.68.0` (`SRC-VERIFY-KANI`) | Bit-precise CBMC bounded model checking of `InlineBuf`, ordered algebra, and bitmap padding | Isolated `[workspace]` in `verification/kani` using path dependency + `#[path]` on `src/adaptive/seq/buf.rs` | Requires compatible nightly compiler (`kani 0.68.0` bundles `nightly-2026-08-21` < MSRV `1.99.1`); native helpers run in `make bounded` | **Retained as explicit release gate** (`make kani`, [`.github/workflows/proofs.yml`](../../../.github/workflows/proofs.yml)) |
| `Verus` / `Creusot` | Current upstream (`SRC-VERIFY-VERUS`, `SRC-VERIFY-CREUSOT`) | Deductive SMT/Why3 verification of unbounded functional invariants | None | Both require custom toolchain pins and intrusive ghost/specification macros (`verus!`, Pearlite `#[ensures]`) incompatible with a zero-dependency `#![no_std]` library on stable Rust `1.99.1` | **Deferred** (re-evaluate for isolated kernel mirrors after `frost-wlt`). Superseded on 2026-11-13 for Verus at spec level: see [formal validation tools](formal-validation-tools.md) |

## Source coverage model, denominators, and baseline analysis

### Denominator and exclusion rules

1. **Source denominator**: Only files in `src/**/*.rs` enter the coverage
   denominator; `tests/`, `benches/`, `examples/`, and `verification/` are
   excluded via `--ignore-filename-regex '(^|/)(tests|benches|examples|verification)/'`.
   Because this regex filters by file path and `cargo llvm-cov --all-features --locked`
   compiles the library with `cfg(test)`, inline `#[cfg(test)]` modules inside
   `src/**/*.rs` (such as unit tests in `src/integer/**`) remain part of this
   file-scoped library-source denominator (not a test-free production-code
   denominator). Custom `harness = false` benchmark targets in `benches/` do not
   execute during default `cargo llvm-cov` runs.
2. **Feature and target exclusions**:
   - Host coverage runs use `--all-features --locked`. On 64-bit host targets
     (`aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`),
     `#[cfg(target_has_atomic = "ptr")]` is active; the non-atomic `Rc` fallback
     in `src/integer/mod.rs` is checked by `make embedded` on
     `thumbv6m-none-eabi` and 32-bit arithmetic is checked under Miri on
     `i686-unknown-linux-gnu`.
   - Production code must not use `#[coverage(off)]` to hide untested branches.
3. **Unsafe code coverage (`src/adaptive/seq/buf.rs`)**:
   - `src/adaptive/seq/buf.rs` is the sole module permitted to contain `unsafe`
     blocks (enforced by `Cargo.toml` lints and the `rust-unsafe-block-scope`
     rule in `verification/ast-grep`).
   - Baseline coverage in `buf.rs` is `19 / 22` functions (`86.36%`),
     `107 / 121` lines (`88.43%`), and `146 / 165` regions (`88.48%`). Any edit
     to `buf.rs` requires `make miri` (Stacked Borrows and Tree Borrows) and
     `make bounded` in addition to line/region coverage.

### Observed baseline (`6ab0918dbbf655894d8d3176b81091bd28899c5f`)

From [`verification/crate-standards-validation/baseline-coverage.json`](../../../verification/crate-standards-validation/baseline-coverage.json):

| Subsystem / Representative files | Lines covered / total (%) | Functions covered / total (%) | Regions covered / total (%) | Primary gap analysis |
| --- | --- | --- | --- | --- |
| **Crate total (`src/**`)** | **7,089 / 9,183 (77.20%)** | **1,343 / 1,894 (70.91%)** | **11,067 / 14,230 (77.77%)** | Pre-`api_contracts.rs` baseline; branches and doctests unmeasured on stable |
| Integer profile (`src/integer/**`) | 3,055 / 3,271 (93.40%) | 404 / 425 (95.06%) | 5,271 / 5,667 (93.01%) | High coverage from exhaustive 32,768-pair and output-policy suites |
| Set algebra kernels (`ordered/set_ops.rs`, `hash_set_ops.rs`, `backends/bitmap.rs`) | 833 / 844 (98.70%) | 141 / 142 (99.30%) | 1,289 / 1,310 (98.40%) | Exhaustive 6-element / 66,564-bitmap suites cover nearly all branches |
| Concurrency (`src/concurrent/**`) | 117 / 128 (91.41%) | 33 / 36 (91.67%) | 179 / 195 (91.79%) | Guarded lookup, live wrappers, and snapshot retry loop well covered |
| Inline unsafe buffer (`src/adaptive/seq/buf.rs`) | 107 / 121 (88.43%) | 19 / 22 (86.36%) | 146 / 165 (88.48%) | Covered by 33,614-trace exhaustive and Miri suites |
| Mutable sequence & ordered family (`vec/mutable.rs`, `ordered/mutable.rs`) | 183 / 465 (39.35%) | 49 / 131 (37.40%) | 237 / 617 (38.41%) | Addressed by new [`tests/api_contracts.rs`](../../../tests/api_contracts.rs) obligations |
| Capability traits & plans (`src/write.rs`, `src/plan.rs`, `src/iter.rs`) | 71 / 174 (40.80%) | 23 / 55 (41.82%) | 111 / 267 (41.57%) | Default trait methods and error display paths need direct contract tests |
| Native backend capability impls (`std_btree.rs`, `std_hash.rs`, `hashbrown.rs`, `indexmap.rs`) | 130 / 420 (30.95%) | 32 / 110 (29.09%) | 147 / 491 (29.94%) | Thin capability impls over native collections under-exercised in baseline |

### Portable adoption floor, scoped ratchet, and target stages

To respect `CR-COV-1` (a coverage number from `aarch64-apple-darwin` does not
prove the `x86_64-unknown-linux-gnu` baseline on Ubuntu CI) while preventing
both silent regressions and false completion claims, coverage enforcement in
[`verification/coverage/coverage-policy.json`](../../../verification/coverage/coverage-policy.json)
distinguishes:

1. **Portable integer adoption floor (`COVERAGE_MIN_* = 77 / 70 / 77`)**:
   `--fail-under-lines 77 --fail-under-functions 70 --fail-under-regions 77`.
   Because `77 / 70 / 77` is below the `aarch64-apple-darwin` `6ab0918` baseline
   (`77.197% / 70.908% / 77.772%`), it is a coarse cross-target adoption floor,
   not an exact non-regression check against `6ab0918`.
2. **Scoped exact non-regression ratchet (`aarch64-apple-darwin`)**:
   When `make coverage-gate` runs on `aarch64-apple-darwin` with the matching
   compiler (`rustc 1.99.1`), coverage tool (`cargo-llvm-cov 0.9.1`), target,
   and exclusion regex, it additionally verifies via `jq -e` that
   `source-summary.json` percentages (`lines`, `functions`, `regions`) do not
   drop below
   [`verification/crate-standards-validation/baseline-coverage.json`](../../../verification/crate-standards-validation/baseline-coverage.json),
   and writes both `target/coverage/source-summary.json` and
   `target/coverage/scope-metadata.json` even on failure. With the six
   `tests/api_contracts.rs` suites added in `frost-wou`, same-scope host
   coverage reaches approximately `79.16%` lines (`7,269 / 9,183`), `73.76%`
   functions (`1,397 / 1,894`), and `79.43%` regions (`11,303 / 14,230`) (still
   below the `90% / 85% / 90%` target; see the
   [recorded observation](../../../verification/crate-standards-validation/after-coverage.json)).
3. **Follow-up Beads adoption stages and [`CRATES.md`](../../../CRATES.md) §5 target standard**:
   Follow-up Beads tasks (`frost-wou.6` through `frost-wou.10`, preceding the
   multi-crate workspace split in `frost-wlt.1` through `frost-wlt.6`) raise the
   floor through `84 / 80 / 84` toward the target standard (`lines >= 90%`,
   `functions >= 85%`, `regions >= 90%`, `instantiations` report-only,
   `branches` provisional until first measurement), paired with 100% public API
   obligation catalog and live benchmark inventory gates (`make catalog`,
   `make catalog-inventory`).

## Public API semantic obligations versus structural coverage

Structural coverage cannot detect whether an operation returned the **wrong**
equivalent representative or leaked an element on early iterator drop. Frost
requires every public family and capability to be verified against six semantic
obligation classes (cataloged by `verification/catalog` and tested in
[`tests/api_contracts.rs`](../../../tests/api_contracts.rs),
[`tests/exhaustive_collections.rs`](../../../tests/exhaustive_collections.rs),
and [`verification/fuzz/src/lib.rs`](../../../verification/fuzz/src/lib.rs)):

1. **Representative identity**: First-key/first-element retention on
   construction and insertion; last-value retention on map construction; left
   representative retention on `union` and `intersection`.
2. **Freeze/thaw partner identity and capacity laws**: Consuming `freeze()` and
   `thaw()` return the exact named partner while preserving elements, hashers,
   and `Vec` zero-sized capacity conventions.
3. **Strict duplicate/sorted validation recovery**: `try_from_unique`,
   `try_from_sorted_unique`, and `try_freeze_with` reject invalid inputs and
   return the documented `DuplicateKey`, `DuplicateElement`, or `Rejected`
   payload.
4. **Iterator and partial-consumption ownership**: Exact `size_hint`,
   `DoubleEndedIterator` symmetry, `FusedIterator` termination, and single-drop
   accounting on partial `into_iter()` and `drain()`.
5. **Encoded integer domain separation and canonical hashing**: Checked `IntKey`
   imports, representation-independent `Eq` and `Hasher` event sequences, and
   `IntSetLimits` propagation (`min` for commutative operations, left for
   `difference`).
6. **Snapshot publication identity**: Unconditional `publish`, pointer-identity
   `publish_if_current`, and retrying `update` returning the exact displaced
   `Arc<F>` version.

## Concurrency and formal proof linkage

Neither concurrency models nor bounded proofs may verify a detached copy of an
algorithm:

- **Retry scheduler model ([`tests/publication_model.rs`](../../../tests/publication_model.rs))**:
  Includes [`src/concurrent/publication.rs`](../../../src/concurrent/publication.rs)
  directly and explores all 6 terminal schedules for 2 writers and all 198
  terminal schedules for 3 writers under noncommuting updates `4*x + k`,
  asserting the exact bound of `n + n(n+1)/2` atomic steps (at most 9 for 3
  writers).
- **Loom model ([`verification/loom/src/lib.rs`](../../../verification/loom/src/lib.rs))**:
  Includes `../../../src/concurrent/publication.rs` via `#[path]`, pins
  `loom = "=0.7.3"`, sets `max_branches = 256` as a hard failure ceiling, and
  clears `max_permutations`, `max_duration`, `preemption_bound`, and
  `checkpoint_file` so environment variables cannot silently truncate schedule
  exploration.
- **Kani bounded harnesses ([`verification/kani/src/lib.rs`](../../../verification/kani/src/lib.rs))**:
  Links `frost-collections` by path dependency and includes
  `../../../src/adaptive/seq/buf.rs` via `#[path]`. Sets workspace
  `default-unwind = "1"` and explicit per-harness `#[kani::unwind(N)]` bounds
  (`16` for `ordered_set_algebra`, `sorted_constructor_preserves_or_rejects_input`,
  `inline_prefix_preserved`, and `inline_transfer_drops_each_owner_once`; `4`
  for `inline_zero_capacity`; `32` for `dense_padding_does_not_create_members`).

## Evidence ledger and primary-source bibliography

All external tool documentation and local binary interfaces below were checked
on **2026-11-09 (UTC)**. Note: the GitHub MCP server in this environment
returned `401 Bad credentials`, so primary upstream documentation was verified
through the native web search lane and direct CLI/source inspection of the
installed binaries on `aarch64-apple-darwin`.

| ID | Primary source | Supports | Verification method & limits |
| --- | --- | --- | --- |
| `SRC-VERIFY-LLVM-COV` | `cargo-llvm-cov` `0.9.1` CLI (`cargo llvm-cov --help`) and upstream `taiki-e/cargo-llvm-cov` documentation | `--json --summary-only`, `--ignore-filename-regex`, `--fail-under-lines`, `--fail-under-functions`, `--fail-under-regions`, and nightly-only `--branch` / `--doctests` flags; automatic `cfg(coverage)` and `cfg(coverage_nightly)`. | Checked against installed `cargo-llvm-cov 0.9.1` on `aarch64-apple-darwin` and upstream docs on 2026-11-09. Stable `1.99.1` does not populate branch or MC/DC counts. |
| `SRC-VERIFY-CARGO-MUTANTS` | `cargo-mutants` documentation (`https://mutants.rs/config.html`) | Configuration keys `examine_globs`, `exclude_globs`, `additional_cargo_args`, `timeout_multiplier`, and `minimum_test_timeout`; custom config path via `--config .config/mutants.toml` (default is `.cargo/mutants.toml`). | Checked via `mutants.rs` primary documentation on 2026-11-09. `cargo-mutants` is not installed on the local macOS host; `make mutants` fails closed. |
| `SRC-VERIFY-CARGO-FUZZ` | `cargo-fuzz` `0.13.2` CLI (`cargo fuzz run --help`), `libfuzzer-sys 0.4.13`, and Rust Fuzz Book (`https://rust-fuzz.github.io/book/cargo-fuzz.html`) | `[package.metadata] cargo-fuzz = true`, `fuzz_target!` macro, `arbitrary::Arbitrary` structured inputs, `--fuzz-dir`, `--target-dir`, and libFuzzer `-runs=<N>` / `-max_total_time=<S>` options. | Checked against installed `cargo-fuzz 0.13.2` and cached `libfuzzer-sys 0.4.13` on `aarch64-apple-darwin`; executed 64-run smoke on `nightly-2026-11-08`. |
| `SRC-VERIFY-NEXTEST` | `cargo-nextest` `0.9.151` CLI and configuration reference (`https://nexte.st/docs/configuration/reference/`) | `.config/nextest.toml` profiles (`[profile.default]`, `[profile.ci]`, `[profile.mutants]`), `retries = 0`, `slow-timeout`, filter overrides, and JUnit XML output (`[profile.ci.junit]`). | Checked against installed `cargo-nextest 0.9.151` and `nexte.st` docs on 2026-11-09. Nextest does not execute rustdoc tests. |
| `SRC-VERIFY-PROPTEST` | `proptest` `1.12` documentation (`https://docs.rs/proptest/latest/proptest/test_runner/enum.FileFailurePersistence.html`) | `FileFailurePersistence::SourceParallel` `.proptest-regressions` seed files, deterministic replay before random generation, and `PROPTEST_CASES` environment scaling. | Checked against `docs.rs/proptest` and repository `tests/integer_sets.proptest-regressions` on 2026-11-09. |
| `SRC-VERIFY-LOOM` | `loom` `0.7.3` documentation (`https://docs.rs/loom/0.7.3/loom/model/struct.Builder.html`) and [`verification/loom/src/lib.rs`](../../../verification/loom/src/lib.rs) | `Builder::new()`, `max_threads`, `max_branches`, and clearing `max_permutations`/`max_duration`/`preemption_bound` to avoid truncated schedule exploration. | Checked against `verification/loom/src/lib.rs` (inspected by Flash research subagent and verified in source) and `docs.rs` on 2026-11-09. |
| `SRC-VERIFY-MIRI` | Miri documentation (`https://github.com/rust-lang/miri`) and [`Makefile`](../../../Makefile) | `-Zmiri-strict-provenance`, `-Zmiri-tree-borrows`, and cross-target `--target i686-unknown-linux-gnu` execution on `nightly-2026-11-08`. | Checked against `Makefile` and `docs/verification.md` on 2026-11-09. |
| `SRC-VERIFY-KANI` | Kani `0.68.0` documentation (`https://model-checking.github.io/kani/tutorial-loop-unwinding.html`) and [`verification/kani/src/lib.rs`](../../../verification/kani/src/lib.rs) | `#[kani::proof]`, `#[kani::unwind(N)]`, `[workspace.metadata.kani.flags] default-unwind = "1"`, and unwinding assertions. | Checked against `verification/kani` source (inspected by Flash research subagent and verified in source) on 2026-11-09. Local `cargo kani` binary is not installed. |
| `SRC-VERIFY-VERUS` | Verus documentation (`https://verus-lang.github.io/verus/guide/`) | SMT-backed deductive verification inside `verus! { ... }` blocks for low-level systems and pointer code; requires Verus custom toolchain/macros. | Evaluated via primary documentation search on 2026-11-09; deferred for `frost-collections` 0.4.0. |
| `SRC-VERIFY-CREUSOT` | Creusot documentation (`https://creusot.rs`) | Why3/Coma deductive translation with Pearlite `#[requires]`/`#[ensures]` contracts for safe Rust algorithms; requires Creusot nightly driver and Why3 provers. | Evaluated via primary documentation search on 2026-11-09; deferred for `frost-collections` 0.4.0. |

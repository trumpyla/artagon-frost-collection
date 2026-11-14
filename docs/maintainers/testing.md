# Testing and verification standards

This guide defines the maintainer testing standards for `frost-collections`
under epic `frost-wou` (which precedes and gates the multi-crate reorganization
epic `frost-wlt`). Every change must preserve Frost's semantic laws, keep
optional verification tooling outside the library dependency graph, and separate
measured baseline numbers from target thresholds.

Read [design philosophy](../design/philosophy.md),
[architecture](../design/architecture.md), [maintainer notes](guide.md), and
[verification scope](../verification.md) alongside this guide. The primary-source
evaluation and rationale live in the
[Rust verification strategy report](../research/reports/rust-verification-strategy.md).

## 1. Crate-local target layout

While `frost-collections` remains a single library package (`frost-wou`), test
and verification targets follow a strict layout so that future workspace
decomposition (`frost-wlt`) does not mix runtime dependencies with verification
harnesses:

| Path | Role | Dependency boundary |
| --- | --- | --- |
| `src/**/*.rs` | Library implementation; `#![no_std]` with additive `alloc` and `std` features; inline `#[cfg(test)]` modules only for private kernel invariants (`src/integer/**`) | Production dependencies only (`Cargo.toml` `[dependencies]`) |
| `tests/*.rs` | Public API contract, differential, property, exhaustive finite-domain, and concurrency integration suites (`tests/concurrent/*.rs` are submodules of `tests/concurrent.rs`) | Library public surface plus `[dev-dependencies]` (`proptest`, reference crates) |
| `examples/*.rs` | User-facing executable scenarios with `test = true` and explicit `required-features` | Library public surface |
| `benches/*.rs` | Calibrated `harness = false` wall-clock benchmarks sharing `benches/support/mod.rs` | Library public surface; never run as pass/fail CI speed gates |
| `verification/loom` | Standalone `[workspace]` exploring `src/concurrent/publication.rs` via `#[path]` under `loom = "=0.7.3"` | Isolated `Cargo.lock`; zero impact on root `Cargo.toml` |
| `verification/kani` | Standalone `[workspace]` proving six bounded harnesses over public APIs and `#[path]`-included `src/adaptive/seq/buf.rs` | Isolated `Cargo.lock`; native helper smoke runs in `make bounded` |
| `verification/fuzz` | Standalone `[workspace]` (`cargo-fuzz` / `libfuzzer-sys 0.4.13`) with shared differential oracles in [`verification/fuzz/src/lib.rs`](../../verification/fuzz/src/lib.rs) | Isolated `Cargo.lock`; native corpus replay runs in `make fuzz-check` |
| `verification/int-key-downstream` | Standalone downstream crate testing `IntKey` coherence and compile-fail domain separation | Isolated `Cargo.lock`; runs in `make integers` |
| `verification/coverage` | Source-coverage policy ([`coverage-policy.json`](../../verification/coverage/coverage-policy.json)) and denominator documentation ([`README.md`](../../verification/coverage/README.md)) | Executed via `cargo-llvm-cov` (`make coverage`, `make coverage-gate`) |
| `verification/mutation` | Mutation policy and survivor triage ledger ([`survivor-triage.json`](../../verification/mutation/survivor-triage.json), [`README.md`](../../verification/mutation/README.md)) | Executed via `cargo-mutants` and [`.config/mutants.toml`](../../.config/mutants.toml) (`make mutants`) |
| `verification/catalog` | Structural test/proof metadata, contract references, and live benchmark inventory checks ([`verification/metadata/catalog.json`](../../verification/metadata/catalog.json), [`verification/metadata/bench-inventory.json`](../../crates/frost-collections-core/verification/metadata/bench-inventory.json)); semantic obligation coverage remains unmeasured | Executed via `make catalog` and `make catalog-inventory` |

## 2. Public API semantic obligations beyond percentages

Line, region, and function coverage measure whether a line executed during a
test run; they do **not** prove that Frost's semantic contracts held. Every
public type and capability implementation must satisfy explicit semantic
obligations:

1. **Representative retention**:
   - Map and set construction from iterators or entries retains the **first**
     equivalent key/element representative; map construction retains the **last**
     value without replacing the stored key.
   - Set `union` and `intersection` retain the **left** representative.
   - Tests must use composite keys whose `Eq`/`Ord`/`Hash` inspect an identifier
     while carrying a distinct observable tag (as in
     [`tests/api_contracts.rs`](../../crates/frost-collections-core/tests/api_contracts.rs) and
     [`verification/fuzz/src/lib.rs`](../../verification/fuzz/src/lib.rs)),
     never only primitive integers where all equal values are identical.
2. **Ownership and drop accounting**:
   - Consuming transitions (`freeze`, `thaw`, `into_vec`, `into_map`, `into_set`,
     `into_inner`), partial iteration (`into_iter`, `drain`), and fallible
     constructors (`try_from_unique`, `try_from_sorted_unique`, `try_freeze_with`)
     must drop each consumed element exactly once and preserve rejected inputs
     when promised by `Rejected`.
   - Use non-`Clone` tracked values with per-element `Cell` drop counters and
     zero-sized types (`ZST`).
3. **Iterator laws**:
   - Verify ascending order (for ordered and integer families), `DoubleEndedIterator`
     front/back interleaving, exact `size_hint` / `ExactSizeIterator::len` at
     every prefix, `FusedIterator` stability after `None`, and agreement between
     `next()`, `count()`, and `fold()`.
4. **Feature additivity and profile isolation**:
   - Enabling an optional backend or framework feature must never alter an
     ordinary family's representation, hasher, bounds, or `freeze`/`thaw`
     partner.
   - Verify under `--no-default-features` (core-only), `--no-default-features --features alloc`,
     each isolated feature (`make features`), and bare-metal targets (`make embedded`).
5. **Encoded integer invariants (`int-set`)**:
   - Checked `IntKey` round-trip and monotonicity validation, representation-independent
     `Eq` and exact `Hasher` event sequences across `Sorted`, `Dense`, `Roaring`,
     for all five policy modes (`Auto` plus four explicit representation
     candidates: `Sorted`, `Dense`, `Roaring`, and `Chunked`), `IntSetLimits`
     inheritance (minimum for commutative ops, left for difference), and
     immutable container sharing across versions.

## 3. Source coverage denominators, exclusions, and ratchet policy

Coverage is measured with `cargo-llvm-cov` (`0.9.1` validated on
`aarch64-apple-darwin` with Rust `1.99.1`) over `src/**/*.rs` using:

```sh
CARGO_TARGET_DIR=target/coverage cargo llvm-cov --all-features --locked \
  --json --summary-only \
  --ignore-filename-regex '(^|/)(tests|benches|examples|verification)/' \
  --output-path target/coverage/source-summary.json
```

This exclusion regex is a **file-scoped library-source denominator** over
`src/**/*.rs`—not a test-free production-code denominator. Because `cargo
llvm-cov --all-features --locked` compiles the library target with `cfg(test)`,
inline `#[cfg(test)]` modules inside `src/**/*.rs` (such as unit test modules
under `src/integer/**`) remain in `src/**/*.rs` and are included in both the
numerator and denominator unless refactored or filtered separately. Default
`cargo llvm-cov --all-features --locked` runs unit, integration, and example
test targets; custom `harness = false` benchmark targets (`benches/`) do not
execute in the default coverage run. Every `make coverage` and
`make coverage-gate` run writes `target/coverage/scope-metadata.json` recording
the host target, compiler/tool versions, exclusion regex, and active
enforcement mode (preserving `source-summary.json` and `scope-metadata.json`
even when the gate fails).

### Measured baseline, portable adoption floor, and scoped ratchet

The authoritative `aarch64-apple-darwin` baseline at commit
`6ab0918dbbf655894d8d3176b81091bd28899c5f` is recorded in
[`verification/crate-standards-validation/baseline-coverage.json`](../../verification/crate-standards-validation/baseline-coverage.json):

| Metric | Observed `aarch64-apple-darwin` baseline (`6ab0918`) | Post-`api_contracts.rs` same-scope observation | Portable adoption floor (`COVERAGE_MIN_*`) | Scoped ratchet (`aarch64-apple-darwin`) | Target ([`CRATES.md`](../../CRATES.md) §5) |
| --- | --- | --- | --- | --- | --- |
| Lines | 7,089 / 9,183 (`77.197%`) | ~`79.16%` (`7,269 / 9,183`) | `77%` (`--fail-under-lines 77`, below baseline) | `>= 77.196994%` | `>= 90.0%` |
| Functions | 1,343 / 1,894 (`70.908%`) | ~`73.76%` (`1,397 / 1,894`) | `70%` (`--fail-under-functions 70`, below baseline) | `>= 70.908131%` | `>= 85.0%` |
| Regions | 11,067 / 14,230 (`77.772%`) | ~`79.43%` (`11,303 / 14,230`) | `77%` (`--fail-under-regions 77`, below baseline) | `>= 77.772312%` | `>= 90.0%` |
| Instantiations | 5,434 / 8,602 (`63.171%`) | Report only | Report only | Report only | Report only (not gated) |
| Branches | `UNMEASURED` on stable `1.99.1` | Nightly diagnostic (`make coverage-branch`) | Unmeasured | Unmeasured | Provisional (set after first measurement) |

- **No fabricated attainment or cross-target assumption (`CR-COV-1`)**: Do not
  report the `90%` line / `85%` function / `90%` region targets as achieved
  while the codebase is at the `77.197%` / `70.908%` / `77.772%` baseline (or
  the ~`79.16%` / `73.76%` / `79.43%` post-test observation recorded in
  [after-coverage.json](../../verification/crate-standards-validation/after-coverage.json)), and do not claim that a macOS baseline
  proves the `x86_64-unknown-linux-gnu` baseline on Ubuntu CI.
- **Portable adoption floor vs. scoped ratchet**: `77 / 70 / 77` is a coarse
  portable integer adoption floor **below** the `aarch64-apple-darwin` baseline
  used across targets (including `ubuntu-latest` CI). When `make coverage-gate`
  runs on `aarch64-apple-darwin` with the matching compiler (`rustc 1.99.1`),
  tool (`cargo-llvm-cov 0.9.1`), target, and exclusion regex, it additionally
  enforces an exact `jq -e` non-regression comparison against
  [`verification/crate-standards-validation/baseline-coverage.json`](../../verification/crate-standards-validation/baseline-coverage.json).
  Follow-up Beads tasks (`frost-wou.6` through `frost-wou.10`, preceding the
  multi-crate migration in `frost-wlt.1` through `frost-wlt.6`) own raising the
  floor toward the `CRATES.md` §5 targets as subsystem tests land (beginning
  with [`tests/api_contracts.rs`](../../crates/frost-collections-core/tests/api_contracts.rs), mapped in
  [`verification/crate-standards-validation/api-contracts.md`](../../verification/crate-standards-validation/api-contracts.md)).
- **Exclusions**: Never sprinkle `#[coverage(off)]` on production functions to
  inflate percentages. Target-specific branches (such as the non-atomic `Rc`
  branch in `src/integer/mod.rs`) are verified by `make embedded`
  (`thumbv6m-none-eabi`) and documented in
  [`verification/coverage/README.md`](../../verification/coverage/README.md).

## 4. Mutation testing and survivor triage

Mutation testing runs via `make mutants` using
[`.config/mutants.toml`](../../.config/mutants.toml). It mutates only
`src/**/*.rs` with `--all-features --locked`.

- **Denominator**: `Killed / (Killed + Survived + Timeout)`, excluding
  unviable (non-compiling) mutants.
- **Survivor triage**: Every surviving mutant in a reviewed module must be
  classified in [`verification/mutation/survivor-triage.json`](../../verification/mutation/survivor-triage.json)
  as `test-gap` (must be killed by a new test), `equivalent-mutant`,
  `performance-fast-path`, `defensive-overflow-guard`, or `diagnostic-only`.
  See [`verification/mutation/README.md`](../../verification/mutation/README.md)
  for the per-tier score targets (`>= 90%` for Tier 1 kernels and unsafe
  storage, `>= 85%` for Tier 2 families/adaptive profiles, `>= 75%` for Tier 3
  delegation wrappers).

## 5. Deterministic reproducibility, seeds, and fuzzing

- **No timing assertions or sleeps**: Correctness tests must never call
  `thread::sleep` or assert wall-clock durations. Concurrency tests use explicit
  barriers or deterministic schedule exploration ([`tests/publication_model.rs`](../../crates/frost-collections-concurrent/tests/publication_model.rs)
  and [`verification/loom/src/lib.rs`](../../verification/loom/src/lib.rs)).
- **Nextest profile ([`.config/nextest.toml`](../../.config/nextest.toml))**:
  `retries = 0` is mandatory in all profiles (`default`, `ci`, `mutants`).
- **Property tests (`proptest 1.12`)**:
  Commit any `.proptest-regressions` file (such as `tests/integer_sets.proptest-regressions`)
  to Git so discovered counterexamples replay deterministically before random
  generation on every `make test` and `make nextest` run. Override case counts in
  deep nightly runs with `PROPTEST_CASES=1024`.
- **Isolated fuzzing ([`verification/fuzz`](../../verification/fuzz/README.md))**:
  `make fuzz-check` replays the checked-in seed corpus deterministically on
  stable Rust `1.99.1`. `make fuzz-smoke` runs bounded `libFuzzer` exploration
  on `nightly-2026-11-08` using an ephemeral working corpus directory
  (`target/fuzz-corpus`) so tracked seed files are never mutated in place.

## 6. Gate tiers and Make commands

| Tier | When to run | Make targets / workflows | Tooling failure policy |
| --- | --- | --- | --- |
| **Tier 1: PR / pre-review gate** | Every code or doc change before review; every pull request | `make check`, `make nextest`, `make fuzz-check`, `make coverage-gate` ([`.github/workflows/ci.yml`](../../.github/workflows/ci.yml), [`.github/workflows/verification.yml`](../../.github/workflows/verification.yml)) | Missing required tools (`cargo`, `jq`, `cargo-nextest`, `cargo-llvm-cov` in coverage job) fail the command with a non-zero exit code. |
| **Tier 2: Nightly / deep exploration** | Scheduled nightly CI or after touching set kernels, adaptive promotion, or integer storage | `make coverage-branch`, `make fuzz-smoke`, `PROPTEST_CASES=1024 make test`, `make bounded`, `make loom`, `make mutants` | Each optional Make target checks `"$(CARGO)" <tool> --version` first and exits non-zero if the tool is absent—never faking a skip or pass. |
| **Tier 3: Release candidate gate** | Before tagging or completing a milestone ([`releasing.md`](releasing.md)) | `make verify`, `make embedded`, `make miri`, `make bounded`, `make loom`, `make coverage-gate`, `make fuzz-smoke`, `make shell-check`, `make tools-check`, and `make kani` ([`.github/workflows/proofs.yml`](../../.github/workflows/proofs.yml)) | Any unrun or tool-blocked gate (such as Kani when its bundled nightly compiler lags `rust-version = "1.99.1"`) must be recorded as an open gate, never a pass. |

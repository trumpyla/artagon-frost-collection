# Source coverage policy and baseline tracking

This directory defines the source-coverage denominators, exclusion rules,
scope-tagging requirements, portable adoption floors, and scoped ratchet policy
for `frost-collections`. The authoritative observed baseline on
`aarch64-apple-darwin` is recorded in
[`../crate-standards-validation/baseline-coverage.json`](../crate-standards-validation/baseline-coverage.json),
and the six targeted public-API contract tests added at the root are mapped in
[`../crate-standards-validation/api-contracts.md`](../crate-standards-validation/api-contracts.md).

## Denominator and exclusion rules

1. **File-scoped library-source denominator**: library source files under
   `crates/*/src/**/*.rs` of the library packages, compiled with `--all-features --locked`. Where inline
   `#[cfg(test)]` modules reside inside `src/` files (such as `crates/frost-collections-integer/src/integer/**`),
   their lines are included in the file-scoped denominator; this is file-scoped
   library-source coverage, not a test-free production-code denominator.
2. **Excluded non-library paths**:
   `--ignore-filename-regex '(^|/)(tests|benches|examples|verification|fuzz|frost-collections-test-support)/'`.
   The fuzz package and the test-support package are harness code, not
   library source.
   Default `cargo llvm-cov --all-features --locked` runs unit, integration, and
   example test targets (`tests/`, `crates/frost-collections/examples/`); custom `harness = false`
   benchmark targets (`benches/`) do **not** execute in the default coverage run.
   Excluded directories define the filename denominator, not the set of executed
   target kinds.
3. **Conditional compilation (`#[cfg]` and target scope, `CR-COV-1`)**:
   - A coverage percentage is valid only within its exact scope (revision,
     compiler version, host/build target, features, `cargo-llvm-cov` version,
     and ignore regex). A measurement on `aarch64-apple-darwin` does not prove
     the `x86_64-unknown-linux-gnu` baseline on Ubuntu CI.
   - Host `cargo llvm-cov --all-features` on 64-bit targets compiles the
     `#[cfg(target_has_atomic = "ptr")]` (`Arc`) branch in `crates/frost-collections-integer/src/integer/mod.rs`.
     The `#[cfg(not(target_has_atomic = "ptr"))]` (`Rc`) branch is verified by
     `make embedded` (`thumbv6m-none-eabi`).
   - Do not add `#[coverage(off)]` to production functions to raise percentages.
     Any future exclusion must be listed in [`coverage-policy.json`](coverage-policy.json)
     with a concrete justification (`CR-WAIVE-2`).
4. **Unsafe code coverage (`crates/frost-collections-core/src/adaptive/seq/buf.rs`)**:
   - The sole runtime `unsafe` module is `crates/frost-collections-core/src/adaptive/seq/buf.rs` (observed
     `aarch64-apple-darwin` `6ab0918` baseline: 19/22 functions = 86.364%,
     107/121 lines = 88.430%, 146/165 regions = 88.485%).
   - Line/region coverage in `buf.rs` is necessary but never sufficient: changes
     there also require `make miri` (Stacked Borrows and Tree Borrows with
     strict provenance) and `make bounded`.

## Portable adoption floor versus scoped ratchet

`make coverage-gate` distinguishes two mechanisms and writes
`target/coverage/scope-metadata.json` before running `cargo llvm-cov` so both
the scope metadata and `target/coverage/source-summary.json` are preserved even
when a gate fails:

1. **Portable integer adoption floor (`77 / 70 / 77`)**:
   `--fail-under-lines 77 --fail-under-functions 70 --fail-under-regions 77`.
   Because `77 / 70 / 77` is **below** the `aarch64-apple-darwin` `6ab0918`
   baseline (`77.197% / 70.908% / 77.772%`) and runs across host targets
   (including `ubuntu-latest` in [`.github/workflows/verification.yml`](../../.github/workflows/verification.yml)),
   it is a coarse cross-target **adoption floor**, not an exact non-regression
   check against `6ab0918`.
2. **Scoped exact non-regression ratchet (`aarch64-apple-darwin`)**:
   When `make coverage-gate` runs with matching target (`aarch64-apple-darwin`
   with no conflicting `CARGO_BUILD_TARGET`), compiler (`1.99.1` via
   `"${RUSTC:-rustc}" -vV` with no conflicting `RUSTUP_TOOLCHAIN` and empty
   `RUSTFLAGS` and `CARGO_ENCODED_RUSTFLAGS`), tool (`cargo-llvm-cov 0.9.1`), and ignore regex, it
   additionally compares `target/coverage/source-summary.json` against
   [`../crate-standards-validation/baseline-coverage.json`](../crate-standards-validation/baseline-coverage.json)
   using `jq -e` and fails if any of the three percentages drops below the
   recorded `6ab0918` baseline.

Any scope mismatch uses only the portable floor and records
`portable_adoption_floor_only_cross_target`; it does not pass the exact
ratchet. The catalog targets validate structural metadata and benchmark
inventories, not the completeness of semantic API obligations.

| Metric | Observed `aarch64-apple-darwin` baseline (`6ab0918`) | Portable adoption floor (`COVERAGE_MIN_*`) | Scoped ratchet (`aarch64-apple-darwin`) | Target ([`CRATES.md`](../../CRATES.md) §5) | Follow-up Beads task |
| --- | --- | --- | --- | --- | --- |
| Lines | 7,089 / 9,183 (`77.197%`) | `>= 77%` (below baseline) | `>= 77.196994%` | `>= 90.0%` | `frost-wou.9` (currently ~`79.16%` with `crates/frost-collections-core/tests/api_contracts.rs`) |
| Regions | 11,067 / 14,230 (`77.772%`) | `>= 77%` (below baseline) | `>= 77.772312%` | `>= 90.0%` | `frost-wou.9` (currently ~`79.43%` with `crates/frost-collections-core/tests/api_contracts.rs`) |
| Functions | 1,343 / 1,894 (`70.908%`) | `>= 70%` (below baseline) | `>= 70.908131%` | `>= 85.0%` | `frost-wou.9` (currently ~`73.76%` with `crates/frost-collections-core/tests/api_contracts.rs`) |
| Instantiations | 5,434 / 8,602 (`63.171%`) | Report only | Report only | Report only | Not gated |
| Branches | `0 / 0` on stable 1.99.1 | Nightly diagnostic (`make coverage-branch`) | Unmeasured | Provisional (set after first measurement) | `frost-wou.7` |
| Doctests & isolated profiles | Unmeasured | Unmeasured | Unmeasured | Report per profile | `frost-wou.7` |
| Public API obligations | Unmeasured; inventory incomplete | Not measured by the source floor | Not measured by the source ratchet | 100% dispositioned, zero `uncovered` | `frost-wou.6` |

## Commands

```sh
make coverage         # Emit target/coverage/source-summary.json + scope-metadata.json
make coverage-gate    # Enforce portable floor (77/70/77) + scoped aarch64-apple-darwin ratchet
make coverage-branch  # Nightly branch-coverage JSON summary (+nightly-2026-11-08 --branch)
```

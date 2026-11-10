# Standards implementation evidence

These observations cover the standards implementation on top of `6ab0918`,
before commit or publication. Production `src/`, `Cargo.toml`, and the root
`Cargo.lock` are unchanged. The crate reorganization remains future work in
`frost-wlt`, which depends on the standards and adoption epic `frost-wou`.

## Coverage observation

The [baseline](baseline-coverage.json) and [final observation](after-coverage.json)
use Rust 1.99.1, cargo-llvm-cov 0.9.1, `aarch64-apple-darwin`, all features,
and the same filename exclusions. The final `make coverage-gate` passed its
portable adoption floor and exact matching-scope baseline comparison.

| Metric | Before | After | Target |
| --- | --- | --- | --- |
| Lines | 7,089 / 9,183 (77.197%) | 7,269 / 9,183 (79.157%) | 90% |
| Functions | 1,343 / 1,894 (70.908%) | 1,397 / 1,894 (73.759%) | 85% |
| Regions | 11,067 / 14,230 (77.772%) | 11,303 / 14,230 (79.431%) | 90% |

The denominator includes inline unit-test modules inside `src/`. It is
file-scoped library-source coverage, not a test-free production denominator.
Host tests and example tests execute; benchmarks and instrumented doctests do
not. The JSON observations preserve per-file results, scope, and the raw
summary digest. The final artifact also records test-source digests.

The targets are not attained. Branch, MC/DC, instrumented doctest, isolated
profile, non-host, semantic API obligation coverage, and mutation scores remain
unmeasured. A zero branch denominator is not a zero-percent branch result.
The [six API contract tests](api-contracts.md) document their independent
oracles and finite bounds; executing a public method does not prove all its
obligations.

## Completed local checks

- `make check`: formatting, documentation links, Clippy, rustdoc, all-feature
  tests, example tests, the 14-test catalog validator and actual catalog check,
  and all seven live benchmark inventories passed. The inventory contains
  922 exact benchmark names; the catalog has 327 records, of which 21 are
  curated. These are structural inventory counts, not API coverage.
- `cargo clippy --locked --manifest-path verification/catalog/Cargo.toml
  --all-targets -- -D warnings`: passed.
- `CARGO_TARGET_DIR=target/api-contracts make features`: isolated feature
  Clippy, tests, and rustdoc passed.
- `CARGO_TARGET_DIR=target/api-contracts make profiles`: core-only, alloc-only,
  and default-profile checks passed.
- `CARGO_TARGET_DIR=target/api-contracts make embedded`: atomic and non-atomic
  embedded library builds passed.
- `make coverage-gate COVERAGE_TARGET_DIR=target/coverage-final
  COVERAGE_OUTPUT_DIR=target/coverage-final`: passed; the recorded mode is
  `scoped_exact_ratchet_enforced`.
- `make fuzz-check`: all three deterministic replay tests passed, including
  125 productive policy triples and rejection/input-preservation checks.
- `make fuzz-smoke` for both algebra targets with `FUZZ_RUNS=64` and
  `FUZZ_MAX_TIME=10`: both bounded runs passed.
- `make shell-check SHELL=/bin/bash` and `gmake shell-check SHELL=/bin/bash`:
  all 35 tests passed under GNU Make 3.81 and 4.4.1 with Bash 3.2.57, including
  11 coverage-gate regression cases. ShellCheck and shfmt passed too.
- Isolated catalog and fuzz `cargo fmt --check`: passed.
- `actionlint .github/workflows/verification.yml`: passed static validation.

The core-only normal dependency tree contains only Frost. The `alloc,int-set`
tree contains its existing Roaring dependency chain; the new isolated catalog
and fuzz packages do not enter either tree.

## Evidence limits and adoption

Local workflow validation does not establish GitHub runner execution or issue
form rendering. Mutation and Kani tools are unavailable on this host. No new
Miri or full release-candidate verification result is claimed for this change.
Benchmark listing validates answers and inventory membership without timing;
it supports no performance comparison.

The remaining adoption work is tracked by `frost-wou.6` through `frost-wou.10`:
complete API obligations, additional coverage scopes, mutation baselines,
meaningful tests toward the high targets, and automated dependency-isolation
guards. Package boundaries and migration evidence belong to `frost-wlt.1`
through `frost-wlt.6`.

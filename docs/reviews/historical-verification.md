# Verification: performance review of Frost 0.4.0

Current revision: unreleased performance pass, **Rust 1.99.1, edition 2024**.
The package version and dependency lockfile are unchanged.

## Checks executed for the revised source

| Gate | Result | Evidence |
| --- | --- | --- |
| Unchanged input, all features | 215 tests passed | `verification/history/performance-baseline-tests.log` |
| Revised source, all features | **231 tests passed**: 3 library, 173 integration/property, 10 example, 45 doctest | `verification/history/performance-tests.log` |
| All-target/all-feature Clippy, warnings denied | Passed | `verification/history/performance-clippy.log` |
| Rustfmt | Passed | `verification/history/performance-extra-verification.log` |
| Default-disabled `alloc,fixedbitset-0_5` tests | Passed | Same extra-verification log |
| Core-only library check | Passed | Same extra-verification log |
| `thumbv7em-none-eabihf`, `alloc,fixedbitset-0_5` library build | Passed without default features | Same extra-verification log |
| All-feature rustdoc, warnings denied | Passed | Same extra-verification log |
| Release benchmark corpus | 140 cases in each of four sequential runs; answers asserted before timing | `verification/history/performance-summary.csv` and four raw run CSVs |
| Final documentation and Cargo source package | 45 doctests passed; rustdoc warnings denied; package built and verified | `verification/history/performance-delivery.log` |

Sixteen added cases cover skewed ordered kernels and iterator state (4), long
bitmap spans and padding boundaries (3), single-search adaptive entries and panic
behavior (6), and owning Rayon traversal/drop behavior (3). Repeated profile runs
are not counted as new cases. No unsafe implementation was changed.

The benchmark compared the uploaded source and revised source using one harness,
compiler and target. Runs were baseline/revised/revised/baseline on CPU 0. Input
construction was excluded, while result materialization included its allocation.
The summary uses the median of each version's two run medians; native controls
and all slower observations remain in the raw evidence. See
[PERFORMANCE_REVIEW.md](performance-review.md) for interpretation and limitations.

This pass did not rerun the full nineteen-isolated-feature matrix or historical
Miri runs below. The bare-metal build is a compilation check, not hardware
execution. Historical claims are not counted as new validation of this patch.

---

# Supplied historical verification record

The following record and its named logs arrived with the attachment. They are
preserved for provenance and describe the earlier 0.4.0 implementation.

# Verification record: Frost 0.4.0

Verified on 2026-11-08. This revision adds frozen set operations to the reviewed
0.3.0 architecture. Normal builds, tests, Clippy, documentation, benchmarks and
package verification use **Rust 1.99.1**, edition **2024**. The toolchain file,
Cargo manifest and normal CI jobs retain that exact target.

## Executed checks

| Gate | Result | Evidence |
| --- | --- | --- |
| Rustfmt | Passed | Fail-fast `scripts/verify.sh` |
| All-target/all-feature Clippy with `-D warnings` | Passed | `verification/history/set-operations-pinned.log` |
| All-feature rustdoc with `-D warnings` | Passed | Same complete pinned log |
| Library unit tests | **3 passed** | Same complete pinned log |
| Integration/property tests | **157 passed** | Same complete pinned log |
| Executable example tests | **10 passed** | Same complete pinned log |
| Doctests | **45 passed**: 24 positive, 21 compile-fail | Same complete pinned log |
| Core-only, alloc-only and default profiles | Passed | Same complete pinned log |
| Nineteen optional integrations independently | Passed | Same complete pinned log |
| Ten actual example binaries | All executed successfully | Same complete pinned log |
| Core-only and alloc-compatible bare-metal builds | Both passed | Same complete pinned log |
| Nightly doctests and annotated error codes | **45 passed** | `verification/history/set-operations-nightly.log` |
| Strict-provenance Miri, Stacked Borrows | **37 passed**, two long random-operation cases filtered | `verification/history/set-operations-miri-stacked.log` |
| Strict-provenance Miri, Tree Borrows | **12 passed** | `verification/history/set-operations-miri-tree.log` |
| Release-mode comparison benchmark | Six comparisons completed; answers checked before timing | `verification/history/set-operations-bench.log` |
| Extracted Cargo package, all features | Passed; 95 files packaged | `verification/history/set-operations-package.log` |

The complete native all-feature test suite has **215 distinct cases**: 3 library
unit tests, 157 integration/property tests, 10 example tests and 45 doctests.
Repeated feature, example, nightly and Miri runs are not counted as additional
cases. Each property test counts as one test, regardless of generated inputs.

## New set-operation coverage

| Target | New cases | Coverage |
| --- | ---: | --- |
| Bitmap cardinality helper unit tests | 3 | Actual overflow and subtraction before addition |
| `ordered_set_ops` | 6 | Standard-set differential behavior, left representatives, non-Clone elements, partial hints/counts/Clone/fusion, borrowed keys, comparison bounds |
| `hash_set_ops` | 5 | Different hashers, collisions, non-Clone/non-Debug representatives, lazy construction, left representatives under both scan directions, iterator behavior, randomized standard-set comparison |
| `bitmap_set_ops` | 7 | Roaring32/64 algebra, extreme members, dense/sparse/empty inputs, unequal universes, word boundaries, dirty padding, raw input preservation |
| `set_operations` example | 1 | Actual complete ordered/hash/bitmap scenario |

The 18 new integration/property cases combine with the existing 139 to make
157. The three arithmetic unit cases and two additional positive doctests are
counted separately. All ten examples have explicit Cargo targets with
`test = true` and tests invoking their actual scenario. README Rust blocks
are included in rustdoc.

### Integration breakdown

| Target | Passed |
| --- | ---: |
| `adapters` | 14 |
| `advanced` | 14 |
| `backend_review` | 4 |
| `bitmap_set_ops` | 7 |
| `concurrent` | 10 |
| `contracts` | 23 |
| `core_review` | 14 |
| `differential` | 5 |
| `ecosystem_review` | 12 |
| `families` | 13 |
| `hash_set_ops` | 5 |
| `inline_safety` | 12 |
| `mutable_sets` | 5 |
| `ordered_set_ops` | 6 |
| `root_regressions` | 2 |
| `sets_review` | 4 |
| `specialized` | 7 |
| **Total** | **157** |

## Feature matrix and bare-metal builds

The complete command exited zero and its log ends with
`All requested verification gates passed.`:

```sh
FROST_NO_STD=1 ./scripts/verify.sh
```

The fail-fast script obtains features and examples from Cargo metadata. It runs
formatting, warnings-denied lint/documentation, tests and example binaries.
Alongside core, alloc, default and all-feature profiles, it independently checks:

- `hashbrown-0_17`, `indexmap-2`, `smallvec-1`, `arrayvec-0_7`;
- `imbl-7`, `roaring-0_12`, `fixedbitset-0_5`, `slotmap-1`, `phf-0_14`, `fst-0_4`;
- `dashmap-6`, `scc-3`, `crossbeam-skiplist-0_1`, `arc-swap-1`;
- `serde-1`, `rayon-1`, `arbitrary-1`, `borsh-1`, `schemars-1`.

Each isolated feature receives library Clippy with warnings denied, tests, and
warnings-denied rustdoc. This covers individual integrations and their union;
it does not enumerate all possible feature subsets.

Actual library builds for `thumbv7em-none-eabihf` use no features, then
`alloc,hashbrown-0_17,indexmap-2,smallvec-1,arrayvec-0_7,fixedbitset-0_5,slotmap-1,phf-0_14,serde-1,borsh-1,schemars-1`.
Host dev-dependency unification therefore cannot conceal a `std` requirement.
These are cross-target compilation checks, not embedded hardware executions.

## Ownership diagnostics

Nightly is an additional diagnostic toolchain. These commands passed:

```sh
cargo +nightly test --doc --all-features --locked
MIRIFLAGS="-Zmiri-strict-provenance" \
  cargo +nightly miri test --locked \
  --test inline_safety --test families --test core_review -- --skip under_random_ops
MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-strict-provenance" \
  cargo +nightly miri test --locked --test inline_safety
```

Stacked Borrows reran 14 `core_review`, 11 `families` and 12 `inline_safety`
cases. Tree Borrows reran all twelve inline ownership cases. The two filtered
family cases are longer random-operation tests that passed natively.

The new algebra uses safe Rust and native backend operations; no additional
unsafe implementation was introduced. Miri covers the existing ownership
suites. The new algebra suites ran natively with adversarial and differential
coverage; they are not claimed as additional Miri cases. These checks do not
prove all unsafe dependencies correct or exhaust concurrent schedules.

## Benchmark and environment

The optimized comparison benchmark completed:

```sh
cargo bench --bench set_operations --all-features --locked
```

Construction is outside timing, answers are checked before measurement, and
results are medians of five samples after warmup. The six cases compare direct
kernels with the explicit binary-search, full-left-scan, materialize-then-count,
or per-member probing baselines in the source. `docs/set-operations.md` records the
results and baseline definitions. The hash case demonstrates avoiding traversal
of the much larger input; it is not a comparison against the standard library's
already size-aware intersection implementation.

| Environment item | Observed value |
| --- | --- |
| rustc | `1.99.1 (58cf8f9c3 2026-10-13)` |
| Cargo | `1.99.1 (5f94df478 2026-08-27)` |
| LLVM | `23.1.1` |
| Host target | `x86_64-unknown-linux-gnu` |
| Reported CPU | AMD EPYC 9V74 80-Core Processor |
| CPUs in process affinity | 9 |
| Nightly diagnostics | `1.102.0-nightly (f5cedb83a 2026-11-07)` |

No task-specific CPU flags were added. Warm-data integer microbenchmarks on one
host do not establish performance for every density, overlap, hasher, platform
or workload. Raw records are `verification/history/set-operations-environment.txt` and
`verification/history/set-operations-bench.log`.

## Packaging and historical evidence

`cargo package --all-features --allow-dirty --locked` packaged 95 files and
compiled the extracted package on Rust 1.99.1. Cargo warned that the manifest
has no documentation, homepage or repository URL. These were not invented for
an unpublished local project; `publish = false` remains set. No publishing
command or remote CI execution was performed.

The review ZIP contains source, lockfile, licenses, examples, tests, benchmark,
CI configuration, design documents and verification evidence. Build output is
excluded. Cargo packaging excludes `verification/**`; the review ZIP keeps it.

Older logs without the `set-operations-` prefix belong to the preceding 0.3.0
review. In particular, `incoming-inline-reproducer.log` and
`incoming-borsh-reproducer.log` intentionally record failures against the
original attachment. Earlier `pinned-initial.log` and `pinned-lint-initial.log`
record a resolved test-only inference/lint issue from that integration work.
The current authoritative pass is `verification/history/set-operations-pinned.log`.

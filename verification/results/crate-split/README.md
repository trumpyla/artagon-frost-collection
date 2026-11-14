# Crate-split evidence (`frost-wlt`)

Baselines recorded before any file moved, and the tools that produced them.
They let each step of the [crate-split plan](../../../docs/design/crate-split-plan.md)
be checked against the single-package tree instead of against expectations.
Nothing here is a Make gate.

## Baseline

Source: commit `ab15901` plus the `frost-wlt.7` changes (error codes on four
`compile_fail` doctests, three curated catalog records, the empty-selection
guard in the Makefile). No test body differs from `ab15901`.
Host: Linux x86-64, 4 CPUs, 15 GB. `cargo 1.99.1`.

| File | Content | Produced by |
| --- | --- | --- |
| `baseline/test-counts.<profile>.tsv` | Executed tests per target: target, passed, failed, ignored | `cargo test <profile flags> --locked` piped through `test-counts.sh` |
| `baseline/api-paths.<profile>.txt` | Public item paths reachable from `frost::`, with inherent items and implemented traits per type | `api_paths.py --methods` over rustdoc JSON from `nightly-2026-08-21` |
| `baseline/compile-fail-uncoded.txt` | The compiler error of each `compile_fail` doctest that cannot name an error code | Each block run as an ordinary doctest once |
| `crate-names.txt` | crates.io index lookups for the candidate package names | `curl` against the sparse index |
| `layout-inventory.tsv` | Every tooling or configuration line keyed on the single-package layout: file, line, kind, text | `git grep`, collected by an agent; see the plan, section 5 |

Profiles: `all-features`, `default`, `alloc` (`--no-default-features --features alloc`),
`core-only` (`--no-default-features`), `int-set` (`--no-default-features --features int-set`).

| Profile | Targets | Tests passed |
| --- | ---: | ---: |
| `all-features` | 41 | 413 |
| `default` | 34 | 213 |
| `alloc` | 33 | 82 |
| `core-only` | 31 | 18 |
| `int-set` | 34 | 165 |

Gates run on the same tree, all passing: `make check`, `make profiles`,
`make features` (20 isolated features), `make embedded` (`thumbv7em-none-eabihf`
and `thumbv6m-none-eabi`), `make bounded`, `make doctest-codes`
(`nightly-2026-11-08`; 60 doctests, error codes checked), and two Miri lanes
under Tree Borrows with strict provenance: `--test inline_safety` (12 tests) and
`--test exhaustive_collections inline_ownership_exhausts_three_operation_traces`
(1 test).

`make shell-check` passes (ShellCheck 0.9.0, shfmt, 39 Bats cases) after one
ShellCheck false positive (SC2154 on the `$stderr` that Bats sets) in
`verification/shell/makefile.bats` was annotated; the scripts added
here pass ShellCheck and shfmt.

Not run for the baseline: the remaining `make miri` lanes including the survey
shards, `make loom`, `make kani`, `make verus`, fuzzing, coverage, nextest,
mutation testing and `make package`.

Installed for this work: `nightly-2026-11-08` with Miri, both embedded targets,
ShellCheck 0.9.0, shfmt, Bats 1.10.0, ast-grep 0.51.2, cargo-nextest 0.9.151,
cargo-llvm-cov 0.9.1. Kani 0.68.0 was already present. Not installed:
cargo-mutants, cargo-fuzz, Verus.

## Limits

- `api_paths.py` lists paths, inherent item names and implemented trait paths.
  It does not compare signatures, bounds or auto traits; the contract tests
  and `compile_fail` doctests cover those.
- rustdoc strips `#[doc(hidden)]` items, so the path lists omit them.
- A test count that matches does not show the tests are the same tests; the
  moves change no test body, which `git diff` shows.

## Tools

| Tool | Use |
| --- | --- |
| [`test-counts.sh`](test-counts.sh) | `cargo test … 2>&1 \| bash test-counts.sh` |
| [`api_paths.py`](api_paths.py) | `python3 api_paths.py <dir of rustdoc JSON> frost --methods` |
| [`relink.py`](relink.py) | `python3 relink.py OLD=NEW…` after a move, to rewrite Markdown links |

## After `frost-wlt.8` (workspace scaffold)

On the tree with the package at `crates/frost-collections`: the executed test
counts of all five profiles are identical to the baseline files, line for line.
Gates that passed: `make check`, `profiles`, `features` (20 features),
`integers`, `examples`, `package` (scratch workspace), `embedded`, `bounded`,
`loom`, `fuzz-check`, `doctest-codes`, `shell-check` (40 Bats cases),
`tools-check`, and the two Miri lanes over `buf.rs` named above. Not run: the
other `make miri` lanes, `kani`, `nextest`, coverage, mutation, `fuzz-smoke`.

## After `frost-wlt.9` (core extracted)

Public paths, inherent items and trait implementations under `frost::` are
identical to the baseline in all five profiles. Integration and unit test
counts are identical; the 60 doctests under all features are now 36 in `frost`
and 24 in `frost_collections_core`, and the sums match the baseline in every
profile. Gates that passed: `make check`, `profiles`, `features` (39
package-feature pairs), `feature-isolation`, `integers`, `examples`, `package`,
`embedded`, `bounded`, `loom`, `fuzz-check`, `doctest-codes`, `shell-check` (43
Bats cases), `tools-check`, and the two Miri lanes over `buf.rs`.

## After `frost-wlt.11` (integer extracted)

Public paths, inherent items and trait implementations under `frost::` are
identical to the baseline in all five profiles, and test counts sum to the
baseline. The alloc-only integer lane resolves `frost-collections-core` with
`[alloc,int-set]` and `roaring` with no features. Gates that passed:
`make check`, `profiles`, `features` (58 package-feature pairs), `integers`,
`examples`, `package`, `embedded`, `bounded`, `loom`, `fuzz-check`,
`doctest-codes`, `shell-check`.

Miri: the 32-bit lane `--lib integer::storage::word::tests` passed on the
integer package (4 tests). The other integer Miri lanes were stopped and are
not run: the maintainer instructed on 2026-11-14 that Miri is not to be run in
this session.

## After `frost-wlt.12` (concurrent extracted; facade is re-exports only)

Public paths, inherent items and trait implementations under `frost::` are
identical to the baseline in all five profiles. Test counts equal the baseline
plus one in every profile: the new `compile_fail,E0432` doctest showing that
`frost::__private` does not resolve. The four `compile_fail` doctests without
an error code were re-run as ordinary doctests and fail with the recorded
error. Gates that passed: `make check`, `profiles` (with `feature-isolation`
and `feature-forwarding`), `features` (78 package-feature pairs), `integers`,
`examples`, `package`, `embedded`, `bounded`, `loom`, `fuzz-check`,
`doctest-codes`, `shell-check` (45 Bats cases), `tools-check`. Miri was not
run, by maintainer instruction.

`api_paths.py` now prints an external re-export by the last segment of its
source, so routing `arc_swap` through another package does not change its
line; the baseline files are reproduced unchanged by the updated tool.

## After `frost-wlt.13` (tests and benches with their owners)

In each of the five profiles, the executed-test count of every non-doctest
target equals the baseline line for that target (split targets are summed
under their shared name), and doctests equal the baseline plus one. Public
paths, inherent items and trait implementations under `frost::` are identical
to the baseline in all five profiles. All 397 baseline catalog IDs are present.
Gates that passed: `make check`, `profiles`, `features` (78 pairs), `integers`,
`examples`, `package`, `embedded`, `bounded`, `loom`, `fuzz-check`,
`doctest-codes`, `shell-check`, `tools-check`. Miri was not run.

`move_targets.py` and `doctest_alias.py` are the tools that moved the targets
and added the crate aliases.

## Final layout (`frost-wlt.14`–`.17`, evidence for `.6`)

Run on 2026-11-14 against the tree committed with this section, on the same
Linux x86-64 host (4 CPUs, 15 GB). Tools: cargo and rustc 1.99.1,
`nightly-2026-11-08` for `doctest-codes`, cargo-nextest 0.9.151, cargo-llvm-cov
0.9.1, Kani 0.68.0, ShellCheck 0.9.0, Bats 1.10.0, ast-grep 0.51.2, jq 1.7.

| Check | Result |
| --- | --- |
| `make check` | Pass |
| `make profiles` (with `feature-isolation`, `feature-forwarding`) | Pass, four library packages |
| `make features` | Pass, 78 package-feature pairs |
| `make integers`, `examples`, `embedded` | Pass |
| `make package` | Pass; 414 tests in the scratch copy of the packaged file sets |
| `make bounded`, `loom`, `fuzz-check` | Pass |
| `make doctest-codes` | Pass; 61 doctests with error codes checked |
| `make shell-check`, `tools-check` | Pass; 45 Bats cases |
| `make nextest` | Pass; 353 tests |
| `make coverage-gate` | Pass in portable-floor mode: lines 81.37%, functions 75.74%, regions 81.94% (`final/coverage-totals.json`) |
| Kani, nine harnesses, one at a time | All verify (`kani-after-move/results.tsv`) |
| `make integer-ab`, one round | Runs with a workspace-layout baseline under `--locked` and with `origin/main` under `AB_LOCKED=`; 630 cases each. One round is a smoke run, not a measurement |
| Test counts, five profiles | Every non-doctest target equals its baseline line; doctests are the baseline plus one (`final/test-counts.*.tsv`) |
| Public API under `frost::`, five profiles | Identical to `baseline/api-paths.*.txt` |
| Catalog | 402 records; all 397 baseline IDs present |
| Dependency trees | No proof, model or fuzz tool in any library package (`dep-trees/`) |

Not run on this host for the final layout:

- **Miri**, by maintainer instruction of 2026-11-14. Before that instruction
  the two lanes over `buf.rs` passed after the scaffold and after the core
  extraction, and one 32-bit integer lane passed after the integer extraction.
  GitHub CI runs `make miri`; the next section records its results.
- **Kani** `ordered_set_algebra` and `dense_padding_does_not_create_members`,
  which did not finish before the split either (`frost-k4n`).
- **Verus**: not installed on this host.
- **The exact coverage ratchet**: it applies only on the reference host
  (aarch64-apple-darwin), and its baseline predates the split.
- **`make coverage-branch`** and **`make miri-survey`**.

Fuzzing, mutation testing, benchmarks and GitHub CI had not run when this
section was written. The next section records them.

## After the final layout: fuzzing, mutation testing, benchmarks, CI

Run on 2026-11-14 on the same host, against commit `b28862b` plus the changes
committed with this section (a fuzz-oracle correction and a recipe fix, both
described below). Added tools: cargo-fuzz 0.13.2 and cargo-mutants 25.3.1, the
versions the workflows pin.

### Fuzzing

| Run | Result |
| --- | --- |
| `make fuzz-smoke FUZZ_TARGET=ordered_set_algebra` (64 runs) | Pass |
| `make fuzz-smoke FUZZ_TARGET=integer_set_algebra` (64 runs), first attempt | **Fail**: the oracle panicked on the input `4444` (`final/fuzz/integer_set_algebra.before-fix.log`) |
| The same input replayed on the pre-split revision `89a38ed` | The same panic at the same assertion, so the defect predates the split |
| `make fuzz-smoke`, both targets, after the correction | Pass |
| Both targets with `FUZZ_RUNS=2000000 FUZZ_MAX_TIME=120` | Pass: 1,102,273 and 579,171 executions (`final/fuzz/results.tsv`) |

The defect was in the fuzz oracle, not in the library. The input decodes to
two empty operands built with a forced `Dense` policy, and an `Auto` output
policy. The library returns an operand's storage, which is `Dense`. The
oracle required every empty `Auto` result to be `Sorted`. The library's
behaviour is the documented one
([integer-sets.md](../../../crates/frost-collections-integer/docs/integer-sets.md),
"Shared ownership and changed-version costs"), and the test
`empty_results_share_an_eligible_empty_operand` pins it. The oracle now
accepts an empty result that is an empty operand's storage. The input is a
seed (`seed-empty-dense-operands`). A fuzz-library test replays it and runs
all 375 policy combinations with one or both operands empty (catalog
`T-LIB-049`). The nightly workflow runs this target with 256 executions, so it
was exposed to the same failure before this correction.

### Mutation testing

`cargo mutants --config .config/mutants.toml` ran over one file in each
library package: `publication.rs` (concurrent), `plan.rs` (core) and
`policy.rs` (integer). The same three files were mutated on the pre-split
revision `89a38ed` with its own configuration.

| Tree | Mutants | Caught | Missed | Unviable | Timeout |
| --- | --- | --- | --- | --- | --- |
| Pre-split `89a38ed` | 34 | 10 | 8 | 16 | 0 |
| Split | 34 | 10 | 8 | 16 | 0 |

Every mutant has the same outcome on both trees (compared by file, column and
description; line numbers differ by one). So each package's own tests catch
what the single package's tests caught. The eight missed mutants are gaps
that predate the split: the `Display`, `Debug` and `source` implementations of
`Rejected`, `LengthMismatch` and `IntSetError`, the `max_roaring_partitions`
accessor, and one constant in `IntSetPolicy::auto` (`frost-m8d`). The outcome
lists are in `final/mutants/`. The nightly set (`buf.rs`, `publication.rs`,
`set_ops.rs`; 183 mutants) and the whole workspace (4,226 mutants) were not
run.

### Benchmarks

Raw observations are in `final/benchmarks/`, and `bench_compare.py` prints the
figures below from them. The host is a shared 4-CPU cloud machine, not a
reference host. These runs compare the two trees with each other; they do not
replace the measurements recorded in the package documents. Ratios are old
over new: above 1 the split tree is faster.

Same binary (`make integer-ab`, 630 integer-profile cases, three rounds):

| Comparison | Geometric mean | Median | 5th to 95th percentile | Cases beyond ±10% |
| --- | --- | --- | --- | --- |
| Pre-split `89a38ed` against the split tree | 0.997 | 1.000 | 0.924 to 1.043 | 39 |
| Split tree against an identical copy (control) | 1.003 | 1.001 | 0.925 to 1.125 | 63 |

The control links commit `b28862b` under renamed packages against the working
tree, whose library sources are identical. It differs from 1 by more than the
comparison of interest does. The 39 cases beyond ±10% were run again with
nine rounds against both baselines (`integer-ab.outliers.9-rounds.csv`,
`integer-aa.outliers.9-rounds.csv`). The largest differences reproduce in the
control at the same size: sorted equality is 0.729 in both, and
`series.iterate.chunked.fold_sum` is 1.353 and 1.488. They come from where
each library sits in the harness binary, not from the split. Five
`u32.runs.*.is_subset` cases of about 2 ns are 4% to 12% slower against the
pre-split tree and about 21% faster in the control; this harness does not
settle them.

Separate binaries (the seven bench targets; 914 cases, each present on both
trees under the same name):

| Cases | Count | Geometric mean | Median | Cases beyond ±10% |
| --- | --- | --- | --- | --- |
| Native controls (standard library, native Roaring, raw slices) | 155 | 0.999 | 1.003 | 50 (32%) |
| Frost cases | 759 | 0.990 | 1.000 | 121 (16%) |

The native controls run code that the split did not touch, and they vary more
between the two binaries than the Frost cases do. No difference that can be
attributed to the split is visible at this resolution. That is a statement
about these two methods on this host. A difference smaller than the control's
spread would not be seen.

### GitHub CI

The workflow `Rust verification` runs on every push.

| Commit | Run | Result |
| --- | --- | --- |
| `873fcc7` (tests and benches moved) | [29](https://github.com/trumpyla/artagon-frost-collection/actions/runs/43541158365) | All 11 jobs pass: `make check`, profiles and examples on Linux, macOS and Windows; the feature matrix; the packaged-source test; the integer profile; embedded; bounded and Loom; shell tooling; the dependency audit; and `ownership`, which runs `make miri` (36 minutes) |
| `b28862b` (final layout) | [30](https://github.com/trumpyla/artagon-frost-collection/actions/runs/43545458278) | Nine jobs pass. `host (windows-latest)` fails in `make check` (below). `ownership` (`make miri`) had run for 51 minutes without a result when the commit with this section was pushed, and that push cancels it |
| `237b3c6` (the two fixes and this record) | [31](https://github.com/trumpyla/artagon-frost-collection/actions/runs/43550135598) | All 11 jobs pass, including `host (windows-latest)` and `ownership` (`make miri`) |

`make miri` took 36 minutes in run 29 and 79 minutes in run 31, against a job
limit of 90. No Miri lane and no source under Miri changed between those
commits except one doc comment, which points to the runners. The limit leaves
little margin.

The Windows failure was in `make catalog-inventory`. Commit `b28862b` changed
the recipe to take each bench target's path from Cargo metadata and to strip
the package directory with a shell pattern. On Windows the metadata paths use
backslashes, so the pattern did not match, and the tool received an absolute
path that is in no inventory. The recipe now computes the package-relative
path in `jq` with forward slashes. Three Bats cases cover it, one with Windows
paths; that case fails against the old recipe. `make shell-check` now runs 48
Bats cases.

After these changes `make check`, `shell-check`, `fuzz-check` and
`tools-check` pass on this host, and the catalog has 403 records.

## Maintainer decisions, 2026-11-14

The maintainer decided the points that the final-evidence task left open
([decision record](../../../docs/design/decisions.md#crate-split-execution-2026-11-13)):

- The plan is approved as executed. Package names and boundaries may be
  revisited in the API refactor.
- The GitHub CI runs of `make miri` on the split tree are the Miri evidence
  for the epic. Miri was not run on the execution host after the instruction
  of 2026-11-14.
- Three lanes are waived for this epic and stay tracked: the two Kani
  harnesses that do not finish (`frost-k4n`), Verus (`frost-v3s`), and the
  exact coverage ratchet on the reference host (`frost-c7b`). A waiver is not
  a result: none of the three has passed on the split tree.
- Miri moves out of the push workflow. `ci.yml` keeps `make doctest-codes`;
  `verification.yml` runs `make miri` in its nightly and release tiers.

Run [32](https://github.com/trumpyla/artagon-frost-collection/actions/runs/43557293563)
passed all 11 jobs, `make miri` included, on commit `3bd103c`, the last commit
before the workflow change. With these decisions `frost-wlt.6` and the epic
are closed.

The workflow change was checked only by parsing the two files. `actionlint`
is not installed on this host, and the new nightly job has not yet run on
GitHub.

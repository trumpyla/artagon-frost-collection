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

Not run, and therefore not evidence for the final layout:

- **Miri**, by maintainer instruction of 2026-11-14. Before that instruction
  the two lanes over `buf.rs` passed after the scaffold and after the core
  extraction, and one 32-bit integer lane passed after the integer extraction.
  The lanes in `make miri` name the owning packages but were not executed on
  the final tree.
- **Kani** `ordered_set_algebra` and `dense_padding_does_not_create_members`,
  which did not finish before the split either (`frost-k4n`).
- **Verus**, **cargo-fuzz** (`make fuzz-smoke`) and **cargo-mutants**: not
  installed on this host.
- **Benchmarks**: listed and matched against their inventories by
  `make check`, but not measured. Earlier measurements are for the single
  package.
- **The exact coverage ratchet**: it applies only on the reference host
  (aarch64-apple-darwin), and its baseline predates the split.
- **`make coverage-branch`**, **`make miri-survey`**, and **GitHub CI**.

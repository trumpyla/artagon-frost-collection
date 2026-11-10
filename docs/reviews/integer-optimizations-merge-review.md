# Integer optimizations: merge review

Date: 2026-11-10. Reviewed branch: `integer-optimizations` at `6aaf871`,
against `main` at `22697a8`, followed by corrections at `23704b0` and
`727ea33`. The Miri runner correction below preserves the full test matrix.
The earlier Linux timings and verification remain historical evidence in
[the optimization record](../integer-optimization-validation.md).

## Findings and corrections

1. **Sorted difference could overflow its kernel-selection arithmetic.**
   `small * (large.bit_width() + 1)` used `usize` even when the product
   exceeded the target width for addressable slice lengths. Debug builds
   panicked; wrapping in release could choose the wrong cost strategy.
   The corrected comparison cancels the common `small` term and uses
   checked multiplication. An overflowing product cannot be cheaper than
   the representable right-hand cost, so it selects the merge path.
   `sorted_difference_cost_comparison_handles_large_lengths` compares with
   exact `u128` costs without allocating huge slices. It failed with an
   overflow panic before the correction and passes after it. Catalog case:
   `T-LIB-038`.
2. **The documented benchmark command did not produce a clean CSV stream.**
   `make integer-ab > run.csv` included echoed recipes, version output,
   and baseline preparation diagnostics. Recipes are now quiet and setup
   diagnostics go to stderr. The Bats regression
   `integer-ab keeps setup diagnostics out of its CSV stream` failed before
   this change and passes after it.
3. **Shell validation omitted the new baseline helper.** `make shell-check`
   now includes `prepare-baseline.sh` in ShellCheck and shfmt. The benchmark
   target also honors Make's `SHELL`, allowing execution with Bash 3.2.
4. **The full planner matrix exhausted Miri's 32-bit address space.** The
   first Miri gate ended with an internal compiler error during
   `early_survey_exit_selects_the_same_backend_as_a_complete_survey`.
   Miri never reuses stack addresses, so this is an interpreter resource
   limit rather than a diagnosed undefined-behavior failure. The Make gate
   now runs all 88 shape/word-width/policy tuples in separate interpreter
   processes, with four concurrent processes by default. Inputs, per-tuple
   comparison assertions, target, provenance checking, and interpreter
   validation remain intact. Native runs still execute the complete matrix
   and its aggregate assertion that all three backends were selected.
   A Bats case verifies that the runner schedules every tuple exactly once.
   The selector is accepted only under `cfg(miri)`, using the documented
   [`-Zmiri-env-forward` flag](https://github.com/rust-lang/miri/blob/master/README.md#miri--z-flags-and-environment-variables)
   to forward one fixed test parameter. The first failure is not counted as
   a pass; its compiler report is retained in `target/integer-review-miri-ice.txt`.
   The 200,000-code alternating `u64` case with Dense disabled passed alone.
   Miri's isolated clock is simulated; its printed test duration is not a
   wall-clock measurement. CI allows 90 minutes for ownership and 120 minutes
   for the combined release gate to accommodate the full matrix; these are
   time budgets, not measured CI durations.

## Contract review

Source review covered all production changes, their tests, the benchmark
driver and setup script, catalog changes, and updated contract documentation.
No additional unresolved correctness finding was identified in that scope.

| Changed behavior | Asserting evidence inspected |
| --- | --- |
| Sorted algebra, skewed merges, and planner shortcuts | `sorted_kernel_matches_member_algebra_across_shapes`, `early_survey_exit_selects_the_same_backend_as_a_complete_survey`, and existing representation-pair tests |
| Normalization order, codec errors, and width boundaries | `construction_normalizes_every_arrival_order_identically` and `construction_reports_contract_errors_on_every_path` |
| Native Roaring cache and run probes | `cached_native_statistics_match_a_fresh_summary` and both `sorted_roaring_builder_*` tests |
| Partial traversal and borrowed representative identity | `public_iterator_folds_resume_after_partial_consumption`, `folds_match_stepwise_traversal_for_merge_and_seek_kernels`, and existing representative/finite-domain tests |
| Policy-compatible root sharing | `scattered_sorted_results_share_an_equal_operand`, `empty_results_share_an_eligible_empty_operand`, and existing output-policy tests |
| Collection bulk paths, first keys/last values, clone panic progress | `collected_ordered_families_match_entry_by_entry_insertion`, `bulk_extension_keeps_representatives_policies_and_partial_progress`, and existing ownership tests |
| Iterator end queries and adaptive empty-tree ranges | `iterator_end_queries_match_a_complete_traversal` and `adaptive_ordered_range_on_an_emptied_tree_matches_the_sorted_form` |
| Hash extension reservation | `hash_extension_reserves_like_the_standard_collections` |

This is a scoped contract-to-test map, not a complete public API coverage
inventory or a proof over arbitrary inputs and user implementations.

## Fresh execution

Host: `aarch64-apple-darwin`, Rust `1.99.1`. Diagnostic nightly:
`nightly-2026-11-08`. Shell floor: `/bin/bash` `3.2.57`.

| Command | Result |
| --- | --- |
| `make check` | Passed on the corrected tree, including formatting, links, Clippy, rustdoc, tests, examples, catalog, and benchmark inventory. |
| `make nextest` | Passed: 307 tests, zero skipped. |
| `make fuzz-check` | Passed deterministic fuzz-oracle regressions. |
| `make integers` | Passed alloc-only integer tests, release integer tests, and the isolated downstream codec package. |
| `make embedded` | Passed both pointer-atomic and non-atomic targets, core-only, and alloc-compatible integrations. |
| `make profiles` | Passed core-only, alloc-only, and default profiles. |
| `make coverage-gate` | Passed portable floors and the exact macOS baseline ratchet. |
| `make shell-check SHELL=/bin/bash` | Passed ShellCheck, shfmt, and all 37 Bats cases using Bash 3.2. |
| `actionlint .github/workflows/ci.yml .github/workflows/verification.yml` | Passed validation of the workflows with revised Miri time budgets. |
| `make bounded tools-check` | Passed finite collection models, production retry schedules, native Kani helpers, and ast-grep rule tests/source scan. This did not run the Kani solver. |
| `make miri` | Passed the revised complete gate, including both aliasing models, 32-bit storage/hash/iterator checks, and all 88 isolated planner tuples. The original single-process failure remains recorded above; fresh output is in `target/integer-review-miri.log`. |
| `AB_FILTER=__validation_only__ make integer-ab AB_BASELINE=22697a8 AB_ROUNDS=1 SHELL=/bin/bash` | Passed release build and all setup assertions; selected no timing cases. No new performance claim. |

Coverage scope: `cargo-llvm-cov 0.9.1`, all features, host test/example
targets, source files including inline test modules, excluding
`(^|/)(tests|benches|examples|verification)/`. Lines: 7,955/9,794 (81.223%);
functions: 1,492/1,975 (75.544%); regions: 12,561/15,347 (81.847%).
The generated scope and summary are under `target/coverage/`. These are
source metrics, not measured public-obligation coverage.

The two archived final A/B CSVs and their A/A control each contain the same
630 unique case names. The control's recorded ratio range is 0.620–1.718,
consistent with the historical report. This was a data check, not a timing run.

The prior independent scratch differential suite was not available and was
not rerun. This review makes no fresh Linux/Windows runtime, performance,
mutation, symbolic-proof, or unbounded memory-safety claim.

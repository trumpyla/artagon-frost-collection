---
id: FROST-CRATE-SPLIT-PLAN-REVIEW
status: review
reviewed_base: ab15901
updated: 2026-11-13
implementation: unchanged
---

# Adversarial review: crate-split execution plan

Reviewed on 2026-11-13: the first draft of
[`docs/design/crate-split-plan.md`](../design/crate-split-plan.md) and of the
rewritten `frost-wlt` tasks. The maintainer asked for the epic to be reviewed
adversarially and corrected before it is executed. This record lists what the
review found and what changed. The plan and tracker now in the repository are
the corrected ones.

## Method and independence

| Reader | Scope | Method |
| --- | --- | --- |
| Technical reviewer | Layering, aliasing, feature forwarding, coherence, hidden internals, packaging, test owners | Compiled two workspaces on Rust 1.99.1: a synthetic one with the plan's names and layering, and the real sources laid out as the draft said, with every test and bench copied into every package. Type-checked; ran no test binary |
| Plan reviewer | Tracker fidelity, `CRATES.md` and decision records, tooling inventory, acceptance criteria, honesty of claims | Read-only comparison with the original tracker text, the standards, the Makefile, CI and configurations. Ran no build |
| Integrator | The reviewers' claims about source | Read the cited lines; ran the gates named below |

The plan's author is the integrator. The reviewers worked from the repository
alone, in separate contexts, without the author's reasoning and without each
other's findings. All three are runs of AI models; no person reviewed it. The
spike workspaces are not in the repository, so their compile results are
reported, not reproducible from it. The disk filled during the review; the
technical reviewer therefore type-checked but did not run tests.

## Result

The draft was not executable as written. Three findings would have failed
outright (T01, T02, P02), and five would have let verification weaken without
any gate noticing (P01/T05, T07, P03, P04, T09). One (P07/T04) showed that the
draft's adaptive package cannot be extracted without opening a seal, which is
a contract change the author may not make. The corrected plan drops that
package, tightens the gates, and moves the catalog design ahead of the first
move. No finding was dismissed.

## Findings and dispositions

Severity: **B** blocker, **H** high, **M** medium, **L** low. "Compiled" means
the technical reviewer has compiler or Cargo output for it.

| ID | Sev | Finding | Disposition |
| --- | --- | --- | --- |
| T01 | B | `cargo package` cannot resolve `publish = false` path siblings: "no matching package named `frost-collections-core` found". With `publish = true` Cargo's local overlay works. Compiled, offline. | D13: `make package` copies each member's `cargo package --list` file set into a scratch workspace and tests it. Decided in `.8`, applied to siblings in `.9`. |
| T02 | B | After the first extraction the facade's `tests/advanced.rs` (26 errors), `benches/snapshots.rs` and `tests/publication_model.rs` no longer compile: they name optional crates directly or include a moved file. Compiled. | D7: facade dev-dependencies without default features, added in `.9`; the `#[path]` is repointed in `.12`. |
| P01, T05 | B | The "lowest package in which it compiles" rule sent `integer_sets` and `integer_output` to the `std`-only concurrent package, or to the integer package with fewer tests. Either way the alloc-only and i686 Miri lanes would run the same count against a different build. Compiled with `cargo tree`. | D14: the rule adds "same resolved Frost features in every lane"; both targets stay in the integer package and their `arc-swap-1` cases move to a concurrent target. |
| P02 | B | The per-task gates left out `make integers`, `examples`, `package`, `loom` and `fuzz-check`, all of which key on moved paths and run in CI. | Gates for `.8`–`.13` are `make verify`, `embedded`, `bounded`, `loom`, `fuzz-check` and `doctest-codes`. |
| P07, T04 | H | The draft made "the serde seal" hidden-public so an adaptive package could implement it. `StrictEntries` and `Canonical` are documented as sealed; this opens them, against the epic's own acceptance. | D5: the adaptive profiles stay in core. Extracting them is `frost-adp`, a maintainer decision. No seal changes. |
| T06 | H | The adapter code shared by family and adaptive types is eleven `macro_rules!` and nine helpers. Sharing them across crates needs `#[macro_export]` and a rewrite to `$crate` paths, not a visibility change. | Removed by D5. |
| T03 | H | Glob layering produces `ambiguous glob re-exports` warnings in the library and a hard error only downstream (`prelude` is ambiguous). An `adapters` module was missing from the draft's list. A private `use` can silently remove a globbed name. Compiled. | D6: each layer defines `prelude` locally and the concurrent package `backend`; `adapters` exists only in core after D5; the two lints are denied; the facade re-exports by name. |
| T04 | H | `pub use core::*` leaks `__private` and exported macros to `frost::`. `pub(crate)` items cannot be re-exported (E0364). Compiled. | D6, D8: the facade does not glob; one `__private`, in core; wrappers there. `.12` asserts that `frost::__private` does not resolve. |
| P03 | H | "Lanes available on the host were run" let the only unsafe file move with no Miri run. The host lacked the pinned nightly, the embedded targets and most tools. | D16 and plan section 9. The nightly with Miri, both embedded targets, ShellCheck, shfmt, Bats, ast-grep, cargo-nextest and cargo-llvm-cov were installed. `.9` and `.11` require the Miri lanes to pass. Still missing: cargo-mutants, cargo-fuzz, Verus. |
| P04 | H | `make check` runs the catalog, yet the draft split the catalog after the extractions that move catalogued unit tests. The tool hard-codes six manifests; harness records lie outside any package root; four contracts have cases in two packages; the tool's own 14 cases had no owner. | D11: workspace mode for the tool, a root catalog for repository tooling, cross-file contracts and IDs, all in `.8`; records move with their code in each task. |
| P05 | H | The draft said "accepted for execution" before the maintainer saw it, linked a review record that did not exist, and `.7` work had begun. | Status is `proposed`; `.1` requires that the plan was shown; this record exists; risk recorded in plan section 8. No file had moved. |
| P06 | H | A dozen rules and about twenty policy passages change but were not listed. | Plan section 6 lists them with a task each. |
| P08 | H | (a) `benches/support/mod.rs` would have been copied into each package. (b) The README is crate documentation with doctests and must move with the package. (c) Examples name optional crates directly. | D12; the README moves in `.8`; D7. |
| P09 | H | Tooling keyed on the layout had no owner: `integer-ab` and its baseline script, the shell-test fixtures, mutation and coverage configuration, ast-grep rule, fuzz and Kani recipes, five historical manifests, package `exclude`. | A per-line inventory is committed as evidence and `.15` closes on a grep that must return nothing. |
| P10 | H | Acceptance criteria were not measurable; the empty-selection guard summed over binaries. | Criteria reworded in the tracker: `git diff -M100%`, `--find-copies-harder`, `cargo tree -e features`, multiset count rule, name sets of harnesses. The guard checks each binary; two shell tests cover it. |
| T07 | M | A `cfg(feature = "alloc")` gate in a test moved to a package that always has `alloc` is false while the code is present. Compiled. | D14: such a gate is removed when the target moves and listed in the migration table. |
| T08 | M | At the virtual root `--no-default-features` still resolves core with `std`. Compiled. | Recipes select with `-p`; each profile lane asserts core's features with `cargo tree`. |
| T09 | M | On stable a `compile_fail` doctest whose alias is wrong still passes; only the nightly notices. `use … as frost` fails with a nested `mod`. Compiled. | The pinned nightly is installed and `make doctest-codes` is a gate. `extern crate` form required. The four doctests whose error has no code are re-run as ordinary doctests in `.16`. |
| T10 | M | The draft told `.11` to rewrite the Miri name filters, but the module path is unchanged; rewriting is how an empty lane returns. | Clause removed; the guard stays. |
| P11 | M | D2–D10 went beyond the instruction and "each is reversible" was untrue for the structural ones. The library name `frost_core` equals the crates.io crate's. The draft did not decide whether the concurrent package still merits a crate. | Plan section 1.1 states the cost of undoing each; default library names (D4); D9 keeps the concurrent package and says why. |
| P12 | M | `int-key-downstream` cannot depend on the facade from inside the integer package. Splitting Kani and fuzz rewrites harness code. | D15; `.14` requires equal proof, target and test name sets and an empty diff of bounds. With D5 the Kani package is not split. |
| P13 | M | 211 links and about 311 prose mentions point into moving paths and change in several tasks; some are in records kept verbatim; the link checker covers only root and `docs/`. | A script rewrites links at each move; verbatim records change in path only; `.8` extends the checker. |
| P14 | M | Risks missing: tool-poor host, void coverage ratchet, unproven benefit, link churn, bisecting, CI not run, stale lockfiles, mutation scope, early start, untracked defects. | Plan section 8; `frost-r0a` and `frost-k4n` created. |
| T11 | L | Each layer re-documents the layer below (184 to 1,179 pages). | `#[doc(no_inline)]` on sibling globs (D6). |
| T12 | L | jq filters on the facade's name see only the facade. | `.8` and `.15` select packages from metadata. |
| P15 | L | The weak forwards in `std` were not in the feature map. | Plan section 4. |

Task numbers in findings refer to the reviewed draft. Its `.10` (extract an
adaptive package) no longer exists; see `frost-adp`.

## First review, S01–S25

| Finding | Answered by |
| --- | --- |
| S01 orphan rule for live backends; is the concurrent crate worth it | D9 |
| S02 `make package` with unpublished siblings | D13 |
| S03 names unchecked | D4 and the committed receipt |
| S04 shared internals understated; adapter helpers | D5 removes the adapter split; D8 for the rest |
| S05 `__private` mechanics, pins, lockstep | D3, D8 |
| S06 `int-set` and the Roaring adapter | D10, `frost-r0a` |
| S07 doctest plan | D7, `make doctest-codes`, error codes added |
| S08 Miri lanes passing empty | Guard in `scripts/require-tests.sh`, shell tests |
| S09 uncompletable tasks, per-file owners | D14 and plan section 5; owners are confirmed by compiling in `.13` |
| S10 standards work first | D2; the three contracts were curated first |
| S11, S12 split now; benefit unproven | D1; stated as an accepted risk, not argued |
| S13 policy and links too late | `.8`, plan section 6, link script |
| S14 tooling inventory | Layout inventory; `.15` criterion |
| S15 dropped criteria, reused IDs | Table below; `.2`–`.5` closed, new IDs |
| S16 weak per-step gates | Gates of `.8`–`.13` |
| S17 conflicts with `CRATES.md` | Plan section 6 |
| S18 facade tests naming optional crates; lost re-export | D7; `.16` compares the path lists, which include re-exported crates |
| S19 later adoptions without owners | Plan section 5 |
| S20, S23 misstatements in the proposal | The proposal is superseded |
| S21 catalog, coverage, mutation risks | D11; plan section 8 |
| S22 missing decisions | D3, D4; licence and docs.rs metadata are inherited workspace fields in `.8` |
| S24 other risks | Plan section 8 |
| S25 weak forwards; plan-type alternative | Plan section 4; the plan types do not move under D5 |

## Original tracker criteria

| Original | Now |
| --- | --- |
| Epic: flat `crates/frost-*`, `frost-api` facade | Changed by D4, flagged as a default |
| Epic: every module and harness has an owner | Epic acceptance; plan sections 3 and 5 |
| Epic: import spelling and contracts preserved | Epic goal and acceptance, with the contract list restored |
| Epic: layout satisfies `frost-wou` | Epic acceptance names `CRATES.md` section 1; the dependency is dropped by D2 |
| Epic: profiles, features, downstream codecs, embedded, package tests, Miri and model gates verified | Epic acceptance and `.6`, with D16 |
| Epic: no placeholders, no unsupported claims | Epic acceptance |
| `.1` mapping, graph, seal and coherence plan, feature map | Plan sections 2–5; `.1` |
| `.2` moves without weakening or duplicating; stable IDs; policy in the same change as the first move | `.8`, `.9`, `.11`, `.12`, `.13` |
| `.3` harness linkage, isolation, no duplicates, bounds preserved, package exclusions | `.14` |
| `.4` paths and `compile_fail` resolve; partners, bounds, hashers, auto traits | `.16` |
| `.5` observational help, propagation, members exercised, shell-check on the minimum Bash | `.15` |
| `.6` receipts, benchmarks, denominators, no reuse | `.6` |

## Not checked by this review

Test execution on any split layout; a compiled state after only the first
extraction; `include_str!` of the README in an extracted facade; the catalog
tool on several packages; Clippy pedantic on the split; embedded builds, Miri,
Loom, Kani, fuzzing, coverage and mutation on a split; `cargo package` with a
named registry; anything on GitHub CI.

[Plan](../design/crate-split-plan.md) ·
[First review](crate-split-adversarial-review.md) ·
[Crate standard](../../CRATES.md)

---
id: FROST-CRATE-SPLIT-PLAN
status: proposed
epic: frost-wlt
updated: 2026-11-13
supersedes: docs/design/proposals/crate-split.md
---

# Crate split: execution plan for `frost-wlt`

The maintainer asked on 2026-11-13 for the crate-split epic to be reviewed
adversarially, corrected, shown, and executed. This record is the corrected
plan. It replaces the [proposal](proposals/crate-split.md). It was itself
reviewed by two readers who did not write it; the
[plan review](../reviews/crate-split-plan-review.md) lists their 27 findings
and what changed. The [first review](../reviews/crate-split-adversarial-review.md)
(S01–S25) is answered there too. The tracker tasks under `frost-wlt` carry the
acceptance criteria; this record carries the design they refer to.

Status is `proposed` until the maintainer has seen it. Decisions D2–D16 below
are the author's defaults, taken so that execution is not blocked. The
maintainer may overrule any of them; section 1.1 says what each costs to undo.

## 1. Decisions

| # | Decision | Reason | Answers |
| --- | --- | --- | --- |
| D1 | Split now, before the API refactor. | Maintainer instruction of 2026-11-13. The refactor may redraw these seams; that cost is accepted. | S11, S12 |
| D2 | `frost-wlt` no longer waits for all of `frost-wou`. The parts that protect a move are pulled into `frost-wlt.7`. `frost-wou.6`–`.9` stay open and are re-based on the new layout; `frost-wou.10` (automated dependency-isolation check) stays open and is checked by hand in `.14`. | The instruction to execute. Every contract now has a curated case before its code moves. | S10 |
| D3 | Nothing is published. Every package is `publish = false`. The facade is the only supported entry point; the other packages are internal. | No external consumer exists. Publishing needs its own decision on names, versions and order. | S02, S03, S05, S22 |
| D4 | Package names: facade `frost-collections` (library `frost`); `frost-collections-core`, `frost-collections-integer`, `frost-collections-concurrent`, and `frost-collections-test-support`, each with its default library name. A package's directory is `crates/<package name>`. | `frost-core` exists on crates.io as an unrelated crate; the names chosen returned 404 from the index on 2026-11-13 ([receipt](../../verification/results/crate-split/crate-names.txt)). One rule: `frost-collections` plus a domain. The facade keeps the existing package name. Default library names avoid sharing `frost_core` with that crate. | S03, P11 |
| D5 | Three code packages. The adaptive profiles stay in the core package. | Extracting them needs the serde seal `adapters::serde::private::Sealed` to be implemented from another crate, which opens it, and eleven adapter macros to be exported and rewritten. Opening a seal relaxes a contract, which only the maintainer can approve. It is tracked as `frost-adp`. | P07, T04, T06, S12 |
| D6 | Layered roots. The integer and concurrent packages re-export the core package's public surface and add their own module at the `frost::` path it has today, as `std` does for `core`. Each defines `prelude` locally; the concurrent package also defines `backend`. Sibling globs are `#[doc(no_inline)]`. `ambiguous_glob_reexports` and `hidden_glob_reexports` are denied in the workspace lints. The facade names each re-export explicitly and does not re-export `__private`. | Tests and doctests can run in the owning package against its own surface with the `frost::` spelling kept. Explicit facade re-exports make its surface reviewable and keep hidden items out of it. | S07, S18, T03, T04, T11 |
| D7 | A test or doctest in the integer or concurrent package names its package `frost` with `extern crate <library> as frost;` (hidden in doctests). Examples stay in the facade. The facade lists as dev-dependencies the optional crates its tests, benches and examples name directly, without default features. | Rendered documentation and test bodies keep the supported spelling. Examples document the supported entry point. | S07, T02, T09, P08 |
| D8 | Items another package needs are `#[doc(hidden)] pub` and reached through `frost_collections_core::__private`. Inherent functions and `#[non_exhaustive]` construction go through wrapper functions there. Members depend on each other by path with an exact version and share one workspace version. No algorithm is duplicated. | Rust has no visibility between `pub(crate)` and `pub`. | S04, S05 |
| D9 | Capability-trait implementations for `dashmap`, `scc` and `crossbeam-skiplist` types live in core's `backends`, behind the same feature names. The live-access traits and their implementations stay in the concurrent package, which is kept: it isolates the `std`-only code, the `arc-swap` dependency, the snapshot seal and the Loom model. | The orphan rule requires the first part. | S01 |
| D10 | A move preserves behaviour. Known defects are tracked, not fixed inside a move: `frost-r0a` (`int-set` exposes the Roaring adapter against CR-DEP-1) and `frost-k4n` (two Kani harnesses do not finish). The core package has an `int-set` feature so today's surface is kept. | A move that also changes behaviour cannot be checked against a baseline. | S06 |
| D11 | Each package owns its catalog and benchmark inventory under `<package>/verification/metadata/`, with package-relative paths. A root catalog holds the records of repository tooling. One invocation of the catalog tool checks the workspace: IDs are unique across files, a case may cite a contract defined in another file, and the union of case IDs equals the 397 present before the move. | CR-LAYOUT-9, CR-META-1. | S21, P04 |
| D12 | `frost-collections-test-support` holds the workload factories and the dependency-free benchmark timing helpers. It depends on no Frost package and is only a dev-dependency. | Seven benches in three packages share the timing helpers; copying them would duplicate code. | P08 |
| D13 | While nothing is published, `make package` copies each member's `cargo package --list` file set into a scratch workspace and builds and tests it there. | `cargo package` cannot resolve `publish = false` path siblings (compiled evidence, T01). | S02, T01 |
| D14 | A moved test keeps its executed-test count and the resolved feature set of every Frost package in each lane. `tests/integer_sets.rs` and `tests/integer_output.rs` go to the integer package; their `arc-swap-1` cases move to a target in the concurrent package. A gate on a feature the new package always has (`int-set`, `alloc`, `std`) is removed when the target moves. | Otherwise the alloc-only and i686 Miri lanes would silently run against a `std` build. | P01, T05, T07 |
| D15 | `int-key-downstream` stays with the facade. | It checks what a consumer of the supported entry point can implement. | P12 |
| D16 | A lane that cannot run on the host blocks closing the epic unless the maintainer waives it in `docs/design/decisions.md`. | "Not run" must not read as verified. | P03 |

### 1.1 What undoing a decision costs

D3, D4, D8, D13 and D16 are cheap to change: names, visibility and recipes.
D2 costs nothing to reverse before `.8`. D5 is reversed by `frost-adp`. D6, D7
and D11 are structural once `.13` is done: reversing them means moving tests,
doctest aliases and catalog files again. D9 and D14 follow from Rust's rules
and from keeping today's lanes; they are not preferences.

## 2. Packages

| Package | Responsibility | Depends on | `no_std` | Unsafe |
| --- | --- | --- | --- | --- |
| `frost-collections-core` | Capability traits, plans, `Frozen` and its seal, the five families and their set algebra, adaptive profiles, native backends, framework implementations | — | yes; `alloc` and `std` optional | `deny`, allowed only in `src/adaptive/seq/buf.rs` |
| `frost-collections-integer` | The typed integer-set profile, codec, policies, private storage | core | yes; always `alloc` | `forbid` |
| `frost-collections-concurrent` | Live guarded access, snapshot publication and its seal | core; integer when `int-set` | no; always `std` | `forbid` |
| `frost-collections` | Facade: re-exports, the complete prelude and backend modules, feature forwarding, README as crate documentation, examples | all three | follows features | `forbid` |
| `frost-collections-test-support` | Workload factories and benchmark timing helpers | — | — | `forbid` |

Each package owns code; none is a placeholder.

## 3. Module mapping

| Today (`src/`) | Package | Path there |
| --- | --- | --- |
| `frozen.rs`, `iter.rs`, `plan.rs`, `read.rs`, `write.rs`, `backend.rs`, `mutable.rs`, `backends/`, `family/`, `adaptive/`, `adapters/` | core | `src/`, unchanged |
| `concurrent/{dashmap,scc,skiplist}.rs`: capability-trait implementations | core | `src/backends/` |
| `integer/` | integer | `src/integer/` (module `integer`, so unit-test paths keep the `integer::` prefix) |
| `concurrent/{mod,live,publication,snapshot}.rs`; live-trait implementations from `dashmap.rs`, `scc.rs`, `skiplist.rs` | concurrent | `src/concurrent/` (module `concurrent`) |
| `lib.rs` | each root, split by ownership | `src/lib.rs` |
| `README.md` (crate documentation, with doctests) | facade | `crates/frost-collections/README.md`; a short workspace README replaces it at the root |

`prelude` is defined in every layer: the lower layer's names plus its own.
`backend` is defined in core and extended in the concurrent package
(`arc_swap`). The facade's are the complete ones.

### 3.1 Items that become `#[doc(hidden)] pub`

The list is closed by compiling and recorded in the migration table. Expected:
the ordered cursor constructors and the `intersection_len`, `is_disjoint` and
`is_subset` kernels in `family::ordered::set_ops`, and construction of
`Rejected`, all for the integer package. No seal is opened:
`frozen::sealed`, `Frozen::new_unchecked`, the serde seal and the snapshot seal
stay private to the package that defines them. The cursor constructors take
slices that must be strictly ascending; passing others gives wrong results, not
undefined behaviour.

## 4. Features

Every package declares every feature of the packages below it under the same
name and forwards it, including the weak forwards in `std` (`indexmap?/std`,
`borsh?/std`, `schemars?/std`, `roaring?/std`).

| Feature | Owned by | Forwarded by |
| --- | --- | --- |
| `alloc`, `std` | core | integer (always `alloc`), concurrent (always `std`), facade |
| Backend and framework lines; `dashmap-6`, `scc-3`, `crossbeam-skiplist-0_1` | core; the concurrent package also uses the last three for live access | every layer above |
| `arc-swap-1` | concurrent | facade |
| `int-set` | integer; core has `int-set` for the Roaring adapter it exposes today | concurrent, facade |
| `advanced`, `concurrent`, `frameworks` | facade aggregates | — |

In the facade `int-set` enables the integer package and `std` the concurrent
package. Recipes always select a package with `-p`; none relies on feature
unification at the workspace root. A recipe compares the feature tables of
adjacent layers, and each profile lane first asserts, with
`cargo tree -e features`, that core resolves without `std` (or `alloc`) where
the profile says so.

## 5. Tests, benches, examples, harnesses

A target moves to the lowest package in which it compiles, executes its
baseline test count, and resolves the same Frost features in every lane (D14).
A target whose cases separate cleanly by owner is split and the split is listed
in the migration table. Expected owners, to be confirmed by compiling:

| Package | Tests | Benches | Isolated harnesses |
| --- | --- | --- | --- |
| core | Every target that uses only core paths, including the adaptive, family, backend, adapter, set-algebra and formal-model targets | `set_operations`, `ordered_sets`, `adaptive_maps`, `performance_review` | Ordered-set fuzz target; the Kani package (it includes `adaptive/seq/buf.rs` by path); Verus proofs |
| integer | `integer_sets`, `integer_output` without their `arc-swap-1` cases; integer cases of `metadata_and_factories` | `integer_sets`, `integer_output` | Integer fuzz target |
| concurrent | `concurrent`, `publication_model` (includes `publication.rs` by path), the `arc-swap-1` integer cases | `snapshots` | Loom (includes `publication.rs` by path) |
| facade | Targets that need the complete prelude or all packages; every example | — | `int-key-downstream` |

Historical evidence under `verification/` stays where it is (CR-LAYOUT-8). Its
manifests that depend on the root package by relative path get the new path
and, where one is committed, a refreshed lockfile; nothing else in them
changes. Links inside records kept verbatim are changed only in their path.

Every tooling file keyed on today's layout is listed, with its line, in
[the layout inventory](../../verification/results/crate-split/layout-inventory.tsv).
`.15` closes when no row's file still names a root-relative `src/`, `tests/`,
`benches/` or `examples/` path.

## 6. Rules that change

| Rule or text | Change | Task |
| --- | --- | --- |
| One-package policy: `AGENTS.md`, implementation contracts, `repository-rules.md`, `guide.md`, `github-workflow.md`, `testing.md`, `documentation-rules.md`, root README, `docs/README.md`, `docs/verification.md`, coverage policy, `decisions.md` rows that name it | Replaced by the package list, in the same change as the scaffold | `.8` |
| CRATES.md target table and section 9 row; `frost-api` | Names from D4 | `.8` |
| CR-BOUND-1 | The support package may also hold dependency-free measurement helpers | `.13` |
| CR-BOUND-2 | The facade holds no algorithms; sibling roots mirror `frost::` paths (D6) | `.12` |
| CR-BOUND-3, CR-LAYOUT-7 | Name the new paths of `buf.rs`, `publication.rs` and the harnesses | `.8`, `.9`, `.12`, `.14` |
| CR-LAYOUT-5 | Benchmark support comes from the support package | `.13` |
| CR-LAYOUT-6 | The fuzz package is split by owner in one task | `.14` |
| CR-LAYOUT-9 | Examples and `int-key-downstream` are facade artifacts (D7, D15) | `.13`, `.14` |
| CR-DEP-1 | Unchanged in substance; `frost-r0a` records that `int-set` exposes the Roaring adapter | `.9` |
| CR-DEP-5 | States the scratch-workspace package test (D13) | `.9` |
| CR-DEP-7 | Lints are inherited; `unsafe_code` is `forbid` per package except core | `.8` |
| CR-FEAT-4 | Isolated-feature lanes run per package | `.15` |
| CR-API-1 | Sibling roots and hidden items are not obligations; `frost::` paths are | `.12` |
| CR-COV-2, CR-WAIVE-2 | Coverage paths, baseline and denominator restated for the workspace | `.15` |
| CR-MAKE-1, CR-MAKE-2, CR-MAKE-4 | Recipes select packages from `cargo metadata`; the evidence scripts under `verification/results/crate-split/` are not build tooling | `.8`, `.15` |
| CR-META-3, CR-META-5, catalog schema | Per-package catalogs, a root catalog, counts per package (D11) | `.8`, `.13` |

## 7. Migration table

Filled in by each task: every public path, bound, feature or auto trait that
changes, every item made `#[doc(hidden)] pub`, every split or moved artifact,
and every edit to a moved test other than its alias line.

| Task | Change | Reason |
| --- | --- | --- |
| `.8` | `src/`, `tests/`, `benches/`, `examples/`, `README.md` and the package manifest moved to `crates/frost-collections/`; 131 files are exact renames (`git diff -M100%`). No Rust source line changed. | Workspace scaffold. |
| `.8` | Root `Cargo.toml` is a virtual workspace; package version, edition, Rust version, licence, `publish` and lints are inherited. Dependency versions stay in the package until a second package shares them. | CR-DEP-7. |
| `.8` | Root `README.md` is a new workspace overview. The former README is the crate documentation at `crates/frost-collections/README.md`. | It is included by `src/lib.rs` and must be inside the packaged file set. |
| `.8` | The catalog is split: 361 records and all 12 contracts in `crates/frost-collections/verification/metadata/catalog.json`, 36 harness and tooling records in the root file, which also lists the isolated manifests. The benchmark inventory moved with the package. Two new records, `T-MAIN-015` and `T-MAIN-016`, cover the catalog tool's workspace tests. | D11. |
| `.8` | `make package` tests a scratch workspace instead of an extracted `.crate`. | D13. |
| `.8` | `make integer-ab` accepts only baselines from before this change until `.15` teaches `prepare-baseline.sh` the workspace layout. | The script archives a root package. |
| `.9` | Everything in `src/` except `integer/`, `concurrent/` and a new facade root moved to `crates/frost-collections-core/src/`. `lib.rs` moved with it and lost the integer and concurrent modules, the `int-set` prelude names and the README include. | Core extraction. |
| `.9` | `frost::backend::arc_swap` is re-exported by the facade's own `backend` module; core's `backend` no longer names `arc_swap`. The path is unchanged for users. | `arc-swap` is a dependency of the concurrent code only. |
| `.9` | The `MapTypes`, `ElementType` and `SetContains` implementations for `dashmap`, `scc` and `crossbeam-skiplist` types moved from `concurrent/{dashmap,scc,skiplist}.rs` to `backends/` of core. Bodies unchanged. | D9; the orphan rule. |
| `.9` | New hidden interface `frost_collections_core::__private`: `rejected(input, error)` builds a `Rejected`; `set_ops::{union, intersection, difference, symmetric_difference, intersection_len, is_disjoint, is_subset}` wrap the ordered kernels. Nothing became `pub` at its definition and no seal changed. Eleven call sites in `integer/{storage,chunked,set}.rs` use them. | D8. |
| `.9` | 24 doctests in core gained the hidden line `# extern crate frost_collections_core as frost;`. Core has `serde` (derive) and `serde_json` as dev-dependencies for one doctest. | D7. |
| `.9` | The facade gained dev-dependencies `imbl`, `slotmap` and `fixedbitset`, which `tests/advanced.rs` names directly. | D7, T02. |
| `.9` | Doctest counts split between `frost` and `frost_collections_core`; the sums equal the baseline in every profile. Unit and integration counts are unchanged. | Doctests live with their items. |
| `.9` | Records `T-LIB-001`–`003` (bitmap unit tests) moved to core's catalog. Contracts stay in the facade's catalog until `.13` settles test ownership. | D11. |
| `.9` | Profile and isolated-feature lanes run once per library package; `make feature-isolation` asserts the facade's no-`std` profiles resolve core without `std`. Lockfiles of the Kani, fuzz, downstream and `revision2-integration/ab-harness` packages gained the new dependency. | CR-MAKE-4, T08. |
| `.11` | `src/integer/` moved to `crates/frost-collections-integer/src/integer/`. The package root re-exports the core package and adds `integer` and its own `prelude`. The facade re-exports `integer` from it under `int-set`. | Integer extraction; D6. |
| `.11` | The integer package always enables `alloc` and `int-set` on core; its own `alloc` and `int-set` features are empty. The 21 integer unit tests are no longer behind `cfg(feature = "int-set")`; their catalog records drop that condition. | D14, T07. |
| `.11` | Six integer doctests gained the hidden alias. `roaring` is no longer a dependency of the facade, only a dev-dependency for `tests/advanced.rs`. | D7. |
| `.11` | The 32-bit Miri lanes that select integer unit tests by name select the integer package; the names are unchanged. | T10. |
| `.11` | New record `T-LIB-046` marks the facade's library target, which no longer has unit tests. Records `T-LIB-004`–`024` moved to the integer package's catalog. | CR-META-2, D11. |

## 8. Risks accepted

- The API refactor may redraw these seams (D1). The benefit of splitting now
  was not demonstrated by measurement (S12); it is taken on instruction.
- Hidden `pub` items are an unversioned interface between packages. Publishing
  is blocked on a decision that must address it (D3).
- Links and catalog paths change in up to four tasks (`.8`, `.9`–`.12`, `.13`,
  `.14`). A script rewrites links; prose mentions are fixed by hand.
- Benchmarks recorded for one crate are not comparable until re-measured;
  cross-crate inlining may differ. The coverage ratchet's reference host is not
  this host, so its baseline is re-recorded, not compared.
- `cargo-mutants` tests the mutated package by default; until `.13` moves the
  tests, core mutants would look unkilled.
- This host lacks some tools (section 9). CI has not run on the new layout.
- `git mv` keeps blame for moved files; a split file keeps it for one half.
  Bisecting across the scaffold commit needs the old paths.
- Isolated packages keep their own lockfiles, which are refreshed when their
  path dependency moves.
- Preparation for `.7` (baselines, the empty-selection guard, error codes,
  three curated records) was done while this plan was under review. No file
  moved before it was shown.

## 9. Host prerequisites

Installed on the execution host: Rust 1.99.1, `nightly-2026-11-08` with Miri,
both embedded targets, `jq`, Kani 0.68.0. Each task records which of
ShellCheck, shfmt, Bats, ast-grep, cargo-nextest, cargo-llvm-cov,
cargo-mutants, cargo-fuzz and Verus were available when it ran. A lane whose
tool is missing is reported as not run and falls under D16.

[Tracker](../../.beads/issues.jsonl) ·
[Crate standard](../../CRATES.md) ·
[Decisions](decisions.md)

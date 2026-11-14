---
id: FROST-CRATE-SPLIT-ADVERSARIAL-REVIEW
status: review
reviewed_base: 3fd0711
updated: 2026-11-13
implementation: unchanged
---

# Adversarial review: crate-split proposal

Reviewed on 2026-11-13: [`docs/design/proposals/crate-split.md`](../design/proposals/crate-split.md),
the proposed revision of epic `frost-wlt`. The maintainer asked for an
adversarial review before deciding on it.

## Method and independence

| Reader | Scope | Method |
| --- | --- | --- |
| Technical reviewer | Module graph, coherence, shared internals, features, tooling keyed on paths | Built the proposed four crates and a facade from copies of the real source in a scratch workspace and compiled them on Rust 1.99.1; ran six original test targets against the facade |
| Plan reviewer | Tracker fidelity, `CRATES.md` and decision records, decisions A–F, release engineering, risks | Read-only comparison with `.beads/issues.jsonl`, `CRATES.md`, `docs/design/decisions.md`, the Makefile and CI; three small Cargo experiments |
| Integrator | The reviewers' most severe claims | Read the cited source lines |

The proposal's author is the integrator. The two reviewers worked from the
repository alone, in separate contexts, and did not see the author's reasoning
or each other's findings. All three are runs of AI models; no person reviewed
it. The scratch workspaces were not kept, so the compile results below are
reported, not reproducible from this repository.

## Result

The split is technically feasible: with the corrections in S01, S04, S05 and
S06 the scratch split compiled under all features and 109 original tests passed
against the facade. The proposal is not acceptable as written. It misses one
coherence blocker and one packaging blocker, understates the internals that
must be shared, states three things as fact that are false, changes a tracked
dependency without presenting it as a decision, and does not make the case that
splitting now is worth its cost.

No finding was dismissed. Dispositions are proposals for the maintainer; the
proposal's text has not been changed beyond a pointer to this record.

## Findings

Severity: **B** blocker, **H** high, **M** medium, **L** low. "Both" means both
reviewers found it independently.

| ID | Sev | Finding | Evidence | Proposed disposition |
| --- | --- | --- | --- | --- |
| S01 | B | `frost-concurrent` as specified does not compile. It implements `MapTypes`, `ElementType` and `SetContains` for `DashMap`, `DashSet`, the `scc` and skip-list types. The traits would be in `frost-core` and the types are foreign: E0117 six times, E0210 three times. Section 3.2 tested "the two cases" and missed this; section 3.1 omits the `concurrent → read` edge. It fails at the first extraction, not the last. | Both; compiled. `src/concurrent/dashmap.rs:7,40,55`, `scc.rs:6,33,48`, `skiplist.rs:6,21,35`; supertraits at `src/concurrent/live.rs:33,112` | Move the nine implementations into `frost-core` behind `dashmap-6`, `scc-3`, `crossbeam-skiplist-0_1` features there (compiled; `tests/concurrent.rs` passes 21 of 21), or record an API change that drops the supertraits. Then re-decide whether the remaining snapshot code, about 380 lines, justifies a crate. |
| S02 | B | `make package` and CR-DEP-5 cannot pass once the facade depends on an unpublished sibling. `cargo package -p` fails to resolve the sibling, and the extracted package has no registry to resolve an exact-pinned sibling from. | Both; Cargo experiment offline. Makefile 163–174, `ci.yml:126` | Add a decision on how an extracted package is tested while siblings are unpublished, and make it an acceptance criterion of the first extraction. |
| S03 | H | Crate names were never checked. `frost-core` is, from the reviewer's prior knowledge, an existing unrelated crate on crates.io. Decision B calls `frost-collections` a "registry identity", but the package is `publish = false` and the decision record says it is unpublished. | Not checked online. `Cargo.toml:11` | Add a naming decision; check all five names before any scaffold. |
| S04 | H | Section 3.3 lists 10 items that must cross a crate boundary. A compile finds 15, plus about 25 private adapter helpers shared by the family and adaptive halves of each adapter file. Missing: `Keys` and `Values` tuple fields; construction of the `#[non_exhaustive]` `Rejected`; the serde `Sealed` trait; `SeqIntoParIter`'s private field; two more call sites of `from_sorted_unique_unchecked`. Duplicating the helpers adds about 860 lines, including the borsh canonical-encoding checks. | Compiled. `src/iter.rs:127,132`, `src/plan.rs:82`, `src/integer/set.rs:724`, `src/adapters/serde.rs:261` | Complete the table from a compile; decide whether adapter helpers are duplicated or exported. The original epic forbids "duplicating production algorithms". |
| S05 | H | The `__private` scheme does not work as described. Re-exporting a `pub(crate)` item fails (E0364, E0365), so each item becomes `pub` at its definition. Six of the ten are inherent functions and cannot live in a module. Exact version pins do nothing while nothing is published. After publication they force every crate to release in lockstep: two frost crates at different patch levels fail to resolve, and across minor versions the graph gets two copies of `frost-core` whose seals and plans do not match. Hidden `pub` items still fall under CR-API-1 and the rustdoc lints. | Compiled; resolution consequences are inference from Cargo's rules | Specify free-function wrappers in `__private` with their `cfg`; say that pins are a post-publication mechanism; add decisions on lockstep versions, publish order, and whether siblings are supported entry points. |
| S06 | H | The `int-set` row of the feature map removes public API. The Roaring adapter and the `roaring` re-export are gated on `any(roaring-0_12, int-set)` in modules assigned to `frost-core`, but `int-set` forwards only to `frost-integer`. A program that compiles today under `int-set` alone fails in the split. Today's behaviour also contradicts CR-DEP-1 ("exposes no `roaring` type"). | Both; compiled. `src/backends/bitmap.rs:11–180`, `src/backend.rs:39` | Add `frost-core/int-set`, or record the removal. Resolve the CR-DEP-1 discrepancy either way; it exists without any split. |
| S07 | H | The doctest plan rests on two false claims. Stable 1.99.1 rustdoc does not check `compile_fail` error codes: in the scratch split, all 12 such doctests in `concurrent` reported ok although the real error was an unresolved import. Eight `compile_fail` blocks have no code at all. And a facade dev-dependency does not compile a second copy of the crate for doctests or integration tests; only unit tests see two copies, as CR-BOUND-1 already says. The real cost of that dev-dependency is feature unification, which turns `std` on in alloc-only lanes. The count of `use frost::` lines is 65, not 49. | Both; compiled | Rewrite decision E and the sentence in section 3.4. Give every `compile_fail` block an error code and gate them with a nightly doctest run before any move. |
| S08 | H | `make miri` would pass while running nothing. Its lanes select unit tests by name, such as `integer::storage::word::tests`; in `frost-integer` the names lose the `integer::` prefix, and a filter that matches nothing exits 0. | Ran: "0 passed; 21 filtered out". Makefile 196–213 | Rewrite each filter in the extraction that renames it and assert a non-zero executed count. The second part is worth doing today. |
| S09 | H | Several tasks cannot be completed as written. The two main integer test files use `frost::concurrent`; `examples/frameworks.rs` uses `InlineVec` but is assigned to `frost-core`; `benches/performance_review.rs` uses `frost::adaptive`; every bench includes `tests/support/workload.rs`, yet the shared support package is created after the last extraction; the two freeze implementations that cut the `family → adaptive` edge must leave in the first extraction, not the adaptive one. Also, 19 of 28 test targets are gated on a feature name; moved into a package without that feature they compile to zero tests. | Both. `tests/integer_sets.rs:995`, `examples/frameworks.rs:27`, `benches/support/mod.rs:9` | Replace section 3.5 with a per-file table of every test, bench, example and harness; move the support package and the edge cut into the first extraction; state which files are split. |
| S10 | H | The proposal reverses a tracked dependency inside a findings row. The records say `frost-wlt` depends on `frost-wou`; the proposal makes `frost-wou.6`–`.9` depend on `frost-wlt`. The three contracts with no curated case, `C-INLINE-OWNERSHIP`, `C-INT-SET-POLICY-LIMITS` and `C-SNAPSHOT-PUBLICATION`, are exactly the three areas being extracted, so the "fixed oracle" is weakest where the move happens. | `docs/design/decisions.md:292`, `CRATES.md:28`, CR-META-5 | Present it as a decision of its own, quoting what it supersedes. Curating those three contracts first is the cautious order. |
| S11 | H | Decision F ("split before the API refactor; these seams survive any refactor") is an assertion the decision record contradicts: it opens a redesign of `IntKey`, of set algebra as a trait or expression layer, and of snapshot payload admission. Those are the integer codec, the kernels `frost-integer` would import, and the seal edges. No API-refactor epic exists in the tracker. | `docs/design/decisions.md:342` | State F as an open question; list which proposed algebra changes cross the four boundaries; present "refactor first, then extract" as a real option. |
| S12 | H | The case for splitting at all is not made. The headline benefit, unsafe confinement as a compiler check, already exists: `unsafe_code = "deny"` with one `allow` in `buf.rs`, plus an ast-grep rule. No compile-time measurement, layering defect or consumer need is cited, and the proposal says there are no external consumers. Single-crate alternatives were not weighed. | Both. `Cargo.toml:95`, `src/adaptive/seq/buf.rs:4` | Add a "problem solved today" section with evidence, and compare with a layering lint inside one crate and with a smaller split. |
| S13 | H | Policy and documents are updated too late. The original task requires the one-package policy to change "in the same change as the first move"; the proposal defers it to a task after the last extraction. `make check` checks links, and about 200 links point into moved paths, so they must be fixed in each extraction. The link checker scans only root Markdown and `docs/`, so package documents would go unchecked. | Tracker text of `frost-wlt.2`; `scripts/check-doc-links.sh` | Update policy in the scaffold task; make link and document updates part of every extraction; extend the link checker. |
| S14 | H | "Four Makefile recipes select one package" undercounts what changes. At a workspace root, `--no-default-features` still turns on `frost-core/std` through the std-only sibling, so the core-only, alloc-only, embedded and Miri lanes need package selection or they go falsely green. Also keyed on paths: the ast-grep rule, `.config/mutants.toml`, the nightly mutation workflow, the coverage policy, the shell tests, the A/B harness, and eight isolated manifests that depend on the root by relative path. | Both; Cargo experiment | Replace the sentence with the full inventory and add "no recipe relies on workspace-root feature unification" to the scaffold's acceptance. |
| S15 | H | The revised epic and tasks drop acceptance criteria from the originals without saying so, among them fixed freeze and thaw partners, the enumerated verification matrix, "without weakening tests or duplicating production algorithms", the complete module and artifact mapping, and the seal and coherence plan. Task IDs `.2`, `.3` and `.5` are reused with different meanings while other documents cite them. | `.beads/issues.jsonl` | Carry each criterion into a named task or mark it dropped with a reason; give changed tasks new IDs. |
| S16 | H | "Each extraction passes `make check`" is weaker than the standing rules, which require the feature matrix, embedded builds, the migration table, changelog and catalog updates in the same change as a layout move. | `CRATES.md` section 9; `docs/design/decisions.md:344` | Per-extraction acceptance is `make verify`, `make embedded`, the relevant Miri, Loom and Kani lanes, and the changelog and migration rows. |
| S17 | H | The proposal contradicts or silently requires changing several `CRATES.md` rules: CR-BOUND-2 (paths under `frost::` are contract; the facade holds no algorithms), the recorded facade name `frost-api`, CR-LAYOUT-6 (two fuzz locations during the move), CR-LAYOUT-9 (cross-package tests), CR-MAKE-2 and CR-MAKE-4 (discovery from `cargo metadata`; the catalog tool hard-codes six manifest paths). | `CRATES.md` | Add a table: rule, change required, task that makes it. |
| S18 | M | "Tests stay put as a fixed oracle" is false for three targets: `tests/advanced.rs` and `benches/snapshots.rs` use optional dependency crates directly, which resolves only because they belong to the same package today. The scratch facade also lost `frost::backend::arc_swap` and no test, bench or example noticed. | Compiled; 26 errors in `tests/advanced.rs` | Add facade dev-dependencies or rewrite those sites; add a public-API diff to the parity task. |
| S19 | M | Section 3.5 and the tasks predate later adoptions: `verification/verus` and `make verus`, the Kani `set_laws` harnesses, the dev-dependencies `exhaust`, `itertools` and `proptest-state-machine`, and CR-TOOL-5. There are 28 test targets, not 27. | Commits after `ad931a6` | Add owners for each. |
| S20 | M | Section 6.1 misreads the tracker in places: the original task `.2` presupposes several moves, not one; only `.1` and `.4` speak of preserving paths; the epic's scope does list internal visibility. | `.beads/issues.jsonl` | Quote the tracker text in each row. |
| S21 | M | Catalog, inventory and coverage risks are missing. Per-package catalog files need ID uniqueness across files and cross-file contract references, which the schema lacks; three contracts have cases in more than one proposed package; all 24 unit-test records change target, selector and `cfg`. The coverage ratchet would compare across layouts on the reference host. cargo-mutants tests only the mutated package by default, so core mutants would look unkilled while tests sit at the facade. | Both; mutants point is inference | Add these to the risk section and a catalog design item to the scaffold task. |
| S22 | M | Decisions are missing or not independent. Missing: whether siblings are published at all, versioning and publish order, crate names, which path is the contract path, README, licence and changelog location per crate, docs.rs metadata, lockfiles. A and F are one question; C depends on the publishing decision; F gates the others but is listed last. | Plan reviewer | Add them to section 5 and order F first. |
| S23 | M | Internal contradictions and misstatements: "tests stay put until every package is extracted" against decision D; "small extractions" against a first extraction that moves over half of `src`; the `frozen → backends` row is reversed; "two harnesses include production files" then lists three. | Both | Correct the text. |
| S24 | M | Other risks absent from section 7: the proposal and the decisions it relies on exist only on this branch; historical harness packages depend on the root by path; bisect and blame across the whole-tree move; CI time; cross-crate inlining against benchmark baselines recorded for one crate. | Inference | Add to section 7. |
| S25 | L | Section 3.2 says `From<BTreeMap>` is the only alternative for the `Compact` plan; a plan type local to the family crate also compiles. Per-crate `std` must carry the weak forwards (`indexmap?/std` and others). One intra-doc link in `snapshot.rs` has no target in `frost-concurrent`. | Compiled | Correct the text. |

## Confirmed, with no change needed

- E0117 for `FreezeWith<Compact> for BTreeMap` outside the trait's crate
  reproduces, and the two `Inline` and `AdaptiveHash` freeze implementations
  compile in `frost-adaptive`; they are the only `family → adaptive` code edges.
- `frost-core` compiles alone once those two implementations and the adaptive
  halves of the adapters are removed. No intra-doc link from core points upward.
- The `frost-integer` items in section 3.3 are complete apart from `Rejected`.
- `#![forbid(unsafe_code)]` compiles in core, integer, concurrent and the
  facade. No unsafe function or invariant crosses a boundary.
- The section 3.4 line counts, the 23 `compile_fail` blocks, the test split, the
  four recipes that check the package name, and the tracker status all match.
- All 26 Cargo features appear in the feature map; the defective rows are S01,
  S06 and S09.

## Not checked

Embedded builds, Miri, Kani, Loom, fuzzing, coverage, nextest, cargo-mutants and
ast-grep on the split; the full test suite, benches and examples (six test
targets ran); Clippy and rustdoc with warnings denied; any performance effect;
crate-name availability online; the state after only the first extraction.

## What this changes

Three findings need a maintainer decision before the proposal can be revised,
because they change its recommendation and not only its text:

1. **Whether to split now (S11, S12).** The technical review shows the split
   can be made to compile. The plan review shows its benefit today is unproven
   and that the planned API refactor reopens the seams it would freeze.
2. **Whether the standards work comes first (S10).** The contracts without a
   curated case are the areas that would move.
3. **Whether sibling crates are ever published or supported directly (S02, S03,
   S05).** That decides naming, pins, and how packages are tested.

Four findings are defects or gaps in the repository today, whatever is decided:
the `int-set` and CR-DEP-1 discrepancy (S06), `compile_fail` blocks without
error codes that stable rustdoc does not check anyway (S07), Miri lanes that
would pass on an empty selection (S08), and the three contracts with no curated
case (S10).

[Proposal](../design/proposals/crate-split.md) ·
[Design decisions](../design/decisions.md) ·
[Crate standard](../../CRATES.md)

# Proposal: revise the crate reorganization epic `frost-wlt`

Status: **proposal for the maintainer**, 2026-11-12, against `27b1439` plus the
value-domain branch. It answers most of what task `frost-wlt.1` must produce
and proposes changes to the epic and its tasks. It changes no tracker record,
package, or source file. [Crate layout](../crate-layout.md) remains the record
of constraints; this document supplies the evidence that one lacked.

> **Not ready to accept.** An [adversarial review](../../reviews/crate-split-adversarial-review.md)
> on 2026-11-13 found two blockers and fifteen high-severity defects in this
> text. `frost-concurrent` does not compile as specified (S01), `make package`
> cannot pass with unpublished siblings (S02), section 3.3 and the feature map
> are incomplete (S04, S06), and decisions E and F rest on claims the review
> found false or unsupported (S07, S11). The text below is unchanged pending
> the maintainer's decisions listed in that review.

## 1. Recommendation

Split now, along the seams that do not depend on the coming API refactor, and
do it in small extractions that each leave `make check` green:

| Package | Contents | Depends on |
| --- | --- | --- |
| `frost-core` | `read`, `write`, `plan`, `frozen`, `iter`, `backends`, `family` (with its sorted-slice kernels), `mutable`, and the framework impls for those types | — |
| `frost-adaptive` | `adaptive` (the only `unsafe`), its framework impls, and the `Inline`/`AdaptiveHash` freeze plans now implemented in `family` | `frost-core` |
| `frost-integer` | `integer` | `frost-core` |
| `frost-concurrent` | `concurrent`: live access, snapshot store, publication retry helper | `frost-core`, `frost-adaptive`, optional `frost-integer` |
| facade (`frost-collections`, library `frost`) | Re-exports, prelude, `backend` re-exports, README doctests, feature forwarding | all four |

Keep `family` inside `frost-core` for this stage. Separating it needs an API
change (section 3.2), and that belongs to the refactor, not the move.

Restructure the epic so that tooling comes first, code moves bottom-up one
package at a time, harnesses move with the code they include, and tests stay
put as a fixed oracle until every package is extracted (section 6).

## 2. What changed since the epic was written

- **API compatibility is no longer a constraint** ([decision of 2026-11-12](../decisions.md#api-compatibility-is-not-a-constraint-2026-11-12)).
  The epic, `frost-wlt.1`, `frost-wlt.2`, and `frost-wlt.4` still require every
  current `frost::` path and import spelling to survive.
- **The one-package policy moved.** It is now stated in
  [`implementation-contracts.md`](../../maintainers/implementation-contracts.md#toolchain-and-layout)
  and the short [agent entry](../../maintainers/agent-rules.md); `frost-wlt.2`
  still names `AGENTS.md`.
- **`frost-wou` is half done.** `.1`–`.5` are closed. `.6`–`.9` are obligation
  curation, branch and profile coverage, a mutation baseline, and coverage
  targets. All four are keyed to file paths and a single-package coverage
  denominator, yet the epic waits for all of them.

## 3. Evidence

### 3.1 Module graph

`crate::` references between top-level modules in `src/`, extracted on
2026-11-12 and each listed edge confirmed in source. Macro-generated paths and
every `cfg` combination were not traced, so completeness is **PARTIAL** in the
sense of [TOOLS.md](../../../TOOLS.md).

| From | To | What crosses |
| --- | --- | --- |
| `frozen` | `backends` | `Eligible` implementations for native types live with the seal |
| `backends` | `family` | `Compact` turns `Vec`, `BTreeMap`, `BTreeSet`, `HashMap`, `HashSet`, `SmallVec`, `ArrayVec` into family types; `Sorted` turns `BinaryHeap` into `FrozenVec` |
| `family` | `adaptive` | `MutableVec: FreezeWith<Inline<N>>`, `MutableHashMap: FreezeWith<AdaptiveHash>` |
| `adaptive` | `family` | `FrozenOrderedMap`, `FrozenVec`, `MutableHashMap`; crate-private `from_sorted_unique_unchecked`, `range_indices`, `extend_reservation` |
| `integer` | `family` | crate-private sorted-slice kernels in `family::ordered::set_ops`; `TryFreezeWith<IntSetPolicy>` for the ordered set families |
| `concurrent` | `family`, `adaptive`, `integer` | `SnapshotValue` seal implementations |
| `adapters` | `family`, `adaptive`, `frozen` | Framework impls reach private fields and `Frozen::new_unchecked` |

Only `concurrent` and the crate root reference `integer`, and only the root
references `concurrent`, so both sit at the top of the graph. There are two
cycles. First, `family` depends on the plans in `plan`, while `backends`, which
must stay with the `frozen` seal, produces family types. Second, `family` and
`adaptive` depend on each other.

### 3.2 Which cycles Rust lets us cut

A throwaway three-crate workspace on Rust 1.99.1 checked the two cases:

- `impl<K, V> FreezeWith<Compact> for BTreeMap<K, V>` outside the crate that
  defines `FreezeWith` and `Compact` fails with **E0117**. Neither the trait,
  the plan, nor `BTreeMap` is local. So while `btree.compact()` returns a
  `FrozenOrderedMap`, the families and the plans must share a crate. A family
  crate could instead offer `impl From<BTreeMap<K, V>> for FrozenOrderedMap<K, V>`,
  which compiles. That is an API change, so it is deferred to the refactor.
- `impl<T, const N: usize> FreezeWith<Inline<N>> for MutableVec<T>` in a crate
  that defines `Inline` **compiles**. The `family`→`adaptive` edge is removed
  by moving the two impls into `frost-adaptive`. They need only public
  accessors (`MutableVec::into_vec`, `MutableHashMap::into_map`), which exist.

The orphan rule also keeps each framework impl with its type: `Serialize for
Frozen<B>` lives in `frost-core`, and the Rayon impls that match on adaptive
representations live in `frost-adaptive`. There is no separate adapters crate.

### 3.3 Items that become visible to a sibling crate

| Item | Today | Needed by | Proposed |
| --- | --- | --- | --- |
| `SetUnion::new` and the other cursor constructors; `intersection_len`, `is_disjoint`, `is_subset` | `pub(crate)` in `family::ordered::set_ops` | `frost-integer` | `#[doc(hidden)] pub mod __private` in `frost-core` |
| `FrozenOrderedMap::from_sorted_unique_unchecked` | `pub(crate)` | `frost-adaptive` | same |
| `family::ordered::range_indices`, `family::extend_reservation` | `pub(crate)` | `frost-adaptive` | same |
| `Frozen::new_unchecked`, `frozen::sealed` | `pub(crate)` | `backends`, core framework impls | stays private; all users are in `frost-core` |
| `concurrent::publication::update` | `pub(crate)` | `tests/publication_model.rs`, Loom (both by `#[path]`) | stays private; harnesses move with it |

Pin members to each other with exact versions (`version = "=x.y.z"`) so a
hidden module is never used across mismatched releases.

### 3.4 Size of the move

| Package | Lines in `src/` today |
| --- | --- |
| `frost-core` | about 9,200, plus most of the 1,600 lines of framework impls |
| `frost-integer` | about 5,600 |
| `frost-adaptive` | about 3,000, plus the adaptive framework impls |
| `frost-concurrent` | about 700 |

Doctests in moved modules have 49 `use frost::` lines and 23 `compile_fail`
blocks; 12 of the `compile_fail` blocks are in `concurrent`. Two harnesses
include production files by `#[path]`:
`verification/kani` includes `src/adaptive/seq/buf.rs`, and both
`verification/loom` and `tests/publication_model.rs` include
`src/concurrent/publication.rs`. Four Makefile recipes select exactly one
package named `frost-collections` and fail otherwise. The catalog tool assumes
`tests/` and `benches/` at the repository root.

By the type names they use, 12 of the 27 integration tests need only
`frost-core`, 10 also need `frost-adaptive`, 1 needs `frost-integer`, 2 need
`frost-concurrent`, and 2 need both of the last two. This is a name match, so
it is **PARTIAL**: several tests import the prelude, which reaches every
package. A test that uses `frost-integer` and `frost-concurrent` cannot live in
`frost-integer`: a dev-dependency back on `frost-concurrent` compiles a second
copy of the integer types, and the two copies do not unify.

### 3.5 Self-contained packages

The maintainer decided on 2026-11-13 that every package is self-contained
([decision](../decisions.md#self-contained-packages-and-formal-models-2026-11-13),
[CRATES.md](../../../CRATES.md) CR-LAYOUT-9 and CR-LAYOUT-10). Each package
therefore ends its extraction owning these artifacts, with catalog IDs kept:

| Package | Tests, benches, examples (by the types they use) | Harnesses and models | Documents |
| --- | --- | --- | --- |
| `frost-core` | Family, capability, backend, and set-algebra tests and benches; the `core`, `mutable`, `sequences`, `static_maps`, `advanced`, `set_operations`, `frameworks` examples; `tests/formal_models` | Ordered-set fuzz target; Kani harnesses for ordered algebra and bitmap padding | `set-operations.md`, `frozen-api.md`, the `algebra/` chapters, `set-algebra.md`, `collection-algebra.md` |
| `frost-adaptive` | Adaptive and inline tests, adaptive benches, `adaptive` example | Kani harness for `seq/buf.rs`; its Miri lanes | Adaptive sections of the architecture |
| `frost-integer` | Integer tests, benches, and `integer_sets` example | Integer fuzz target; `int-key-downstream`; integer views and output harnesses that are not historical evidence | `integer-sets.md`, `integer-views.md`, `integer-roadmap.md`, `persistent-integer-storage.md` |
| `frost-concurrent` | Concurrent, snapshot, and publication-model tests; snapshot bench; `concurrent` and `snapshots` examples | Loom package | `snapshots.md` |
| Facade | Tests that need several packages or reach them all through the prelude | — | README (crate docs), migration table |

Each package also gets its own `verification/metadata/` records, README, and
changelog. Historical evidence under `verification/` and the research corpus
stay where they are (CR-LAYOUT-8). The fuzz package splits by owner, as does
the Kani package, whose harnesses cover two packages today.

## 4. Feature map

Features stay additive, and each facade feature forwards to the package that
owns the code.

| Facade feature | Forwards to |
| --- | --- |
| `alloc` | `frost-core/alloc`, `dep:frost-adaptive` |
| `std` (default) | `alloc`, `frost-core/std`, `frost-adaptive/std`, `frost-integer?/std`, `dep:frost-concurrent` |
| Backend lines (`hashbrown-0_17` … `fst-0_4`, `roaring-0_12`) | `frost-core/<same>` |
| Framework lines (`serde-1` … `schemars-1`) | `frost-core/<same>`, `frost-adaptive?/<same>` |
| `int-set` | `alloc`, `dep:frost-integer`, `frost-concurrent?/int-set` |
| `dashmap-6`, `scc-3`, `crossbeam-skiplist-0_1`, `arc-swap-1` | `frost-concurrent/<same>` |
| `concurrent`, `advanced`, `frameworks` | Unchanged convenience aggregates |

`frost-core`, `frost-adaptive`, and `frost-integer` stay `no_std` with `alloc`
optional or required as today. `frost-concurrent` requires `std`. Every package
except `frost-adaptive` gets `#![forbid(unsafe_code)]`. `frost-adaptive` gets
`#![deny(unsafe_code)]` with an allowance on `seq/buf.rs` only, so the
confinement rule becomes a compiler check rather than a review convention.

## 5. Decisions for the maintainer

| # | Question | Recommendation |
| --- | --- | --- |
| A | Separate `family` from `frost-core` now? | **No.** It requires replacing `native.compact()` → family with `From` conversions. Decide that in the API refactor, then split if it still pays. |
| B | Facade package name: `frost-api` (current target) or `frost-collections`? | **`frost-collections`.** It is the existing package and registry identity, and the Makefile and docs already use it. The import name `frost` is unchanged either way. |
| C | How do siblings share internals? | **`#[doc(hidden)] pub mod __private` plus exact version pins.** Do not promote the kernels to public API until a consumer outside Frost needs them. |
| D | Where do tests, benches, and examples live during the move? | **At the facade while the code moves, then into the package in the same task.** Each extraction first moves code with tests unchanged as a fixed oracle, then, once green, moves the package's own artifacts (section 3.5). No package is left half self-contained between tasks. |
| E | Doctest paths in moved modules | **Spike in the scaffold task.** Either rewrite them to `frost_core::…`, or keep `frost::…` through a facade dev-dependency. The second keeps user-facing docs right but compiles a second copy of the crate for doctests. Pick using the `compile_fail` error codes on 1.99.1. |
| F | Split before or after the API refactor? | **Before, for stage 1 only.** These seams (unsafe, encoded integers, concurrency) survive any refactor of the family API. Later boundaries such as family/core, a kernels crate, or an expression layer wait for the refactor design. |

## 6. Proposed changes to the tracker

### 6.1 Findings on the current epic

| # | Finding | Change |
| --- | --- | --- |
| 1 | The epic and `.1` are blocked by all of `frost-wou`. Its open tasks `.6`–`.9` would be redone after the move, because paths, package scopes, and coverage denominators all change. | Block `frost-wlt` only on `frost-wou.10` (the dependency-isolation guard). Make `frost-wou.6`–`.9` depend on `frost-wlt` instead. |
| 2 | `.2` moves every package, test, benchmark, and example in one change. | Replace with one extraction per package, bottom-up, each passing `make check`. |
| 3 | Tooling (`.5`) comes after the move, but four Make recipes and the catalog tool fail on a second package. | Do tooling first, in a scaffold task with no code split. |
| 4 | Harness colocation (`.3`) is separate from the code those harnesses include by path. | Move the Kani harness with `frost-adaptive`, and Loom and the publication model with `frost-concurrent`. |
| 5 | `.1`, `.2`, and `.4` require every public path to survive. | Keep the `frost` import name. Allow path changes, recorded in a migration table and the changelog. |
| 6 | No task owns the visibility work in section 3.3. | Make it part of each extraction's acceptance criteria. |
| 7 | Acceptance does not use the compiler to enforce unsafe confinement. | Add the `forbid`/`deny(unsafe_code)` attributes from section 4. |
| 8 | `.2` points at `AGENTS.md` for the package policy. | Point at `implementation-contracts.md` and `agent-rules.md`. |

### 6.2 Revised epic text

> **Goal.** Split the single library into `crates/frost-core`,
> `crates/frost-adaptive`, `crates/frost-integer`, `crates/frost-concurrent`,
> and the facade `crates/frost-collections` (library `frost`), with an acyclic
> graph. Keep the semantic contracts in `implementation-contracts.md`,
> additive features, `no_std`/`alloc`/`std` support, unsafe confined to
> `frost-adaptive`, and Rust 1.99.1, edition 2024. Public paths may change;
> each change is listed in a migration table and the changelog. Publish
> nothing.
>
> **Out of scope.** Separating families from core, a kernels crate, an
> algebra or expression crate. These follow the API refactor.
>
> **Acceptance.** Every module, test, benchmark, example, and harness has an
> owner. Each extraction was merged green. The full matrix in `frost-wlt.6` is
> recorded against the final layout. No placeholder package exists.

### 6.3 Revised tasks

| Task | Title | Depends on | Acceptance, in brief |
| --- | --- | --- | --- |
| `.1` (revise) | Approve stage-1 boundaries | — | Decisions A–F recorded in `docs/design/decisions.md`; this proposal accepted or amended. |
| `.7` (new) | Workspace scaffold, no split | `.1`, `frost-wou.10` | Root becomes a virtual workspace and the current package moves unchanged to `crates/frost-collections`. Make recipes, the catalog tool (per-package metadata files), `make package`, and CI handle any number of members. The doctest strategy (decision E) is chosen. `make check` and `make verify` results match the pre-move run, and catalog IDs are unchanged. |
| `.2` (revise) | Extract `frost-core` | `.7` | Modules from section 1; `__private` module; `forbid(unsafe_code)`; core framework impls; the facade re-exports; embedded builds pass. Then its artifacts from section 3.5, including `tests/formal_models`, with catalog IDs kept. |
| `.8` (new) | Extract `frost-adaptive` | `.2` | Adaptive profiles, their framework impls, and the `Inline`/`AdaptiveHash` freeze impls moved from `family`; Miri lanes green. Then its artifacts, including its Kani harness. |
| `.9` (new) | Extract `frost-integer` | `.2` | Integer profile; atomic and non-atomic embedded builds pass. Then its artifacts, including the downstream codec package and integer fuzz target. |
| `.10` (new) | Extract `frost-concurrent` | `.8`, `.9` | Live access, snapshots, and seals; `int-set` forwarding works. Then its artifacts, including Loom and `publication_model` on the moved `publication.rs`. |
| `.4` (revise) | Facade and feature parity | `.10` | Migration table for every changed path; each isolated feature, core/alloc/std, and embedded builds pass; each `compile_fail` keeps its semantic intent. |
| `.3` (revise) | Facade tests and shared test support | `.10` | Cross-package tests settle in the facade; any test support used by several packages becomes a `publish = false` support package (CR-BOUND-1); no package refers to another package's `tests/`. |
| `.5` (revise) | Update policy and design documents | `.10` | `CRATES.md`, implementation contracts, agent entry, crate layout, architecture, and README describe the workspace. The one-package rule is removed in the same change as the last extraction. |
| `.6` (keep) | Record verification evidence | `.3`, `.4`, `.5` | Unchanged. |

Order: `.1` → `.7` → `.2` → (`.8` ∥ `.9`) → `.10` → (`.4` ∥ `.3` ∥ `.5`) → `.6`.

## 7. Risks and costs

- **Hidden internals become a semver-exempt interface.** Exact version pins
  contain this while nothing is published. It needs a real decision before any
  release.
- **Doctests.** 49 import lines and 23 `compile_fail` blocks need either a
  path rewrite or the dev-dependency arrangement. `compile_fail` error codes can
  change when a type comes from a different crate.
- **Coverage.** The scoped ratchet in `make coverage-gate` is defined for one
  package. It must be re-baselined in `.6`, not compared across layouts.
- **Build cost.** More crates add link and metadata work for small changes and
  remove recompilation of untouched crates. Measure both; do not assume either
  way.
- **Order is forced.** The facade re-exports every package, so a package can
  be extracted only after everything it depends on has left the facade. That
  makes extraction bottom-up: `frost-core` first, `frost-concurrent` last.
- **Value.** There are no external consumers, so the benefits are
  compiler-enforced layering, unsafe confinement, and per-package features and
  tests. If the API refactor will redraw `frost-core`, the alternative is to do
  only `.7` now, refactor inside the single package, and extract afterwards.

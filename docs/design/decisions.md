# Selected design: Frost 0.3.0

Decision date: 2026-11-08. Required compiler: **Rust 1.99.1**, edition **2024**.
The project toolchain and required CI jobs are pinned to that compiler.

## Decision

Use the earlier reviewed design as the architectural baseline, then adopt the
attached project's additions when they satisfy the same public contracts.
The result is one implementation, not two competing APIs backed by duplicate
collection implementations. Canonical family modules contain the types;
`frost::mutable` and the prelude reexport those same types.

The earlier revision supplied the complete five-family model, honest capabilities
for advanced storage, guard-scoped live concurrency, frozen snapshot publication,
and a reproducible verification workflow. The attachment supplied useful mutable
adaptive profiles, richer crate-owned entry APIs, and five framework integrations.
Neither source is accepted wholesale: reproduced defects and unsupported
guarantees are corrected in the selected implementation.

## Comparison and selection

| Area | Earlier reviewed design | Attached project | Selected implementation |
| --- | --- | --- | --- |
| Ordinary families | Five mutable/frozen pairs, including sets | Three map/vector pairs; ordered set frozen only | Five fixed pairs; preserve richer family modules and expose convenience reexports |
| Public mutation | Small entry handles and explicit operations | Occupied/vacant entry handles, more collection operations | Keep crate-owned entry handles and the richer operations; no mutable backend escape hatch |
| Type identity | Private family representation | Same foundation, organized by subcategory | Keep private storage and stable family names; features never select a different representation |
| Mutation capabilities | Map insert and set mutation traits | MapInsert and MapRemove share MapTypes | Adopt shared associated types; SetInsert shares ElementType |
| Mutable adaptive storage | Frozen profiles and explicit policies | InlineVec, AdaptiveHashMap, AdaptiveOrderedMap | Keep separately named profiles with closed variants, explicit policies and documented promotion/demotion |
| Exact frozen backends | Audited Frozen<B>, consuming extraction | Same model | Keep native construction and wire contracts distinct from family contracts |
| Advanced collections | Fourteen optional backend features | hashbrown and indexmap | Retain all fourteen reviewed backend features |
| Concurrent access | Scoped callbacks and SnapshotStore | Absent | Retain separate live access and frozen publication APIs |
| Ecosystem integrations | Deferred for family wrappers | Serde, Rayon, Borsh, Arbitrary, Schemars | Adopt all five, extend to new sets, correct bounds and wire/ordering guarantees |
| Iterator abstraction | Crate-owned family map iterators | Reusable wrappers and adaptive iterator enums | Keep reusable wrappers; dispatch at a bulk-operation boundary where implemented, not an unsupported universal promise |
| Duplicate laws | Explicit first representative, last map value | Mostly explicit; some native bulk-constructor reliance | Explicit family insertion/dedup; strict constructors document rejected-input ownership |
| Rust target | Stable tracking, minimum 1.99.1 | Minimum 1.88 | Exact toolchain 1.99.1, edition 2024, per the user's final instruction |
| Verification | Isolated features, tested executable examples, Miri and embedded builds | Framework tests require all five features together; no executable examples directory | Independent feature tests, nine executable examples, compile-fail checks, pinned compiler, embedded and Miri gates |

## 1. Families define semantics and mutation state

The five ordinary pairs are MutableVec/FrozenVec, MutableHashMap/FrozenHashMap,
MutableOrderedMap/FrozenOrderedMap, MutableHashSet/FrozenHashSet, and
MutableOrderedSet/FrozenOrderedSet. Each mutable family's `freeze()` has one
paired frozen target; explicitly named freeze plans may convert to a separate
profile. Each ordinary frozen family's `thaw()` has one named mutable result.

The wrapper is justified because it owns a contract. Map insertion preserves the
first equivalent stored key and replaces its value. Set insertion preserves the
first stored representative. Keys are never exposed mutably. Ordered families
promise ascending traversal and borrowed ranges; hash families promise neither
ordering nor an Ord bound. Native conversions are explicit interoperability
contracts, not the generic application interface.

The attached entry APIs are retained because their public occupied/vacant handles
are crate-owned. Their native fields remain private. Public type aliases and
reexports do not create another representation or another set of semantics.
The additional methods still need maintenance and tests; that cost is accepted
for a stable family API, not for an exact `Mutable<B>` facade.

### Rejected: exact-backend mutable wrappers

`Mutable<Vec<T>>` would still expose the backend in every signature containing it.
Forwarding an entire native API would add maintenance without providing family
independence. Native mutable backends instead implement the open capability traits
directly. Applications choose a semantic family or explicitly use a native type.

### Rejected: feature-selected storage for ordinary families

Cargo unifies features across dependencies. If a feature changed the representation
of MutableHashMap, another dependency could change its size, bounds, ordering, or
hasher policy. Features therefore add integrations and separate types. This follows
[Cargo's additive feature guidance](https://doc.rust-lang.org/cargo/reference/features.html).

A backend marker parameter could be added for a separately specified requirement,
but different marker instantiations are different Rust types. A default marker
is convenient spelling, not backend-independent type identity. It is not added
to the ordinary families in this revision.

## 2. Adaptive storage remains explicit and local to one family

The attachment's adaptive types are useful because their variants obey one
semantic contract. They do not combine trees, insertion-order maps and hash maps
under one universal enum.

| Profile | Transition | Deliberate cost or limit |
| --- | --- | --- |
| InlineVec<T, N> | Inline buffer spills to Vec; shrink may move it inline again | Every value carries space for the largest variant; moving an inline value moves its elements |
| AdaptiveHashMap | Linear entries promote to a hash table | Lookup uses one hash-family query relation; representation-changing mutation may need a cloneable hasher |
| AdaptiveOrderedMap | Sorted vector promotes to a B-tree | Small insertion can shift entries; large insertion follows tree behavior |

Removal does not automatically demote storage, which avoids repeated conversion
near a threshold. Explicit `shrink_to_fit` allows demotion. Freeze selects the
profile's documented frozen representation. Thresholds are user policies; the
defaults are not benchmark-derived recommendations.

The ordered adaptive profile freezes to the same compact FrozenOrderedMap as the
ordinary ordered family. Its inverse is explicit: `AdaptiveOrderedMap::from_frozen`
takes a policy. `FrozenOrderedMap::thaw` keeps its single ordinary-family result.
The frozen adaptive hash map preserves its hasher but does not store its former
growth threshold: `thaw` uses the default and `thaw_with` accepts a chosen policy.

### Panic behavior is part of the contract

Allocation and user Hash/Ord callbacks can fail during representation changes.
The inline ownership code must not leak or double-drop an initialized element.
Adaptive map promotion remains memory-safe and leaves a valid object after a user
callback panics, but entries moved into an unfinished replacement may be dropped.
It does not promise transactional rollback. Adding catch_unwind or cloning every
key/value would not be an acceptable hidden cost for this API.

## 3. Capabilities must fit the storage

Retain the earlier distinction between borrowed entries, borrowed values, decoded
values, borrowed elements and reconstructed members. Slot maps synthesize keys;
compressed sets synthesize integers; FST streams lend from reusable buffers.
None should be forced into a trait promising a stable stored-key reference.

Likewise, member cardinality is not a bitset universe length. Roaring counts retain
their natural u64 width. FixedBitSet traversal excludes padding beyond its logical
universe even if native raw blocks contain nonzero padding. Retaining/extracting
the native backend does not rewrite those raw blocks.

The attachment's MapTypes supertrait for map mutation is adopted: it avoids
independent Key/Value associated types that could disagree between read and write
capabilities. SetInsert follows the same principle with ElementType.
Insertion-ordered map/set removal remains explicit because shift and swap removal
have different ordering and cost contracts.

## 4. Live concurrency and snapshots have different guarantees

Keep concurrent maps/sets native for insertion, removal, async operations and
traversal. The shared capabilities execute synchronous callbacks while protection
is held. Returned owned values must not borrow the released entry guard.
Exclusive mutable-value access is present only where the native backend supports
it; no adapter exposes mutable keys.

Live concurrent collections are not eligible for Frozen<B>. SnapshotStore accepts
only a sealed set of frozen payloads and atomically publishes whole owned versions.
A reader retains an Arc or guard to one version. Unconditional publication does
not make a multiwriter read/derive/publish sequence transactional. The subsequent
integration adds identity-based conditional publication and retrying update;
see [snapshot contracts](snapshots.md). Structural freezing does not remove
element interior mutability.

## 5. Adopt framework integrations with explicit semantics

Serde, Borsh, Schemars, Rayon and Arbitrary are useful capabilities that family
wrappers cannot inherit automatically. The attachment's integrations are retained,
extended to the added set families, and tested separately.

### Serialization and schemas

Encode logical elements/entries, not private representation tags, capacity,
hashers or growth thresholds. Decode through the family contract. Policies are
reconstructed from defaults unless an explicit constructor is used afterward.

Serde hash-map iteration order remains unspecified. Strict helpers reject duplicate
or unsorted input according to the selected helper. Borsh family encoders sort
map/set members; family decoders reject noncanonical ordering and duplicate
collapse. Exact Frozen<B> retains native B's decoding rules. User-defined key
serialization and trait-law consistency still matter: equal collections need not
produce equal bytes when equality ignores serialized representative metadata.

Strict validation is not an input-resource budget. Initial Serde reservation is
cautious, but the adapters do not impose a maximum decoded collection size. Callers
processing untrusted input must supply the appropriate format/request limits.

### Parallel iteration and construction

Keep crate-owned Rayon wrapper types and closed variant dispatch. Slice-backed
families support direct indexed parallel traversal. Native hash/tree traversal
may stage references or owned entries in a temporary vector, and family parallel
construction stages input before applying duplicate rules. These costs are
documented rather than hidden behind a claim of zero allocation.

Duplicate winners follow encounter order only when the parallel source defines
one. Unordered sources, including par_bridge-style work, do not acquire a stable
first or last duplicate. Hasher Send/Sync bounds are required only when the
hasher actually crosses a thread boundary, not merely because a method is parallel.

## 6. Verification is an architectural requirement

Pin Rust 1.99.1 and edition 2024. Exercise core-only, alloc-only, default, each
individual optional feature, and all features together. Check real no_std builds
on a bare-metal target so test dependency unification cannot conceal a std leak.
Framework test modules must not depend on unrelated framework features.

Every executable example has an explicit Cargo test target that runs its actual
scenario. Rust documentation examples are doctested. Negative examples check
frozen mutation, guard lifetime escape, inappropriate snapshot payloads and
unsupported capabilities. Nightly verifies annotated error codes and runs Miri;
the ordinary crate and required CI remain on Rust 1.99.1.

`docs/reviews/historical-review.md` records findings and reproductions. `docs/reviews/historical-verification.md` records the
actual checks and their limits. Performance claims require workload-specific
benchmarks and are not inferred from passing tests or from a representation name.

## Performance review of the 0.4.0 design

The subsequent optimization pass retains the architecture selected here. Its
measurements identify repeated comparison work, inappropriate scan strategies,
and unnecessary buffer staging as actionable costs. It preserves the ordinary
family types, exact wrappers, adaptive profiles, and additive features.

Ordered algebra now uses a merge or monotonic seeking kernel according to an
internal cardinality heuristic; that changes execution strategy without changing
storage semantics. Dense algebra separates full words from padding boundaries.
Adaptive entry access reuses its first search result. Existing contiguous buffers
move into owning Rayon traversal directly.

These choices have explicit limits: intersection/difference cursor state costs
an additional eight bytes on the measured x86-64 build, small-case dispatch is
not free, and some native or simpler probing algorithms remain faster for some
inputs. Hash compaction policy, direct small frozen-map construction, owned
ordered results, and alternative wide-value layouts remain proposals requiring
workload measurements. See [PERFORMANCE_REVIEW.md](../reviews/performance-review.md) for the
full findings, current evidence, and ranked follow-up decisions.


## Revision 2 integer integration — 2026-11-09

The reviewed main archive and supplied revision 2 share an older base. Merge
revision 2's useful mechanisms into main while retaining the newer generic
set-algebra work and output-policy contracts. The
[integration review](../reviews/revision2-integration-review.md) records concrete
findings; [current validation](../revision2-integration-validation.md) records
only checks executed against the resulting source.

| Area | Integrated decision | Reason and cost |
| --- | --- | --- |
| Root ownership | Share all `FrozenIntSet` storage; O(1) clone | Related snapshots avoid a full copy for cloning or policy-compatible identity results. Empty values allocate a root; final release still destroys reachable storage. |
| Chunk ownership | Explicit Chunked candidate with immutable Array, Bitmap, and Full containers | Changed results retain untouched payloads. A flat directory is rebuilt in O(C) for C containers; this is not a persistent tree. |
| Public policies | Preserve `IntSetLimits`, `limits()`, and all four `*_with_policy` methods | Existing callers retain checked output selection. Empty, self, and no-op paths cannot bypass it. |
| Limit composition | Componentwise minimum for union/intersection/XOR; left limits for difference | Reject revision 2's larger-limit rule, which would silently weaken a strict operand. |
| Automatic planning | Retain the reviewed score and 128-byte Dense preference; do not automatically choose Chunked | Reject the unconditional 4 KiB Dense floor, which can expand tiny sparse sets. Explicit policies remain available. |
| Hashing | Keep canonical content-selected member/run events across all four representations | Sharing and physical container boundaries are not semantic identity. Preserve exact Hasher-event regressions. |
| Conversion | Keep bounded byte promotion and representation-aware result enforcement | Avoid restoring full sorted staging for every policy change or an unbounded byte importer extent. |
| Built-in codecs | Add narrow integers, characters, address-sized integers, and narrow unsigned nonzero values | Words remain sealed to u32/u64; logical-domain imports remain checked. |
| Alloc-only targets | Private Arc/Rc alias selected by pointer-atomic target capability | Preserve targets without pointer atomics; normal auto traits follow the selected pointer. Features do not change that selection. |
| Evidence | Preserve old reports and raw revision samples as historical | Measurements from either input tree are not measurements of the merged implementation. |

This revision changes the fixed implementation available to every `int-set`
consumer, rather than conditionally changing its representation when an unrelated
feature is enabled. Ordinary collection families, backends, hasher defaults,
freeze/thaw pairs, Rust 1.99.1, and edition 2024 remain as before.

The earlier proposal for a separate `PersistentIntSet` is narrowed to possible
future directory/path-copying and bounded-sharing contracts. Its APIs are not
implemented. The current Chunked candidate already provides immutable payload
sharing within `FrozenIntSet`; the
[persistent design](persistent-integer-storage.md) records that boundary. Generic
borrowed set expressions and lazy integer evaluation remain proposals.

## Research corpus and rationale, 2026-11-09

A supplied research handoff asked for a curated research corpus, a root README
rationale, and a Frozen API guide. It contained two overlapping collection
algebra reports already preserved in this repository, a computed-sets summary,
templates, and draft README text. This record states what the integration
adopted. It changes documentation only; source, features, CI, and the Rust
1.99.1 / edition 2024 toolchain are unchanged.

| Area | Decision | Reason and limit |
| --- | --- | --- |
| Package layout | Keep one library package, `frost-collections`, imported as `frost`. Record a `frost-api` facade over flat `crates/frost-*` packages as an **unadopted proposal**. | The handoff describes that layout as a user decision, but the maintainer policy and the current task keep one package. Adopting a split needs its own decision, an updated maintainer policy, a migration plan, and feature and CI changes. Installation instructions stay as they are. |
| Package names | Adopt no naming rule. Preserve Frost1's plural names and Frost2's singular names as alternatives. | No package is created, so a rule would be premature. A future proposal applies one rule to every domain. |
| Raw inputs | Keep the original reports byte for byte under `docs/reviews/supplied/`. Do not copy them again. Put curated, cited derivatives under `docs/research/`. | Raw files are evidence, not instructions or policy. Only curated files get corrected examples, portable citations, and per-claim status. |
| Frozen API | Make [the Frozen API guide](frozen-api.md) the canonical guide to current frozen contracts, with proposals in a separate section. | Contracts were scattered across the README, architecture, and rustdoc. The guide links each claim to source and tests. |
| Engine motivations | Describe symbolic execution, authorization, and computed sets in the README as motivating workloads with explicit outside-scope boundaries. | Frost implements no engine, policy language, solver, identity protocol, durable store, or consistency protocol. |
| Complexity claims | Do not transfer an upstream bound to Frost algorithms. Derived bounds name every variable. | `fst` documents n-ary operations as `O(n1 + n2 + ...)`; its implementation uses a binary heap and byte-key comparisons. See [collection algebra](collection-algebra.md#n-ary-sorted-merge-cost). |
| Missing artifacts | Import the computed-sets summary under a summary name. Do not reconstruct the full computed-sets report, the original research template, its ZIP, or a standalone algebraic-foundations report. | None of these were supplied or recovered. A summary's statement that a file exists is not evidence that it exists. |
| Research status | Imported research is not accepted design. | Lineage runs from question to source, claim, synthesis, focused research, design proposal, implementation task, and verification evidence. Each step needs its own review. |

The [research index](../research/README.md) and
[integration audit](../research/integration-audit.md) record the corpus, the
claim matrix, unresolved citations, and missing artifacts.

## Collection naming dispositions, 2026-11-09

Retain `LookupRef<Q>`, `SetContains<Q>`, `SetGet<Q>`, and `Entries` with
`OrderedEntries`; the last pair promises ascending traversal. Select
`SeqMultiMap`, `SetMultiMap`, and `BagMultiMap` as distinct proposed contracts.
Put bimap insertion on `MutableBiMap` or a builder and keep `FrozenBiMap`
structurally immutable. Retain `FrozenIntSet` and distinguish the proposed
`IntSetExpr`, `FrozenIntSetView`, and `PersistentIntSet`. These proposed types
are not implemented by this decision. The [naming disposition table](collection-algebra.md#naming-dispositions)
records their meanings and the remaining semantic decisions. Package names and
map-difference spelling are outside this decision.

## Crate standard and reorganization target, 2026-11-09

The user authorized two linked tracker epics: standards epic `frost-wou` and
reorganization epic `frost-wlt`, which depends on it. This record adopts the
standard and narrows the earlier package-layout decision. Source, features,
public APIs, Rust 1.99.1, and edition 2024 are unchanged by it.

| Area | Decision | Reason and limit |
| --- | --- | --- |
| Canonical standard | Adopt [CRATES.md](../../CRATES.md) for package layout, dependency placement, the feature/target matrix, public API obligations, coverage accounting, and test/benchmark metadata. It applies to the current package now. | Rules were spread across `AGENTS.md`, the contributor guide, and verification notes, and did not define obligations, metadata, or a target layout. |
| Package layout | The flat `crates/frost-*` layout with a `frost-api` facade (library name `frost`) is now the tracked target of `frost-wlt`, not only a research proposal. The one-package policy stays in force until the migration change updates it. | Package boundaries, names, and the module mapping are not approved. The [crate layout design](crate-layout.md) records the coherence, seal, feature, and unsafe-confinement constraints that decision must satisfy. This supersedes the earlier "unadopted proposal" status only in that respect. |
| Coverage accounting | Public API obligations are the unit of coverage. Line, region, and function percentages are reported with their scope beside the measured baseline, and targets are labeled as targets. `make coverage-gate` enforces a portable 77/70/77 adoption floor everywhere and an exact baseline ratchet only in the matching scope. | Same-scope source coverage moved from 77.197/70.908/77.772% (lines/functions/regions) to 79.157/73.759/79.431%, below the 90/85/90% targets. The denominator includes inline test modules in `src/`. Branch, profile, doctest, obligation, and mutation coverage are unmeasured. |
| Test and benchmark metadata | Every test and benchmark case has a sidecar record (`frost-verification-catalog/1`, `frost-bench-inventory/1`) with a stable ID, checked by an isolated tool in `make check`. | Records add no runtime or dev-dependency. Discovery is structural: 306 of 327 case records and all 922 benchmark entries are `unreviewed`, not semantic coverage. |
| Implementation versus adoption | `frost-wou.1`–`.5` implement the standard, catalog, gates, and templates. `frost-wou.6`–`.10` remain open: obligation curation, branch/profile/Linux baselines, mutation baseline, high-target tests, and a dependency-isolation guard. | The standard is in force now; its numeric targets and complete obligation inventory are not yet attained. |
| Fuzz package placement | Target layout uses `<package>/fuzz/`, the cargo-fuzz default. The current checkout keeps isolated fuzz work under `verification/fuzz/`. | One location per package after the migration; isolated packages stay outside the workspace. |

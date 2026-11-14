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
see [snapshot contracts](../../crates/frost-collections-concurrent/docs/snapshots.md). Structural freezing does not remove
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
[persistent design](../../crates/frost-collections-integer/docs/persistent-integer-storage.md) records that boundary. Generic
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
| Frozen API | Make [the Frozen API guide](../../crates/frost-collections-core/docs/frozen-api.md) the canonical guide to current frozen contracts, with proposals in a separate section. | Contracts were scattered across the README, architecture, and rustdoc. The guide links each claim to source and tests. |
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
| Test and benchmark metadata | Every test and benchmark case has a sidecar record (`frost-verification-catalog/1`, `frost-bench-inventory/1`) with a stable ID, checked by an isolated tool in `make check`. | Records add no runtime or dev-dependency. Discovery is structural: 323 of 345 case records and all 922 benchmark entries are `unreviewed`, not semantic coverage. |
| Implementation versus adoption | `frost-wou.1`–`.5` implement the standard, catalog, gates, and templates. `frost-wou.6`–`.10` remain open: obligation curation, branch/profile/Linux baselines, mutation baseline, high-target tests, and a dependency-isolation guard. | The standard is in force now; its numeric targets and complete obligation inventory are not yet attained. |
| Fuzz package placement | Target layout uses `<package>/fuzz/`, the cargo-fuzz default. The current checkout keeps isolated fuzz work under `verification/fuzz/`. | One location per package after the migration; isolated packages stay outside the workspace. |

## Integer and family optimizations, 2026-11-10

This series changes costs, not contracts. No public item, feature, policy
limit, representation candidate, or Hash event stream changed. The
[validation record](../../crates/frost-collections-integer/docs/integer-optimization-validation.md) holds the
measurements, their limits, and the gates that ran.

| Area | Decision | Reason and limit |
| --- | --- | --- |
| Planner survey | Stop the Roaring survey at the container boundary where its score reaches the selected candidate's score. | The score only grows, so the selected backend cannot change; a unit test compares the planner with a complete survey. The memory score and the 128-byte Dense preference are unchanged. |
| Partition limit | Bound the partition count from the cached bounds and scan members only when the span exceeds the limit. | The bound is exact for 32-bit words. It replaces a full scan of every Sorted operand before union and symmetric difference. |
| Native statistics | Keep the partition count, container count, and score beside each stored Roaring value. | Storage is immutable, so the cache cannot go stale. Eligibility checks on shared, empty, and equal-result shortcuts were O(containers). The cache takes no part in equality, ordering, or hashing. It enlarges every storage root by 8 bytes for 32-bit words and 16 bytes for 64-bit words, for every representation. The score is stored saturated to 64 bits; a `u128` field raised the root alignment to 16 bytes and cost 32 and 40 bytes. |
| Sorted results | Merge with operation-specific loops, bulk tails, and whole-segment copies when one operand is at most 1/32 of the other. | The 1/32 ratio matches the ordered family's seek ratio. It is a structural allowance, not a measured crossover. |
| Construction | Classify staged codes with early-exit scans: ascending input skips the sort, descending input is reversed, and dense unsorted input is sorted through a temporary bitmap no larger than the staged codes. | The validation pass is unchanged, so error variants and indices equal the baseline on every path; a differential test checks this against a reference. Temporary staging can double for the bitmap path. A first design tracked order per staged code and skipped the validation pass; it measured 7% to 24% slower on ascending input and was replaced. |
| Chunked Array pairs | Unchanged. A direct merge kernel was written, measured, and withdrawn. | In the same-executable comparison it was 17% to 35% slower than the ordered cursors for union, difference, and symmetric difference on three datasets, and no faster elsewhere beyond the control variation. An earlier separate-executable probe had reported a gain for intersection. |
| Roaring builder | Probe for runs once per 32 codes. | A per-code comparison slowed input without runs. Shorter runs may be appended member by member; the built value is the same. |
| Public iterators | Forward `fold` and `count` from `IntSetIter` and `EncodedValues`; add `fold` to the ordered union and difference cursors. | `for` loops and `collect` call `next` and do not gain. A bulk extraction method would be a new public item and is not part of this series. |
| Hash stream | Unchanged. | Hashing a large scattered set costs about 2.5 times a plain `write_u64` loop with SipHash. The one-byte tag before the members leaves every following 8-byte write unaligned in the hasher's buffer, and the hasher state stays behind a reference. A `u64` tag or block writes would be faster but change the event stream, which is a contract decision. |
| Result sharing | Share an equal eligible operand on two paths that returned a new root: the direct merge for a Sorted operand with more partitions than the native limit, and an empty result with an empty operand. | The rustdoc already states the sharing rule; an independent review found 247 deviations in a directed sweep and none after the change. Each shortcut checks result eligibility first. Results stay equal and hash the same. |
| Empty sets | `FrozenIntSet::new` allocates a shared header, as before. An empty result allocates one only when no operand is empty and eligible. | The allocation in `new` is a recorded contract. An allocation-free empty value would add a branch to every storage access. |
| Mutable ordered collection | `from_iter` stages, sorts, and bulk-builds, as the adaptive ordered map already does. | Collection now needs O(n) temporary staging. The duplicate rule is applied by crate code before the tree is built. |
| Emptied adaptive tree | `range` and `range_mut` return an empty range for an empty tree. | A std tree emptied by removal validates bounds; the Sorted form and the documented contract do not. `MutableOrderedMap` and `MutableOrderedSet` keep native behavior, as documented. |
| Not changed | The inline buffer's clone, `from_vec`, and `retain` paths; `MutableOrderedMap::append`; the linear table kept by `thaw`. | The first three are in or beside the crate's only `unsafe` module and need Miri, which was unavailable. The others need a size-ratio gate or a policy decision. The validation record lists the measurements. |

## API compatibility is not a constraint, 2026-11-12

The maintainer stated on 2026-11-12 that the public API does not need to stay
compatible and that the library is to be refactored. This record states what
that changes. It approves no specific redesign, package boundary, or name.

| Area | Decision | Reason and limit |
| --- | --- | --- |
| Public API shape | Names, module paths, signatures, trait bounds, and type parameters of existing public items may change. | The crate is unpublished at 0.4.0 (`publish = false`), and the [release procedure](../maintainers/releasing.md) still gates publication. Compatibility was a self-imposed constraint, not a user commitment. |
| Semantic contracts | Not relaxed. The representative rules, the structural meaning of frozen, snapshot publication semantics, the integer codec laws, and the other rules in the [implementation contracts](../maintainers/implementation-contracts.md) stay in force. | This is the integrator's reading of the maintainer's statement, which named API compatibility only. Changing one of these contracts needs its own record. |
| Feature additivity | Not relaxed. Enabling a feature still never changes a type's representation, hasher, bounds, or freeze/thaw partner. | Additivity is a property of one build under Cargo feature unification, not of compatibility between versions. |
| Safety | Not relaxed. Unsafe stays confined, with its written invariants and Miri coverage. | A refactor that moves `src/adaptive/seq/buf.rs` moves it as a unit. |
| Crate layout | [Crate layout](crate-layout.md) constraint 5 (every current `frost::` path resolves through the facade) is no longer required, and constraint 1 no longer forces existing set algebra to stay as inherent methods: it can become a trait or an expression layer. Constraints 2, 3, 4, and 6 to 9 follow from Rust's rules or from contracts that remain. | The package list is still the output of `frost-wlt.1`. No package is approved or created by this record. |
| Value domains | The [formal algebra](algebra/README.md) documents were written under the compatibility constraint. Their statements that no domain parameter or binding is added to existing types are now design positions to be argued on their merits, not requirements. | The merits still weigh against a mandatory parameter on every family: it would inflate bounds for callers that have no runtime-scoped domain. A redesign of `IntKey`, of source bindings, or of snapshot payload admission is now open. |
| Process | A breaking change carries its design record, changelog entry, rustdoc, tests, and catalog updates in the same change. Existing catalog IDs are kept; a removed public item retires its records explicitly. | [CRATES.md](../../CRATES.md) accounts coverage per public obligation, so a silent removal would leave stale obligations. |

## Value-domain algebra integration and rules entry, 2026-11-12

A supplied delivery added a formal specification of value domains and
collection algebras, a research supplement with a finite model, and a
restructure of the agent rules. This record states what the integration
adopted. Source, features, manifests, CI, and the toolchain are unchanged.

| Area | Decision | Reason and limit |
| --- | --- | --- |
| Formal chapters | Add [docs/design/algebra](algebra/README.md) as **proposed contracts**. | They name value domains, key identity, collection carriers, operation partiality, and evaluation context separately. Nothing in them is implemented or accepted design; each chapter says so. |
| Authority | Source, rustdoc, and tests remain authoritative for implemented behavior; [CRATES.md](../../CRATES.md) for layout, obligations, and metadata. | The chapters were written from three source files and the existing documents. The [destination review](../reviews/value-domain-algebra-destination-review.md) checked their statements about current source against a full checkout and found none wrong. |
| Recommended spellings | Record `difference_by_key` for map restriction and `absolute_difference` for bag counts as recommendations. | Neither names an implemented item. `Q-MAP-1` and `Q-BAG-1` still own the decision. |
| Evidence | Keep the finite model as research tooling under `docs/research/formal-models/evidence`, run with Node by hand. *Superseded 2026-11-13: replaced by Rust tests in `tests/formal_models`; see the next record.* | It is not a Make, build, or CI step, imports no Frost code, and is not Rust test evidence. The delivered model overstated its strength; it was revised and is now accompanied by a mutant run. |
| Research questions | Six seeded questions stay local to the supplement, each tied to the corpus question it refines. | The corpus manifest and its dependency order are unchanged. |
| Provenance | Record the delivery archive by size and SHA-256 without committing it. | The archive is 1.3 MB, mostly earlier revisions of files already here. Its inventory therefore cannot be verified from this repository alone. |
| Agent rules | `AGENTS.md` is a relative symlink to [the short entry](../maintainers/agent-rules.md). The previous text is [implementation contracts](../maintainers/implementation-contracts.md), unchanged apart from its header. The entry states inline the rules that hold for every change. | The delivery requested the symlink. A short auto-loaded file is easier to keep current, but an agent that does not follow its link sees fewer rules, so the inline list covers the package, feature, unsafe, and gate rules. A checkout without symlink support shows a one-line file holding the target path. |
| Stale references | Repoint catalog contract sources, `CRATES.md`, the crate-layout design, the root README, and the research prompt template to the implementation contracts. | The catalog tool checks only that a source path is a file, so the stale paragraphs would not have been reported. No catalog ID changed. |

## Self-contained packages and formal models, 2026-11-13

The maintainer stated on 2026-11-13 that each crate should be self-contained,
with its formal models and all other artifacts kept with it. This record
states what that means and what changed now, before any package exists.

| Area | Decision | Reason and limit |
| --- | --- | --- |
| Package contents | Everything a package owns lives under its directory: code, tests, benchmarks, examples, fuzz targets, Loom, Kani, and downstream harnesses, formal models, catalog and inventory records, README, changelog, and the design documents for its contracts ([CRATES.md](../../CRATES.md) CR-LAYOUT-9). | Moving or extracting a package then carries all of its evidence. Repository-wide documents (philosophy, architecture, decisions, maintainer guides, research corpus) and historical evidence stay at the root. |
| Cross-package tests | A test that needs several packages lives in the lowest package that depends on all of them; the facade holds the rest. | A dev-dependency back on a higher package would compile a second copy of the lower one's types. |
| Formal models | Executable models are Rust test targets in the owning package's `tests/formal_models/`; proof harnesses go in `<package>/verification/<tool>/` (CR-LAYOUT-10). The Node model under `docs/research/formal-models/evidence` was ported to Rust and removed. | The repository builds with Rust and Make only. Rust models run in `make check`, are catalogued, and can run the real types. Today the single package is the owner, so the target is `tests/formal_models`; at the split it moves to `frost-core`. |
| Tooling | Settled the same day; see [Formal validation tools](#formal-validation-tools-2026-11-13). | The hand-written generators were replaced. |
| Contract obligations | A catalog contract may name `law:<ID>` obligations for a law in `docs/design/algebra/catalog.json` that has no public item yet. | Curated model cases need a contract, and proposed operations have no public path to cite. |

## Formal validation tools, 2026-11-13

The maintainer approved the three recommendations of the
[formal validation tools report](../research/reports/formal-validation-tools.md)
on 2026-11-13.

| Area | Decision | Reason and limit |
| --- | --- | --- |
| Finite models and real-type tests | `exhaust` and `itertools` generate the finite domains, `proptest` the sampled ones, and `proptest-state-machine` drives real types against a reference model. They are dev-dependencies; [CRATES.md](../../CRATES.md) CR-DEP-2 counts them as test libraries. | No generator, enumerator, runner or shrinker is hand-written. bolero was rejected because its current release does not resolve with Frost's `indexmap`; quickcheck and arbtest duplicate proptest. |
| Bounded refinement | Kani harnesses check real operations against the reference definitions of the laws, starting with `SET-REP` and `SET-BOOLEAN` on `FrozenOrderedSet` (`verification/kani/src/set_laws.rs`). | Covers only the stated bounds. A law on a type Frost does not implement has no refinement harness. |
| Unbounded proofs | Verus, pinned by `VERUS_VERSION` in the root `Makefile`, proves laws for their reference definitions in `verification/verus`, which is not a Cargo package and does not link to Frost. Verifying production code with Verus stays deferred until after `frost-wlt`. | The pinned release uses Rust 1.99.1. A Verus proof says nothing about Frost code (CR-TOOL-5). Lean with Mathlib, Aeneas and Creusot were not adopted. |

## Crate split execution, 2026-11-13

The maintainer asked on 2026-11-13 for the crate-split epic `frost-wlt` to be
reviewed adversarially, corrected, shown, and executed. The
[crate split plan](crate-split-plan.md) is the corrected design; the
[plan review](../reviews/crate-split-plan-review.md) records the 27 findings
it answers. This entry records what is decided and by whom.

| Area | Decision | Reason and limit |
| --- | --- | --- |
| Timing | The split runs now, before the API refactor. | Maintainer instruction. It supersedes the ordering under which `frost-wlt` waited for `frost-wou`; the standards tasks `frost-wou.6`–`.10` stay open. |
| Packages | Three code packages, a facade and a dev-only support package: `frost-collections-core`, `-integer`, `-concurrent`, `frost-collections`, `frost-collections-test-support`. | Author's default (plan D4, D5). It replaces the `frost-api` facade name recorded on 2026-11-09. `frost-core` is taken on crates.io. |
| Adaptive profiles | They stay in the core package. | Extracting them would open the serde seal, which relaxes a contract. Only the maintainer can approve that; it is tracked as `frost-adp`. |
| Publication | Nothing is published; every package is `publish = false`; the facade is the only supported entry point. | Author's default (plan D3). |
| Contracts | No semantic or safety contract changes. Known defects are tracked as `frost-r0a` and `frost-k4n`, not fixed inside a move. | The API-compatibility decision relaxes no contract. |
| Unrun lanes | A verification lane that cannot run on the host blocks closing the epic unless waived here. On 2026-11-14 the maintainer waived three for this epic: Verus, the exact coverage ratchet, and the two Kani harnesses of `frost-k4n`. | Plan D16. Each stays tracked: `frost-k4n`, `frost-v3s` and `frost-c7b`. The waiver closes the epic; it is not evidence that those lanes pass. |
| Miri | The maintainer instructed on 2026-11-14 that Miri is not to be run in the execution session. Miri lanes are recorded as not run from that point; they are left to CI (`make miri`) or a later local run. | Maintainer instruction. Before it, two lanes over `buf.rs` passed after the scaffold and after the core extraction, and the 32-bit word-storage lane passed on the integer package. On 2026-11-14 the maintainer decided that the GitHub CI runs of `make miri` on the split tree are the Miri evidence for the epic, and that CI runs Miri in its nightly and release tiers, not on push or pull request. |
| Approval | The maintainer approved the plan as executed on 2026-11-14. Package names and boundaries may be revisited in the API refactor. | Maintainer decision. It approves the executed layout. It does not approve an adaptive package (`frost-adp`) or publishing. |

Decisions D2–D16 of the plan began as defaults taken so that execution was not
blocked. The maintainer approved them with the plan on 2026-11-14; the plan
states what undoing each one costs.

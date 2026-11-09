---
id: frost-research-synthesis-foundations-and-engine-roadmap
kind: synthesis
level: landscape
research_status: curated
implementation_status: mixed
source_validation: partial
input_files: [docs/reviews/supplied/frost-collection-algebra-research-1.md, docs/reviews/supplied/frost-collection-algebra-research-2.md, docs/reviews/supplied/frost-computed-sets-summary.md]
input_members: []
input_sha256: [543d67b2f7c3724f9c96849ea77c8dd24c3a36792395c46e6050d1383ebd8f6d, 046cdaf91f7f2912ce966835951d00dd42ae230f5a76914b5eeb5b77f8862ad3, 6f39fdbcf2d7816a3ea33eb40e371be7115b1b452cf040a7f8fe84a60de8177c]
origin_created_at: 2026-11-09
imported_at: 2026-11-09
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca
parent_ids: [frost-research-report-collection-algebra-survey, frost-research-report-collection-algebra-implementation-notes, frost-research-report-computed-sets-summary]
related_ids: [frost-research-prompt-algebraic-foundations, frost-research-prompt-frozen-api-contracts, frost-research-prompt-computed-sets-and-durable-execution, frost-research-prompt-authorization-engine-integration]
supersedes: []
open_questions: [Q-ALG-1, Q-FROZEN-1, Q-CSET-1, Q-NARY-1, Q-AUTHZ-1, Q-MAP-1, Q-BAG-1, Q-MM-1, Q-BIMAP-1, Q-PREFIX-1]
---

# Foundations and engine roadmap: a decision-oriented landscape

**New synthesis, written 2026-11-09** against baseline `82becb9`. It is not
recovered research and not an accepted design. It organizes what the three
curated reports, the repository's existing design documents, and the
inspected source establish; where they conflict; which design alternatives
remain; what is implemented versus planned; and which focused questions to
research next, in dependency order. The algebraic-foundations deep dive the
task requested remains a **gap**: this landscape summarizes verified basics
and hands the deep dive to `Q-ALG-1`; it does not impersonate it.

## 1. Executive map

Frost's role is a **semantics-conscious, representation-aware collection
substrate** for engines that repeatedly combine facts, relationships,
permissions, and symbolic state. The value proposition is reusable algebra
with explicit laws, clear ownership transitions, stable public abstractions,
specialized kernels, and observable costs. It is not a claim to beat native
collections universally, and not a policy engine, solver, identity server, or
durable store.

Decisions this landscape supports now:

| Decision | Position | Strongest evidence | Key uncertainty |
| --- | --- | --- | --- |
| One algebra or several? | Several, with shared capabilities below and shared execution machinery beside them | Three inputs and the existing design agree; implemented set contracts already separate membership, representatives, and order | Which laws hold per domain (`Q-ALG-1`) |
| Frozen means what? | Structural immutability with named costs; fixed partners; sealed admission to exact wrappers and snapshots | Inspected source and compile-fail doctests | Written standard for new families (`Q-FROZEN-1`) |
| Computed sets? | Expressions over sources at a defined revision with terminal-specific execution; typed borrowed views first | General set-algebra design; finite oracle; Zanzibar precedent at section level | Revision agreement, overlays, budgets, durable cursors (`Q-CSET-1`) |
| Authorization and symbolic engines? | Consumers with explicit boundary obligations; influences are not dependencies | Boundary sources checked narrowly | Data contracts at the boundary (`Q-AUTHZ-1`) |
| Workspace split? | Not now; one package; crate names are proposals with three conflicting spellings | `Cargo.toml` | A future naming decision, after consumers exist |

### Lineage

```text
question -> source -> claim -> synthesis -> focused research
         -> design/ADR proposal -> implementation task -> verification evidence
```

Research does not become accepted design by being imported. Accepted design
does not become implemented by appearing in a diagram. An implementation is
not validated by a benchmark plan. Each stage leaves an artifact: a `Q-…` id
here, a `SRC-…` id in the catalog, a `CL-…` claim below, a child report, a
design document under `docs/design`, source and tests, and a verification
record.

## 2. Semantic taxonomy

| Domain | Logical model | Equivalence | Representative rule (implemented or proposed) | Order | Multiplicity | Failure / overflow | Callback effects |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Set | `T → {0,1}` | `Eq`/`Hash` or `Ord` per family | First inserted; left on union/intersection overlap (implemented for frozen families) | Ascending (ordered) or unspecified (hash) | None | Checked count arithmetic (implemented) | None |
| Map | `K ⇀ V` | Key equivalence per family | First key, last value (implemented); left key on merge/join (proposed) | As the family | One value per key | Strict builders report duplicates (implemented) | Resolver call count and order undefined (proposed) |
| Bag | `T → ℕ` | Element equivalence | Left representative when present on left (proposed) | Unspecified | First-class | Additive sum overflow must be an explicit error (proposed) | None |
| Sequence | Finite word | Positional | Occurrence identity preserved (proposed; `FrozenVec` implemented for storage) | Positional | Positions distinct | Length overflow | None |
| Multimap | Relation, or `K → Set/Bag/Seq<V>` | Pair or key equivalence | Fiber policy in the type (proposed) | Fiber-dependent | Fiber-dependent | Fiber-dependent | None |
| Bimap | Partial bijection | Both projections functional | Left pair on intersection (proposed) | Unspecified | None | Union not closed; checked merge reports conflicts and preserves inputs (proposed) | None |
| Encoded integer set | Canonical unsigned codes | Codec bijection | Not applicable: values are decoded, not stored objects (implemented) | Ascending by code | None | Checked imports; limits per policy (implemented) | None |
| Prefix index | Prefix relation on keys | Byte, UTF-8, path, or bit prefix (distinct policies) | Stored key (proposed) | Lexicographic for FST (implemented via native stream) | None | Domain mismatch | None |

Distinctions the taxonomy enforces: representative identity is not equality;
`Fn` does not prove purity; typed domains and dictionary revisions do not
collapse into interchangeable ids; laziness does not imply bounded work or
memory; frozen, persistent, query, snapshot, and materialized are different
axes.

| Axis | Meaning | Frost today |
| --- | --- | --- |
| Frozen collection | Structural immutability boundary; consume to recover ownership | Families, `Frozen<B>`, adaptive frozen profiles, `FrozenIntSet` |
| Computed set / query | Description of how to obtain members from sources | Proposal (typed views, `IntSetExpr`) |
| Persistent collection | Versioned representation sharing unchanged structure | `FrozenIntSet` root sharing and explicit Chunked containers; imbl adapters; generic persistent families proposed |
| Snapshot | A view at a revision or consistency boundary | `SnapshotStore` whole-value versions; durable snapshots proposed |
| Materialized answer | An owned result independent of the query | `collect`, eager integer algebra, `into_*` extraction |

Output modes, adopted from the implementation notes, name what a caller
receives from each access model:

| Output mode | Meaning | Frost today |
| --- | --- | --- |
| Stored borrowed | `&T` or `(&K, &V)` into source storage | `Elements`, `Entries`, `LookupRef`, `SetGet` |
| Lending reconstructed | A borrow valid only until the next cursor advance | FST `stream()`; never exposed as `Elements` |
| Decoded by value | A value such as `u32` rebuilt from compressed storage | `ValueElements`, `LookupValue`, `FrozenIntSet` iteration |
| Shared owned | `Arc<T>` or a shared persistent node | `SnapshotStore::load`; integer root sharing |
| Independently owned | A cloned or consumed value | `cloned()` and `collect()`, `into_inner`, `thaw` |

"Zero-copy" is not a contract; one of these modes is.

## 3. Established facts (claim ledger)

Classes: **I** implemented and inspected; **U** documented upstream; **D**
derived reasoning; **P** proposal; **H** historical; **X** unverified.

| Claim | Statement | Class | Evidence |
| --- | --- | --- | --- |
| CL-REP-1 | Frost keeps the first equivalent key or element, the last map value, and the left representative on set union and intersection | I | `SRC-FROST-DOC-SET-OPERATIONS`, `SRC-FROST-TESTS` |
| CL-REP-2 | Rust `HashSet::intersection` may yield either side's representative; `HashMap::get_key_value` returns the stored key for an equal lookup key | U | `SRC-RUST-HASHSET-INTERSECTION`, `SRC-RUST-HASHMAP-GET-KEY-VALUE` |
| CL-ALG-1 | Membership-equivalent rewrites (XOR reassociation, nonlocal cancellation, operand swap, distribution) can change representatives or order | D (finite oracle) | `SRC-FROST-VERIFICATION-SET-ALGEBRA`, `SRC-FROST-DOC-SET-ALGEBRA` |
| CL-ALG-2 | Bag sum and max-union are distinct; additive sum can overflow; bimap union is not closed; sequences keep occurrence identity | D | Curated survey sections 2 and 3 |
| CL-ALG-3 | Literature attributions for bag laws, relational algebra, ART, succinct dictionaries, FST construction | X | Input-supplied bibliography; not read |
| CL-FROZEN-1 | `Frozen<B>` is sealed by `Eligible`, has no `Deref` to the backend, no universal thaw; `into_inner` consumes; `Retain` keeps storage, `Compact` performs the backend's conversion | I | `SRC-FROST-SRC-FROZEN`, `SRC-FROST-SRC-PLAN`, `SRC-FROST-DOC-ARCHITECTURE` |
| CL-FROZEN-2 | Families have fixed named partners; strict builders return the rejected item; frozen is structural, not deep immutability, pinning, or cheap clone | I | `SRC-FROST-DOC-ARCHITECTURE`, `SRC-FROST-DOC-PHILOSOPHY` |
| CL-FROZEN-3 | Borrowed `Elements`/`Entries`/`LookupRef`, decoded `ValueElements`/`LookupValue`, and lending FST streams are distinct access models | I | `SRC-FROST-SRC-READ`, `SRC-FROST-SRC-FST-ADAPTER` |
| CL-SNAP-1 | `SnapshotValue` is sealed; `publish` is unconditional; `publish_if_current` compares allocation identity and returns the rejected value plus a possibly superseded observed version; `update` may rerun its callback and returns the displaced version | I | `SRC-FROST-SRC-SNAPSHOT`, `SRC-FROST-DOC-SNAPSHOTS` |
| CL-INT-1 | `FrozenIntSet` uses a safe `IntKey` codec, canonical codes, absolute Dense coordinates, explicit Chunked, shared `Arc`/`Rc` root; commutative operations take the smaller limits, difference keeps left limits | I | `SRC-FROST-SRC-INTEGER`, `SRC-FROST-DOC-INTEGER-SETS` |
| CL-NARY-1 | `fst` 0.4.7 n-ary operations are heap-based. For `k ≥ 0` streams, `N` total keys, and keys of at most `L ≥ 0` bytes, the merge's own work is bounded by `O(k + N · (L + 1) · log(k + 1))` plus per-advance stream decoding and I/O, with `O(k · (L + 1))` bytes of merge workspace excluding stream internals. The upstream statement is `O(Σ n_i)` with stream-proportional memory; the fixed-fan-in reading is this corpus's interpretation, and none of this is measured performance | U + D | `SRC-FST-0_4_7-RAW-OPS`, `SRC-FST-0_4_7-OPBUILDER-DOCS`; derivation in the audit |
| CL-NARY-2 | A left-nested lazy union of `k` disjoint unordered sources costs `O(n k²)` probes | D | `SRC-FROST-DOC-SET-ALGEBRA` section 8 |
| CL-CSET-1 | Zanzibar composes usersets with union, intersection, exclusion (section 2.3.1); evaluates checks at a consistent snapshot no earlier than a client token (2.2, 2.4.4); cancels subtrees (3.2.3); Leopard merges a periodic offline index with timestamped updates (3.2.4) | U (sections and pages Opus-checked) | `SRC-ZANZIBAR-2019` |
| CL-CSET-2 | Lazy evaluation avoids result containers but not scans, buffering, deduplication, recursion, or spill | D | `SRC-FROST-DOC-SET-ALGEBRA`, `SRC-FROST-DOC-LAZY-VIEWS-SOURCE-AUDIT` |
| CL-CSET-3 | Index revision, durable snapshot, contextual facts, and cache must agree; a stale or incomplete index is not proof of absence | D | This synthesis; `SRC-ZANZIBAR-2019` for the consistency model |
| CL-AUTHZ-1 | SpiceDB permissions compose with `+`, `&`, `-` | U (mutable docs) | `SRC-SPICEDB-SCHEMA` |
| CL-AUTHZ-2 | GNAP delegates authorization to software; DID verification relationships are purpose-specific; a PeerId is a multihash of an encoded public key | U (boundary only) | `SRC-GNAP-RFC9635`, `SRC-DID-CORE-1_0`, `SRC-LIBP2P-PEER-IDS` |
| CL-AUTHZ-3 | Flattening correlated grants into independent sets invents permissions; unknown, error, conditional, and stale states are not absence | D | Section 6 |
| CL-SYM-1 | Union of symbolic environments is not a sound program-state merge; partial evaluation preserves unknowns | D | Section 6 |
| CL-ROAR-1 | Roaring 0.12.0 exposes count kernels and rank/select | U (source) | `SRC-ROARING-0_12_0-SRC` |
| CL-PKG-1 | Frost is one package, `frost-collections` 0.4.0, library `frost`, Rust 1.99.1, edition 2024 | I | `SRC-FROST-CARGO` |

## 4. Conflicting recommendations

| Topic | Frost1 | Frost2 | Frost3 | Repository | Resolution |
| --- | --- | --- | --- | --- | --- |
| Crate names | `frost-bags`, `frost-multimaps`, `frost-bimaps`, `frost-sequences`, `frost-integer-maps`, `frost-prefix-index` | `frost-bag`, `frost-multimap`, `frost-bimap` | `frost-set-algebra`, `frost-set-kernels`, `frost-integer-sets`, `frost-core`, `frost-concurrent` | One package | Record all; propose one policy (singular domain names, `frost-` prefix) for a future decision; create nothing |
| Map difference spelling | `difference_by_key` | `difference_keys` | — | Neither exists | Decide in `Q-MAP-1` |
| List-valued multimap name | `SeqMultiMap` | `ListMultiMap` | — | Neither exists | Decide in `Q-MM-1` |
| Bimap mutation | `try_insert(&mut self)` on `FrozenBiMap` | `try_insert(&mut self)` on a `BiMap` trait | — | Frozen types have no `&mut` structure | Mutable family or consuming operation (`Q-FROZEN-1`, `Q-BIMAP-1`) |
| Target restrictions | assumes workspace split | "none given" | — | Rust 1.99.1, edition 2024, `no_std` core, embedded builds | Repository constraints bind |
| Verification tooling | proptest/Kani/Loom per crate | workspace commands and CI matrix | finite oracle, Miri, Loom, Kani | Make targets, isolated packages, pinned nightly | Make targets bind; historical commands not runnable |
| Roaring version | unversioned | 0.11.5 | unversioned | 0.12.0 locked | Lockfile binds |
| N-ary complexity | "total linear" | "O(n₁+n₂+…)" | "memory proportional to streams" | Heap-based on source inspection; conservative derived bound | Qualified statement binds (CL-NARY-1) |

## 5. Design alternatives

| Area | Status quo | Conservative extension | Specialization | Rejected |
| --- | --- | --- | --- | --- |
| Generic algebra | Binary frozen-family operations | Typed borrowed views over set sources with terminal-specific plans | Encoded integer expressions; FST stream algebra; runtime plan arena | Universal `Set<T>` enum; universal `Clone + Hash + Ord` bound; blanket `union` methods overriding inherent ones |
| Maps | Lookup and traversal capabilities | `merge_with`/joins as typed views with explicit resolver contract | Sorted `KeyRun` kernel; rank-indexed frozen integer map | Silent value choice on overlap; renaming `LookupRef` |
| Bags, multimaps, bimaps | None | Separate semantic types with reference models first | Posting-list multimaps over integer sets | Aliasing bags to vectors; unchecked bimap union |
| Frozen lifecycle | Families, plans, exact wrappers, snapshots | Written standard for new families | Sealed marker traits for the standard | `&mut` methods on frozen types; universal `.thaw()` |
| Computed sets | Snapshots plus existing operations | Typed views with revisioned sources | Overlays, durable cursors, budgets, incremental engine | Promising constant memory, cheap exact counts, or enumeration of wildcard universes |
| Package layout | One package | Module boundaries mirroring the responsibility map | Workspace split once consumers justify it | Creating crates in a documentation change |

## 6. Boundaries: authorization, symbolic analysis, and the engine stack

```text
identity / transport authentication
    -> credential and delegation verification
    -> typed facts + policy revision + request context
    -> collection algebra / query execution        <- Frost's layer
    -> decision + explanation + obligations
    -> enforcement at resource, actor, or tool boundary
```

Frost belongs in the fourth line. Proof-chain verification, identity and
audience binding, caveat enforcement, expiry and revocation, replay
protection, obligations, audit policy, and check-to-use consistency belong to
the engine. Influences named in the supplied handoff (Vanadium-style blessings
and caveats, Macaroons, OAuth, GNAP, DID and libp2p PeerId bindings, UCAN,
ZCAP, credentials, Kameo and libp2p actor systems) are influences and
possible consumers; none is a dependency or a supported protocol, and only
the narrow facts in CL-AUTHZ-2 have been checked.

An illustrative deny-overrides rule is valid only over fully evaluated sets
on a common universe of complete request tuples:

```text
request  = (principal, action, resource, context)
Effective = (Granted ∩ ScopeAllowed ∩ ContextAllowed) \ Denied
```

Counterexample for flattening: principal `p` may `read` `r1` and `write`
`r2`. Projecting to independent sets `{p} × {read, write} × {r1, r2}` grants
`write r1`. Correlation must be kept in the tuple. Unknown, error, and
conditional outcomes stay distinct until an engine rule combines them; a
missing, stale, or incomplete index is not proof of absence (CL-AUTHZ-3,
CL-CSET-3).

Symbolic engines can use Frost for candidate-state sets, symbolic
environments, dependency indexes, retained versions, and terminal-specific
queries. The language semantics, SMT solving, path conditions, abstract
domains, widening, and a sound state-merging algorithm remain the engine's;
a union of environments is not a merge of program states (CL-SYM-1).

## 7. Current versus planned capability matrix

Anchored in inspected source at baseline `82becb9`.

| Capability | Current | Planned or proposed | Outside scope |
| --- | --- | --- | --- |
| Ordinary families (vec, hash map, ordered map, hash set, ordered set) with fixed partners | Implemented | — | — |
| Exact `Frozen<B>` with sealed eligibility and plans | Implemented | — | Universal thaw |
| Adaptive profiles | Implemented | — | Feature-selected storage |
| Borrowed set algebra and scalar counts (two frozen families, bitmap wrappers) | Implemented | Typed views over mixed sources | Universal set enum |
| Typed integer profile (`FrozenIntSet`, policies, limits, sharing, Chunked) | Implemented | `IntSetExpr`; persistent directories | In-house Roaring replacement |
| Snapshot publication (`SnapshotStore`) | Implemented | Durable snapshots and revisioned sources | Distributed consistency protocols |
| Live concurrent access via guarded callbacks | Implemented | — | Snapshot semantics from live maps |
| Framework adapters (Serde, Borsh, Rayon, Arbitrary, Schemars) | Implemented | — | — |
| Map merge/join algebra | — | Proposed (`Q-MAP-1`) | — |
| Bags, multimaps, bimaps, sequence algebra, prefix index | — | Proposed (`Q-BAG-1`, `Q-MM-1`, `Q-BIMAP-1`, `Q-PREFIX-1`) | — |
| Computed sets with overlays, budgets, durable cursors | — | Research (`Q-CSET-1`) | Authorization engine, policy language |
| Workspace split, `frost-api` facade | — | Proposed in the supplied research and its handoff; unadopted; current policy keeps one package | Created by a docs change |
| Solver, identity server, protocol support | — | — | Outside scope |

## 8. Prioritized research questions

Ordered so that no question needs an answer a later one would produce.

| Question ID | Decision to unlock | Why existing evidence is insufficient | Depends on | Child brief | Acceptance criteria |
| --- | --- | --- | --- | --- | --- |
| `Q-ALG-1` | Which rewrites a planner may apply per domain and terminal | Laws beyond sets are definitions without an oracle or literature audit | — | [algebraic-foundations](../prompts/algebraic-foundations.md) | One law table per domain with oracle evidence and counterexamples |
| `Q-FROZEN-1` | The written standard for new frozen families and wrappers | Current rules are scattered; input sketches violated them | — | [frozen-api-contracts](../prompts/frozen-api-contracts.md) | A one-page standard traced to source; rejected proposals listed |
| `Q-NARY-1` | Cost model for n-ary sorted and unordered operations | Inputs transfer one library's documented bound to generic algorithms | `Q-ALG-1` (which rewrites allow flattening) | [computed-sets prompt, Q-NARY-1 section](../prompts/computed-sets-and-durable-execution.md#embedded-question-q-nary-1); [seed block](#q-nary-1-cost-model-for-n-ary-sorted-and-unordered-operations) | All variables defined; heap versus frontier versus probe bounds stated |
| `Q-CSET-1` | Whether and how to specify computed sets with revisions, overlays, budgets, cursors | Only a summary of the missing report exists; obligations unstated | `Q-ALG-1`, `Q-FROZEN-1`, `Q-NARY-1` | [computed-sets-and-durable-execution](../prompts/computed-sets-and-durable-execution.md) | Compatibility rule, terminal table, cost model |
| `Q-AUTHZ-1` | Data contracts at the engine boundary | Influences unaudited; correlation and outcome lattice undefined | `Q-CSET-1` | [authorization-engine-integration](../prompts/authorization-engine-integration.md) | Boundary contract with each obligation attributed |
| `Q-MAP-1` | Map merge/join planning: resolver contract, representatives, sorted/hash plans, spellings | Two inputs differ on names; no compile-tested shape | `Q-ALG-1`, `Q-FROZEN-1` | [seed block](#q-map-1-map-merge-and-join-planning) | Compile-tested proposal with reference model |
| `Q-BAG-1` | Bag arithmetic, overflow contract, storage | Literature unread; overflow policy unspecified | `Q-ALG-1` | [seed block](#q-bag-1-bag-arithmetic-overflow-contract-and-storage) | Laws under sum and max-union; error contract |
| `Q-MM-1` | Multimap fiber semantics and naming | Fiber policies only sketched | `Q-ALG-1`, `Q-BAG-1` | [seed block](#q-mm-1-multimap-fiber-semantics-and-naming) | Per-fiber law table |
| `Q-BIMAP-1` | Two-index integrity, checked merge, composition | Transactional contract and panic behaviour unspecified | `Q-FROZEN-1` | [seed block](#q-bimap-1-two-index-integrity-checked-merge-and-composition) | Inverse-consistency model; conflict classes |
| `Q-PREFIX-1` | Prefix and rank indexes: domains, FST versus ART | Attributions unverified; domain policies only named | `Q-FROZEN-1` | [seed block](#q-prefix-1-prefix-and-rank-indexes) | Relation definitions; representation comparison plan |

The four questions with dedicated prompts carry every launch field in their
prompt files; `Q-NARY-1` has a section in the computed-sets prompt. The six
seeded follow-ups below carry the same fields here under their own headings,
compactly, so a maintainer can launch them without reconstruction. Each
heading is the brief location recorded in `manifest.json` for questions
without a dedicated prompt. A fuller brief can be produced with the
[subtopic handoff template](../templates/subtopic-handoff.md) once a
question's dependencies are answered. Stable ids are reserved here.

### 8.1 Seeded follow-up blocks

#### Q-NARY-1: cost model for n-ary sorted and unordered operations

Parent: this synthesis section 3 (CL-NARY-1, CL-NARY-2) and the audit's
derivation. Decision: which n-ary execution strategies a planner may choose
and how their costs are reported. Inherited assumptions to revalidate: the
heap derivation (source-based, not measured); the `O(n k²)` probe count for
left-nested unordered union (derived). Alternatives: binary heap merge;
frontier merge with `O(k)` state per emitted key; candidate-first probing by
selectivity; materialization target as the duplicate index. Required primary
evidence: `fst` 0.4.7 source; Rust 1.99.1 `BTreeSet` operation source; any
cited search-engine cursor contract at a pinned version. Experiments: work and
allocation counters over synthetic streams varying `k`, `N`, `L`, skew, and
empty inputs; no timing assertions. Acceptance: every variable defined;
empty, single-stream, and empty-key cases covered; no bound transferred
between algorithms. Owning module: future planner work under the general
set-algebra design; one package. Downstream: a section of the `Q-CSET-1`
report; a cost table for `docs/design/set-algebra.md`.

#### Q-MAP-1: map merge and join planning

Parent: curated survey sections 2 and 5; implementation notes sections 4 and 5. Decision: the resolver
contract, representative rule, join variants, spellings (`difference_by_key`
versus `difference_keys`), and sorted/hash plans for a map-algebra proposal.
Inherited assumptions to revalidate: left stored key on overlap (consistent
with set rules, untested for maps); resolver invoked only on collisions
(Frost1) versus option-valued for every key (Frost2). Alternatives: typed
views with a collision-only resolver; option-valued total resolver; join
views that retain both values with a separate reduction step. Required
primary evidence: `src/read.rs` (`LookupRef`, `Entries`, `OrderedEntries`);
Rust 1.99.1 `BTreeMap` and `HashMap` documentation for stored-key lookup and
iteration cost. Experiments: a `BTreeMap` reference model for joins and
merges with labelled representatives and resolver call counting; exhaustive
small-domain tests; compile checks of proposed signatures with
non-`Clone`/non-`Default` values on Rust 1.99.1. Acceptance: one resolver
contract with call count and order stated; one spelling chosen; sorted and
hash plans with representative recovery stated. Owning module: a future
`src/map` module within the one package. Downstream: a design proposal under
`docs/design`; test files; an update to section 7 here.

#### Q-BAG-1: bag arithmetic, overflow contract, and storage

Parent: survey section 2 (bag operation table); implementation notes section 2. Decision:
the canonical bag operations, their laws under sum versus max-union, the
overflow error contract, and the first storage shape. Inherited assumptions
to revalidate: the five-operation table (definitions, accepted); the
literature attributions (`SRC-INPUT-ALBERT-1991`,
`SRC-INPUT-HERNICH-KOLAITIS-2017`, unread). Alternatives: checked `try_sum`
at construction; terminal-deferred `try_count`/`try_cardinality`; saturating
counts (rejected for addition). Required primary evidence: the two papers if
obtainable; Rust integer overflow semantics in the reference. Experiments: a
`BTreeMap<T, u64>` reference model; exhaustive small-multiplicity cases
including overflow boundaries; representative tests with equal elements
carrying metadata. Acceptance: law table per union variant; explicit error
type and where it surfaces; distinct-count versus total-count queries named.
Owning module: a future `src/bag` module; one package. Downstream: design
proposal; tests; `Q-MM-1` input.

#### Q-MM-1: multimap fiber semantics and naming

Parent: survey section 5;
implementation notes section 4. Decision: whether fiber policy lives in the
type (`SetMultiMap`, `BagMultiMap`, and `SeqMultiMap` or `ListMultiMap`), the
relation-level operations on set-valued multimaps, and the projections.
Inherited assumptions to revalidate: fiber policy in the type (both inputs);
Guava and `multimap` crate behaviours (unverified). Alternatives: one type
with a policy parameter; three types sharing machinery; relation-only model
(`BTreeSet<(K, V)>`) with views. Required primary evidence: `Q-BAG-1`
results for bag fibers; Rust std documentation for the fiber containers.
Experiments: `BTreeSet<(K, V)>` and `BTreeMap<K, Vec<V>>` reference models;
projection and join tests against a nested-loop oracle; order tests for
sequence fibers. Acceptance: per-fiber law table (order, duplicates,
representatives); projection names fixed. Owning module: a future
`src/multimap` module; one package. Downstream: design proposal; tests.

#### Q-BIMAP-1: two-index integrity, checked merge, and composition

Parent: survey sections 2 and 5; implementation notes section 4; the Frozen API guide
proposal section. Decision: the mutable/frozen bimap pair, the conflict
classes a checked insert and merge report, transactional behaviour under
panic and allocation failure, and composition. Inherited assumptions to
revalidate: `try_insert` belongs on a mutable family (corrected, accepted);
`bimap` crate conflict classes (unverified); composition of aligned partial
bijections is a bijection (derived). Alternatives: two backing maps with an
inverse invariant; a single relation index with two projections; publish
immutable snapshots instead of concurrent mutation. Required primary
evidence: `Q-FROZEN-1` standard; `src/frozen.rs` and family builder
protocols. Experiments: two-`BTreeMap` reference model with an inverse
checker after every operation; panic injection into comparisons and hashing
to show no half-updated state escapes; drop counting. Acceptance: conflict
enum and returned ownership specified; invariant tests pass under injected
panics; composition law stated. Owning module: a future `src/bimap` module;
one package. Downstream: design proposal; tests; Frozen API guide update.

#### Q-PREFIX-1: prefix and rank indexes

Parent: survey sections 5 and 6;
implementation notes section 5. Decision: the four prefix relations
(`get_exact`, `entries_with_prefix`, `matching_prefixes`, `longest_prefix`),
the key-domain policies (byte, UTF-8, path component, bit prefix), and
whether FST, a radix tree, or a rank-indexed layout serves each workload.
Inherited assumptions to revalidate: FST for immutable byte dictionaries
(adapter exists; `SRC-FROST-SRC-FST-ADAPTER`); ART for mutable prefix
indexes and succinct rank/select layouts (unverified attributions,
`SRC-INPUT-ART-2013`, `SRC-INPUT-RRR-2002`); Roaring rank/select
(`SRC-ROARING-0_12_0-SRC`, verified). Alternatives: native `fst` streams
behind lending views; an in-house radix tree; presence bitmap plus rank
directory for integer keys. Required primary evidence: the ART and succinct
papers if read; `fst` 0.4.7 range and prefix stream APIs at the pinned
version. Experiments: a sorted `Vec<(Vec<u8>, V)>` reference model with
naive prefix filtering; domain-policy tests showing byte-prefix and
path-prefix disagreement; rank-to-value index checks with non-`Clone`
values. Acceptance: relation definitions per domain policy; representation
comparison plan with workloads named; no claim beyond the FST adapter as
implemented. Owning module: `src/backends/fst.rs` for the existing adapter;
a future `src/index` module; one package. Downstream: design proposal;
benchmark plan per `docs/maintainers/benchmarks.md`.

## 9. Evidence-backed roadmap

1. **Specify before building.** Run `Q-ALG-1` and `Q-FROZEN-1` in parallel;
   they share no dependency. Gate: law tables and the frozen standard are
   reviewed with the [checklist](../templates/review-checklist.md).
2. **Map algebra first among new domains** (`Q-MAP-1`), because joins inform
   multimaps, integer maps, and prefix maps. Gate: compile-tested typed-view
   signatures with non-`Clone`/non-`Default` members, a reference model, and
   exhaustive small-domain tests, within the one package.
3. **Computed sets** (`Q-NARY-1`, `Q-CSET-1`) after the laws exist. Gate: a
   finite model of revisions and overlays; a terminal table; a cost model;
   no promise of constant memory or cheap exact counts.
4. **Boundary contract** (`Q-AUTHZ-1`) once computed sets have obligations to
   attribute. Gate: a paragraph the README can carry without naming any
   protocol as supported.
5. **Remaining domains** (`Q-BAG-1`, `Q-MM-1`, `Q-BIMAP-1`, `Q-PREFIX-1`) one
   at a time, each with a reference model, representative tests with
   observable metadata, overflow and conflict tests, and documented panic
   behaviour, following `docs/design/collection-algebra.md`.
6. **Kernels and persistence last**, each admitted only with differential
   equivalence to the reference model and retained slower observations, per
   `docs/maintainers/benchmarks.md`.

Conditions that would change the recommendation: recovery of the original
citation mappings or the missing computed-set report (would raise
`source_validation` and could reorder `Q-CSET-1`); an explicit maintainer
decision to split the workspace (would add a naming question before step 2); or a
consumer workload demonstrating that typed views cannot express its queries
(would promote the runtime arena within `Q-CSET-1`).

## 10. References and lineage

Source ids resolve in [SOURCES.md](../SOURCES.md). Hazards and corrections are
in the [integration audit](../integration-audit.md). Parents: the three
curated reports. Children: the four prompts. Next review trigger: completion
of any child report, which must update sections 3, 4, 7, and 8 here.

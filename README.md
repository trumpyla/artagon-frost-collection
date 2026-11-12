# frost-collections

Frost gives Rust applications dependable collection semantics as their storage
and execution needs change. Build mutable values, freeze their structure,
combine sets, and publish snapshots with explicit rules for representatives,
traversal, ownership, and cost.

The library provides mutable/frozen families, small capability traits, typed
integer sets, adaptive storage, and optional native collection adapters.
Families own their mutation and freeze contracts; native backends retain their
specialized APIs. Concurrent collections use a separate access model.

**Target:** **Rust 1.99.1, edition 2024**. `rust-toolchain.toml` pins 1.99.1;
Cargo declares `rust-version = "1.99.1"`, and required CI jobs use that exact
release. Version **0.4.0** adds frozen set algebra to the reviewed family,
adaptive-storage, framework and concurrency design. The library is
`no_std`; `alloc` and `std` add capabilities. The default enables only `std`.

The first sections below explain why Frost exists and which contracts it keeps.
Code examples start at [Core API](#core-api).

## Why Frost

Authorization engines, symbolic executors, static analyzers, and query engines
repeatedly combine facts, relationships, permissions, and candidate states.
They intersect candidates, join indexed facts, keep older versions, and often
only need to know whether any result exists. Frost aims to be a collection
substrate for that work that is precise about meaning and honest about cost:

- **Precise semantics.** Each family states its duplicate, representative,
  order, and ownership rules, and keeps them when a private kernel changes.
- **Explicit ownership.** Construction, freezing, publication, and recovery are
  named transitions with documented costs.
- **Representation-aware execution.** Sorted, hashed, bitmap, and encoded
  integer storage each expose only the operations they can honestly support.
- **Observable costs.** Allocation, sharing, copying, and retries are
  documented rather than hidden behind a "zero-cost" label.

Frost does not replace `Vec`, `HashMap`, or every specialized index. It claims
no speedup without a measurement for the relevant workload.

## Semantic laws

Related collections do not share one algebra. Each abstraction has its own
logical model, and its operations need their own laws:

| Abstraction | Logical model | Law that differs | In Frost today |
| --- | --- | --- | --- |
| Set | Membership in a domain | Union, intersection, difference; a complement needs an explicit universe | Implemented: ordered and hash families, bitmap wrappers, typed integer profile |
| Map | Finite partial function from keys to values | Overlapping keys need an explicit value policy | Implemented families; map merge and join are proposals |
| Bag (multiset) | Finite-support count of each element | Additive sum differs from max-union; sums can overflow | Proposal |
| Sequence | Ordered finite word | Concatenation keeps order and repeated occurrences | Implemented sequence families; sequence algebra is a proposal |
| Multimap | Key to a set, bag, or sequence of values | The per-key collection decides duplicates and order | Proposal |
| Bimap | Partial bijection | Union can break uniqueness on either side | Proposal |

Membership is not the whole contract. Two Rust values can be equal and still be
different stored objects. Each Frost operation therefore states:

- **Equivalence:** which relation decides "same element": `Eq` with `Hash`,
  `Ord`, or a lossless integer codec.
- **Representative:** which stored object survives. Frost families keep the
  first equivalent key or element and the last map value; union and
  intersection keep the left operand's representative. Exact `Frozen<B>`
  wrappers keep their native backend's rules.
- **Order:** ascending, insertion order, byte-lexicographic, or unspecified.
  A stable traversal order is not a sorted order.
- **Multiplicity:** sets and maps collapse duplicates; sequences keep each
  occurrence; a bag counts them.
- **Failure and overflow:** whether a failure returns an error, uses checked
  arithmetic, or panics, and which input the caller gets back.
- **Callback effects:** how often user hashing, comparison, resolver, or update
  callbacks run. An `Fn` bound does not prove that a callback is pure.

An optimizer rewrite is valid only when it keeps every observation the API
promises. Two expressions with the same members can still return different
representatives, orders, or errors. The
[collection algebra guide](docs/design/collection-algebra.md) covers the
proposed map, bag, sequence, multimap, and bimap laws.

## Value domains and formal algebra

A value domain states which values are admissible. A key domain additionally
states logical identity. Neither one decides multiplicity, traversal, conflict
resolution, ownership, or whether two indexed sources share a revision.
Those are separate collection and operation contracts.

The proposed [formal model](docs/design/algebra/formal-model.md) starts with
admissible values and key equivalence, then defines finite sets, sequences
(mathematical lists), partial maps, bags, multimap fibers, and partial bijections.
Its [domain contract](docs/design/algebra/domains.md) distinguishes validation,
normalization, lossless encoding, runtime source compatibility, and conversions
that can collapse distinct keys. A mathematical law is an optimization permission
only under its representative, order, effect, failure, and context assumptions.

Read the [formal-algebra index](docs/design/algebra/README.md) for each domain,
[frozen lifecycle refinement](docs/design/algebra/frozen.md),
[computed-source model](docs/design/algebra/execution-and-computed-sets.md), and
[adversarial review](docs/reviews/value-domain-algebra-review.md). The
[terminology registry](docs/design/algebra/terminology.md) preserves implemented
names and the selected `SeqMultiMap`, `SetMultiMap`, and `BagMultiMap` proposals.
This documentation adds no mandatory `ValueDomain` parameter, production algebra
API, crate, solver, or authorization engine.

## Frozen lifecycle

```text
construct or update      mutable family, native backend, or builder
  -> validate/normalize  duplicates, order, codec and length checks
  -> freeze              seal, compact, freeze, or an explicit freeze plan
  -> retain or publish   own the value, share it, or publish a snapshot
  -> evaluate            lookups, traversal, algebra, counts
  -> materialize         construct a result with explicit element ownership;
                         the frozen inputs stay frozen
  -> leave (optional)    extract the backend, thaw, or use owning iteration;
                         each consumes the frozen value
```

Freezing is a structural ownership transition. A frozen value has no insertion,
removal, reordering, or mutable element access. Materializing a result reads
the frozen inputs and leaves them unchanged. Leaving the frozen state is a
separate step that consumes the value.
Frozen does **not** mean deeply immutable (elements can have interior
mutability), pinned, cheap to clone, serialization-compatible, or fresh with
respect to any outside source.

Five related concepts stay separate:

| Concept | Meaning | In Frost today |
| --- | --- | --- |
| Frozen collection | An owned value with no structural mutation API | Implemented: `Frozen<B>`, frozen families, adaptive frozen profiles, `FrozenIntSet` |
| Computed set or query | A description of how to answer from sources | Proposal; today's borrowed set iterators are lazy but are not reusable plans |
| Persistent version | A version that shares unchanged structure | Partial: `imbl-7` exact wrappers keep native persistent sharing; Chunked integer sets share containers but copy a flat directory; a persistent integer family is a proposal |
| Snapshot | A version tied to a publication or revision boundary | Implemented in memory with `arc-swap-1`; no durable snapshots |
| Materialized answer | A constructed result collection; elements may borrow, share ownership, or be independently owned | Implemented: collected results, integer and bitmap algebra results |

Materializing a collection of references still depends on the sources' lifetimes.
A query description does not freeze the data that it reads. A stable snapshot
can still be too old for a revocation requirement. The
[Frozen API guide](docs/design/frozen-api.md) maps each of these contracts to
source and tests.

## Engines Frost is meant to serve

These workloads motivate the design. Frost implements none of these engines; it
supplies collection operations whose contracts an engine can rely on.

### Symbolic execution and analysis

Symbolic executors and static analyzers keep candidate-state sets, symbolic
environments, dependency indexes, worklists, and retained versions. Frost
collections can hold those structures. The engine still owns language
semantics, path conditions and solver calls, abstract domains, widening, and
sound state merging. A union of collection members is not a sound merge of
program states, and partial evaluation must keep "unknown" separate from
"false" or "absent".

### Authorization and access control

```text
identity and transport authentication
  -> credential and delegation verification
  -> typed facts + policy revision + request context
  -> collection algebra and query execution        <- Frost's layer
  -> decision + explanation + obligations
  -> enforcement at the resource, actor, or tool boundary
```

The engine around Frost must authenticate callers, verify credentials and
delegation chains, bind identities and audiences, enforce caveats and expiry,
handle revocation and replay, and bind each decision to the actual invocation.
Authentication is not authorization, and an identifier is not proof of
delegated authority.

The following deny-overrides formula is an illustration only. It holds only
when every set is fully evaluated over one universe of complete request tuples
`(principal, action, resource, context)`:

```text
Effective = (Granted ∩ ScopeAllowed ∩ ContextAllowed) \ Denied
```

It is not a general policy semantics. Splitting correlated grants into separate
principal, action, and resource sets can invent permissions. Unknown values,
evaluation errors, conditional grants, and obligations need explicit engine
rules. A missing, stale, or incomplete index is not proof of absence.

The design is informed by research on relationship-based authorization,
Vanadium-style blessings and caveats, Macaroons, OAuth and GNAP grants, DID and
libp2p `PeerId` identity bindings, UCAN and ZCAP capabilities, verifiable
credentials, and Kameo/libp2p actor systems. These are influences and possible
consumers, not dependencies or supported protocols. Frost verifies no proof
chain and implements none of these protocols. A DID and a `PeerId` are different
identifiers; binding one to the other is a separate cryptographic concern. The
[source catalog](docs/research/SOURCES.md) records which specific claims were
checked against primary sources; most protocol details are not yet audited.

### Computed sets, indexes, and durable data

A computed set is an expression evaluated against sources at a defined revision,
not necessarily a resident container. Membership, existence, bounded
enumeration, exact or estimated count, and materialization are different
terminal operations and can use different plans. Lazy execution can avoid
building result containers, but it can still scan, buffer, deduplicate, follow
recursive relations, or spill. It does not make every expression constant-memory
or every exact count cheap, and it cannot enumerate an unbounded universe.

Google's Zanzibar paper describes userset rewrite rules combined by union,
intersection, and exclusion (section 2.3.1); checks evaluated at a consistent
snapshot no earlier than a client-supplied token (sections 2.2 and 2.4.4);
cancellation of subtrees once a result is known (section 3.2.3); and a Leopard
index with a periodic offline build plus an online update layer (section 3.2.4)
([Zanzibar, USENIX ATC 2019](https://research.google/pubs/zanzibar-googles-consistent-global-authorization-system/),
read 2026-11-09). Frost implements none of this. Overlays,
durable cursors, and revisioned indexes are research topics. Index revision,
durable snapshot, request context, and cache semantics must agree; a frozen
in-memory index alone does not prove that an authorization answer is fresh.

## Current, proposed, and outside scope

| Area | Status | Reference |
| --- | --- | --- |
| One library package, `frost-collections`, imported as `frost` | Implemented | [Cargo.toml](Cargo.toml) |
| Five mutable/frozen family pairs, exact `Frozen<B>` wrappers, adaptive profiles | Implemented | [Frozen API guide](docs/design/frozen-api.md) |
| Lazy borrowed set algebra and scalar counts for ordered and hash sets; bitmap algebra | Implemented | [Set operations](docs/set-operations.md) |
| Typed integer profile with shared storage and an explicit Chunked policy | Implemented (`int-set`); its algebra is eager | [Integer design](docs/design/integer-sets.md) |
| Whole-value snapshot publication | Implemented (`arc-swap-1`) | [Snapshot contract](docs/design/snapshots.md) |
| Generic borrowed set expressions; borrowed integer expressions | Proposal | [Set algebra](docs/design/set-algebra.md), [integer views](docs/design/integer-views.md) |
| Map merge and join, bags, multimaps, bimaps, sequence algebra, prefix indexes | Proposal | [Collection algebra](docs/design/collection-algebra.md) |
| Persistent tree directories; a separate persistent integer family | Proposal | [Persistent storage](docs/design/persistent-integer-storage.md) |
| Computed sets over durable or indexed relations | Research | [Research synthesis](docs/research/synthesis/foundations-and-engine-roadmap.md) |
| A `frost-api` facade over a `crates/frost-*` workspace | Reorganization target (epic `frost-wlt`); boundaries not approved; current policy keeps one package | [Crate layout](docs/design/crate-layout.md), [design decisions](docs/design/decisions.md) |
| Authorization or policy engine, solver, identity server, durable store, distributed consistency protocol | Outside scope | — |

### Naming and compatibility

Research terminology maps to the existing API: `LookupRef<Q>` returns stored
map keys and values; `SetContains<Q>` tests membership; `SetGet<Q>` retrieves a
stored representative. `Entries` together with `OrderedEntries` promises
ascending map traversal. These public names are retained.

The proposed multimap contracts are `SeqMultiMap`, `SetMultiMap`, and
`BagMultiMap`, with different order and duplicate laws. Proposed bimap insertion
belongs on `MutableBiMap` or a builder; `FrozenBiMap` stays structurally
immutable. `FrozenIntSet` remains the implemented owned integer set;
`IntSetExpr`, `FrozenIntSetView`, and `PersistentIntSet` describe distinct future
query, storage-view, and persistent-version contracts. None of these future
types is implemented by the naming decision. The
[naming table](docs/design/collection-algebra.md#naming-dispositions) records
every selected spelling and its rationale.

This unreleased revision integrates shared integer storage and explicit Chunked
sets while retaining the main project's checked output policies and generic
set-algebra design. Guarded snapshots, conditional publication, deterministic
models, finite exhaustive checks, and isolated proof harnesses remain available.
[Integration validation](docs/revision2-integration-validation.md) distinguishes
executed gates from remaining release requirements;
[GitHub import checks](verification/revision2-integration/github-import-ci/README.md)
record the subsequent Make and CI adaptation. The
[integration review](docs/reviews/revision2-integration-review.md) explains the
selective merge; earlier reviews and benchmarks remain historical evidence.

## Documentation map

| Category | Artifact | Purpose | Status |
| --- | --- | --- | --- |
| Index | [Documentation index](docs/README.md) | Complete inventory of design, research, review, and maintenance documents | Navigation |
| Frozen API | [Frozen API guide](docs/design/frozen-api.md) | Families, exact wrappers, plans, recovery, access models, costs, snapshots | Current contracts; proposals marked |
| Principles | [Philosophy](docs/design/philosophy.md), [architecture](docs/design/architecture.md), [decisions](docs/design/decisions.md) | Durable principles, current architecture, and recorded choices | Current design |
| Naming | [Naming dispositions](docs/design/collection-algebra.md#naming-dispositions) | Stable public capabilities and distinct proposed collection names | Current decision; future types marked |
| Formal domains | [Formal model](docs/design/algebra/formal-model.md), [value domains](docs/design/algebra/domains.md), [domain chapters](docs/design/algebra/README.md) | Denotations, validity, closure, observations and refinement obligations | Proposed contracts; no production API |
| Algebra | [Set operations](docs/set-operations.md), [collection algebra](docs/design/collection-algebra.md), [general set algebra](docs/design/set-algebra.md) | Implemented set contracts; proposed algebra for other collections | Implemented and proposal |
| Integers | [Integer design](docs/design/integer-sets.md), [integer views](docs/design/integer-views.md), [roadmap](docs/design/integer-roadmap.md) | Codecs, storage, policies; deferred expressions | Implemented and proposal |
| Versions | [Snapshots](docs/design/snapshots.md), [persistent storage](docs/design/persistent-integer-storage.md) | Publication, retries, sharing, and retention | Implemented and proposal |
| Research | [Research index](docs/research/README.md), [synthesis](docs/research/synthesis/foundations-and-engine-roadmap.md), [sources](docs/research/SOURCES.md), [integration audit](docs/research/integration-audit.md) | Curated reports, decision map, source evidence, corrections and gaps | Research; not accepted design |
| Templates | [Research prompt](docs/research/templates/research-prompt.md) | Start a landscape, deep-dive, or validation study | Template |
| Examples | [examples/](examples/) | Executable scenarios run as tests | Implemented; see feature requirements |
| Verification | [Verification scope](docs/verification.md), [testing standard](docs/maintainers/testing.md), [benchmark method](docs/maintainers/benchmarks.md) | Model assumptions, bounds, gate tiers, and measurement rules | Current guidance |
| Standards | [Crate standard](CRATES.md), [crate layout](docs/design/crate-layout.md) | Package layout, dependency placement, API obligations, coverage accounting, test and benchmark metadata; reorganization target | Current standard; target marked |
| Evidence | [Supplied inputs](docs/reviews/supplied/README.md), [integration validation](docs/revision2-integration-validation.md) | Unmodified inputs and dated results | Historical evidence |
| Contributing | [Contributor guide](CONTRIBUTING.md), [maintainer guide](docs/maintainers/guide.md), [release procedure](docs/maintainers/releasing.md) | How to change, verify, and release | Current process |

Reading paths:

- **New contributor:** this README, the Frozen API guide, `examples/`, the
  philosophy, then the contributor guide.
- **Frozen API implementer:** architecture, the Frozen API guide, the linked
  source modules and tests, then verification scope.
- **Algebra or planner implementer:** set operations, collection algebra,
  general set algebra and its review, then the research synthesis.
- **Authorization-engine integrator:** the authorization section above, the
  snapshot contract, then the research synthesis and its authorization prompt.
- **Researcher starting a subtopic:** the research index, the synthesis, the
  research prompt template, then the integration audit's open gaps.

## Core API

This example works even with every feature disabled:

```rust
use frost::prelude::*;

let frozen = [3, 1, 2].seal();
assert_eq!(frozen.get(1), Some(&1));
assert_eq!(frozen.len(), 3);
let mut array = frozen.into_inner();
array.sort_unstable();
assert_eq!(array, [1, 2, 3]);
```

With the default features:

```rust
# #[cfg(feature = "std")]
# {
use frost::prelude::*;
use std::collections::{BTreeMap, HashMap, HashSet};

let retained: Frozen<Vec<i32>> = vec![1, 2, 3].seal();
let compact: FrozenVec<i32> = vec![1, 2, 3].compact();
let ordered: FrozenOrderedMap<_, _> = BTreeMap::from([(2, 'b'), (1, 'a')]).compact();
let hashed: FrozenHashMap<_, _> = HashMap::from([("task", 7)]).compact();
let members: FrozenHashSet<_> = HashSet::from(["ready", "done"]).compact();

assert_eq!(retained.as_ref(), compact.as_ref());
assert_eq!(ordered.iter().map(|(k, _)| *k).collect::<Vec<_>>(), [1, 2]);
assert_eq!(hashed.get("task"), Some(&7));
assert!(members.contains("ready"));
# }
```

A frozen collection has no structural mutation API or mutable element access.
Leaving that state consumes the frozen value. Interior mutability **inside an
element** is still possible; frozen does not imply deeply immutable or pinned.

```compile_fail,E0594
use frost::prelude::*;
let mut frozen = [1, 2].seal();
frozen[0] = 9;
```

```compile_fail,E0451
let _ = frost::Frozen { inner: [1, 2] };
```

## Mutable families and fixed partners

Use a family name in application signatures when the backend should remain an
implementation detail. Entry and map-iterator handles are crate-owned, and map
families do not dereference to their native backends.

```rust
# #[cfg(feature = "std")]
# {
use frost::prelude::*;

let mut sequence = MutableVec::new();
sequence.push(1);
let mut sequence: MutableVec<_> = sequence.freeze().thaw();
sequence.push(2);
assert_eq!(sequence.freeze().as_slice(), &[1, 2]);

let mut counts = MutableHashMap::new();
counts.entry(String::from("verified")).and_modify(|n| *n += 1).or_insert(1);
let frozen: FrozenHashMap<_, _> = counts.freeze();
assert_eq!(frozen.get("verified"), Some(&1));
let mut counts: MutableHashMap<_, _> = frozen.thaw();
assert_eq!(counts.remove("verified"), Some(1));

let mut ordered = MutableOrderedMap::new();
ordered.insert("b", 2);
ordered.insert("a", 1);
assert_eq!(ordered.freeze().keys().copied().collect::<Vec<_>>(), ["a", "b"]);

let members: MutableOrderedSet<_> = [3, 1, 3].into_iter().collect();
assert_eq!(members.freeze().iter().copied().collect::<Vec<_>>(), [1, 3]);
let members: MutableHashSet<_> = ["ready", "done"].into_iter().collect();
assert!(members.freeze().contains("ready"));
# }
```

Each family has one named frozen partner:

| Mutable | Frozen | Consuming native extraction |
| --- | --- | --- |
| `MutableVec<T>` | `FrozenVec<T>` | `into_vec()` |
| `MutableHashMap<K, V, S>` | `FrozenHashMap<K, V, S>` | `into_map()` |
| `MutableOrderedMap<K, V>` | `FrozenOrderedMap<K, V>` | `into_map()` |
| `MutableHashSet<T, S>` | `FrozenHashSet<T, S>` | `into_set()` |
| `MutableOrderedSet<T>` | `FrozenOrderedSet<T>` | `into_set()` |

Call `freeze()` to consume a mutable family and `thaw()` to recover its paired
mutable family. Explicit native conversions support integration with existing
libraries. There is no exact-backend `Mutable<B>` wrapper or universal map enum.
The canonical modules are `family::vec`, `family::hash_map`, `family::ordered`,
`family::hash_set`, and `family::sets`. `frost::mutable` provides convenient
reexports. Incoming native extraction names such as `into_hash_map` and
`into_btree_map` remain available alongside `into_map` aliases.

## Explicit adaptive profiles

Adaptive profiles are separate public types with closed representations. Cargo
features never add a variant to an existing type. Removal does not automatically
demote a mutable profile; `shrink_to_fit` makes that decision explicit.

| Mutable profile | Storage choices | Frozen result and recovery |
| --- | --- | --- |
| `InlineVec<T, N>` | inline buffer / Vec | `InlineFrozenVec<T, N>`; `thaw` returns `InlineVec` |
| `AdaptiveHashMap<K, V, S>` | linear entries / hash table | `AdaptiveFrozenHashMap<K, V, S>`; `thaw_with(policy)` restores a chosen growth policy |
| `AdaptiveOrderedMap<K, V>` | sorted vector / B-tree | `FrozenOrderedMap<K, V>`; explicit `AdaptiveOrderedMap::from_frozen(frozen, policy)` reconstructs the profile |

`AdaptiveFrozenHashMap::thaw()` uses the default growth threshold. It preserves
its hashing policy and current representation, but does not recover a former
nondefault threshold. Use `thaw_with` when that threshold matters.
`FrozenOrderedMap::thaw()` always returns `MutableOrderedMap`; there is no
ambiguous inverse selected by a feature or runtime tag.

```rust
# #[cfg(feature = "std")]
# {
use frost::prelude::*;
use frost::adaptive::HashRepr;

let policy = AdaptiveHash::linear_up_to(2);
let mut map = AdaptiveHashMap::with_policy(policy);
map.extend([("a", 1), ("b", 2), ("c", 3)]);
assert_eq!(map.representation(), HashRepr::Table);
let mut map = map.freeze().thaw_with(policy);
map.remove("c");
assert_eq!(map.representation(), HashRepr::Table);
map.shrink_to_fit();
assert_eq!(map.representation(), HashRepr::Linear);
# }
```

## Frozen set operations

`FrozenOrderedSet` and `FrozenHashSet` provide lazy borrowed `union`,
`intersection`, `difference`, and `symmetric_difference` iterators. They allocate
no intermediate set and require no element cloning. Union and intersection keep
the left set's stored representative when two elements are equivalent.

Use `is_subset`, `is_superset`, and `is_disjoint` for relations, or
`intersection_len`, `union_len`, `difference_len`, and
`symmetric_difference_len` when only the result count is needed. The scalar
queries allocate no result collection.

```rust
# #[cfg(feature = "alloc")]
# {
use frost::family::FrozenOrderedSet;

let left: FrozenOrderedSet<_> = [1, 2, 4, 7].into_iter().collect();
let right: FrozenOrderedSet<_> = [2, 3, 4, 8].into_iter().collect();
assert_eq!(left.intersection_len(&right), 2);
assert_eq!(left.union_len(&right), 6);
assert_eq!(left.difference(&right).copied().collect::<Vec<_>>(), [1, 7]);

// Materialize when an independently owned result is required.
let common: FrozenOrderedSet<_> = left.intersection(&right).copied().collect();
assert!(common.is_subset(&left));
# }
```

Frozen Roaring bitmaps/treemaps and `FixedBitSet` backends expose the same named
algebra and scalar queries under their existing features. Their algebra methods
produce a frozen bitmap using the native representation's word/container
kernels. `FixedBitSet` operations ignore raw padding and document the resulting
logical universe for differently sized operands.

See `docs/set-operations.md` for result ownership, complexity and boundary contracts,
and `examples/set_operations.rs` for the tested complete scenario.

## Feature catalog

Every feature is additive. Enabling one never changes an existing public type's
representation or selects a different hasher. Backends are reexported from
`frost::backend`, with their native specialized APIs intact.

| Feature | Backends | Environment |
| --- | --- | --- |
| none | arrays and slice capabilities | `core` |
| `alloc` | Vec, boxed slices, deque, linked list, heap, B-tree map/set; ordered and sequence families | `alloc` |
| `std` (default) | standard hash map/set and their mutable/frozen families | `std` |
| `hashbrown-0_17` | `SwissTable` hash map/set | `alloc`; supply a hasher explicitly |
| `indexmap-2` | insertion-order map/set | `alloc`; `std` enables its default hasher |
| `smallvec-1` | inline vectors that can spill | `alloc` |
| `arrayvec-0_7` | fixed-capacity inline vectors | `core`; compact conversion needs `alloc` |
| `imbl-7` | persistent vector, hash map/set, ordered map/set | `std` |
| `roaring-0_12` | compressed `u32` and `u64` integer sets | `std` |
| `int-set` | typed frozen integer profile with fixed Sorted/Dense/Roaring/Chunked candidates | `alloc` |
| `fixedbitset-0_5` | dense bitsets | `alloc` |
| `slotmap-1` | generational slot maps | `alloc` |
| `phf-0_14` | compile-time perfect-hash maps/sets, including declaration-order variants | `core` at runtime |
| `fst-0_4` | compressed byte-string maps/sets and lending streams | `std` |
| `dashmap-6` | sharded concurrent maps/sets | `std` |
| `scc-3` | concurrent hash maps/sets | `std` |
| `crossbeam-skiplist-0_1` | concurrent ordered skip maps/sets | `std` |
| `arc-swap-1` | atomic publication of whole frozen snapshots | `std` |
| `serde-1` | logical serialization/deserialization and strict decoding helpers | `alloc` |
| `rayon-1` | parallel traversal, construction and extension | `std` |
| `arbitrary-1` | generation for family and adaptive types | `std` |
| `borsh-1` | canonical family serialization and strict family decoding | `alloc` |
| `schemars-1` | schemas for the logical Serde shape | `alloc` |
| `frameworks` | all five framework integrations | `std` |
| `advanced` | the eight advanced nonconcurrent backend features above | `std` |
| `concurrent` | the four concurrent/publication features above | `std` |

For this source distribution, use a path dependency:

```toml
[dependencies]
frost = { package = "frost-collections", path = "../frost", features = ["smallvec-1", "roaring-0_12", "concurrent"] }
```

This archive is not a claim that version 0.4.0 has been published to crates.io.

## Typed frozen integer sets

Enable `int-set` for `FrozenIntSet<K>`, a separate profile for lossless integer
codecs. `IntKey::Word` selects `u32` or `u64` at compile time. Built-in codecs cover
`u8/u16/u32/u64`, `i8/i16/i32/i64`, `usize/isize`, `char`, and the corresponding
fixed-width unsigned nonzero types. Applications can implement `IntKey` for their
own domain types. Signed encodings preserve numeric order by changing their
unsigned ordering coordinates; narrow and character decoders reject invalid codes.

```rust
# #[cfg(feature = "int-set")]
# {
use frost::integer::{FrozenIntSet, IntSetPolicy};
use frost::TryFreezeWith;

let left = vec![-4_i64, -1, 0, 7].try_freeze_with(IntSetPolicy::auto()).unwrap();
let right: FrozenIntSet<i64> = [-1, 7, 8].into_iter().collect();
assert_eq!(left.intersection(&right).iter().collect::<Vec<_>>(), [-1, 7]);
assert_eq!(left.union_len(&right), 5);
# }
```

The complete profile has fixed Sorted, Dense, Roaring, and Chunked candidates.
Automatic construction chooses among the first three using the existing memory
score; it does not select Chunked or force small inputs into a 4 KiB Dense window.
`IntSetPolicy::chunked()` explicitly chooses shared 2^16-code containers. Dense
windows use absolute encoded coordinates, including for differently based sets.
Cardinality is `u64`; inclusive spans use `u128` so the full `u64` span does not
overflow.

All representations share immutable storage, making `clone()` an O(1) reference
count increment. A result equal to an operand reuses its allocation when the
result policy permits that representation. Chunked storage also shares untouched
containers when deriving a changed result:

```rust
# #[cfg(feature = "int-set")]
# {
use frost::integer::{FrozenIntSet, IntSetPolicy};
let before = FrozenIntSet::try_from_iter(
    [1_u32, 65_537], IntSetPolicy::chunked(),
).unwrap();
let copy = before.clone();
assert!(copy.shares_storage_with(&before));
let update: FrozenIntSet<u32> = [2].into_iter().collect();
let after = before.union(&update);
assert_eq!(after.shared_chunks(&before), 1);
assert_eq!(before.iter().collect::<Vec<_>>(), [1, 65_537]);
# }
```

The chunk directory is a flat sorted index. A changed result copies directory
entries and shares unaffected payloads; it does not have logarithmic tree-path
updates. Chunked containers use sorted `u16` arrays, fixed bitmaps, or a full
container marker. Retained versions retain their reachable allocations, and
final-owner destruction can release many containers. `new()` allocates a shared
storage header even when its member buffer is empty.

The profile supports `no_std + alloc`. Shared storage uses `Arc` on targets with
pointer-width atomics and `Rc` otherwise, retaining the selected pointer's normal
auto traits. Enabling `std` also enables the dependency's supported runtime CPU
dispatch. The existing `roaring-0_12` integration keeps its `std` requirement and
native adapter API. Consumers that disable default features must explicitly add
`std` if they need standard-library families or the dependency's std APIs.

Algebra preserves compact outputs: small updates can copy and edit Dense bits,
while distant Dense windows can promote to Roaring. Each set retains its Dense
payload and Roaring partition limits. Union, intersection, and symmetric
difference use the smaller limit from each operand; difference keeps the left
limits. `IntSetLimits`, `limits()`, and all four `*_with_policy` methods remain
available. A forced construction backend is not a permanent result type. Empty,
identical, and shared operands still obey the requested output policy. These
limits do not bound Sorted or Chunked storage, temporary allocations, retained
versions, or total process memory.

`contiguous_payload_bytes()` reports exact Sorted/Dense buffers; `payload_bytes()`
provides a broader storage estimate, including a statistics-based Roaring proxy.
Neither is a measurement of resident memory. `chunk_count()` and `shared_chunks()`
make chunk retention observable. Logical equality and canonical member/run Hash
event streams ignore representation, sharing, policies, and construction history.
Hash bytes are not a serialization format. Count and predicate kernels keep
values encoded; some mixed pairs allocate temporary conversion storage.

Iteration yields decoded keys by value. Bulk set operations use canonical codes
without reconstructing keys. Checked imports reject invalid and noncanonical
codes and test ordering on the observed input. Custom codecs must preserve the
complete value and remain stable; equality alone cannot verify discarded
metadata. Ordinary ordered sets retain their existing `freeze()` result.

See [the integer design](docs/design/integer-sets.md),
[revision integration review](docs/reviews/revision2-integration-review.md), and
[current validation](docs/revision2-integration-validation.md) for the current
contracts and executed checks. The earlier
[output review](docs/reviews/integer-output-review.md),
[output validation](docs/integer-output-validation.md), and
[comparative benchmarks](docs/integer-output-benchmarks.md) are retained as
historical evidence; their measurements are not results for this merged source.

The [general set-algebra design](docs/design/set-algebra.md) and
[borrowed integer expressions](docs/design/integer-views.md) remain proposals;
current integer algebra is eager. The
[persistent-storage design](docs/design/persistent-integer-storage.md) now
separates the implemented flat shared chunks from proposed persistent tree
directories and a possible future `PersistentIntSet`. The
[integer roadmap](docs/design/integer-roadmap.md) records the remaining admission
gates. No lazy expression engine or separate persistent family is implemented.

## Framework contracts

The five framework integrations cover the ordinary mutable/frozen families and
the applicable adaptive profiles, including the mutable ordered set and both
hash-set families. Frozen adapters expose only read-only traversal.

- Serde encodes logical elements or map entries. Family decoding applies the
  family duplicate policy; strict helpers reject duplicates or unsorted input.
- Borsh family encoders use ascending keys/elements for maps/sets, and family
  decoders reject nonascending or duplicate input. Exact `Frozen<B>` delegates
  to the native backend's wire contract. Different stored representatives may
  serialize differently even when a custom key equality ignores their metadata.
- Decoding reconstructs default hashers and adaptive policies. Runtime policies,
  capacities, and storage variants are not serialized.
- Rayon results follow the family duplicate rule when the parallel source has a
  defined encounter order. Unordered sources do not acquire deterministic winners.
  Slice-backed profiles support indexed parallel traversal. Native hash/tree
  parallel traversal may allocate a temporary buffer; this is not hidden by an
  allocation-free claim.
- Arbitrary generation exercises adaptive variants with their invariants intact.
  Schemars describes the logical data shape, not internal representation fields.

```rust
# #[cfg(feature = "serde-1")]
# {
use frost::prelude::*;
// serde_json is the format crate used by this example.
let map: FrozenOrderedMap<String, u32> =
    serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
assert_eq!(map.keys().map(String::as_str).collect::<Vec<_>>(), ["a", "b"]);
assert_eq!(map.thaw().get("b"), Some(&2));
# }
```

## Contracts that matter

- `LookupRef` returns a stored key and value; `ValueRef` lends only a value;
  `LookupValue` decodes a value. Compressed storage does not fabricate references.
- `Elements` lends stored elements; `ValueElements` yields reconstructed values.
  `Cardinality` preserves a bitmap's natural `u64` count.
- Frozen families keep the **first equivalent key/element representative** and,
  for maps, the **last value**. Strict constructors reject duplicates.
  Exact `Frozen<B>` wrappers preserve the native backend's construction rules.
- `Retain` does not rebuild or allocate. Moving inline storage costs up to the
  inline representation's size. `Compact` may allocate, move, shrink or rehash;
  it does not promise that every backend has zero spare capacity.
- Hashers are preserved. Feature activation never silently switches to a faster
  non-cryptographic hasher. Choose a hasher for your input and threat model.
- Adaptive thresholds are caller policies, not benchmark-derived recommendations.
- Concurrent callbacks keep guards scoped. They are synchronous; do not block,
  re-enter the same collection, or await while accessing it under a guard.
  Live traversal/length observations do not constitute a consistent snapshot.
- Publish a complete frozen value when readers need a stable whole-collection
  view. Element interior mutability remains the caller's responsibility.

Representative rules matter when equality ignores some data. Write `PartialEq`,
`Eq`, and `Hash` by hand so that they use the same fields; deriving them would
compare and hash every field.

```rust
# #[cfg(feature = "std")]
# {
use frost::prelude::*;
use std::hash::{Hash, Hasher};

// Equality and hashing use only `id`; `origin` is metadata.
struct Key {
    id: u32,
    origin: &'static str,
}
impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Key {}
impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

// Construction keeps the first equivalent key and the last value.
let map: FrozenHashMap<Key, &str> = [
    (Key { id: 7, origin: "first" }, "a"),
    (Key { id: 7, origin: "second" }, "b"),
]
.into_iter()
.collect();
let (stored, value) = map.get_key_value(&Key { id: 7, origin: "query" }).unwrap();
assert_eq!((stored.origin, *value), ("first", "b"));

// Intersection borrows the left operand's representative.
let left: FrozenHashSet<Key> = [Key { id: 1, origin: "left" }].into_iter().collect();
let right: FrozenHashSet<Key> = [Key { id: 1, origin: "right" }].into_iter().collect();
assert_eq!(left.intersection(&right).next().map(|k| k.origin), Some("left"));
assert_eq!(right.intersection(&left).next().map(|k| k.origin), Some("right"));
# }
```

## Snapshot reads and optimistic updates

With `arc-swap-1`, load one version for a consistent structural view. Use a
short-lived guard or `peek` for scoped reads; use `load` or `guard.into_owned()`
to retain an owned Arc. Conditional publication compares allocation identity.
`update` retries after conflict and returns the displaced predecessor; its
callback may run more than once.

```rust
# #[cfg(feature = "arc-swap-1")]
# {
use frost::concurrent::SnapshotStore;
use frost::family::FrozenVec;

let store = SnapshotStore::new(FrozenVec::from([0_usize]));
let before = store.load_guard();
let displaced = store.update(|version| FrozenVec::from([version.as_slice()[0] + 1]));
assert_eq!(displaced.as_slice(), &[0]);
assert_eq!(before.as_slice(), &[0]);
assert_eq!(store.peek(|version| version.as_slice()[0]), 1);
# }
```

Guarded reads can fall back to reference counting. Contention, allocation,
callbacks, and destructors prevent an unconditional bounded-latency guarantee.
[The snapshot contract](docs/design/snapshots.md) explains ownership, retries,
panics, progress, and old-version retention.

## Examples and verification

The `examples/` programs contain assertions and tests that execute their actual
scenarios. Cargo declares their feature requirements. This README is included
in rustdoc, so its Rust blocks are doctested.
The root Makefile requires GNU Make 3.81+ and Bash 3.2+. Targets that discover
Cargo features, examples, or package paths also require `jq` 1.6+.

```sh
rustup toolchain install 1.99.1 --profile minimal --component clippy,rustfmt
make
make check
make verify
```

`make check` runs formatting, Clippy, rustdoc, tests and examples, plus the API
obligation catalog and live benchmark inventory. The isolated fuzz, coverage,
mutation, and model-checking tools have separate targets and stay outside the
library dependency graph. The [testing standard](docs/maintainers/testing.md)
defines the pull-request, nightly, and release gates; the
[standards evidence](verification/crate-standards-validation/README.md) records
executed checks and the coverage scopes still to measure. Coverage targets are
goals, not claims of measured coverage.

Use the [issue forms](.github/ISSUE_TEMPLATE/) for bug reports, proposals, and
research or verification requests, and the
[PR template](.github/pull_request_template.md) to describe a change and its
evidence. [GitHub intake and Beads](docs/maintainers/github-workflow.md) explains
how reports connect to the standards epic `frost-wou` and its dependent crate
reorganization epic `frost-wlt`.

See [the documentation index](docs/README.md),
[design philosophy](docs/design/philosophy.md),
[contributor guide](CONTRIBUTING.md), and
[maintainer notes](docs/maintainers/guide.md). [AGENTS.md](AGENTS.md) records the
same invariants and verification expectations for automated changes, and
[CRATES.md](CRATES.md) defines the layout, coverage, and metadata standard.
[Integration validation](docs/revision2-integration-validation.md) records the
reviewed source's executed results;
[GitHub import checks](verification/revision2-integration/github-import-ci/README.md)
record the PR checkout and Make workflow separately.
[Verification scope](docs/verification.md) states model assumptions and bounds.
No throughput or latency advantage is claimed without workload-specific evidence.

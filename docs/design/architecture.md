# Frost 0.4.0 architecture

Frost separates collection semantics, mutation state, storage policy, and access
capabilities. Application signatures can name a semantic family; algorithms can
request a small capability; specialized code can retain a native backend. Each
choice has an explicit contract and a documented way to recover owned data.

This revision targets **Rust 1.99.1, edition 2024**. The compiler is pinned in
[rust-toolchain.toml](../../rust-toolchain.toml), declared in [Cargo.toml](../../Cargo.toml),
and used by required CI jobs. The crate remains `#![no_std]`: `alloc` enables
allocating families, and the default `std` feature adds standard hash families.
Optional integrations that require the standard library enable `std` explicitly;
enabling `std` alone does not activate them. Nightly checks are additional
diagnostics, not a library requirement.

[DESIGN_DECISIONS.md](decisions.md) explains the comparison that produced
this combined design. This document describes the resulting architecture and
its boundaries. [README.md](../../README.md) contains the user-facing feature catalog
and tested introductory examples.

Version 0.4.0 adds frozen set algebra without changing the family identities or
freeze/thaw pairings. [SET_OPERATIONS.md](../set-operations.md) records the lazy
borrowed family operations, direct scalar kernels, frozen bitmap results, and
left-representative rule.

The [general set-algebra proposal](set-algebra.md) broadens future expression
composition to mutable/frozen families and compatible native backends, with
separate borrowed, encoded-value, and lending-stream boundaries. It is a design,
not an implemented change to the current API or kernels.

The subsequent [performance review](../reviews/performance-review.md) keeps these public
boundaries. It adds shape-aware ordered kernels and simpler dense word loops,
reuses adaptive lookup results and owned parallel buffers, and measures their
costs against the supplied 0.4.0 source. Internal algorithm selection does not
change a family's representation or its public method signatures.

## 1. Public type identity and family ownership

An ordinary family owns its mutation API, duplicate policy, traversal contract,
and frozen partner. Its representation is private. Today the mutable families
use standard vectors, hash tables, and B-trees; that fact does not make those
native types part of ordinary application signatures.

Canonical family modules contain or reexport the public types and their named
handles. `frost::family`, `frost::mutable`, and the prelude provide convenient
spellings for the **same types**. They do not create a second implementation or
introduce conversion between otherwise identical wrappers.

| Family | Canonical public module | Fixed mutable/frozen pair | Current storage transition |
| --- | --- | --- | --- |
| Contiguous sequence | `family::vec` | `MutableVec<T>` / `FrozenVec<T>` | `Vec<T>` / boxed slice |
| Hash map | `family::hash_map` | `MutableHashMap<K, V, S>` / `FrozenHashMap<K, V, S>` | Native table / shrunk native table |
| Ordered map | `family::ordered` | `MutableOrderedMap<K, V>` / `FrozenOrderedMap<K, V>` | B-tree / strictly ascending boxed entries |
| Hash set | `family::hash_set`, with mutable implementation and handles in `family::sets` | `MutableHashSet<T, S>` / `FrozenHashSet<T, S>` | Native table / shrunk native table |
| Ordered set | `family::ordered`, with mutable implementation and handles in `family::sets` | `MutableOrderedSet<T>` / `FrozenOrderedSet<T>` | B-tree / strictly ascending boxed elements |

Every ordinary mutable family's consuming `freeze()` returns its named frozen
partner. Every ordinary frozen family's `thaw()` returns its named mutable
partner. Explicit freeze plans may select a different profile without changing
those named methods' results. Ordered thawing rebuilds a tree; hashing and sequence thawing recover
the corresponding owned storage. A boxed zero-sized sequence follows `Vec`'s
zero-sized capacity convention on thaw, rather than promising capacity equal to
length for every `T`.

Native interoperability is explicit: `from_vec`, `from_map`, `from_set`, native `From`
implementations, and consuming `into_vec`, `into_map`, or `into_set` methods.
The retained names `into_hash_map`, `into_hash_set`, `into_btree_map`, and `into_btree_set` are
also available on the applicable frozen types. Adopting an already constructed
native map preserves the representatives that native map currently contains;
it cannot reconstruct entries discarded before adoption.

There is no exact-backend `Mutable<B>` facade. Native mutable backends already
participate directly in capability traits, while family wrappers earn their
maintenance cost by defining a stable semantic API. There is also no universal
map enum that imposes ordering bounds on hash-map callers or combines ascending
and insertion-order traversal into an ambiguous contract.

Sources: [family modules](../../src/family/mod.rs), [mutable reexports](../../src/mutable.rs),
[public exports](../../src/lib.rs).

### Entries, iterators, and mutation access

Ordinary mutable map `Entry`, `OccupiedEntry`, and `VacantEntry` handles are
crate-owned types with private backend fields. Operations such as `or_insert`,
`and_modify`, occupied replacement, and entry removal expose the family contract.
They do not expose a native entry type or a mutable key. `get_mut`, mutable
iteration, and `IndexMut` lend values only.

Family map iterators are named crate-owned wrappers or adapters. Adaptive map
iterators use private representation dispatch. Concrete iterators implement
`DoubleEndedIterator`, `ExactSizeIterator`, `Clone`, or `FusedIterator` where
their representations support the promise. Generic capability traits do not
silently require every one of those properties. Contiguous sequences use
standard slice views and slice iterators because contiguous access is itself
their contract; they do not need a new trait for every slice operation.

Maps and sets do not dereference to a native mutable backend. Frozen sequences
can dereference to `[T]`, but never to `&mut [T]`. Owning iteration consumes a
frozen value and returns owned elements, ending the frozen state explicitly.

Sources: [iterator wrappers](../../src/iter.rs),
[hash-map entries](../../src/family/hash_map/mutable.rs),
[ordered-map entries](../../src/family/ordered/mutable.rs).

## 2. Semantic laws and trait coherence

Construction from entries retains the **first equivalent key representative**
and the **last value**. Set construction and ordinary insertion retain the
first equivalent stored element. Replacing a map value does not replace its
stored key. Ordered construction uses stable sorting and in-place duplicate
reduction; mutable maps enforce the same rule by insertion. No family test
depends on an unspecified native bulk constructor's representative choice.

Hash families use hash/equality lookup and have unspecified traversal order.
Ordered families use `Ord` equivalence for lookup and uniqueness, promise
ascending traversal, and validate ranges consistently with the corresponding
standard B-tree operation, including its empty-collection behavior. Whole
ordered-collection equality, ordering, and hashing compare the stored ascending
sequence using the relevant element traits.

These contracts rely on the caller's ordinary Rust trait laws: equivalent hash
keys must hash consistently, borrowed queries must agree with their keys, and
ordering must be coherent. A key whose equality ignores a metadata field can
retain observably different metadata from another equivalent key. Freezing
does not make inconsistent user trait implementations consistent, and interior
mutation of a key must not invalidate its lookup or ordering relation.

Strict `try_from_unique` builders report duplicates instead of silently applying
the default duplicate rule. A `DuplicateKey` or `DuplicateElement` returns the
rejected item, not the complete input or partially built collection. Other
consumed entries are dropped. The constructor documents its validation order:
input-order mutable/hash builders and ordered builders that first collect and
sort are not interchangeable recovery protocols. Validated sorted-input
construction is a separate operation; it can return an unchanged input through
`Rejected` when validation happens before commitment.

The open traits in `read` and `write` model operations, not membership in one
universal collection category:

- `MapTypes` supplies one key and value identity shared by `LookupRef`, `Entries`,
  `MapInsert`, and `MapRemove`. Read and write capabilities cannot accidentally
  declare different associated key/value types on the same map.
- `ElementType` provides the element identity used by element traversal,
  representative lookup, and `SetInsert`.
- Query relations belong on individual implementations. `LookupRef<Q>` does
  not impose a universal `Borrow`, `Ord`, or backend-specific `Equivalent` bound.
- Traversal uses generic associated iterator types. Algorithms add only the
  iterator bounds they need. GAT-based traversal is not dyn-compatible;
  suitable lookup and endpoint traits remain usable through trait objects.
- Removal traits exist where removal has one clear meaning. Insertion-ordered
  maps and sets retain explicit native shift-versus-swap removal choices.

Downstream collection types can implement these open capabilities under Rust's
normal coherence rules. Downstream code cannot use an open capability impl to
declare a backend eligible for the crate's exact frozen wrapper or snapshot
store; those two eligibility boundaries are sealed.

Sources: [read capabilities](../../src/read.rs), [write capabilities](../../src/write.rs),
[duplicate errors](../../src/family/mod.rs),
[ordered validation](../../src/family/ordered/frozen.rs).

## 3. Exact freezing and explicit plans

`Frozen<B>` preserves backend `B` and its own semantics. Native construction,
comparison, traversal, and decoding rules remain native rules. For example,
`Frozen<BTreeMap<K, V>>` constructed with `collect()` is not a promise to apply
`FrozenOrderedMap`'s duplicate representative policy.

An audited backend is eligible only when shared access cannot mutate its
collection structure and an independently mutable clone cannot mutate the
frozen structure through shared ownership. Persistent collections may share
immutable nodes because their mutation uses copy-on-write. Live concurrent
collections do not satisfy this eligibility contract.

Construction of exact wrappers is curated. `Default`, collection, and decoding
require sealed `Eligible` implementations where appropriate; other construction
paths are explicit freeze-plan implementations. `Frozen<B>` exposes neither a
public shared backend accessor nor `Deref<Target = B>`. `into_inner()` consumes
the wrapper and recovers the backend. Layout and `repr(transparent)` are not
public guarantees.

| Plan or operation | Contract |
| --- | --- |
| `Retain`, reached by `seal()` | Preserve the backend without allocating or rebuilding. Moving inline storage still moves its inline elements. |
| `Compact`, reached by `compact()` | Perform the backend's documented compact conversion. It may allocate, shrink, rehash, or move elements. It does not universally eliminate spare capacity. |
| `Sorted` | Convert a heap to an ascending sequence with the priority-order choice explicit. A heap deliberately has no ambiguous `Compact` conversion. |
| `ExactLen<N>` through `TryFreezeWith` | Validate a fixed-length conversion; a length mismatch returns the original input through `Rejected`. |
| `Inline<N>` or `AdaptiveHash` | Select a separately named adaptive frozen profile according to an explicit policy. |
| `IntSetPolicy` through `TryFreezeWith` | Validate a lossless integer codec and select `FrozenIntSet<K>` storage; rejection returns the owned source through `Rejected`. |

Standard vectors, B-trees, and hash collections compact into their ordinary
frozen families. Specialized backends can keep a specialized exact wrapper:
hashbrown and indexmap compaction preserve their backend contracts and hashing
policy while shrinking storage. A missing plan is a compile-time error; there
is no silent conversion to an unrelated representation.

`FreezeWith<P>` gives each source/plan pair one associated output type.
`TryFreezeWith<P>` makes recoverable errors explicit. Neither trait turns a
panic or allocation abort into `Err`. The blanket convenience trait `FreezeExt`
has a closed method set, `seal` and `compact`, to avoid accumulating blanket
method names that collide with downstream extension traits. Additional plans
use `freeze_with` or `try_freeze_with` directly.

Freezing is structural: no insertion, removal, reordering, or mutable element
access is exposed while the value remains frozen. Stored cells, atomics, locks,
or shared pointers retain their own interior-mutability behavior. Frozen values
are not deeply immutable and are not pinned.

Sources: [exact wrapper and eligibility](../../src/frozen.rs),
[plan definitions](../../src/plan.rs), [native adapters](../../src/backends/mod.rs).

## 4. Adaptive profiles, policy, and cost

Adaptive storage is a separately named profile within one semantic family. Its
private representation set is closed. Features never add a variant, switch the
default hasher, or introduce new key bounds into an existing profile. An enum
still carries its largest variant and layout overhead; no exact size or ABI is
promised. Diagnostic representation tags do not participate in logical equality.

| Profile | Storage and operations | Freeze and recovery |
| --- | --- | --- |
| `InlineVec<T, N>` | Inline initialized prefix, then `Vec<T>` after growth or reservation exceeds the inline capacity | `InlineFrozenVec<T, N>` stores an inline prefix or boxed slice; `thaw` returns `InlineVec` |
| `AdaptiveHashMap<K, V, S>` | Linear entries for small maps; a native hash table after promotion | `AdaptiveFrozenHashMap` stores boxed linear entries or a shrunk table; `thaw_with(policy)` restores a chosen growth policy |
| `AdaptiveOrderedMap<K, V>` | Sorted vector with binary search and shifting insertion; B-tree after promotion | Both variants freeze to `FrozenOrderedMap`; explicit `from_frozen(frozen, policy)` reconstructs an adaptive mutable profile |

Mutable removal never automatically demotes a representation. Explicit
`shrink_to_fit` allows inline or small-map demotion and avoids repeated
conversion merely because a workload oscillates around a threshold. Freezing
may select a compact representation again. Threshold defaults—8 entries for
linear hashing and 32 for sorted mutable maps—are policy placeholders, not
measured crossover recommendations.

The adaptive hash profile keeps one hash-family query contract even when a
linear lookup needs only equality. Table conversion may clone its hasher because
the native table cannot generally yield an owned hasher separately from its
entries. Those representation-changing operations require `S: Clone` where
needed. Read access, removal, frozen thawing, and native extraction do not gain
an unnecessary clone bound. Ordinary explicit-hasher family builders require
neither `Clone` nor `Default`; ordinary freeze/thaw preserves that chosen hasher.

A frozen adaptive hash map preserves its hasher and current representation but
does not store the former growth threshold. `thaw()` uses the default policy;
`thaw_with(policy)` is required to recover a nondefault policy. In the ordered
case, `FrozenOrderedMap::thaw()` always returns `MutableOrderedMap`.
`AdaptiveOrderedMap::from_frozen` is the explicit alternative, avoiding an
ambiguous inverse based on construction history.

### Allocation and dispatch boundaries

Collecting a sequence that fits within `N` directly populates the inline buffer
without a temporary vector. A spill, native-vector adoption, serialization,
parallel construction, or another operation can still allocate. Freezing a heap
vector into a box may shrink or move its allocation. Converting a tree to sorted
frozen entries flattens and moves entries without needing new comparisons.
Constructing an ordered family from unsorted entries sorts first; stable sorting
can allocate temporary storage even though duplicate reduction is in place.

Linear hash lookup is a scan; table lookup follows the native hash table's
behavior. Sorted insertion can shift a suffix. Promotion rebuilds a different
structure. Some bulk read methods select their representation once before a
loop, while iterator advancement and mutating operations have their own dispatch
boundaries. There is no universal promise of one branch for every bulk operation,
nor a throughput claim based solely on being inline or adaptive.

### Panic and ownership guarantees

Inline storage concentrates its unsafe implementation in the internal
`adaptive::seq::buf` module. Its invariant is an initialized prefix with one
owner per element; owning iteration tracks a remaining initialized range.
Mutation outside that module uses safe operations. Checked reservation arithmetic
rejects logical length overflow even for zero-sized elements, for which a native
vector can legitimately have `usize::MAX` capacity.

Retain, partial initialization, cloning, and owning iteration must maintain
correct ownership if user code or a destructor panics. Regression tests cover
these paths, zero-sized destructors, alignment, and mixed-end iteration. A second
panic during unwinding can still abort under ordinary Rust rules.

Adaptive map promotion may invoke user hashing or ordering. A panic leaves a
valid collection but can drop entries already moved into an unfinished
replacement; it does **not** promise transactional rollback. Other ordinary
mutation callbacks likewise do not undo mutations performed before a panic.
Recoverable validation errors and allocation failures remain distinct from this
unwinding guarantee.

Sources: [adaptive architecture](../../src/adaptive/mod.rs),
[inline ownership](../../src/adaptive/seq/buf.rs),
[inline mutation](../../src/adaptive/seq/mutable.rs),
[adaptive hashing](../../src/adaptive/hash_map/mutable.rs),
[adaptive ordering](../../src/adaptive/ordered_map/mutable.rs),
[core regressions](../../tests/core_review.rs), [ownership regressions](../../tests/inline_safety.rs).

## 5. Advanced storage gets honest capabilities

The explicit `int-set` feature adds the typed integer profile in
[`frost::integer`](../../src/integer/mod.rs). `FrozenIntSet<K>` preserves the
logical key domain while `IntKey::Word` selects sealed `u32` or `u64` machinery.
Its private candidates are sorted codes, aligned dense windows, and width-specific
Roaring containers. The feature enables this complete fixed profile with `alloc`;
`std` optionally enables the dependency's supported runtime CPU dispatch.
Other dependency features do not add candidates. Ordinary `freeze()` pairings
retain their documented meaning. See [integer design](integer-sets.md) for
codec laws, retained representation limits, bounded planning, and encoded bulk
operations. Result selection preserves compact storage across mixed operations.

Native optional backends are reexported from `frost::backend` so specialized
algorithms retain their full native APIs. Frost implements only capabilities
that accurately describe each representation.

| Storage distinction | Capability boundary |
| --- | --- |
| Stored map keys and values | `LookupRef` and `Entries` can lend their stored representatives. |
| Generational slot maps | `ValueRef` lends a stored value without inventing a stored key reference. Keys and generation checks remain native operations; secondary-map membership is independent of removal from a primary map. |
| Compressed FST maps | `LookupValue` returns a decoded integer. Native streams lend keys from a reusable buffer and therefore do not masquerade as ordinary borrowed `Entries` or `Elements`. |
| Roaring integer sets and dense bitsets | `ValueElements` yields member indices by value; `SetContains` tests membership. A bit index is not an addressable stored element for `SetGet`. |
| Natural wide cardinalities | `Cardinality` returns `u64`; the adapter does not truncate a count to satisfy `Len`. Bitset member count is distinct from its addressable universe length. |
| Insertion-order maps | `IndexedEntries` and `IndexOf` describe positions. Ascending-key traits are not inferred from having some stable traversal order. |
| Ring buffers and heaps | `TwoSlices` exposes actual ring slices; `PriorityRead` exposes heap priority. Heap storage-order traversal is not sorted traversal. |
| Inline, fixed-capacity, persistent, and static collections | Their native storage and ownership contracts remain explicit. `ArrayVec` offers recoverable `TryPush`; persistent storage need not be contiguous; PHF lookup does not require runtime table construction. |

FixedBitSet membership, counting, and value iteration exclude padding outside
its logical universe even when raw blocks contain nonzero padding. Retaining
or extracting the native backend does not rewrite those blocks. Exact FST
freezing is restricted to audited immutable byte-storage types; an arbitrary
user-defined `AsRef<[u8]>` is not enough to establish structural immutability.

Sources: [backend exports](../../src/backend.rs), [bitsets](../../src/backends/bitmap.rs),
[slot maps](../../src/backends/slotmap.rs), [FST](../../src/backends/fst.rs),
[persistent collections](../../src/backends/persistent.rs).

## 6. Live concurrency and whole-version publication

Live concurrent maps and sets use a distinct access model. `ConcurrentLookup`
runs a callback while one matching entry is protected. `ConcurrentSetLookup`
does the same for a stored member. `ConcurrentUpdate` supplies an exclusive
mutable **value** reference only for backends that can provide it; skip-list
entries do not receive an invented exclusive-update capability.

The callback runs once on a hit and not at all on a miss. Its borrowed entry
cannot escape into the returned result. `get_cloned` clones while protected and
then releases protection; a cloned `Arc` or other shared interior state remains
shared. Callbacks are synchronous and should stay short: do not block, reenter
the collection, wait on work that may need its locks, or attempt asynchronous
work while retaining protected access. Unwinding releases guards but does not
roll back changes already made to a value.

Insertion, removal, async access, and live traversal remain native APIs.
Separate queries, traversal steps, and length observations do not constitute a
consistent whole-collection snapshot. Live backends therefore implement neither
the borrowed ordinary-map traversal contract nor `Eligible` for `Frozen<B>`.

With `arc-swap-1`, `SnapshotStore<F>` instead publishes a complete frozen value
whose type satisfies the sealed `SnapshotValue` marker. Eligible exact wrappers,
ordinary frozen families, and supported frozen adaptive profiles qualify;
ordinary mutable families and live concurrent collections do not.
`FrozenIntSet<K>` also qualifies when `int-set` is enabled; its `Send` and `Sync`
behavior retains the logical key's restrictions through `PhantomData<K>`.

`load()` returns an owned `Arc<F>`. A reader loads once and reuses that version
for queries that must observe the same collection structure. `publish(value)`
atomically replaces the current version and returns the version displaced by
that particular swap. Earlier readers retain their old version without holding
a native entry lock or borrowed publication guard. Allocation and reference
counting are real costs; no universal lock-free or allocation-free claim is made
for the complete reader/writer workflow.

`load_guard()` returns a crate-owned `SnapshotGuard<F>` for a protected view;
`peek` scopes a guard around one synchronous callback. A guard can be converted
into an owned Arc without changing its version. Guarded reads can use ownership
fallbacks, so this does not promise reference-count-free reads in every case.

A read/derive/unconditional-publish sequence can overwrite another writer's
work. `publish_if_current` instead compares the expected Arc allocation's
identity and recovers a rejected candidate on failure. `update` uses that
primitive to retry derivation after conflicts and returns the displaced version.
Its callback may repeat; the operation has no fairness or bounded-retry promise.
`Send` and `Sync` follow the payload's actual auto traits. The store preserves
structural versions; it does not deep-freeze elements or shared interior state.
See [the complete snapshot contract](snapshots.md) and
[verification scope](../verification.md) for the linearization and model boundary.

Source: [concurrent access and snapshot publication](../../src/concurrent/mod.rs).

## 7. Five framework contracts

Each integration has its own compatibility-line feature and independent tests.
Enabling Serde does not require Rayon; enabling Schemars does not imply the Serde
implementation feature. The `frameworks` feature is convenience aggregation.
Family types implement integrations explicitly instead of relying on accidental
forwarding from a hidden backend.

### Serde (`serde-1`)

Encoding exposes logical sequences, sets, and map entries. Runtime storage
variants, capacity, hasher state, and adaptive thresholds are absent from the
wire representation. Hash traversal order remains unspecified. Family decoding
uses the family duplicate policy; optional `unique` and `sorted_unique` helpers
provide stricter validation. Decoding reconstructs default hashers and policies.

Initial reservation from a deserializer's hint is capped cautiously, but total
decoded elements and input bytes are not bounded by this adapter. Strictness is
validation, not a resource quota. Exact `Frozen<B>` decoding requires eligibility
and preserves `B`'s own decoding semantics.

### Borsh (`borsh-1`)

Families encode sequences in sequence order and maps/sets in ascending
key/member order with Borsh length prefixes. Hash-family encoding therefore
needs an ordering bound even though ordinary hash lookup does not. Family
decoding rejects duplicate or nonascending input, independent of Borsh's
optional native strict-order setting, and rejects collisions that would shrink
the decoded family. Map-key and set-member zero-sized-type restrictions and `u32` wire-length
limits remain explicit format constraints.

Ascending order does not make arbitrary user serializers deterministic. Stored
representatives that compare equal while carrying differently serialized
metadata can produce different bytes. Exact wrappers retain their backend's
wire and validation rules. Input staging and validation do not impose a total
allocation budget.

### Rayon (`rayon-1`)

Borrowed and owning parallel iteration are available for supported families and
profiles. Mutable traversal is limited to mutable sequences and map values;
frozen elements, keys, and set members remain immutable. Slice-backed traversal
can be indexed directly. Native hash/tree traversal may stage entries or
references in a temporary vector, and parallel construction also stages input.

Construction applies the same first-representative/last-value rule when the
source defines encounter order. An unordered source does not acquire a stable
duplicate winner. Ordered map collection sorts stably and reduces duplicates
before adopting sorted storage. Thread-safety bounds apply to state actually
shared or moved across threads; a hasher does not need an unrelated `Sync` bound
merely because values are traversed in parallel.

### Arbitrary (`arbitrary-1`)

Generated values go through invariant-preserving constructors. Adaptive mutable
generation also varies policies and growth history, exercising a spilled vector
or promoted map whose current length has fallen below its threshold. Frozen
profiles follow their normal construction paths. These are useful fuzzing inputs,
not a claim of uniform sampling or exhaustive coverage.

### Schemars (`schemars-1`)

Schemas describe the logical Serde data shape by forwarding to the corresponding
sequence, map, or set schema. They do not describe private enum tags, native table
layout, capacity, hasher state, or adaptive thresholds. Schema generation adds
no implication that the internal representation or runtime policy will round-trip.

Sources: [framework overview](../../src/adapters/mod.rs), [Serde](../../src/adapters/serde.rs),
[Borsh](../../src/adapters/borsh.rs), [Rayon](../../src/adapters/rayon.rs),
[Arbitrary](../../src/adapters/arbitrary.rs), [Schemars](../../src/adapters/schemars.rs).

## 8. Evolution and verification boundaries

Features are additive integrations, not global storage selectors. Public
dependency features identify compatibility lines; adding an incompatible future
line requires a separately named dependency and adapter if both must coexist.
Private representation does not promise a stable layout, and changing public
iterator capabilities, auto traits, or semantic laws still requires API review.

Verification covers core-only, alloc-only, default, isolated optional features,
and their union; bare-metal builds check that host test dependencies have not
hidden a `std` requirement. Executable examples declare required features and
run their actual scenarios as tests. README and source Rust examples are
doctested, including compile-fail checks for forbidden mutation and invalid
capability use. Ownership-sensitive inline paths receive Miri checks alongside
functional and panic regressions.

[REVIEW.md](../reviews/historical-review.md) records specific findings and before-fix reproductions.
[VERIFICATION.md](../reviews/historical-verification.md) records checks actually completed and their
limits. Passing tests does not prove the absence of all unsafe-code defects or
establish a performance advantage. Choosing thresholds, hashers, and specialized
backends remains a workload decision that should be supported by measurements.

# Persistent integer storage: an adversarial design

Status: **flat shared chunks are implemented; tree directories and a separate
`PersistentIntSet` family remain proposals**. No new performance claim is made
here. The upstream research below records checks from the earlier 2026-11-09
review; issue/PR statuses were not refreshed for this integration.

The current [integer profile](integer-sets.md) supports O(1) root sharing and
explicit Chunked storage with shared Array, Bitmap, and Full containers. Its
directory is flat and copied for changed results. The
[integration review](../../../docs/reviews/revision2-integration-review.md),
[current validation](../../../docs/revision2-integration-validation.md), and
[snapshot publication contract](../../frost-collections-concurrent/docs/snapshots.md) govern implemented behavior.
This document retains the stronger ownership, indexing, and lifecycle proposals
from the [integer roadmap](integer-roadmap.md). The [supplied research](../../../docs/reviews/supplied/lazy-views-research.md)
is preserved separately from the decisions here. The companion
[view design](integer-views.md) and
[adversarial review](../../../docs/reviews/lazy-views-adversarial-review.md) address query
evaluation and the broader research claims.

## 1. Decision and purpose

Revision 2 supplies a concrete first sharing mechanism inside `FrozenIntSet<K>`.
All root clones share storage, and `IntSetPolicy::chunked()` enables sharing of
untouched 2^16-code container payloads. This supersedes the earlier assumption
that `FrozenIntSet::clone()` always copied storage. The public profile still has
eager metadata, eager algebra, retained Dense/Roaring limits, and ordinary
family-independent integer codecs.

A possible future `PersistentIntSet<K>` must justify a stronger contract than
that mechanism: for example, a persistent directory with changed-path copying,
explicit insertion/removal APIs, or chunk/payload limits. Those APIs and limits
are not implemented. “Persistent” here means old values remain available after
deriving new values; it does not mean durable storage or a serialization format.

| Capability | Implemented now | Still proposed |
| --- | --- | --- |
| Root clone and identity reuse | Shared `FrozenIntSet` root; policy-compatible reuse | No additional family is required for this alone |
| Unchanged payload sharing | Explicit Chunked Array/Bitmap/Full containers | Run containers, alternate chunk shapes, or broader owned imports |
| Directory updates | Flat sorted directory copied for changed results | Persistent tree/radix paths with subtree reuse |
| Limits | Existing Dense-byte/Roaring-partition limits | Dedicated reachable-chunk/payload bounds |
| Query evaluation | Eager integer operations | Borrowed lazy expressions and evaluation contexts |

Repeated derivation with retained readers remains the motivating workload.
Compare future tree designs with the implemented flat Chunked candidate, the
compact Dense update path, and native Roaring. Payload sharing alone does not
remove directory copying, reference updates, retained-version memory, or release
costs. The earlier whole-payload implementation is no longer the only baseline.

The initial public profile should be structurally immutable. Construction,
insertion, removal, and algebra produce new values; they never modify a value
visible through another handle. A separate transient builder can be considered
after the immutable contract is measured. A duplicate-permitting staging buffer
would be a builder, not a mutable set family.

Lazy query evaluation is a separate choice. A borrowed expression can defer
iteration or materialization without sharing any materialized storage.
Persistent chunks can share storage while their bounds and cardinalities remain
fully computed and valid. Do not introduce a public “needs repair” state to
obtain either benefit.

## 2. What the upstream evidence actually establishes

These reports concern their respective implementations. They do not establish a
defect in Frost's Rust `roaring` dependency. They provide concrete failure modes
for ownership boundaries, metadata, error paths, and test selection.

| Primary source and checked status | Reported issue or proposal | Consequence for this design |
| --- | --- | --- |
| [CRoaring #927](https://github.com/RoaringBitmap/CRoaring/issues/927), closed; [fix #928](https://github.com/RoaringBitmap/CRoaring/pull/928), merged 2026-10-31 | Frozen sizing/serialization mishandled shared-container tags. The fix unwraps shared containers and writes their underlying representation tags. | Sharing metadata is not a wire format. Exercise every owned/shared representation during serialization, if serialization is added. |
| [CRoaring #931](https://github.com/RoaringBitmap/CRoaring/issues/931), closed completed 2026-10-31 | Allocation failure could leave a failed shared wrapper in source/destination state or leave freed destination containers reachable through stale metadata. | Build replacements privately and commit only after checks succeed. Specify which failures return errors and which follow the allocator's normal behavior. |
| [CRoaring #946](https://github.com/RoaringBitmap/CRoaring/pull/946), merged 2026-11-01 | Added ownership and boundary coverage, rejected frozen views with more than 65,536 containers, and corrected an AND-NOT test that had invoked XOR. | A passing test name is not evidence that the intended operation ran. Test dispatch, malformed counts, and ownership invariants directly. |
| [Java Roaring #874](https://github.com/RoaringBitmap/RoaringBitmap/pull/874), merged 2026-10-19 | Position-indexed sharing flags became detached from moved containers, and missing mutator overrides could write into a source bitmap. | Keep sharing ownership attached to private objects. Do not inherit an extensible mutable API or maintain separate positional ownership flags. |
| [Java Roaring #885](https://github.com/RoaringBitmap/RoaringBitmap/pull/885), **open and unmerged** when checked | Proposes a separate persistent bitmap with shared containers, identity shortcuts, and explicit conversion boundaries. Its reported results include both gains and losses; one-off conversions can erase gains. | Treat it as a proposal and benchmark reference, not a released guarantee or a Rust speedup. Measure complete retained-version workflows and losing cases. |

The earlier review's [CRoaring public header](https://github.com/RoaringBitmap/CRoaring/blob/master/include/roaring/roaring.h)
also warns about mixing its copy-on-write and non-copy-on-write modes. A public
mode flag that changes ownership obligations is unsuitable for this Rust
profile. The safe API should make the legal ownership model the only available
one.

Rust supplies a useful ownership mechanism, not the entire collection proof.
`Arc::clone` shares an allocation; `Arc::make_mut` clones its value when another
strong owner exists, and can dissociate weak references when no other strong
owner exists. `Arc::ptr_eq` tests allocation identity, not logical equality.
These are [standard-library contracts](https://doc.rust-lang.org/std/sync/struct.Arc.html),
not evidence that cached counts, prefix coordinates, budgets, or serialization
are correct. A safe implementation can still return the wrong set.

## 3. Public boundary and additive features

The proposed API shape is deliberately separate:

```text
PersistentIntSet<K: IntKey>
    try_from_iter(values, limits)
    contains(&K), cardinality(), iter(), encoded_values()
    try_with_inserted(K), try_with_removed(&K)
    try_union(&Self), try_intersection(&Self)
    try_difference(&Self), try_symmetric_difference(&Self)
    try_to_frozen(IntSetPolicy)
```

These are design names, not existing methods or a committed compatibility
surface. Fallibility covers validation, arithmetic, and stated policy limits.
It does not promise recovery from every allocation failure. Named result-policy
overrides should exist if callers need limits different from those retained by
the inputs. Avoid an infallible convenience method that silently discards a
limit or turns a routine policy rejection into an undocumented panic.

Retain the existing safe `IntKey` contract, canonical `K::Word`, checked imports,
and by-value iteration. Binary operations require the same logical K. A shared
u64 word type does not permit mixing `UserId` with `OrderId`. A phantom marker
must retain K's relevant lifetime and auto-trait restrictions. No manual
`Send`/`Sync` implementation is needed.

If a later separate family uses a `persistent-int-set` feature, enabling that
feature must not change the existing `FrozenIntSet` candidate set, clone contract,
freeze/thaw pairings, or hasher behavior. The current revision's Chunked candidate
is already part of the complete fixed `int-set` profile, with explicit selection;
it is not enabled by that hypothetical future feature. Conversion between any
future family and `FrozenIntSet` is explicit and is charged in benchmarks.
Whether a consuming conversion can reuse an allocation is an optimization,
never a blanket zero-copy promise.

The implemented `FrozenIntSet` uses a documented target-dependent Shared alias:
Arc where pointer-width atomics exist and Rc otherwise. Its clone cost and auto
traits follow the selected pointer; feature unification never switches that
choice. This preserves the existing allocating profile on non-atomic targets.
A future separately named family that explicitly promises concurrent sharing may
instead require [`target_has_atomic = "ptr"`](https://doc.rust-lang.org/reference/conditional-compilation.html#target_has_atomic).
That would be a new family contract, not a silent limitation on `FrozenIntSet`.

## 4. Proposed directory representation and invariants

The skeleton below describes a future directory-backed profile, including
possible Run containers and subtree metadata. Current Chunked storage has
Array/Bitmap/Full containers, eager root metadata, and a flat directory; it has
no Run variant or tree nodes. Preserve immutable published objects and fixed
encoded coordinates in either design. The proposed shape is:

```text
encoded word = (prefix << 16) | low

u32 keys: prefix fits u16; low is u16
u64 keys: prefix fits 48 bits stored in checked u64; low is u16

Version = Arc<Root>
Root = directory + cached cardinality + cached bounds
Directory entry = prefix + Arc<Chunk>
Chunk = cached u32 cardinality + private Array / Bits / Runs payload
```

Rust has no built-in `u48`; a private checked wrapper can express that invariant.
Public encodings remain u32/u64. A 16-bit split is storage organization, not a
new codec or a type parameter exposed to applications.

Candidate chunks are sorted unique u16 arrays, a bitmap of exactly 1,024 u64
words, and canonical inclusive u16 run endpoints. All constructors and mutation
kernels are private. No public backend accessor, `DerefMut`, mutable container
trait object, or raw shared-buffer import may bypass the invariants.

| Invariant | Adversarial boundary |
| --- | --- |
| Prefixes are strictly increasing and unique within traversal order. Tree child ranges do not overlap. | Insertion/removal at a child split or merge; sparse u64 prefixes. |
| A chunk contains only lows belonging to its directory prefix. Empty chunks are omitted. | Reusing the same low-bit payload at different prefixes. |
| Arrays are sorted and unique; bitmaps have exactly the prescribed extent; runs are ordered, disjoint, and nonadjacent. | Duplicates, all-zero bitmap, adjacent runs crossing a word boundary. |
| Chunk cardinality equals the represented member count and fits u32. | A full chunk has **65,536** members, which does not fit u16. |
| Inclusive run lengths and adjacency checks use wider checked arithmetic. | A run ending at 65,535 must not wrap when adding one. |
| Root/subtree cardinalities and bounds agree with their descendants. | Removing the only member, removing extrema, and merging complete chunks. |
| Logical codes satisfy the key codec; safe trait violations never authorize unchecked access. | Nonzero keys, sparse enums, and a decoder whose behavior changes. |

Use u128 for inclusive spans and count intermediates. A collection containing
the entire u64 universe has 2^64 members and exceeds the current u64 cardinality
contract, even though every individual chunk is valid. Reject true count
overflow explicitly; neither count wrapping nor a truncated `usize` is a
reasonable fallback. Fallible address-sized conversions remain separate from
wide cardinality queries.

Compressing consecutive existing codes into a run does not invent values.
Accepting an arbitrary numeric interval does: a restricted enum may have gaps.
The initial profile should offer no range-fill or implicit-complement operation
that bypasses checked key validity.

Owning the chunk layer also does not automatically provide a fast seek. An
array, bitmap, and run list need different seek kernels; directory navigation
and empty-word scans still cost work. Define exact monotone cursor semantics
before sharing a cursor implementation with borrowed views, and compare it
against the current small-side membership-probing path. A 16-bit chunk bounds
the local search region; it does not establish a measured throughput result.

This layout is not automatically economical. Scattered u64 values can require
one chunk and one index entry per member. An Arc per tiny chunk can overwhelm
its payload. Benchmark packing several small chunks into one immutable leaf,
or a profile with a compact leaf representation, before claiming broad memory
benefits. Such alternatives must preserve a fixed profile and the same ownership
contract. Native `RoaringBitmap` internals are not public chunk handles; wrapping
one native bitmap per chunk is a prototype with its own nested-index overhead,
not proof that a custom container implementation is unnecessary or free.

## 5. The directory determines update complexity

Let C be the number of occupied chunks, b the bytes in a changed chunk, B a
fixed tree fanout, and h the tree height. The following costs assume another
version still retains the source; unique ownership can sometimes avoid copying.

| Directory | Clone a version handle | Change one existing chunk | Main cost to measure |
| --- | --- | --- | --- |
| `Arc<Root>` with a flat immutable entry array | O(1) root reference increment | O(C) entries/reference increments plus O(b) payload work | Directory copying dominates as C grows. |
| `Arc<Root>` containing an ordinary `BTreeMap` | O(1) root reference increment | Cloning a shared whole map is O(C), plus chunk work | Wrapping a mutable tree in Arc does not make it a persistent tree. |
| Actual persistent B+ tree with shared immutable nodes | O(1) root reference increment | O(Bh) copied node entries/references plus O(b) chunk work; O(log C) for fixed B | Path copying, cache misses, node allocation, rebalancing. |

`Arc::make_mut(&mut root)` cannot make a flat directory's clone logarithmic.
Its inner value must still be cloned according to that value's implementation.
Likewise, a tree that copies every node in its `Clone` implementation does not
acquire path copying merely by being placed in an Arc.

The [imbl 7.0.2 ordered map](https://docs.rs/imbl/latest/imbl/struct.GenericOrdMap.html)
documents an immutable B+ tree with O(1) cloning and O(log n) insertion. It is a
useful prototype and benchmark control for a persistent directory. Its
[implementation](https://docs.rs/imbl/latest/src/imbl/ord/map.rs.html) uses shared
nodes and imports `std`; using it successfully does not establish an alloc-only
production dependency. Keep this experimental dependency out of public type
parameters and verify the exact pointer kind and features in the measured build.

The proposed production direction is a persistent directory if the measured
workload has enough chunks for O(C) copying to matter. Keep the flat directory
as a simpler control. Do not publish an O(log C) update claim for a flat first
implementation or commit a custom tree before the benchmark demonstrates why
the added code is justified.

Bulk algebra has different complexity. A straightforward merge still visits
O(Ca + Cb) directory entries, even when every pass-through payload is shared.
A flat result requires an entry and reference update for each output chunk.
A persistent result may reuse whole subtrees where their ranges permit it, but
arbitrary overlapping trees still need alignment and payload work. Repeated
single-key tree insertion can add an avoidable logarithmic factor; compare bulk
builders or tree joins. Sharing does not make arbitrary union O(log C).

## 6. Ownership, identity, and transactional derivation

Published roots, index nodes, and chunk payloads have no mutable structural
access. Initial metadata is eager; no mutable “lent” flag or positional
copy-required table is needed. A new version assembles private replacements and
shares untouched objects. `Arc::make_mut` is permissible on locally owned
replacement paths when its normal contract suffices. Do not infer exclusive
access from a sampled `strong_count`, implement custom reference counting, or
reinterpret a shared reference as mutable.

For equal prefix coordinates and compatible encoding, identical chunk
allocations allow these shortcuts:

| Operation on the same immutable chunk | Result |
| --- | --- |
| Union or intersection | Reuse the chunk. |
| Difference or symmetric difference | Omit the chunk. |

Identity is a sufficient shortcut, not the definition of equality. Different
allocations can contain identical values. A shared low-bit payload at two
different prefixes represents different global members. Subtree reuse must
also preserve its encoded range, logical K, and result limits. Pointer identity
must never appear in a semantic hash.

Logical equality, ordered iteration, and hashing are independent of directory
shape, chunk representation, allocation identity, insertion history, and
limits. If hashes are defined for the new type, equal values must produce the
same Hasher event stream across all its representations. Sharing an algorithm
with `FrozenIntSet` is useful but does not make hash bytes a persistent format.

Prepare changed chunks and index paths before making a new root observable.
Check arithmetic, metadata, and limits before returning it or publishing it.
A returned error leaves every earlier version unchanged. A panic during
derivation unwinds private temporary state; no partial candidate is published.
Allocator termination remains possible. Stable allocation paths must not be
advertised as universally fallible just because the public method starts with
`try_`.

`SnapshotStore` remains a separate publication layer. Its conditional update
compares the allocation identity of the expected published value, not semantic
equality or the identity of an internal shared tree root. A no-op edit may reuse
an internal root while two outer publications remain distinct. Keep the
[existing linearization and retry contract](../../frost-collections-concurrent/docs/snapshots.md): callbacks may repeat,
failed candidates remain unpublished, and readers retain their chosen version.
Persistence can reduce derivation work but cannot make a retrying callback
exactly-once or guarantee fairness.

## 7. Budgets, retained versions, and release costs

Introduce limits whose meaning can be tested. A minimal proposed
`PersistentIntSetLimits` can bound occupied chunks and the sum of actual chunk
payload-buffer bytes reachable from one result. Count a logically reachable
payload even when an older version already owns the same allocation. Limits
must not become easier to bypass merely because a value is shared.

This is a per-result payload contract. It excludes index/reference headers,
allocator overhead, input staging, temporary candidates, and other retained
versions. Name and document that exclusion. A deterministic planning score must
not be described as measured resident bytes. A future work budget would need
its own accounting and error semantics; it is not implied by a result limit.

Use the same default inheritance rule as the current integer profile:
componentwise minimum for commutative binary operations, left-side limits for
difference, and explicit overrides for a deliberate policy change. A no-op,
self-operation, or empty-input shortcut must still check the result's limits.
If the persistent representation cannot satisfy them, return a policy error;
do not silently escape into a different public ownership profile. A separate
explicit conversion to `FrozenIntSet` can choose its own policy.

Let V historical versions be retained. Small edits can approach base storage
plus V times the changed chunk and copied path. If every edit changes every
chunk, retention can approach V complete payloads. Fragmentation, tree nodes,
reference headers, and failed concurrent candidates add cost. A per-root limit
cannot bound total memory while callers retain an arbitrary version graph.

Reference counting has work too. An Arc clone performs an atomic increment;
the standard implementation describes the ordering and overflow handling in
the [Rustonomicon clone discussion](https://doc.rust-lang.org/nomicon/arc-mutex/arc-clone.html).
Sharing thousands of chunks can require thousands of reference updates even
when their data bytes are never copied. Reusing a whole subtree can reduce that
work; it does not make all reference traffic disappear.

Dropping one handle is cheap when another owner keeps its root alive. Dropping
the final owner can release an entire newly unreachable tree and many chunk
allocations. The underlying [Arc drop protocol](https://doc.rust-lang.org/nomicon/arc-mutex/arc-drop.html)
does not provide constant-time destruction for its payload. In this design,
that work can occur on the last reader's thread, not necessarily the writer.
Use an acyclic downward ownership graph, without strong parent links. Deferred
reclamation, if later needed, is another explicit subsystem with queue capacity,
memory, and shutdown costs; it is not part of the initial profile.

Measure clone, edit, failed retry, retained-memory growth, and final release as
separate phases and as a complete lifecycle. No wait-free, bounded-latency, or
process-wide memory-bound claim follows from immutable chunks.

## 8. Validation and serialization boundary

The first profile should have checked logical construction and explicit
conversion, without a public zero-copy or serialized-layout promise.
Structural sharing is an in-memory graph property. Arc addresses, reference
counts, and sharing tags do not belong in a portable representation.

If serialization is later added, define a versioned format independently.
Validate prefix order, uniqueness, chunk extent, representation tags, run
endpoints, payload cardinalities, subtree counts and bounds, and codec validity.
Check lengths, offsets, and budget arithmetic before allocation. Bound the
number of chunks and any encoded hierarchy depth. Reject an empty chunk where
the canonical directory forbids it. Deriving deserialization for private fields
must not create an unchecked path around constructors.

Imported codes for restricted domains require their normal checked decoding.
An encoded array or bitmap can be memory-safe and still contain an invalid
application value. A checksum does not establish these semantic invariants.
Internally constructed chunks may reuse established invariants, but that trust
must come from sealed constructors, not the origin of a byte buffer.

## 9. Verification required before a supported release

Use a version-graph test harness, not just a mutable-set oracle for the latest
root. Keep many earlier roots while branching, cloning, inserting, removing,
and applying all four operations. After every step, compare **every retained
version** with its own `BTreeSet` oracle. Include equal contents reached through
different histories and allocations.

The required adversarial cases are:

- All nine chunk-representation pairs for every binary operation, plus skewed,
  disjoint, empty, identical-pointer, and equal-but-distinct inputs.
- u32/u64 widths, signed extremes and zero crossings, restricted codecs, full
  65,536-member chunks, terminal run endpoints, prefix transitions, u64::MAX,
  and a checked arithmetic harness for otherwise impractically large counts.
- Insertion/removal at tree splits and merges, eviction of the last chunk,
  sparse one-member prefixes, and reuse of identical lows at different prefixes.
- Tight budgets on self/empty/no-op paths; deterministic rejected candidates;
  unchanged old roots after error or panic; and independent lifetime/drop counts.
- Equal-value Hash event streams across shape and ownership histories, without
  relying only on a final numeric hash that could collide.
- Scoped threads and barriers that retain old roots during conflicting
  `SnapshotStore` updates, including a forced retry and release after the store
  itself is dropped.

Use Miri for executed aliasing, lifetime, and boundary paths. A reduced scheduler
model should exercise the production retry helper and state exactly which
publication primitive it assumes. Neither a bounded model nor Miri proves all
dependency internals, all allocators, or unbounded executions. No custom atomic
reference-count algorithm should be introduced just to create something to model.

Add focused mutation checks: corrupt a cached count, forget one path update,
misalign a reused prefix, bypass a result limit, or accidentally call XOR from a
difference case, and show that the relevant test detects it. Test the immutable
boundary itself; tests should not depend on an unsafe mutation mechanism that
the intended API never exposes. Ordinary budget rejection can be injected
deterministically without pretending to recover from global allocator aborts.

## 10. Benchmark and admission gates

Measure the complete workflow with the current optimized `FrozenIntSet` in its
Sorted/Dense/Roaring and explicit Chunked representations, any application-level
`Arc<FrozenIntSet>`, and a genuinely persistent
directory control. Include native Roaring and imbl where their contracts match
the case. Distinguish prepared shared versions from one-off conversion into and
out of the profile. The upstream Java proposal explicitly reports mixed
outcomes; its JVM measurements do not predict Rust allocation or atomic costs.

| Dimension | Required cases and accounting |
| --- | --- |
| Shape | Dense local windows, long runs, mixed container densities, scattered u64 prefixes, tiny sets. |
| Edit footprint | One member, one chunk, a small batch, every chunk, full replacement. Vary C separately from cardinality. |
| Sharing | Vary actual shared-object identity and equal key overlap independently. Equal values do not imply shared allocations. |
| Version lifetime | No old reader, one retained version, multiple branches and long-lived historical readers. Include final release. |
| Algebra | All operations, balanced and skewed inputs, disjoint and heavily overlapping prefixes; output limits included. |
| Concurrency | Reader count, contending writers, retries, rejected-candidate destruction, and externally serialized writers as a control. |
| Measurements | Build/conversion time, allocated bytes/count, peak resident memory, throughput, latency distribution, and clone/drop work. |
| Environment | Identical compiler/target flags, serial competing runs, explicit CPU dispatch, and the same construction/destruction timing boundaries. |

A pointer-sharing hit rate is a diagnostic, not a speed result. Report slow
cases and minimum useful reuse. Root-clone timing alone cannot justify the
profile. Allocation counts do not substitute for resident-memory measurements,
and an RSS high-water mark is not an exact graph-size measurement.

Proceed in stages: first establish invariants with a safe private prototype;
then compare flat and persistent directories; then evaluate retained-version
workflows and verification evidence. Publish a supported separate profile only
if those results demonstrate a useful workload benefit that includes conversion,
retention, and release costs, and if its extra implementation surface is
maintainable. Otherwise retain the experiment and the existing compact profile.

This document records the remaining tree-directory and separate-family design
and its release gates. Implemented flat sharing is described at the top and in
the integer profile; no claim is made that the proposed tree, future family, or
its complete verification and measurement program has been executed.

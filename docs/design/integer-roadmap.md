# Integer collection roadmap and admission criteria

## Status and design boundary

This document evaluates structures proposed after the first `FrozenIntSet`
implementation. Additional family names below remain provisional. The integrated
profile now has fixed Sorted, Dense, Roaring, and explicit Chunked candidates,
with shared immutable roots and shared Chunked payloads. Flat chunk-directory
updates still copy directory metadata; persistent tree paths and a separate
`PersistentIntSet` are not implemented. Generic and integer lazy views remain
proposals. See the [integration review](../reviews/revision2-integration-review.md),
[current validation](../revision2-integration-validation.md), and
[integer design](integer-sets.md) for the current contracts. Earlier research and
benchmark references are historical inputs, not merged-source measurements.

Adding structures should solve a measured application problem. Improving one
membership microbenchmark is not enough if construction, iteration, output
materialization, peak memory, or snapshot retention becomes unacceptable. The
semantic family, ownership model, representation policy, and serialization
format must each be specified before choosing a public type name.

The invariant common to integer profiles remains: `K` chooses a stable canonical
word encoding, storage can change privately, and algebra selects existing valid
codes. A safe `IntKey` implementation never licenses unchecked memory access.
Per-collection dictionaries or base offsets do not become global key encodings.

## 1. Flat immutable Roaring storage

### What it could improve

A flat index followed by immutable container payloads could reduce allocation
count, pointer chasing, and loading overhead. An owned blob could support cheap
shared ownership through `Arc`; a separately validated borrowed view could avoid
copying an already compatible serialized payload. These are distinct benefits.
Putting an ordinary tree of bitmaps inside an `Arc` gives cheap clones, but does
not turn that tree into a single-allocation structure.

The [CRoaring public header](https://raw.githubusercontent.com/RoaringBitmap/CRoaring/master/include/roaring/roaring.h)
provides a useful reference: its frozen representation resembles native C
layout, requires a 32-byte-aligned buffer for `roaring_bitmap_frozen_view`, and
keeps the buffer alive and unchanged. That frozen format is endian-sensitive
and may change; it is distinct from the portable Roaring serialization format.
This is evidence for the design pattern, not a ready Rust-safe storage contract.

### Required contract

Choose deliberately between an internal allocation layout and a public portable
format. For a public format, specify a version, byte order, integer widths,
container tags, cardinality rules, alignment strategy, and checked offset/length
arithmetic. Reject overlapping or out-of-bounds regions, malformed run endpoints,
unsorted keys, duplicate containers, invalid reserved fields, and impossible
cardinalities. A checksum can detect accidental corruption; it cannot replace
structural validation.

An owned form and `FrozenIntSetView<'a, K>` would have different lifetime and
validation obligations. A view must not outlive its backing bytes; all decoded
members must satisfy the codec contract. An arbitrary mapped file is not made
immutable merely by constructing a read-only Rust view if another agent can
change its contents. The API must define the backing-store trust and mutation
conditions or copy into owned immutable storage.

Do not transmute serialized bytes into Rust dependency structs. Their layouts,
ownership, and internal invariants are not a serialization API. Safe checked
byte access is a viable first implementation. Any subsequent unsafe aligned
views need a written invariant, isolated code boundary, and tests against
misalignment and corrupted input. A C-backed implementation should be an
explicit profile, so a dependency feature cannot silently change an existing
Rust profile's storage alternatives.

An arena does not solve small-update persistence. Producing a different logical
set normally means allocating another arena and copying unaffected payloads.
The current explicit Chunked candidate provides payload sharing through a
separate ownership strategy; the proposed flat arena would need its own answer.

### Admission measurements

Measure actual allocation count and peak bytes, cold and warm membership,
iteration, load/validation time, algebra materialization, and clone/drop cost.
Compare against the corrected native Roaring path and a simple `Arc` wrapper.
Exercise little- and big-endian readers or a byte-order-independent parser,
unaligned buffers, truncated files, maximum offsets, and adversarial headers.
Fuzz the parser before treating a borrowed format as production input.

## 2. A rank-indexed integer map

### Proposed shape

`FrozenIntMap<K, V>` could store an integer set as its key index and a dense array
of values in key order. This is useful when key compression is substantial and
lookup can cheaply map a member to its ordinal. The values are actual stored V
objects, so `get` can borrow `&V` while keys are still reconstructed by value.

Rank needs an explicit convention. With inclusive rank, lookup first establishes
membership and then uses `rank(code) - 1`; rank of a nonmember alone is not a
successful lookup. Sorted storage already obtains the ordinal from binary
search. Dense storage needs prefix population counts plus a partial-word count.
Roaring can supply rank operations, but a wrapper must account for prefix work:
the pinned `RoaringTreemap::rank` traverses earlier child bitmaps rather than
providing a cached global prefix sum. A separate immutable prefix index may be
needed for many partitions.

### Semantic decisions

Construction must define duplicate-key handling and preserve the chosen value.
Sorting just encoded keys and losing their association with values is wrong.
If application equality ignores fields, the key codec still has the full-value
round-trip obligation of `IntKey`; this map is not permission to discard key
metadata silently. Applications needing stored key representatives should use
the ordinary map family.

The value array imposes an address-sized allocation limit even when the key set
uses u64 cardinalities. Check conversions before allocating or indexing.
`V: Clone` should not be needed for read-only lookup; consuming construction and
operations can move values. Borrowing operations that materialize a map may
require cloning or a documented sharing strategy.

Set algebra cannot define map conflicts on its own. Union needs an explicit
choice for a key present in both maps: preserve left, preserve right, reject, or
invoke a resolver. Intersection must also specify whose value survives. A
numeric range cannot invent V values or valid restricted keys.

### Admission measurements

Compare against sorted key/value arrays and the existing frozen ordered/hash
maps. Measure hit and miss lookup, build time, iteration, key density, V size,
number of Roaring partitions, and the prefix-index overhead. Differential tests
must cover duplicates, values with drop counters, rank at missing keys, empty
maps, and `u64::MAX`. This is a plausible next family once rank has a measured
use case; adding a public rank API just for speculative future structures is
not necessary.

## 3. A range set or range-aware profile

### Opportunity and representation

A collection with r long consecutive runs can store run endpoints rather than
one element per key or one bit per position. Canonical ranges should be sorted,
disjoint, and nonadjacent. Adjacent ranges must merge, including across what
would otherwise be bitmap or upper-32-bit boundaries. Membership can binary
search ranges; algebra can merge their endpoints in time proportional to the
number of input ranges plus output fragmentation.

For a full u64 key domain, half-open endpoints require an extra endpoint value
for `u64::MAX + 1`. Use u128 endpoints or inclusive u64 endpoints with checked
adjacency arithmetic. A full-domain range has 2^64 members, exceeding the current
u64 cardinality contract. Either reject it explicitly or introduce a separately
designed wider-count family; do not overflow or silently truncate counts.

Compressing existing consecutive valid codes is safe for restricted key types.
Accepting arbitrary numeric intervals is a different constructor: a sparse enum
with codes `{1, 4, 7}` cannot accept `[1, 7]` as three members, nor decode every
position in that interval. A range-building API needs validated members or a
stronger explicit domain capability. The ordinary set operations preserve
validity when they only select existing codes.

### Profile and workload consequences

`FrozenRangeSet<K>` could be an explicit family, or a range-aware integer profile
could have a fixed four-candidate set from its introduction. Adding a feature
that adds a fifth candidate to the already-available four-candidate
`FrozenIntSet<K>` would violate the current profile rule. A versioned design change or a
separate profile should make that decision visible.

Alternating members have one range per member and can make endpoints less
compact than sorted words. Difference can split one long run into many small
runs. A run representation therefore needs output-aware planning as much as the
current bitmap profile. Rank/select would need prefix cardinalities; a simple
run count alone is not an index.

### Admission measurements

Include long runs, separated runs, alternating bits, random gaps, and repeated
subtract-small-set updates that progressively fragment a range. Compare total
heap storage and operations against optimized Roaring run containers, not an
unoptimized array container. Test endpoint arithmetic at zero, signed encoding
boundaries, and the unsigned maximum; model interval splitting and coalescing
independently on a reduced universe.

## 4. Fence indexes and minimal perfect hashing for scattered keys

### Fence index

A fence index stores sampled boundaries for blocks of a sorted array. It can
narrow the final search and improve locality for large scattered u64 sets while
retaining natural ordered iteration and merge algebra. Its extra metadata,
block width, and layout should remain private unless measurements establish a
useful stable tuning parameter. Ordinary binary search on a contiguous array is
the control; a second search level can make smaller sets slower.

The index must describe the actual sorted slice and use checked offsets.
Rebuilding after materialized algebra costs time and memory. It is reasonable
to omit the index for small results and avoid constructing it for count-only
operations. A fence cannot fix a workload dominated by linear intersection
rather than membership.

### Minimal perfect hash function

An MPHF maps known distinct keys to unique slots. It is not an exact membership
test for arbitrary queries. The
[BBHash authors' documentation](https://github.com/rizkg/BBHash)
explicitly describes nonmembers returning another key's index. Exact Frost
membership would have to verify the full key at that slot; a short fingerprint
alone creates an approximate filter with a different public contract.

Preserving ordered iteration typically also requires sorted keys and an
ordinal/permutation relationship. An MPHF over already stored 64-bit keys adds
index space; it does not remove the need for exact keys. A map may benefit by
indexing values, but must still resolve misses correctly. Hash construction
seeds, collision handling, retry bounds, reproducibility, and adversarial build
inputs need explicit policies.

### Admission measurements

Compare fence indexes, branch-aware binary search, the existing frozen hash
family, and an exact MPHF-backed prototype. Include misses, skewed queries,
working sets larger than caches, build cost, peak memory, and algebra/rebuild
cost. For an MPHF, test arbitrary nonmembers rather than only the inserted keys.
Keep an approximate filter separate if that is the actual application need.

## 5. Elias–Fano storage

Elias–Fano represents increasing integer sequences by splitting high and low
bits and indexing a monotone high-bit sequence. The
[Quasi-Succinct Indices paper](https://arxiv.org/abs/1206.4300)
is a primary algorithm reference. Its usefulness here is a hypothesis about
ordered sparse data, not an observed Frost speedup.

For n members in a universe of size U, the familiar space scale is
`n * log2(U / n) + O(n)` bits before all auxiliary indexes and allocation
overheads. A local base can shrink U for clustered large-magnitude keys. Empty
inputs, one member, dense inputs, and full-width spans need explicit arithmetic
handling. Compute span and bit positions without overflowing at `u64::MAX`.

Efficient select, rank, predecessor, and membership need auxiliary indexing.
Membership can find a predecessor and compare it, but a select benchmark alone
does not establish lookup throughput. Algebra usually merges or filters encoded
members and builds another compressed sequence; it does not gain bitmap Boolean
operations automatically. Construction staging can also dominate peak memory.

Use only monotone valid codes and preserve the existing checked codec boundary.
A borrowed or serialized Elias–Fano representation needs its own checked-format
contract, including bit lengths and index consistency. A dependency's succinct
data-structure label does not establish its MSRV, no_std support, safety, or
arithmetic behavior.

Admission requires comparisons against sorted words, fenced sorted words,
Roaring, and range storage on representative sparse ordered workloads. Measure
lookup, select/rank if actually needed, iteration, algebra, build/rebuild time,
and peak memory. Exhaustive small-universe round trips and differential tests
should cover all lower-bit widths before accepting performance claims.

## 6. Persistent chunks for frequent snapshot updates

The [persistent-storage proposal](persistent-integer-storage.md) expands this
section with concrete ownership and directory invariants, verified upstream
reports, and lifecycle admission gates. Borrowed query evaluation is separate;
see the [integer expression design](integer-views.md) and
[adversarial research review](../reviews/lazy-views-adversarial-review.md).

### Workload fit

For many readers and repeated small logical changes, the implemented
`FrozenIntSet` already shares immutable roots, and explicit Chunked storage shares
unaffected Array/Bitmap/Full payloads. A changed Chunked result copies its flat
directory and increments references to reused containers. With C containers,
that directory work remains O(C), even when only one payload changes.

A future persistent directory could copy only changed tree/radix paths and reuse
whole subtrees. A separately named `PersistentIntSet<K>` needs a useful ownership,
update, or limit contract beyond the sharing now available in `FrozenIntSet`.
It would add nodes, indirection, and implementation complexity. The number of
sparse u64 prefixes is an important adversarial workload for both designs.
Wrapping a whole set in another Arc does not remove these derivation costs.

### Required invariants

Every published node is immutable, and its cached cardinality/bounds match the
referenced children. An update must not mutate a chunk retained by any old
snapshot. Reusing a chunk is valid only when the encoded coordinate system and
logical key domain are compatible. Union/intersection may share whole matching
chunks but still require conflict-safe updates to overlapping chunks.

The current `SnapshotStore` publication protocol remains a separate layer. A
retrying update callback may execute more than once after contention; its work
and side effects must obey that API. A persistent collection can make retries
cheaper, but cannot eliminate publication races or promise that a callback runs
exactly once.

Memory retention depends on readers. An old snapshot may keep many obsolete
chunks alive indefinitely. A per-result representation limit cannot bound
process memory while an arbitrary number of historical snapshots is retained.
Expose useful diagnostics and document the ownership cost instead of claiming a
universal memory cap.

### Admission measurements and verification

Measure update sizes, batch sizes, directory/chunk sizes, reader counts, write
contention, retained versions, clone/drop costs, allocation count, and peak
memory. Compare the current flat Chunked candidate, corrected Dense small-update
path, and native Roaring before adding a tree. If updates are rare or replace most members, sharing may lose.

Test old-version immutability, disjoint and conflicting updates, retry behavior,
correct reclamation/drop counts, and logical linearization at publication.
Reduced Loom-style models should describe precisely which publication and
reference-management behaviors they model. They do not prove the entire native
ArcSwap dependency or every allocator/runtime interaction. Exercise the actual
publication implementation under threads as a separate validation layer.

## 7. Static and borrowed storage

Static generated sorted arrays or checked immutable blobs can reduce startup
allocation for applications shipping fixed datasets. Borrowed storage should
have an explicit lifetime, for example `FrozenIntSetView<'a, K>`, and should
implement only capabilities supported by its ownership and validation model.
The name is provisional; an existing borrowed family may be a better fit after
an API review.

The lifetime of the source, its immutability, sortedness, uniqueness, and codec
validity are part of construction. Generated metadata must be verified against
the payload; a static array is not automatically a valid integer set. Retain the
logical K relationship through the correct phantom ownership marker. Returning
keys by value remains appropriate for encoded storage; a borrowed buffer of
words does not contain addressable K objects.

Borrowed inputs do not imply borrowed materialized outputs. An intersection
iterator may borrow both inputs, while a union stored as a complete independent
set generally needs owned storage. Conversion to an owned profile should be
explicit. Validation time must be included when advertising startup speed.

Measure deployment binary size, load/validation time, heap allocation, random
lookup, iteration, and algebra. Verify compile-time generation reproducibility,
lifetime rejection examples, corrupted external blobs, alignment, and byte order
as applicable. If startup allocation is the primary workload problem, this may
deserve priority ahead of more complex indexes.

## 8. Dictionary-compressed sets

A dictionary can assign compact codes to strings or other repeated domain
values, allowing many collections to share one key table. This is a separate
family from `IntKey`: a code obtained from a particular dictionary is not a
stable canonical encoding of an arbitrary application key across all sets.

Operations on raw codes require identical dictionary mappings, not merely
matching Rust dictionary types. Store a reliable mapping identity/version or
verify compatibility; otherwise translate through keys or return a checked
incompatibility error. Pointer equality can establish shared identity for the
same allocation, but pointer inequality does not establish incompatible content.
A hash of dictionary contents alone is not an exact compatibility proof unless
its collision policy is explicitly part of a weaker contract.

Define order independently: numeric dictionary ID order may not equal lexical
or application key order. Logical equality and hashing across compatible or
translated dictionaries must follow the chosen public semantics. Duplicate
dictionary entries and keys whose equality ignores metadata need representative
rules just as ordinary maps and sets do.

A dictionary may physically store full keys and support borrowed key views;
the compressed code set alone cannot. Lifetimes and shared ownership must keep
the dictionary alive, and updates must not remap codes beneath frozen sets.
Retained dictionary versions can dominate snapshot memory even when each set is
small. Serialization needs both dictionary identity and checked code validity.

Admission requires an application with enough reuse to amortize dictionary
construction and retention. Compare total memory across all collections,
construction, translation, lookup, iteration, and set operations. Test distinct
dictionaries containing equal keys in different orders, version changes,
invalid codes, duplicate entries, and representative preservation.

## Priorities and decision gates

| Work | Application signal | Gate before adding public API |
| --- | --- | --- |
| Current output policy, equality/hash, sorted builders, alloc support | Compact collections inflate or exceed intended candidate limits after ordinary operations | Regressions, matched native controls, isolated feature builds, and update-chain measurements |
| Persistent directory / stronger sharing family | Flat Chunked directory copying dominates frequent small updates | Reduction in complete update/retention cost versus the implemented flat candidate, including reference traffic and final release |
| Rank-indexed map | Integer keys are a large share of frozen map memory | Whole-map memory and lookup win including rank metadata and miss handling |
| Static/borrowed profile | Fixed shipped data and startup allocation dominate | Checked load/validation contract and reproducible end-to-end startup improvement |
| Range profile | Long runs remain costly after native run optimization | Wins across range algebra plus bounded behavior under fragmentation |
| Fence index | Large scattered sets spend time in membership | Cache-aware lookup win that pays for metadata and rebuild cost |
| Exact MPHF index | Read-heavy static key sets justify substantial preprocessing | Exact miss verification, total-space accounting, and build/rebuild amortization |
| Flat arena | Allocation count, cache misses, or load cost dominates | Format/lifetime validation and a measured advantage over simple Arc sharing |
| Elias–Fano | Sparse monotone sequences need compact rank/select or ordered reads | Comparative results including auxiliary indexes and result construction |
| Dictionary family | Many sets reuse the same expensive noninteger keys | Explicit mapping identity, representative/order semantics, and whole-system memory benefit |

These priorities are conditional on the application. They do not constitute a
promise to add every candidate or a claim that one representation wins on all
data. Promote an experiment into a supported family only after its semantic
contract, measurements, adversarial tests, and maintenance costs are reviewable.

# Frozen integer profile

`FrozenIntSet<K>` is an explicit integer-domain profile. The key type chooses a
canonical unsigned encoding at compile time. Construction chooses one of a
fixed set of private representations from the observed codes. Algebra dispatches
on the operand representations and keeps its inner work on encoded values.

Enable `int-set` to obtain the complete profile with `no_std + alloc`. The
optional `std` feature forwards Roaring's std support and runtime CPU dispatch.
The native Roaring reexports and adapters remain available with either
`int-set` or `roaring-0_12`; the latter retains its existing std requirement.
Immutable storage uses `Arc` where pointer-width atomics exist and `Rc`
otherwise, so the allocating profile remains available on both target kinds.
Normal pointer auto traits apply; there are no manual Send/Sync implementations.
Enabling another feature does not add representations to this profile. The ordinary `MutableOrderedSet::freeze()`
pairing remains `FrozenOrderedSet`. No new mutable integer family or fictitious
mutable wrapper is introduced.

## Public type and encoding contracts

```rust
pub trait IntKey: Copy + Ord {
    type Word: IntWord;
    fn encode(self) -> Self::Word;
    fn decode(word: Self::Word) -> Option<Self>;
}
```

`IntWord` is sealed to u32 and u64. Its private storage supertrait associates
each width with its backend and iterator implementations. A narrow documented
`private_bounds` allowance marks this sealing boundary. Applications implement
the safe `IntKey` trait and select a primitive word; they do not implement
backend machinery or name private representation types.

Names follow the existing crate vocabulary: `FrozenIntSet` names the immutable
profile, `IntKey` names the logical codec, and `IntWord` names its supported
primitive storage width. `IntSetPolicy` chooses construction or an explicit
operation result; `IntSetLimits` records the retained Dense and Roaring limits.
`IntSetBackend` describes the requested candidate and `IntSetRepr` reports the
observed representation. Backend implementations remain private. These names
avoid exposing a second generic parameter or inventing a mutable family for an
input staging vector.

| Logical type | Word | Encoding |
| --- | --- | --- |
| u8 / u16 | u32 | Widen value; decoding checks the original width |
| i8 / i16 | u32 | Flip the sign bit at the original width, then widen; checked decoding |
| u32 | u32 | Identity |
| i32 | u32 | Same-width bit pattern XOR `1 << 31` |
| u64 | u64 | Identity |
| i64 | u64 | Same-width bit pattern XOR `1 << 63` |
| usize | u64 | Widen value; decoding checks the target's address width |
| isize | u64 | Sign-extend through i64 and flip bit 63; checked target-width decoding |
| char | u32 | Unicode scalar value; reject surrogate and out-of-range codes |
| NonZeroU8 / NonZeroU16 / NonZeroU32 | u32 | Extract value; checked width and nonzero decoding |
| NonZeroU64 | u64 | Extract value; checked nonzero decoding |
| Application newtype | Associated u32 or u64 | Application-defined canonical codec |

The signed mapping is a bijection that preserves numerical ordering. It maps
the signed minimum to zero, negative one immediately below the sign-bit boundary,
zero to that boundary, and the signed maximum to the unsigned maximum. It
changes ordering coordinates; it does not by itself compress anything.

An application can preserve its domain in signatures:

```rust
use frost::integer::{FrozenIntSet, IntKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct UserId(u64);

impl IntKey for UserId {
    type Word = u64;
    fn encode(self) -> u64 { self.0 }
    fn decode(word: u64) -> Option<Self> { Some(Self(word)) }
}

let users: FrozenIntSet<UserId> = [UserId(9), UserId(2)].into_iter().collect();
assert_eq!(users.iter().collect::<Vec<_>>(), [UserId(2), UserId(9)]);
```

Binary methods require the same K. UserId and OrderId are not interchangeable
because both use u64. Encoding uses value conversions, so `repr(transparent)`
is unnecessary. A `PhantomData<K>` preserves the key's lifetime and auto-trait
restrictions despite storing only unsigned words; there are no unsafe Send/Sync
implementations.

### Laws and checks

The codec must reconstruct the complete original value, preserve order, accept
only canonical codes, and remain stable across calls, threads, and collections.
A decoder that accepts a word must re-encode it to that exact word. Per-set
offsets belong in storage, never in the global codec.

Checked construction validates observed encode/decode round trips, canonicality,
and adjacent order after normalizing codes. This is not a proof of arbitrary
downstream code. In particular, equality that ignores metadata cannot detect
that encoding discarded that metadata. A codec can also behave differently for
unseen values or after global state changes. Use ordinary frozen sets to retain
stored representatives with additional data.

`IntKey` remains safe: violated laws may produce a returned error, a checked
panic, or wrong logical answers, but never justify unchecked memory access or
constructing invalid Rust values. User callback panics propagate. Iteration
uses checked decoding and can panic when a previously valid code stops decoding.

## Construction, membership, and capabilities

`try_from_iter(input, policy)` consumes keys, validates, sorts and deduplicates
their words, then selects storage. `try_from_encoded(words, policy)` validates
logical words before they enter storage. It is not unchecked backend
deserialization. `FromIterator` uses automatic construction and panics on an
observed invalid codec; callers requiring a returned error use the checked API.

`IntSetPolicy` also implements an explicit fallible freeze plan for supported
owned sources. `Vec<K>`, `MutableOrderedSet<K>`, and `FrozenOrderedSet<K>` can use
`try_freeze_with(policy)`. An ordinary returned rejection retains that source.
Allocation failure and user-code panics follow Rust's normal behavior and are
not advertised as transactional errors.

```rust
use frost::{TryFreezeWith, integer::IntSetPolicy};
let integers = vec![-4_i64, 2, -4, 0]
    .try_freeze_with(IntSetPolicy::auto())
    .unwrap();
assert_eq!(integers.iter().collect::<Vec<_>>(), [-4, 0, 2]);
```

`contains(&K)` encodes the query once. `iter()` reconstructs K by value;
compressed storage cannot lend a stored `&K`. `encoded_values()` provides typed
canonical words without revealing a backend object. The profile implements
`ValueElements`, `SetContains`, and `Cardinality`, with `try_len()` for explicit
conversion to usize. It deliberately does not implement the address-sized `Len`
trait or promise `ExactSizeIterator` for every cardinality. Both iterators
override `fold`, which selects the representation once for the traversal, so
`for_each`, `sum`, and other `fold` adapters avoid a dispatch per member. A `for`
loop and `collect` call `next` and do not take that path.

Counts are u64. Inclusive encoded spans are u128, because the inclusive span
from zero to u64::MAX is 2^64. Set cardinality itself remains limited to u64::MAX;
count arithmetic checks true overflow rather than overflowing before subtracting
overlap. Cardinality and encoded bounds have fast storage queries; `statistics()`
explicitly scans the codes to report runs and upper-32-bit partitions.

## Fixed representation candidates and planning

| Word | Sorted | Dense | Roaring | Chunked |
| --- | --- | --- | --- | --- |
| u32 | Ascending boxed u32 codes | Aligned local window of u64 bitmap words | RoaringBitmap | Shared containers indexed by the upper 16 bits |
| u64 | Ascending boxed u64 codes | Aligned local window of u64 bitmap words | RoaringTreemap | Shared containers indexed by the upper 48 bits |

The planner provides `auto`, `sorted`, `dense`, `roaring`, and `chunked` policies.
Automatic construction chooses among Sorted, Dense, and Roaring; Chunked requires
explicit selection. Auto is a deterministic memory-oriented heuristic, not a
learned or proven speed selector. Explicit selections make application and
benchmark decisions reviewable. Default limits are 8 MiB of dense payload and
4,096 distinct upper-32-bit Roaring partitions. Builders can change those limits.
Neither limit caps Chunked storage or its directory.

Dense payload is computed from the actual aligned window with wide arithmetic
before allocating. Its byte limit is a hard payload bound. Roaring's partition
cap prevents a scattered u64 input from silently creating a very large number
of tiny child bitmaps. Its storage estimate is a planning proxy, not an allocator
measurement or an upper bound on resident memory. Serialized byte counts would
also fail to establish that bound. Input staging, sorting, candidate construction,
allocator overhead, and overall process memory are outside the dense budget.

The current planner's score is explicit: sorted payload plus 16 bytes; dense
payload plus 24 bytes; and, for Roaring, 32 bytes globally plus a score for each
16-bit container of `32 + min(2*members, 8192, 4*runs + 2)`. The u64 candidate
adds 80 bytes per upper-32-bit partition. These constants are transparent
estimates rather than allocator measurements. Equal scores retain the earlier
sorted/dense candidate. If Dense is already the better choice than Sorted,
Roaring must save more than 128 estimated bytes to replace it. This named
allocation-scale allowance keeps a small estimated saving from selecting a
more complicated representation, including for `0..1000`. It is not a measured
CPU crossover or a universal throughput claim. Explicit sorted/dense policies avoid the
Roaring statistics pass; explicit Roaring checks its partition cap. The dense
payload must also fit the target's allocation-size bound before allocation.

The Roaring score only grows as codes are surveyed, so the survey stops at the
container boundary where it reaches the score of the candidate already selected.
That shortens planning for sets Roaring cannot win; it never changes the
selected backend, which a unit test checks against a complete survey. A
partition limit is checked from the cached bounds first: the bounds cap the
partition count, exactly for 32-bit words, and members are scanned only when the
span itself exceeds the limit. A stored Roaring value keeps its partition count,
container count, and score beside it, computed once when the value is created,
so eligibility checks on shared, empty, and equal-result shortcuts do not rescan
its containers. These are derived values and take no part in equality, ordering,
or hashing.

Construction normalizes validated codes in one of four ways, chosen from what
the validation pass observes. Codes that arrive strictly ascending are already
sorted and unique; that pass also compares decoded keys in arrival order, which
is the comparison the sorted validation pass would make at the same index, so
no sort and no second pass run. Strictly descending codes are reversed. Other
input of at least 64 codes whose aligned span needs a bitmap no larger than the
staged codes is sorted and deduplicated through that temporary bitmap in linear
time; staging at most doubles and is released before planning. All remaining
input is sorted by comparison. The reported error variant and index are the
same on every path: the first pass completes before any order violation is
reported, and indices in the second pass are positions in sorted order.
This integration retains the reviewed score instead of revision 2's unconditional
4 KiB Dense floor, so a tiny sparse set is not promoted solely because its window
fits that floor. The score does not include the shared-root allocation or predict
the full lifecycle cost of reference counting.

The basic payload estimates `4*n`, `8*n`, and `8*ceil(W/64)` explain why local
windows help. The often-quoted 3.125%/1.5625% break-even densities ignore headers,
rounding, alignment and construction costs; they are not throughput thresholds.

### Coordinate alignment

Dense storage aligns its encoded base down to a multiple of 64. Equal vector
indices from two different bases are not equal logical coordinates. Pair kernels
align absolute block numbers before combining words. Alignment can add one word
relative to a window starting exactly at its minimum, but avoids a shifting
interpretation at every pair operation.

For example, one window containing encoded 1 and another containing encoded 65
both have bit one set in their first word. Their bases are 0 and 64 respectively;
ANDing those first words without their bases would invent a shared member.
Tests therefore vary bases, boundary positions,
signed zero crossings, and values near unsigned maximums.

## Algebra and output selection

Union, intersection, difference, and symmetric difference only select existing
input codes. Valid restricted domains therefore remain valid. No range-fill or
unbounded complement API invents codes in enum gaps or nonzero domains. A
complement relative to an explicit valid universe is `universe.difference(&set)`.

### Dispatch and result shape

Same-backend Roaring pairs use native operations, Dense pairs use aligned word
kernels, Sorted pairs use encoded merge/seek algorithms, and Chunked pairs merge
sorted prefixes and combine private containers. Shared container identity gives
union/intersection reuse and difference/XOR omission at the same prefix. Scalar
counts and materialized algebra do not call the logical codec for every member.
Decoding happens when a caller requests values.

| Operands / shape | Materialized result path |
| --- | --- |
| Sorted / Sorted | Operation-specific merge into a reserved buffer with bulk tail copies; whole-segment copies of the larger operand when the other is at most 1/32 of its size; galloping intersection with its kernel selected once; then result planning |
| Dense / Dense with a legal result window | Aligned word operation; trim empty edge words and retain a competitive Dense result |
| Dense with a small update operand | Clone or extend the bitmap and update bits; derive cardinality from changed bits |
| Distant Dense windows | Convert through bounded little-endian byte chunks and combine native Roaring values |
| Roaring with Sorted additions/removals | Small updates can modify a native clone; bulk or scattered shapes select native algebra or encoded merging |
| Intersection involving Sorted | Choose which side to traverse using the skew-aware work estimate, then filter with a typed membership loop |
| Dense / Roaring intersection | Select encoded probing or bounded Dense promotion using the structural work estimate |
| Chunked / Chunked | Merge prefix directories; share untouched or identical containers; combine changed containers (Array pairs by direct merge, or by probing when one array is much smaller) |
| Chunked / Sorted | Promote a proportionate Sorted side to chunks; probe or merge when Sorted dominates |
| Chunked / Dense | Convert bitmap words directly into chunk containers, then merge |
| Chunked / Roaring | Promote Roaring into chunks when proportionate; otherwise promote the chunked side to native Roaring |

These paths avoid enumerating a large compressed result merely because operands
use different representations. Auto result selection preserves a legal Dense
result when its payload plus header proxy is competitive with Sorted. A legal native Roaring result is also retained when its estimated storage score
is no greater than Sorted. This avoids enumerating compressed results merely to
recover a modest estimated Dense saving. Tiny Dense updates need not rescore
every run. Consequently, Auto is deterministic
for a given input path, but is not a globally minimal-memory canonical format:
construction history and operand representations can affect the output. Even
`with_policy(IntSetPolicy::auto())` may retain such a Dense result. An existing
Chunked operand/result can also remain Chunked under Auto; automatic construction
from input codes does not select it. Contents, ordering, equality and hashing do
not depend on that choice.

For sorted/compressed intersection counts, materialized intersections and
disjointness, direction accounts
for the cost of binary-searching sorted storage. With n sorted members and m
compressed members, scan Sorted when `n <= m * bit_width(n) + scan_words`, using
u128 for the comparison. `scan_words` is the Dense buffer length or zero for
Roaring. This retains compressed-side probing for extreme cardinality skew.

Dense/Roaring scalar kernels compare three structural work estimates: promotion
costs `2 * dense_words`, scanning Dense costs `dense_words + 4 * dense_members`,
and scanning Roaring costs `5 * roaring_members`. Arithmetic saturates when
forming these estimates. These are heuristic work units, not measured CPU
cycles. Promotion allocates temporary Roaring and byte buffers without
materializing the requested result. Count, disjointness, and some subset paths
can use it. Chunked/Dense predicates may likewise construct temporary chunks.
A predicate returning a scalar is not a promise of allocation-free execution;
benchmarks must include conversion and destruction.

Dense promotion never reinterprets Rust memory: it uses explicit little-endian
bytes, splits at upper-32-bit boundaries, and limits each temporary byte chunk
to 8 MiB. In particular, it never passes an exact 2^32-bit extent to Roaring
0.12's byte importer. Conversion may traverse zero bitmap words, and the result
and backend allocations are additional to the bounded byte scratch buffer.

### Limits persist across operations

Every set stores two representation limits. The forced backend used for its
construction is not retained as an obligation on all later results.

| Operation | Default result limits |
| --- | --- |
| Union, intersection, symmetric difference | Componentwise minimum of the operands' Dense-byte and Roaring-partition limits |
| Difference | Left operand's limits |
| Explicit `*_with_policy` | Exactly the supplied policy's limits and requested backend |

The minimum rule keeps commutative operations' metadata commutative. It also
means a default-budget singleton can constrain a set with a custom larger
budget. Use an explicit result policy when that is not the intended limit:

```rust
let result = large.union_with_policy(
    &update,
    IntSetPolicy::auto().with_dense_budget(32 * 1024 * 1024),
)?;
```

The same fallible form exists for intersection, difference and symmetric
difference. A forced Dense result that exceeds its byte limit or a forced
Roaring result with too many partitions returns `IntSetError`. Empty, identical,
and disjoint operands follow the same policy rules; shortcuts cannot bypass a
forced result type or a stricter limit. Auto always has Sorted as a fallback
for representable cardinality. These are representation limits, **not total
memory limits**: Sorted and Chunked storage, input staging, temporary conversions,
retained versions, and allocator overhead can exceed them. Allocation failure follows ordinary
Rust behavior rather than returning a policy error.

`with_policy(policy)` applies representation-aware conversion and replaces
retained limits. An already satisfied explicit policy shares the current root.
Other conversions may allocate or enumerate codes; this is not a lazy view.
`policy()` returns an Auto policy containing the retained limits, not the original
construction request. `contiguous_payload_bytes()` reports exact Sorted or Dense
member-buffer bytes and returns `None` for Roaring and Chunked.

`payload_bytes()` extends diagnostics to all candidates. Sorted/Dense buffers
are exact, Chunked counts payloads and directory entries, and Roaring uses its
container statistics plus index-overhead estimates. These diagnostics exclude
storage/reference headers, allocator overhead, temporary candidates, and memory
held only by other versions. They are not resident-memory measurements or
allocator bounds. Serialized size remains a separate wire-size metric.

### Shared ownership and changed-version costs

Every set owns a shared immutable storage root. `clone()` increments one
reference count in O(1), and `shares_storage_with()` exposes root identity.
An empty `new()` allocates that root even though it has no member buffer. Roots
and containers use Arc on pointer-atomic targets and Rc otherwise. The type's
normal auto traits follow that choice and the logical key marker.

Algebra reuses an equal operand's allocation only if that operand satisfies the
result representation and limits. If both operands equal the result, either
eligible operand can be used. Discovering equality can still require a pair
kernel and temporary result before reuse; O(1) cloning is not an O(1) guarantee
for every no-op algebra call. Tight limits or a forced representation may require
conversion even when the mathematical answer equals an input.

Chunked storage splits a code into `code >> 16` and a low `u16`. Its flat directory
holds ascending prefix keys and corresponding shared containers. An Array holds
at most 4,096 sorted low parts; Bitmap holds 1,024 u64 words and a count; Full
represents all 65,536 low parts without a member payload. Empty chunks are omitted.
There is no run-container variant or persistent radix/tree directory in this
implementation. `chunk_count()` reports directory entries; `shared_chunks(other)`
counts matching-prefix containers with the same allocation.

A changed Chunked result copies directory metadata and increments references to
reused containers. With C directory entries this retains an O(C) directory cost,
even when one small chunk changes. Old versions keep their reachable allocations
alive. Dropping the final owner can release all newly unreachable containers and
is not a constant-time operation. These costs and candidate limits distinguish
the current mechanism from the future tree-directory proposal in
[persistent storage](persistent-integer-storage.md).

### Equality, ordering and hashing

Shared-root equality and ordering return immediately. Otherwise equality checks
cardinality and bounds, then uses matching-representation comparisons, including
Chunked prefix/container comparisons with shared-container shortcuts. Mixed
representations compare logical encoded membership. Ordered comparison remains
lexicographic comparison of canonical ascending words.

Hashing selects its event stream from logical content, independently of the
physical representation. It first hashes the u64 cardinality. Empty sets stop
there. A set whose inclusive span equals its cardinality is one complete run;
its cached bounds can be hashed directly with a run-format tag. This shortcut
uses u128 span arithmetic and does not inspect a member buffer.

Other sets with at most 64 members use a member-format tag followed by ascending
u64 codes. Larger sets scout at most 16 coalesced maximal runs into a small stack
buffer. If that describes the whole set, Hash writes the run tag and inclusive
endpoints. If a seventeenth run exists, it uses the member tag and a typed native
member loop instead. The scout stops early and bounds stack storage; finding the
first 16 runs can still traverse a long prefix, so it is not constant-time work.
No per-member storage enum dispatch or heap allocation is needed by Hash.

Sorted codes, Dense word scans, native Roaring ranges, and Chunked container
visitors produce the same maximal runs, including across word, 16-bit container and upper-32-bit
boundaries. Adjacency uses checked arithmetic at u64::MAX. Both values and the
sequence of Hasher calls match for equal sets, regardless of representation,
policy or construction history. Cardinality and a content-selected format tag
keep the variable-length streams distinct; endpoint hashing never includes
physical container boundaries.

The preceding output-policy revision changed the original per-member format.
This integration preserves its canonical events and extends them to Chunked;
Rust Hash output is not a persistent serialization contract. Roaring 0.12 has native Eq but no
native Hash to delegate. A first implementation hashed every run and regressed
sparse/alternating workloads because isolated members required two endpoint
writes each. The bounded scout and member fallback address that measured cost.
Few-run, many-run, alternating, local-sparse and scattered-prefix cases remain
in the comparative harness; no universal Hash throughput advantage is claimed.

The codec laws make these encoded operations correspond to the logical domain.
Root and container identity accelerate eligible cases without participating in
logical equality, ordering, or the Hasher event stream.

## SIMD and deferred additions

The pinned Roaring 0.12.0 source uses fearless_simd, with its std feature enabling
runtime instruction selection on x86/x86-64 for supported operations. Frost
delegates those kernels rather than adding a second unsafe dispatch layer.
The [upstream README](https://github.com/RoaringBitmap/roaring-rs) documents that
behavior. This does not mean every operation or storage pair uses SIMD.

`std::simd` is not enabled on Frost's stable compiler. The actual Rust 1.99.1
[compile probe](../../verification/int-set/portable-simd-probe.log) rejected it
with E0658; the [standard-library documentation](https://doc.rust-lang.org/stable/std/simd/)
also marks portable SIMD experimental. Fresh sorted SIMD kernels require
independent correctness, CPU-dispatch and fallback tests plus measurements.

Offset-narrowed u64 storage, C-backed CRoaring, alternate codecs, extra block-size
parameters, rank/select APIs, serializers, and a mutable integer family are
deferred. They are not needed to establish this profile's width, codec, planning,
and mixed-algebra contracts. See the [adversarial review](../reviews/integer-design-audit.md)
for the original decisions and the [output review](../reviews/integer-output-review.md)
for the preceding corrections. The
[integration review](../reviews/revision2-integration-review.md) and
[current validation](../revision2-integration-validation.md) cover this merged
source. Earlier validation and benchmark reports remain historical evidence. The [roadmap](integer-roadmap.md) records the proposed
future families and their measurement and verification gates.

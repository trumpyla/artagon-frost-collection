# Adversarial review: integer algebra output and retained limits

## Evidence and scope

This review addresses the subsequent user-supplied review of `FrozenIntSet`.
The supplied material available for this revision was the review summary in the
conversation. Its complete external benchmark harness and review files were not
attached. Consequently, the reported microsecond timings, speedup factors, and
resident-memory figures are leads to reproduce, not measurements made by Frost's
maintainers in this revision.

The source baseline is the preceding delivered integer-profile archive. Its
implementation is independently inspectable through the source and dependency
versions in [Cargo.lock](../../Cargo.lock). The relevant pinned dependency is
`roaring 0.12.0`; source observations below refer to that release, rather than
assuming that the current upstream branch is identical. New implementation and
executed evidence are described by this revision's
[validation report](../../crates/frost-collections-integer/docs/integer-output-validation.md) and
[benchmark report](../../crates/frost-collections-integer/docs/integer-output-benchmarks.md). This document records the
reasoning, contracts, and remaining limits.

The central finding is that correct membership is insufficient for an adaptive
immutable collection. A materialized operation chooses the input representation
for the next operation. Converting a compact dense result to a large sorted array
can make an otherwise correct snapshot update expensive indefinitely.

## Findings and disposition

| Finding | Independent assessment | Decision |
| --- | --- | --- |
| Mixed algebra materializes sorted words | Confirmed in the baseline pair dispatch. The result can expand from a bitmap to one word per member. | Use operation-aware output selection and compressed promotion, including a small-update path. |
| A wide Dense/Dense union falls back to Sorted | Confirmed when the dense output window exceeds the operation guard. Long separated runs are a particularly poor sorted fallback. | Consider Roaring before enumerating every member into sorted storage. |
| Constructor limits disappear during algebra | Confirmed. Baseline operations do not retain the caller's policy. | Retain explicit candidate limits in every result; distinguish them from a forced representation. |
| Equality and hashing incur wrapper overhead | Baseline comparison/hash iterators visit members through the representation wrapper. Native equality can avoid this. Native Roaring hashing is not an available direct control. | Add native same-representation equality and a content-selected canonical hash stream, with member hashing for fragmented sets. |
| Sorted input uses a generic Roaring collector | Confirmed in the width adapters. Both pinned native types have checked sorted builders. | Use `from_sorted_iter` after Frost's normalization. |
| Dense/Roaring promotion can outperform probing | The native byte-import API exists and enables a bulk path. The reported factor is workload-specific and was not independently established by source inspection. | Include conversion cost in dispatch and benchmarks; retain cheap probing for sufficiently small operands. |
| Memory planning can prefer a slower membership backend | Expected from a memory-oriented score. Saving a small number of bytes is not evidence of lower lookup latency. | Keep the planner's objective explicit and preserve forced choices; do not infer a universal speed policy from one range. |
| CPU flags affect dense/native comparisons | Confirmed as a methodological issue: ordinary compiler-generated kernels and dependency runtime dispatch have different feature-selection paths. | Record baseline and supported `x86-64-v3` builds with matched wrapper/native flags. |
| The integer profile unnecessarily requires std | Confirmed against the pinned dependency's manifest and source. | Support `alloc`-only `int-set`; forward `std` deliberately when enabled. |

The baseline dense guard deserves precise wording. It is the maximum of the
default 8 MiB allowance and the two existing dense payload sizes. It does not
unconditionally reject every result above 8 MiB. However, it still forgets a
larger configured budget when both operands are small, and it can enlarge a
result beyond a smaller configured budget. The previous design described this
as a construction-only policy. Retaining limits is an intentional contract
improvement for operation chains, not a correction to the membership algebra.

## Result limits are part of the collection contract

`IntSetPolicy` chooses a representation for an individual construction or
explicit operation. `IntSetLimits` records two constraints that survive in the
result: maximum dense payload bytes and maximum distinct upper-32-bit Roaring
partitions. `FrozenIntSet::limits()` exposes them. These are candidate limits;
neither bounds sorted storage, temporary allocations, allocator overhead, or
total resident memory.

The ordinary binary methods combine limits as follows:

| Operation | Dense limit | Roaring partition limit |
| --- | --- | --- |
| Union | Minimum of the two operands | Minimum of the two operands |
| Intersection | Minimum of the two operands | Minimum of the two operands |
| Symmetric difference | Minimum of the two operands | Minimum of the two operands |
| Difference | Left operand's limit | Left operand's limit |

The minimum is componentwise. For example, limits `(16 MiB, 100 partitions)`
and `(8 MiB, 1,000 partitions)` yield `(8 MiB, 100 partitions)` for union.
This makes the limits of commutative results independent of argument order.
Difference inherits the source collection's limits because the right operand
specifies members to remove. Chained results retain the combined limits.

This choice has a visible tradeoff. Combining a set constructed with a large
custom dense budget and an ordinary default-budget singleton lowers a
commutative result's dense allowance to the default. Applications that want the
larger allowance must provide it explicitly with `union_with_policy` or the
corresponding operation. The `*_with_policy` methods return `Result`, replace
the result's limits, and can force a candidate. The override is local to the
returned set; it does not mutate either operand.

A forced constructor choice is not inherited as an eternal backend constraint.
For example, a collection constructed with `IntSetPolicy::dense()` may produce a
Roaring result when its union spans a large gap. Normal operations can therefore
fall back to a permitted representation while retaining their ordinary return
type. This does not promise recovery from allocation failure or a representable
cardinality above `u64::MAX`.

Limits must also apply to empty and identity fast paths. Returning a clone for
`A union empty` cannot accidentally retain a larger allowance than the combined
policy; selecting an existing representation is legal only if that representation
satisfies the result's limits. A zero Roaring partition limit excludes every
nonempty Roaring result, including u32 storage's single upper-32-bit partition.

Checking the final candidate is necessary but does not establish a bound on
peak operation memory. A promotion may temporarily own input storage, a byte
buffer, and native output containers. The current policy is not an allocator
budget or an admission-control system. A future fallible allocation API would
need a separate contract and accounting for those simultaneously live objects.

## Dispatch should preserve useful structure

The dispatch matrix has three different concerns: selecting members, executing
the operation, and selecting the output representation. A per-member merge is a
correct fallback, but should not be the universal conversion mechanism.

For a dense set and a small update, copying the dense words and modifying the
affected bits avoids decoding or enumerating the existing members. The output
window must still be computed from absolute encoded coordinates, checked with
wide arithmetic, and admitted under the result limits. An update outside the
window can require expansion or another candidate. A small input cardinality
alone does not justify allocating a huge dense gap.

For dense operands whose union window is too wide, conversion into Roaring
preserves separated local runs without creating one sorted word per member.
The upper-32-bit partition limit remains relevant for u64 inputs. Promoting
widely scattered words into a large tree of tiny bitmaps merely replaces one
pathology with another.

For mixed intersection, a tiny sorted list or Roaring operand can be cheaper to
probe directly than to promote a large dense set. For large operands, a bounded
dense-to-Roaring conversion can amortize its construction cost over native
container operations. The appropriate threshold depends on the operation,
input shape, retained limits, and hardware. The measured implementation should
state that threshold; it should not claim to select the globally fastest plan.

Sorted output remains useful for small or highly scattered results. The goal is
to stop treating it as a mandatory output for every mixed pair. Rebuilding every
result through a full sorted staging vector would retain much of the original
cost even if the final diagnostic representation changed back to Dense.

The current construction score allows a 128-byte estimated-size preference for
Dense when it is otherwise competitive with Sorted. This is a fixed tolerance
for a small potential memory saving, not a measured universal lookup crossover.
Result planning also preserves an existing legal Dense result when it remains
competitive with Sorted, avoiding a whole-member statistics pass after a tiny
update. A legal native Roaring result is likewise retained when its estimate is
no larger than Sorted's estimate. Partition eligibility is checked before this
retention rule. This prevents an expensive element-by-element conversion merely
to obtain a small estimated improvement over an already compact native result.
Consequently, Auto is an operation-aware heuristic: fresh construction and
`with_policy(Auto)` can select different representations for identical contents.
Logical equality and retained limits remain independent of that history. Auto
does not promise a unique representation or minimum heap usage.

### Regressions caught while reviewing the correction

Independent inspection of the first implementation of the correction found
two dispatch problems before the final benchmark run:

1. The native clone-and-update branch was described as a small sorted update,
   but initially admitted every sorted input size. Large inputs would perform
   repeated native inserts/removals instead of using the sorted builder and
   container algebra. That path is now bounded to 64 sorted updates; larger
   inputs reach the bulk conversion path. The bound is a conservative dispatch
   choice, not a universal measured crossover.
2. Materialized Sorted/compressed intersection initially always filtered the
   sorted operand. A large sorted set intersected with two compressed members
   would scan the entire sorted set, losing the preceding implementation's
   size-ratio optimization. The corrected helper chooses traversal once, retains
   the existing `prefer_sorted_probe` work estimate, and handles both operand
   orders, including an explicitly Sorted output. A counter-based regression
   checks that the large iterator/membership loop is not visited in either
   direction of the extreme-size case.

Both errors would have passed ordinary membership comparison tests. They
illustrate why the acceptance surface needs dispatch regressions and matched
workload measurements in addition to set-algebra oracles.

### The first measured correction still had material regressions

The first complete timing pass is retained under
[`verification/integer-output/first-pass`](../../verification/integer-output/first-pass/comparison.csv),
including its measured source archive, raw measurements, and comparison script.
Its negative results prompted a further correction; they are not silently
replaced by the final measurements.

The first-pass Dense/Roaring intersection used native promotion successfully,
but then result planning converted the compact native result to Dense by
visiting its many runs. That conversion dominated the operation. The preserved
pooled medians for `overlap32` were:

| Build profile | First-pass Frost materialization | Native promotion plus materialization | Frost/native time |
| --- | ---: | ---: | ---: |
| Baseline x86-64 | 5,016.369 microseconds | 46.759 microseconds | 107.28 times |
| x86-64-v3 | 4,867.981 microseconds | 23.547 microseconds | 206.74 times |

These are comparisons with a native control that includes promotion, rather
than with an already converted input. They motivate retaining an eligible
native result that is already competitive with Sorted. The retained limit
checks still apply; changing representation must not be a shortcut around them.

Always hashing maximal runs also imposed unnecessary work on fragmented data.
A singleton run produces two endpoints instead of one member, and native run
discovery is not free. Representative first-pass regressions were:

| Case / profile | Prior member hash | First-pass always-run hash | Time increase |
| --- | ---: | ---: | ---: |
| Million alternating members, Dense, baseline | 4,707.339 microseconds | 6,989.491 microseconds | 48.48% |
| 2,048 sparse prefixes, Sorted, baseline | 8.909 microseconds | 12.181 microseconds | 36.73% |
| 2,048 locally sparse members, Roaring, baseline | 30.684 microseconds | 48.742 microseconds | 58.85% |
| 2,048 locally sparse members, Roaring, v3 | 22.058 microseconds | 49.361 microseconds | 123.78% |

The values above come from the preserved comparison CSV and are observations on
the measured host, not general hardware-independent factors. The revised hash
selector described below keeps compact run streams for suitable logical
contents and falls back to the canonical member stream for fragmented sets.
Final effects, including any remaining regressions, belong to the separate
[final benchmark report](../../crates/frost-collections-integer/docs/integer-output-benchmarks.md).

### Dense-to-Roaring conversion has edge contracts

The pinned `RoaringBitmap::from_lsb0_bytes` API reads bytes in least-significant
bit order. Frost's bitmap words must therefore be emitted with `to_le_bytes()`.
Reinterpreting native-endian word memory would be wrong on a big-endian target
and would introduce an unnecessary alignment/aliasing boundary.

Dense bases are aligned to 64 encoded positions. A u64 dense window must be
split correctly at upper-32-bit boundaries and inserted into the corresponding
native child bitmap. The low-word offset restarts at each partition; treating
all bytes as one u32 bitmap aliases distinct u64 members.

There is also a concrete dependency edge case: pinned 0.12.0 computes a byte
import's bit length through a checked u32 multiplication. One entire 2^32-bit
window occupies 512 MiB, but its bit count does not fit that multiplication.
Bounded chunks avoid relying on an overflowing intermediate even when the
inclusive final bit position is valid. The implementation stages at most 8 MiB
of bytes per chunk and additionally clips each chunk to the partition boundary.
Tests should cover the relevant boundaries with small inputs; they do not need
to allocate half a gigabyte to exercise the splitting invariant.

The import allocates/copies native storage. It is not a borrowed or zero-copy
view. Converting the bitmap's existing set bits also must not turn padding into
members or fill holes in a restricted `IntKey` domain. Optimizing native run
containers is a separate representation choice after conversion.

The dense/Roaring count path may also promote when its structural work estimate
favors conversion. Such a count does not materialize the result set, but it can
allocate temporary native storage and a byte buffer. Allocation-free count
claims must be scoped to paths that actually have that property.

## Equality and hashing require different optimizations

The pinned native `RoaringBitmap` and `RoaringTreemap` implement logical equality.
Their implementations handle equivalent container contents; the treemap also
ignores empty child maps. They do not implement `Hash`. Consequently, a reported
comparison against "native Eq/Hash" must identify the custom native hash loop or
other control used for the hash measurement.

Same-representation equality can dispatch once to sorted slices, dense words
with their canonical coordinates, or the native Roaring comparison. Cardinality
and bounds can reject unequal inputs first. Cross-representation equality still
means equality of the canonical member set. Backend discriminants, retained
limits, and construction history are not part of logical equality.

Hashing representation bytes would violate this contract. Rust's
[Hasher contract](https://doc.rust-lang.org/std/hash/trait.Hasher.html) requires
equivalent values to make matching calls with matching arguments in matching
order. Equal byte concatenations are insufficient when call boundaries or
typed `write_*` methods differ. The
[Hash documentation](https://doc.rust-lang.org/std/hash/trait.Hash.html) also
leaves `hash_slice` unspecified relative to repeated scalar hashing.

The final selector uses only logical content, in this order:

1. Hash the u64 cardinality. An empty set stops here, independently of its
   representation.
2. If the inclusive encoded span equals cardinality, hash a u8 run tag and the
   single run's inclusive u64 endpoints. The span calculation uses u128, so a
   sparse set containing both zero and `u64::MAX` cannot accidentally pass an
   overflowing contiguous-span check.
3. For other sets with at most 64 members, hash a u8 member tag followed by all
   ascending codes normalized to u64. The representation dispatch happens
   outside the member loop, and u32 storage uses the same normalized event form.
4. For larger sets, scout at most 16 maximal runs into a fixed stack buffer.
   Coalesce pieces across physical boundaries before counting runs. If a 17th
   logical run starts, stop the scout and hash the member form; otherwise hash
   the run tag followed by those maximal inclusive endpoint pairs.

A run crossing a Dense word, Roaring container, or upper-32-bit partition must
remain one run. The scout can extend its 16th run across arbitrarily many such
boundaries; the cutoff concerns logical runs, not physical pieces. The native
breakable visitor exposes the next run's start before asking `next_range` to
find its end, so a huge 17th run is not scanned before choosing the member form.
Dense can inspect the current 64-bit bitmap word before stopping. This bounds the
run buffer and allows early termination; long initial runs can still require
substantial prefix traversal. It is not a constant-work hashing guarantee.

The framing is prefix-free for valid sets of a fixed key type. The member form
contains exactly the prefixed number of code events. In the run form, positive
inclusive run lengths sum to that same cardinality, so a complete run sequence
cannot be a proper prefix of another valid sequence with the same cardinality.
The tags separate the forms, and zero cardinality identifies the empty form.
This is a property of the input protocol, not a promise that a hasher never
collides.

Every representation must choose the same form and feed identical methods,
arguments, and call order to the hasher. The backend, retained limits, codec
object layout, and construction history are excluded. Tests exercise both
cardinality thresholds and run-count thresholds across word/container/partition
boundaries, signed zero, and unsigned maximum endpoints. This design retains
compact streams for a small number of long runs without imposing two endpoint
hash calls on every isolated member.

A recording hasher that records each method and argument checks this contract
more directly than comparing only two `DefaultHasher` outputs. Tests should
compare equivalent representations, rather than freeze an implementation's
hash bytes as a portable file format. Hash values remain unspecified across
crate/compiler revisions. A cached fingerprint could help reject equality, but
does not by itself prove equality or implement arbitrary caller-supplied hashers.

## Pinned dependency and feature findings

The local `roaring-0.12.0` package declares `no_std` when its `std` feature is
disabled and imports `alloc`. Its default feature is `std`; that feature enables
optional I/O-related dependencies and forwards `fearless_simd/std`. Native
serialization through standard I/O is separately gated. The byte import and
both sorted constructors used by this profile are available without std.

The profile therefore uses an optional Roaring dependency with
`default-features = false`, enables it with `int-set = ["alloc", "dep:roaring"]`,
and forwards `roaring?/std` from Frost's `std` feature. The weak forwarding does
not enable an otherwise unused optional dependency. The existing
`roaring-0_12` integration retains its std compatibility boundary. Native
re-exports and adapters remain enabled by either integration feature, preserving
the adapters previously obtained through `int-set`.

An all-features test cannot establish the alloc-only claim: Cargo feature
unification can quietly enable std through another path. Verify the isolated
feature combination, inspect its dependency feature graph, and compile an
appropriate target without std in addition to ordinary host tests.

Both native `from_sorted_iter` constructors require strictly increasing input
and reject a bad sequence. Frost already normalizes unsigned words, so it can
use those builders with an internal checked assertion. This invariant does not
depend on trusting arbitrary downstream `Ord` implementations to justify
unchecked memory access. Generic `collect()` is still appropriate for callers
whose inputs are not already normalized.

## Benchmark interpretation and remaining limitations

The [Roaring upstream README](https://github.com/RoaringBitmap/roaring-rs)
documents runtime SIMD selection for supported operations on x86/x86-64 when
std is enabled, and compile-time selection without it. This does not imply that
every native operation is SIMD-accelerated or that Frost's dense `count_ones`
loop has an equivalent runtime dispatcher.

The [rustc code-generation options](https://doc.rust-lang.org/rustc/codegen-options/index.html)
make target CPU/features explicit build inputs. A baseline x86-64 build can
lower population counts differently from a supported `x86-64-v3` build. Exact
instructions and speedups depend on the compiler, target, and operation, so the
user's reported factor must be independently measured before being repeated as
this revision's result. Changing the entire library's default target would also
change deployment compatibility; no such requirement follows from a benchmark.

A useful comparison records construction, output representation and storage
estimates, operation latency, and repeated update chains. It compares equivalent
input data, identical target flags, and native controls that include comparable
conversion costs. A serialized byte count is not a measurement of resident heap
usage, and an estimate should be labeled as such. Tail latency and peak memory
matter particularly when several readers retain old snapshots.

The review's unsuccessful `Iter::advance_to` experiment is a reason to avoid
adopting that optimization on faith. It does not prove that galloping is always
slower for every size ratio or backend. No new sorted/Roaring galloping path is
required for this correction.

The production acceptance surface includes all nine ordered representation
pairs, all four materialized operations and their counts, signed and unsigned
boundary values, restricted codecs, zero/tiny/custom limits, repeated snapshot
updates, and representation-independent equality/hash behavior. Differential
tests and reduced models establish their stated cases and bounds. They are not
unbounded proofs of all compiler outputs, arbitrary downstream codecs, allocator
behavior, or native dependency implementations.

The additional proposed collection structures are evaluated separately in the
[integer roadmap](../../crates/frost-collections-integer/docs/integer-roadmap.md). They are design candidates and
are not implementations or verification claims for this revision.

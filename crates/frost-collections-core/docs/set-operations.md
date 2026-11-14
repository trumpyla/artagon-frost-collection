# Frozen set operations

Implemented in Frost **0.4.0**, on **Rust 1.99.1, edition 2024**.

For the proposed extension to mutable inputs, mixed backends, and composable
generic expressions, see [general set algebra](set-algebra.md). That
proposal is not implemented; the contracts below describe current behavior.

## API and ownership

The supported frozen sets expose four named algebra operations:
`union`, `intersection`, `difference` and `symmetric_difference`.

| Receiver | Algebra result | Scalar count type | Feature |
| --- | --- | --- | --- |
| `FrozenOrderedSet<T>` | Lazy iterator of borrowed `&T`, in ascending order | `usize` | `alloc` |
| `FrozenHashSet<T, S>` | Lazy iterator of borrowed `&T`, unspecified order | `usize` | `std` |
| `Frozen<RoaringBitmap>` | New frozen Roaring bitmap | `u64` | `roaring-0_12` |
| `Frozen<RoaringTreemap>` | New frozen Roaring treemap | `u64` | `roaring-0_12` |
| `Frozen<FixedBitSet>` | New frozen dense bitset | `usize` | `fixedbitset-0_5` |

All five also provide `is_subset`, `is_superset`, `is_disjoint`,
`intersection_len`, `union_len`, `difference_len` and
`symmetric_difference_len`. Scalar queries avoid allocating a result collection.
Bitmap cardinality remains separate from a dense bitset's universe length.

Family algebra borrows both inputs. It does not require `T: Clone`, copy elements,
or allocate an intermediate set. A caller can inspect only the first matching
member, consume the complete iterator, or explicitly materialize an owned
result with `copied()`/`cloned()` and `collect()`. Collection then has the target
family's normal construction costs. Constructing a hash result preserves an
explicit chosen hasher through `from_iter_with_hasher`; collecting with the
ordinary `FromIterator` uses the target type's default hasher.

Hash operands can have different `BuildHasher` implementations. Reading their
contents does not require either hasher to implement `Clone` or `Default`.
Inputs remain unchanged throughout every operation.

The borrowed iterators are named crate-owned types, implement fused forward
iteration and give conservative size hints. Set-operation iterators do not claim
an exact remaining length merely because their inputs know their lengths.

## Representative and ordering laws

An equivalent member can carry metadata ignored by equality or ordering. Union
and intersection always expose the **left operand's stored representative** on
an overlap. Difference exposes left-only elements. Symmetric difference exposes
the unique member from whichever operand contains it.

Membership commutativity does not imply identical representative metadata after
swapping operands. The contract is deliberate and tested. Hash intersection can
scan the smaller right input while looking up and returning the matching stored
element from the left input.

Ordered operations use the family's `Ord` equivalence and produce ascending
traversal. Hash operations use `Eq`/`Hash` and promise no traversal order.
Ordinary Rust trait-law requirements still apply. No operation mutates a key or
converts one equivalence relation into another.

## Execution strategy and cost

Let `n` and `m` be input cardinalities and `c_left`/`c_right` their hash-table
capacities. Scalar kernels and borrowed family iterators use constant auxiliary
space; they do not materialize the result.

| Representation | Strategy | Cost boundary |
| --- | --- | --- |
| Sorted frozen sets | Linear merge for similar cardinalities; monotonic exponential seeking for markedly skewed operands | Worst-case `O(n + m)` comparisons; the skew kernel uses `O(s log(1 + l/s))` for smaller size `s` and larger size `l`; predicates can stop early |
| Hash intersection/disjointness | Scan the smaller cardinality and probe the other table | Expected lookup cost; native traversal also scans capacity, so the full bound includes the chosen table's capacity |
| Hash union/difference/symmetric difference | Visit applicable sides and probe membership | Expected hashing work plus scanned table capacities |
| Roaring bitmaps | Invoke native container operations and count kernels | Cost depends on containers and density; no universal `O(number of members)` claim |
| FixedBitSet | Zip full shared words, mask boundary words, then traverse any suffix; use direct one/two-word paths for small results | Work is proportional to relevant word spans, with early-exit relations |

Hash cardinality formulas use the overlap count and checked addition after
subtraction, avoiding spurious intermediate overflow. For inputs whose native
cardinalities fit the declared count type, union and symmetric-difference counts
use checked result arithmetic. A RoaringTreemap spanning all `2^64` members is
outside its native `u64` cardinality contract; the wrappers do not validate or
repair an already unrepresentable native count. Supporting `u64::MAX` as an
individual member value is distinct from representing that entire universe's
cardinality.

These are specific algorithmic improvements, not a claim that one storage
representation always wins. The ordered kernel uses an internal 32:1 cardinality
heuristic; it is not a universal crossover recommendation and does not switch
collection storage. No CPU-specific instruction requirement is added. Same-slice
scalar queries and separated endpoint ranges stop immediately. Lazy construction
still performs no comparisons; intersection and difference choose their strategy
on first advancement. Intersection bulk folds dispatch once per traversal.

On the measured x86-64 build, intersection and difference iterators grow from
32 to 40 bytes for their private strategy state. The complete tradeoffs and
before/after results are in [PERFORMANCE_REVIEW.md](../../../docs/reviews/performance-review.md).

## Dense bitset universe and padding

Binary operations treat absent positions beyond an operand's logical universe
as absent members:

| Operation | Result universe length |
| --- | --- |
| Union | Maximum of the input universe lengths |
| Symmetric difference | Maximum of the input universe lengths |
| Intersection | Minimum of the input universe lengths |
| Difference | Left input's universe length |

Each operand's final logical block is masked before combining words. Native raw
padding therefore cannot appear as a member or inflate a count. Fresh results
have clean padding; source storage is retained unchanged. Tests cover empty and
unequal universes, block boundaries and deliberately dirty native padding.

Complement is outside this binary-algebra API: it requires a separately chosen
universe. These operations also do not interpret integer members as positions
in an unrelated collection or silently construct a projection index.

## Examples and verification

The README example is doctested. `crates/frost-collections/examples/set_operations.rs` exercises the
ordered, hash, Roaring32, Roaring64 and dense operations under their feature
gates; its unit test calls the actual complete scenario.

Dedicated regression suites verify algebra against standard sets, left-side
representatives, empty/self/disjoint inputs, differing cardinalities and
hashers, iterator progress and size hints, scalar count agreement, full-width
integer values and logical bitmap bounds. `docs/reviews/historical-verification.md` records the final
executed matrix and its limits.

`crates/frost-collections-core/benches/set_operations.rs` compares direct kernels with ordinary membership
scans or materialize-then-count baselines. Build input outside the timed loop;
warm up; report the median of five samples; verify equal results before timing.
Run it with `cargo bench --bench set_operations --all-features --locked`.
Its results are local microbenchmarks, not workload-wide throughput guarantees.

### Historical measurements from the supplied 0.4.0 baseline

The following results came with the original source. The current performance
pass uses `crates/frost-collections-core/benches/performance_review.rs` and the `verification/performance-*`
records, with direct native comparisons and a wider input-shape matrix. Run
`cargo bench --bench performance_review --all-features --locked` for that harness.

On 2026-11-08, Rust 1.99.1's default optimized bench profile ran on an x86-64
Linux host reporting an AMD EPYC 9V74 CPU. Each result is the median of five
samples after warmup. Construction was excluded and each comparison first
asserted equal answers. No task-specific CPU flags were added.

| Case | Direct kernel, median ns | Explicit baseline, median ns | Baseline / kernel |
| --- | ---: | ---: | ---: |
| Ordered intersection, 128 members per side | 117.68 | 677.03 | 5.75 |
| Ordered intersection, 4,096 members per side | 3,956.26 | 48,574.75 | 12.28 |
| Ordered intersection, 65,536 members per side | 61,250.07 | 1,018,909.64 | 16.64 |
| Hash intersection, 65,536 left / 64 right | 443.79 | 548,658.34 | 1,236.30 |
| Roaring intersection count | 2,327.54 | 3,094.48 | 1.33 |
| Dense intersection count | 6,929.40 | 278,563.94 | 40.20 |

Ordered baselines scan left members and binary-search right membership. The
hash baseline deliberately scans the large left set and probes the small right
set; the large ratio demonstrates avoiding that unnecessary work. It is not a
comparison against the standard library's already size-aware set intersection.
The Roaring baseline materializes the intersection before asking for its count.
The dense baseline iterates left members and probes right membership.

These workloads use deterministic integer patterns, warm data and one host.
They do not establish relative performance across all densities, overlap ratios,
hashers, CPUs or application workloads. The source and raw output are included
as `crates/frost-collections-core/benches/set_operations.rs`, `verification/history/set-operations-bench.log` and
`verification/history/set-operations-environment.txt`.

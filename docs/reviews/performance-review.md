# Frost collections: performance review and implemented changes

Review date: **2026-11-08, America/New_York**. Input: `frost-collections(2).zip`, version 0.4.0. Target retained: **Rust 1.99.1, edition 2024**. This is an unreleased optimization pass; the package version, dependencies, public method signatures, and feature model remain unchanged.

## Decision

Keep the present architecture. Ordinary mutable/frozen families, separately named adaptive profiles, small capability traits, and exact backend wrappers serve different purposes. The main avoidable costs found in this review were inside kernels and conversion paths: repeated key comparisons, unnecessary iteration machinery, and staging storage. A universal collection enum would not address those costs and would complicate bounds, ordering, layout, and feature behavior.

This pass implements five focused improvements and adds a broader benchmark corpus. It preserves ordering, first-key/last-value construction rules, left-side representatives in set algebra, explicit hashing policies, and structural freezing. No unsafe code, dependency, CPU-specific intrinsic, or runtime CPU requirement was added.

## Findings and actions

| Priority | Finding | Action | Source |
| --- | --- | --- | --- |
| High | Ordered intersections scanned nearly all of a large slice even when the other set was tiny. | Added monotonic exponential seeking, identity and endpoint shortcuts, and separate bulk kernels. | [ordered kernels](../../src/family/ordered/set_ops.rs) |
| High | Dense bitset reductions carried zero-extension iterator state and checked addition through every word. | Split full-word spans from boundaries and suffixes; added direct one/two-word paths. | [bitmap kernels](../../src/backends/bitmap.rs) |
| Medium | Small adaptive map entry access searched the same representation twice. | Reuse the successful lookup or insertion index across the promotion decision. | [hash entry](../../src/adaptive/hash_map/mutable.rs), [ordered entry](../../src/adaptive/ordered_map/mutable.rs) |
| Medium | Owning Rayon traversal expressed a new collection even when a contiguous buffer already existed. | Transfer the existing boxed-slice/vector allocation directly to Rayon. | [Rayon adapter](../../src/adapters/rayon.rs) |
| Low | Set iterator wrappers did not forward native bulk operations consistently with map wrappers. | Forward `fold`, `count`, and ordered reverse folds. | [set iterators](../../src/family/sets.rs) |
| High, measurement | Earlier large ratios compared against deliberately more expensive algorithms. | Added before/after measurements and appropriate native operations. | [benchmark](../../benches/performance_review.rs) |

## Measured results

These are measurements of the uploaded source against the final revised source on the same x86-64 Linux host reporting an AMD EPYC 9V74. Both used Rust 1.99.1 and Cargo's optimized bench profile, with no task-specific CPU flags. Benchmark execution was sequential and pinned to CPU 0.

The exact same harness ran in **baseline → revised → revised → baseline** order. Each of its **140 cases** calibrates batches to at least about 4 ms (subject to its iteration cap), then records seven timed batches. The table reports the median of the two per-run medians; with two values this is their arithmetic midpoint. Each answer is checked before timing. Input construction is excluded; materialization cases include output allocation and destruction. Hash comparisons use identical deterministic `BuildHasherDefault<DefaultHasher>` policies solely for reproducible benchmarking; public default hashing is unchanged.

| Workload | Original, ns | Revised, ns | Original / revised |
| --- | ---: | ---: | ---: |
| Ordered intersection, 65,536 / 64 distributed members | 47,049.34 | 2,749.79 | 17.11× |
| Ordered intersection, 65,536 / 64 members at the beginning | 50.71 | 48.10 | 1.05× |
| Ordered intersection, 65,536 members per side | 61,563.67 | 61,356.37 | 1.00× |
| Ordered intersection bulk traversal, 65,536 per side | 68,225.09 | 64,293.40 | 1.06× |
| Dense intersection count, 4,096-bit universes | 114.64 | 50.69 | 2.26× |
| Dense intersection count, 262,144-bit universes | 10,366.89 | 3,082.86 | 3.36× |
| Dense intersection count, 262,147-bit universes | 7,437.30 | 3,621.18 | 2.05× |
| Adaptive hash entry hit, 8 entries | 6.58 | 5.18 | 1.27× |
| Adaptive ordered entry hit, 8 entries | 5.48 | 4.33 | 1.26× |

The important gain is avoiding work. Distributed small/large ordered intersection improved about 17× here; larger dense count cases improved about 2–3×. The balanced 65,536-member count stayed essentially unchanged. Earlier candidates slowed balanced traversal and clustered-prefix intersections; those candidates were revised before delivery.

Extremely favorable tail cases show much larger ratios: a singleton or a 64-member cluster near the end previously required traversing almost the entire large input. Those results are present in the complete CSV. They are examples of eliminating a linear scan, not a claim of a similar advantage over native standard-library set operations.

### Limits and counterexamples

The host is shared and warm-data microbenchmarks are noisy. Some unchanged native control cases moved by tens of percent between binaries/runs. Read small nanosecond differences as observations, not statistically established improvements; no confidence interval, hardware-counter study, or cross-machine calibration is claimed.

There are slower observations too. The 16:1 ordered case, some endpoint-only or small ordered calls, and the already-promoted 64-entry adaptive hash map did not improve consistently; the latter measured about 13.09 ns versus 17.25 ns in the final aggregate. The additional iterator state adds dispatch work, and input shape still matters. The full CSV retains every case rather than reporting only wins.

On the 65,536/64 hash-intersection case, revised Frost measured about 819.95 ns versus 800.29 ns for native `HashSet::intersection().count()`. This supports keeping the wrapper architecture, not claiming it beats the native algorithm. Roaring intersection counts still call the same native kernel. Native FixedBitSet count operations are also included on clean-padding inputs; Frost additionally enforces logical-universe masking on retained dirty padding.

Evidence: [full summary](../../verification/history/performance-summary.csv), [baseline run 1](../../verification/history/performance-baseline-1.csv), [baseline run 2](../../verification/history/performance-baseline-2.csv), [revised run 1](../../verification/history/performance-optimized-1.csv), [revised run 2](../../verification/history/performance-optimized-2.csv), and [environment](../../verification/history/performance-environment.txt).

## 1. Ordered sets: select work by operand shape

The original two-pointer merge is a good default for similarly sized ascending slices. It is linear in their combined lengths and accesses memory sequentially. It is a poor fit when a tiny operand asks about a few values spread through a much larger slice.

The revised implementation keeps the merge and chooses exponential seeking when one cardinality is at least **32 times** the other. Seeking advances through the larger slice monotonically: it brackets the next relevant position exponentially, then binary-searches only that bracket. Earlier positions are never reconsidered. With `s` smaller members and `l` larger members, the comparison-work bound is `O(s * log(1 + l/s))`, with constant auxiliary storage. The general worst-case linear bound remains valid. Costs of user `Ord` implementations are additional to the comparison count.

The 32:1 cutoff is an internal heuristic selected conservatively from this matrix, not a generally optimal threshold for every CPU, string type, or distribution. The benchmark includes ratios 8, 16, and 32, clusters at both ends, distributed values, interleaved misses, and separated ranges. Simple smaller-side binary search or a native tree can still be faster for particular distributions; the native/probe rows make that visible.

Scalar intersection, and counts derived from it, choose their kernel once. Same-slice counts and relations return directly. Empty or separated endpoint ranges also stop immediately. Subset and disjoint queries retain early termination.

Lazy intersection and difference retain zero-comparison construction. Their first `next()` chooses a strategy. Intersection's `fold` selects a bulk path once, so adapters such as `map(...).sum()` do not redispatch for every returned member. The scalar merge is allowed to inline; the larger seeking routine and balanced fold body are isolated to avoid unstable code growth in the hot loop. These are implementation choices tested on the pinned compiler, not ABI guarantees.

The stateful intersection and difference cursors are **40 bytes rather than 32 bytes on this x86-64 build**. Union remains 32 bytes. This is a real memory tradeoff. Union and symmetric-difference enumeration still visit their output; no search technique can avoid the cost of producing required members.

The left-representative contract is preserved in both size orderings. If the larger searched set is the left operand, the iterator returns its matching stored reference. It never substitutes the smaller right-side representative merely to simplify the search. New tests check both pointer provenance and partial-cursor `count`, `fold`, cloning, size hints, and fused exhaustion.

## 2. Dense bitmaps: straight word loops and explicit boundaries

For a large result, the implementation now separates:

1. Full words shared by both operands, handled by a direct slice zip.
2. The boundary where the shorter logical universe ends, masking its padding.
3. A suffix from the longer operand when the operation requires one.
4. An optional final partial word, also masked.

The common loop no longer carries infinite zero-extension iterator state through every block. One- and two-word results use direct masked combinations instead, avoiding setup that would dominate small operations. Relations use the same short-span idea and otherwise stop on the first relevant nonzero word.

Per-word checked addition was removed from population-count reductions for a specific reason: all emitted words represent distinct positions inside a universe whose length already fits `usize`. The sum cannot exceed that universe. This proof does not apply to every possible bitmap API; the Roaring `u64` union/symmetric-difference overflow checks remain intact.

Logical universes retain the existing rules: union/symmetric difference use the larger length, intersection the smaller length, and difference the left length. Dirty bits beyond either source's logical length are masked even when they fall inside the other operand's larger universe. Inputs are not rewritten, and outputs have clean padding. The new and existing tests cross zero, word, two-word, and long-suffix boundaries in both operand directions.

No hand-written SIMD or target-feature dispatch was added. The direct loops give the compiler a simpler reduction. Additional CPU-specific work should start with assembly and target-hardware measurements, not an assumption that explicit intrinsics necessarily improve this generic crate.

## 3. Adaptive entries: keep the first search result

Both `AdaptiveHashMap::get_or_insert_with` and `AdaptiveOrderedMap::get_or_insert_with` previously searched the small representation to decide whether promotion was needed, then repeated the same search to return or insert the entry.

The new implementation retains the linear position or the binary-search `Result<usize, usize>`. A hit uses that position. A miss below the threshold uses the already-known insertion point. If promotion occurs, the old position is discarded and the native table/tree entry API performs the required lookup.

This removes the duplicated user equality/ordering work from the small-representation path. Comparison-count tests compare entry access with one ordinary lookup rather than relying on noisy elapsed-time assertions. They also verify closure laziness, first-key representatives, zero/small policies, and the existing panic contract. Promotion still occurs before invoking the factory closure at the threshold; the optimization does not introduce transactional rollback.

## 4. Buffer ownership and iterator forwarding

Owning Rayon iteration over a frozen ordered set now transfers `Box<[T]>` through `into_vec()` directly to the parallel vector iterator. A sorted adaptive ordered map transfers its existing `Vec<(K, V)>` directly. The tree representation still has to produce contiguous staging storage for the existing indexed parallel-iterator contract.

Tests use non-Clone members and values, check ascending/indexed behavior, and check exactly-once destruction when a parallel `take` discards an unvisited suffix. No throughput ratio is claimed for these changes because allocation counts and end-to-end Rayon workloads were not separately benchmarked.

Set wrappers now delegate native `fold` and `count`, and ordered reverse folds, as the map iterator wrappers already did. This expresses the intended bulk operation directly. The old borrowed hash-iterator `count()` was already extremely cheap after optimization on this compiler, so the forwarding change is not advertised as an O(n)-to-O(1) measured speedup.

## 5. Next design improvements, ranked

These were reviewed but deliberately remain proposals, because each changes a workload tradeoff or public API.

| Next change | Why it can help | What must be measured or preserved |
| --- | --- | --- |
| A family-preserving retained-capacity freeze policy | Current hash family freezing always calls `shrink_to_fit`, which can add a rebuild and alter load factor. | Freeze latency, lookup misses/hits, iteration, retained memory, and regrowth. Preserve the existing compact default and the exact `seal()` route. |
| Direct small `AdaptiveFrozenHashMap` construction | Its current `FromIterator` first builds an ordinary hash table, then converts a small result into linear storage. | Small builds versus large builds, duplicate-heavy input, size hints, hashing policy, and first-key/last-value rules. |
| Owned ordered-set algebra or reusable output buffers | Borrowed algebra already produces sorted, unique members; general `collect()` repeats general construction work. | Allocation, cloning cost, output density, and representative retention. The existing `try_from_sorted_unique(Vec<T>)` already avoids sorting, while still validating. |
| Separate key/value storage as an explicit frozen-map profile | Large values increase the stride of binary search through `Box<[(K, V)]>`. | Query-heavy workloads with wide values versus iteration locality, extra allocations, index traffic, and range traversal. |
| A short synchronous snapshot callback, if needed | `SnapshotStore::load()` deliberately returns an owned `Arc`, so loading once per lookup incurs repeated ownership traffic. | Batch size and reader lifetime. Today, load once per logical operation and reuse that version. |

Do not replace stable construction sorting with unstable sorting without retaining encounter provenance: equal keys can carry different metadata, and the family promises the first representative. Rust's stable sort can already recognize sorted inputs, so the avoidable cost is not accurately described as an inevitable O(n log n) pass.

The adaptive map defaults of 8 and 32 are still uncalibrated policy values. Key/value width, equality and hashing cost, hit distribution, and promotion frequency can move the crossover. Likewise, an inline capacity should be assessed in bytes and in the number of collection objects stored together, not only in element counts.

For representation selection, use sorted frozen storage when ascending traversal and locality matter; hash storage when hash lookup semantics and chosen hashing policy are appropriate; dense bitsets when the bounded integer universe is affordable; and compressed integer sets when their native container behavior fits the workload. These are design criteria, not measured universal winners.

## Verification performed for this revision

- Unchanged input: **215** tests passed under all features.
- Revised source: **231** distinct tests passed: **3** library, **173** integration/property, **10** example tests, and **45** doctests.
- All-target, all-feature Clippy passed with warnings denied.
- Rustfmt passed.
- `alloc,fixedbitset-0_5` tests passed with default features disabled.
- Core-only library check passed.
- An actual `thumbv7em-none-eabihf` library build passed for `alloc,fixedbitset-0_5` without default features, covering the modified ordered/adaptive and dense code without host `std` leakage.
- All-feature documentation passed with warnings denied; final documentation edits also passed all 45 doctests.
- `cargo package --allow-dirty --locked --offline` packaged and verified the source successfully (101 files at that check; later added delivery patch is non-executable evidence).
- Both benchmark binaries built with `--locked --offline`; all 140 cases completed in each of four sequential runs.

The added regression suites contain 16 tests: 4 ordered, 3 bitmap, 6 adaptive-entry, and 3 Rayon ownership cases. Repeated feature runs are not counted again. Property tests count as cases, not as each generated input.

This review did not rerun the attachment's complete 19-integration matrix or its historical Miri diagnostics. No unsafe implementation changed. The embedded result is a compile check, not hardware execution; no ARM throughput, cold-cache, expensive-string, adversarial-hasher, allocator-counter, or whole-application performance claim is made.

Current logs are prefixed `performance-` in `verification/`. Earlier logs are retained as historical evidence supplied with the input. See [VERIFICATION.md](historical-verification.md) for the boundary between current and historical checks.

## Reproduce the main measurements

Run the same harness in both source revisions, using the same compiler, target, environment, and features:

```sh
cargo +1.99.1 test --all-features --locked
cargo +1.99.1 clippy --all-targets --all-features --locked -- -D warnings
cargo +1.99.1 bench --bench performance_review --all-features --locked
```

For the unmodified input, copy `benches/performance_review.rs` and its `[[bench]]` declaration from this revision into a separate baseline checkout; do not copy the optimized library source. The measured runs used `--offline` after dependency availability was established, built both binaries first, and executed them in baseline/revised/revised/baseline order on one CPU. Keep the raw per-run medians and native control rows. A large ratio against scanning the wrong side or allocating an unnecessary result is not a claim against an equally appropriate native kernel.

## External primary references

The implementation findings above come from the linked source files and executed local evidence. The following references support the native-contract distinctions:

- [Rust HashSet](https://doc.rust-lang.org/std/collections/struct.HashSet.html): iteration scans capacity in the current implementation; native intersection may select either equal representative; shrinking remains a capacity policy.
- [Rust slice sorting](https://doc.rust-lang.org/std/primitive.slice.html#method.sort): stability, construction costs, and sorted-input behavior.
- [FixedBitSet API](https://docs.rs/fixedbitset/latest/fixedbitset/struct.FixedBitSet.html): native scalar set-operation counts.
- [RoaringBitmap API](https://docs.rs/roaring/latest/roaring/bitmap/struct.RoaringBitmap.html): native intersection cardinality without materializing a bitmap.

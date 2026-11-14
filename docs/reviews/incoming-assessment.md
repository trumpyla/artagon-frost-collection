# Incoming review: integration decisions

The supplied review identifies useful workloads and a missing snapshot update
API. Its ordered patch targets the original linear-merge implementation, while
the integration base already contains seeking kernels, balanced bulk traversal,
same-storage shortcuts, dense bitmap improvements, cached adaptive lookups, and
owned Rayon buffer reuse. Replacing the tree wholesale would discard that work.
The integrated result retains the previous ordered implementation, imports and
strengthens the useful regressions, adapts the snapshot API, and replaces weak
benchmark controls. Speculative storage profiles remain deferred.

The [supplied review](supplied/adversarial-review.md) and
[patch](supplied/review.patch) are historical inputs. Their test counts, timing
claims, and proposed APIs are not assertions about this revision. The
[current validation report](../production-readiness.md) records executed gates.

## Disposition

| Finding or proposal | Decision | Reason and integration |
| --- | --- | --- |
| F1: skewed ordered algebra | **Adapt** | Preserve the prior seeking implementation. The incoming replacement regresses several existing controls; import its useful public regression scenarios. |
| F2: large sorted adaptive thresholds | **Accept finding; defer storage rewrite** | `pop_first` shifts a Vec suffix and `extend` inserts one entry at a time. Document costs and alternatives; add setup-excluding benchmarks. |
| F3/F4: snapshot guarded reads and conditional updates | **Adapt** | Add guarded reads, closure reads, conditional publication, and retrying updates with crate-owned public handles and explicit ownership/retry contracts. |
| F5: owned shared sub-slices | **Defer** | An additive profile is plausible, but boxed ownership, shared ownership, slice retention, and thaw are distinct contracts. The proposed Arc fast thaw is invalid. |
| F6: frozen hash layout | **Retain existing backend** | The supplied dense prototype does not establish a general improvement. A different layout needs a correct prototype and valid native controls. |
| F7: repeated adaptive entry searches | **Already integrated** | Keep the previous cached-search implementation and its comparison-count regressions. |
| F8: SoA/sample-index layouts | **Defer** | They need their own representation contract and benchmarks for key/value widths and cache behavior. Integer set timings do not establish a large-value map result. |
| F9: weak benchmark baselines | **Accept** | Replace scan-large/probe-small controls with standard collection APIs; retain raw samples and native hasher parity. |
| New perfect hash, rank/select, Elias–Fano profiles | **Defer** | Useful research directions, not a substitute for stabilizing and verifying the existing crate. Several complexity and cache claims need correction below. |

## Ordered-kernel comparison

The [standalone driver and exact source fixtures](../../verification/comparisons/ordered/README.md)
compile both implementations in one binary. Every answer is checked against
`BTreeSet`. The run used Rust 1.99.1, optimization level 3, one codegen unit,
thin LTO, and CPU 0 on an AMD EPYC 9V74 host. Other project builds and tests were
held during the run. These identical flags are comparison controls; the crate's
normal Cargo profile is unchanged.

Each batch is calibrated to at least 4 ms and produces seven raw samples.
Workloads run in both implementation orders. Construction is excluded, inputs
are warm, and keys are `u32`. The columns below show **both trial medians in
microseconds**, not confidence intervals. This shared host has visible outliers;
the balanced incoming scalar trial in particular varied substantially.

| Workload and operation | Prior medians, µs | Incoming medians, µs |
| --- | ---: | ---: |
| Balanced 65,536 / 65,536, scalar count | 62.657 / 63.467 | 99.535 / 161.598 |
| Balanced 65,536 / 65,536, fold checksum | 72.892 / 72.644 | 149.236 / 138.930 |
| Spread 64 / 65,536, scalar count | 2.674 / 2.783 | 2.841 / 3.023 |
| Spread 64 / 1,048,576, scalar count | 5.015 / 4.875 | 5.120 / 4.725 |
| Spread 1,024 / 1,048,576, scalar count | 72.213 / 74.710 | 91.961 / 95.844 |
| Head cluster 64 / 65,536, next checksum | 0.075 / 0.079 | 0.246 / 0.250 |
| Tail cluster 64 / 65,536, scalar count | 0.124 / 0.125 | 0.108 / 0.110 |
| Initially balanced, later long gap, count | 7.253 / 6.586 | 4.131 / 4.197 |
| Initially balanced, later long gap, next checksum | 9.248 / 8.316 | 17.715 / 16.057 |
| Ratio 31:1, scalar count | 5.873 / 5.613 | 4.978 / 4.926 |

The incoming scalar kernel helps the constructed long-gap and ratio-31 cases,
and the largest 64-key spread case is approximately tied. Those observations
remain in the evidence. They do not justify replacing balanced and bulk paths
that already perform well. The incoming version also omits the prior
same-storage scalar shortcut and the lazy separated-range iterator shortcut.
The supplied 10–15% balanced-overhead statement concerns an older baseline and
does not describe this comparison.

See all [302 measured rows](../../verification/results/ordered-strategy-comparison.csv)
and [environment/source hashes](../../verification/results/ordered-strategy-environment.json).
No hardware performance counters were collected: an explanation involving a
specific number of cold cache lines, hot B-tree levels, or branch misses is a
hypothesis, not a result of this run. The current public API benchmark is
[ordered_sets.rs](../../crates/frost-collections/benches/ordered_sets.rs).

### Regression coverage retained from the patch

[ordered_performance.rs](../../crates/frost-collections/tests/ordered_performance.rs) checks scalar
counts, explicit `next` traversal, specialized `count`, both operand directions,
and predicate work against a comparison budget well below a full large-side
scan. Separate all-present, late-missing, and all-absent predicate cases prevent
early exits from disguising linear work. Answers are asserted individually.
Existing tests preserve left representatives, lazy construction, same-storage
behavior, partial consumption, hints, folding, cloning, and fusion.

[ordered_set_ops.rs](../../crates/frost-collections/tests/ordered_set_ops.rs) adds the supplied skewed and
shared-prefix differential generators. The latter is named honestly: its
random lengths do not guarantee initially balanced operands. The incoming
private lower-bound test is not transplanted into a different implementation.

The incoming helper's statement that a bad first-probe estimate never costs
more than binary search is too strong. Exponential bracketing followed by a
binary search can require roughly twice logarithmic comparison work. Its
saturating doubling also stops advancing if a probe reaches `usize::MAX`.
That is not a demonstrated public-set bug: nonzero-sized slices are much
shorter, and a valid strictly ordered set of zero-sized values cannot supply
the required long sequence of distinct values. A future generic helper should
nevertheless state and enforce its progress bound explicitly.

## Benchmark corrections

The supplied probe has several limits that matter when interpreting its tables:

- Its `../frost` path does not match the bundled sibling `frost-patched`.
  Its thin-LTO, one-codegen-unit profile also differs from a default Cargo
  benchmark. Different profile outputs cannot be combined as a source-only
  before/after comparison.
- The `pop_first` rows time rebuilding the map plus draining it; the extension
  rows include initial construction. The new
  [adaptive benchmark](../../crates/frost-collections/benches/adaptive_maps.rs) prepares each map and
  incoming batch outside timing. Drops and allocation performed by the
  measured operation remain included. Large cases keep timer overhead small.
- The snapshot probe creates and joins threads inside the measured region and
  lacks a common start barrier. The new
  [snapshot benchmark](../../crates/frost-collections/benches/snapshots.rs) uses persistent workers,
  synchronized starts, a warmup, and checked checksums. It reports aggregate
  elapsed ns/read, including batch barrier/channel overhead, with no writer.
  It is not a per-read latency or mixed-reader/writer claim.
- A hash hit row labeled 4,096 probes actually contains only 143 at `n = 1,000`.
  The generated miss keys are not checked to be absent. The memory estimate
  `capacity * (entry_bytes + 1)` is not allocated SwissTable bytes: usable
  capacity differs from bucket count and omits layout details.
- The dense-map prototype computes a shift of 64 for zero/one-entry inputs,
  stores offsets in unchecked `u32`, and has no general duplicate policy.
  It is a limited experiment, not a production frozen-map implementation.

The common [timing support](../../crates/frost-collections/benches/support/mod.rs) preserves seven raw
samples alongside median/min/max. [set_operations.rs](../../crates/frost-collections/benches/set_operations.rs)
uses native collection controls and one deterministic hash builder. The larger
[performance_review.rs](../../crates/frost-collections/benches/performance_review.rs) remains available;
its subset answers now also come from the independent B-tree reference.

## Corrections to the proposed designs

### Adaptive deque storage and bulk extension

The current [sorted representation](../../crates/frost-collections/src/adaptive/ordered_map/mutable.rs)
has linear middle insertion/removal; draining it through repeated `pop_first`
can be quadratic at a large custom threshold. Consuming `into_iter()` avoids
those shifts for a complete ascending drain. A tree policy is suitable when
repeated ordered mutation is the workload.

A deque can improve endpoint removal, but immutable `range(&self)` cannot call
`make_contiguous(&mut self)`. Native deque `range` uses positional bounds, not
key bounds. Two-slice search and range iteration need a complete design.
Conversion to Vec may move all entries even when the deque is contiguous if
the live region starts after the allocation's beginning; contiguity alone does
not make conversion constant-time. See the
[VecDeque API and conversion contract](https://doc.rust-lang.org/std/collections/struct.VecDeque.html).

A batched sorted merge must preserve the first stored key representative, last
value, non-Clone ownership, and defined behavior when the input iterator or a
comparison panics. Collecting all input before modifying the map changes the
observable partial progress of today's entry-by-entry `extend`. Those choices
are design work, not a mechanical container substitution.

### Shared frozen profiles

Current frozen sequences already expose borrowed slices; the missing proposed
capability is independently owned shared sub-slices. `Arc<[T]>` plus a checked
`usize` range can support that capability, while retaining the whole allocation
until its final view is dropped. It would have a distinct clone/thaw contract.

`Arc::try_unwrap` returns a sized `T`; it does not extract an unsized `[T]` or
recover the Vec allocation from `Arc<[T]>`. An `Arc<Vec<T>>` or
`Arc<Box<[T]>>` design could recover its owned container when unique, with a
different representation and slicing tradeoff. The current generic
`SnapshotStore<F>` still publishes an outer Arc even when `F` contains one;
removing that layer requires a new publication design. See the
[Arc API](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.try_unwrap).

### Indexed layouts, perfect hashing, and compressed sets

A SoA map cannot retain an ordinary `&[(K, V)]` slice API. A sampled index is a
possible additive profile, but 1,048,576 `u32` keys sampled every 16 entries need
256 KiB of sample keys, before other metadata. Cache residency is
machine-dependent; one cold base cache line is not guaranteed for arbitrary
key/value widths, and the sample length is not generally a power of two.
Generated code and measured workloads must establish any branch-free or
performance claim.

The statement that a minimal perfect hash is the only layout that can beat
SwissTable is unsupported. MPHF evaluation itself can read multiple locations;
fingerprints have collisions and do not establish exact membership. Retain
the equality check, account for hash collisions among distinct keys, and define
fallible construction and input recovery before promising a production profile.

Cumulative bitmap superblocks can support rank, but do not alone prove
constant-time select. An actual select index and its bounds are required.
Roaring already provides `rank` and `select`; predecessor queries can be
expressed through its ordered range iterator. The cached 0.12.0 source confirms
these methods, and the [upstream API](https://docs.rs/roaring/latest/roaring/bitmap/struct.RoaringBitmap.html)
documents them. Elias–Fano compression depends on the universe/cardinality
ratio; a blanket 10× saving for sparse inputs is not a valid guarantee.

### Small iterator proposals

An O(1) union `last`/ordinary `max` specialization is semantically possible for
the remaining sorted cursors if equality retains the left representative and
empty/partially consumed states are handled. It must not be generalized to
`max_by` or `max_by_key` with arbitrary comparison rules. It is deferred until
there is a focused workload and regression coverage.

The existing bounded bitmap iterator remains fused after a padding member:
ascending order means all later members are also outside the logical universe.
An exhausted flag is a possible work optimization, not a semantic fix, and
adds state and a branch. Finally, eager separated-range checks in iterator
construction would violate the retained no-comparison construction behavior;
the prior implementation already performs those checks lazily on consumption.

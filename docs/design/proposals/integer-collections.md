# Proposal: further frozen integer collections

> Supplied design proposal, preserved for comparison. The implemented profile is
> described in [the current integer-set contract](../../../crates/frost-collections-integer/docs/integer-sets.md); this proposal
> does not override that contract or the general set-algebra design.

Status: proposal. Revision 2 of the integer profile implemented the first item
(chunked storage with shared containers) and the eager-kernel fixes it depends on.
The remaining items are ordered by the gap they close in the measured profile and
by what they require from the crate's constraints: safe Rust, `no_std + alloc`,
a stable compiler (no portable SIMD), and `roaring` as a dependency whose
containers are not exposed.

The [adversarial review](../../reviews/integer-profile-adversarial-review.md), the
[exploration of lazy views](../../reviews/supplied/lazy-views-exploration.md) and
the [research report](../../reviews/supplied/lazy-views-research-report.md) are the
sources for the gaps, the precedents and the cost figures quoted here.

## 1. Guiding decisions

1. **Algebra stays eager.** `union`, `intersection`, `difference` and
   `symmetric_difference` return sets. No production system surveyed answers
   `contains` on an unmaterialized expression as a primary operation; search
   engines stream through seekable cursors and Postgres materializes bitmap plan
   nodes. Lazy views are an explicit, borrowed, opt-in API (item 3).
2. **Sharing is the fix for versioned workloads**, not laziness. A chain of N
   lazy unions makes `contains` O(N); a chunked set with shared containers makes
   each update O(touched containers). Revision 2 measured 32 µs for 64 snapshot
   updates against 0.9–7 ms for every unshared representation.
3. **Laziness belongs inside materialization**: bulk append, range insertion,
   fused counting, and (for k-way unions) accumulate-then-repair.
4. **Evaluate per window, never per element**, when operands differ: Roaring's
   container-pair dispatch and tantivy's 64-document horizon are the models. With
   `roaring-rs` as a black box this is only possible for frost-owned
   representations, which is why the frozen arena (item 2) precedes views (item 3).

## 2. Catalogue

| # | Collection | Gap it closes | Memory | `contains` | Algebra | Constraints | Order |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | **Chunked storage with shared containers** | snapshot pattern, mixed-pair collapse, cheap clone | ≈ Roaring + 40 B/container | binary search + container probe | per container, shared-pointer shortcuts | safe; done in revision 2 | done |
| 2 | **Frozen chunked arena** (one allocation: key index, descriptor table, container payloads) | zero-copy views, `&'static`/mmap sets, memcpy clone, per-window evaluation for views | ≈ Roaring − per-container allocations | one binary search + container probe | same kernels as chunked, contiguous | safe; needs a descriptor format and a verification matrix | next |
| 3 | **Borrowed expression views** `IntSetView<'a, K>` | one-shot `A ∩ (B ∪ C) ∖ D` without intermediates; k-way intersections | none | one probe per leaf, short-circuiting | windowed evaluation; flattened n-ary AND/OR | needs item 2 for Roaring-class leaves; `AtomicU64` cardinality cache | after 2 |
| 4 | **Rank-indexed map** `FrozenIntMap<K, V>` = set + `Box<[V]>` | there is no integer map; the set already is a perfect index | set + rank directory (≈ 3 %) + values | set probe + `rank` | reuses set algebra; columns share one key set | safe; dense needs a rank directory | after 2 |
| 5 | **Range set** `FrozenRangeSet` | `[0, 10^12]` cannot be built; runs longer than 2^16 | 16 B/run | O(log runs) | interval sweep | safe; planner candidate when `runs ≪ n` (`statistics()` already counts runs) | independent |
| 6 | **Streaming and range constructors** | 8-byte staging per member even for dense/Roaring targets | O(target) | — | — | safe; explicit policies only, or Roaring as universal staging for `auto` | independent |
| 7 | **Sorted + fence index** (every 16th code; Eytzinger optional) | cache-missy binary search in the scattered fallback | +6.25 % | ≈ 2 cache misses | unchanged | safe | independent |
| 8 | **Sorted + minimal-perfect-hash side index** | O(1) membership with ordered iteration for scattered keys | +2.5–3.5 bits/key | 1 hash + 1 probe | merge-based | needs a dependency decision (`ptr_hash`, `ph`, `quickphf`) or an in-house CHD; build O(n) expected | later |
| 9 | **Elias–Fano / partitioned EF** | clustered mid-density data between sorted and Roaring; `next_geq` for seek-based views | n·(2 + ⌈log₂(U/n)⌉) bits | O(1 + log(U/n)) via select | galloping via `next_geq` | in-house; surveyed crates are std-only or expose `unsafe` APIs | later, memory-driven |
| 10 | **Dictionary-compressed set family** | many sets over one universe of n keys: every set becomes a dense bitmap over dictionary ranks | dictionary once + n/8 B per set | dictionary lookup + bit test | word kernels only | safe | later |
| 11 | **Approximate filter** (binary fuse) | prefilter for mixed operations; 0.4 % false positives acceptable | ≈ 9 bits/key | 3 loads | ∧ of filters | `xorf` is `no_std + alloc`; separately named approximate profile | optional |

## 3. Item 2 in more detail: the frozen chunked arena

Revision 2's chunked storage keeps one `Arc<Container>` per 2^16 window and a
boxed key index. A frozen set never needs growable containers, so the next step
is a single allocation:

```text
header | sorted keys (u16 or u48) | descriptors (type:2 | cardinality−1:16 | offset:32) | payload arena
```

Containers are `Array` (`u16 × n`), `Bitmap` (1,024 words), `Run` (`u16` pairs)
and `Full` (no payload). Benefits over both `roaring-rs` and the revision 2
chunked storage:

- `Arc<[u64]>` or `&'static [u64]` storage gives O(1) clone, embedding of constant
  tables, zero-copy deserialization and a defined on-disk layout (CRoaring's frozen
  format is the precedent; `zroar` reports large speedups from the same choice,
  self-reported).
- The key index is a flat sorted array; the scattered-`u64` fixture that costs a
  `BTreeMap` walk per probe in `RoaringTreemap` becomes one binary search.
- Views (item 3) can read a window's bits directly, which `roaring-rs` cannot offer.

Costs: a descriptor format to version, a verification matrix for the kernels
(the chunked container kernels from revision 2 carry over), and a planner candidate
decision. Sharing granularity drops from "per container" to "whole arena" unless
arenas are segmented; the research report notes that production bitmap indexes use
both granularities.

## 4. Item 3 in more detail: borrowed expression views

```rust
pub struct IntSetView<'a, K: IntKey> { /* flattened n-ary tree over &'a FrozenIntSet<K> */ }

impl<K: IntKey> FrozenIntSet<K> {
    pub fn view(&self) -> IntSetView<'_, K>;
}
impl<'a, K: IntKey> IntSetView<'a, K> {
    pub fn union(self, other: impl Into<IntSetView<'a, K>>) -> Self;        // flattens
    pub fn intersection(self, other: impl Into<IntSetView<'a, K>>) -> Self; // flattens
    pub fn difference(self, other: impl Into<IntSetView<'a, K>>) -> Self;
    pub fn contains(&self, key: &K) -> bool;        // short-circuiting leaf probes
    pub fn iter(&self) -> impl Iterator<Item = K>;  // per-window evaluation
    pub fn cardinality(&self) -> u64;               // computed once, cached atomically
    pub fn cardinality_bounds(&self) -> (u64, u64); // O(1) from operand counts
    pub fn materialize(&self, policy: IntSetPolicy) -> Result<FrozenIntSet<K>, IntSetError>;
}
impl<K: IntKey> TryFreezeWith<IntSetPolicy> for IntSetView<'_, K> {}
```

Rules that follow from the measurements and the literature:

- Flatten binary nodes into n-ary AND/OR and order leaves by cardinality; nested
  binary leapfrog loses bound propagation (Arntzenius 2025; 1.2 s versus < 1 ms
  on a 30 M-element example).
- Drive intersections from the smallest leaf and probe the others while the
  size ratio is large (the SvS result of Ding & König, Barbay et al., Culpepper &
  Moffat); switch to per-window kernels when sizes are comparable.
- Never seek through `roaring-rs`'s iterator: `advance_to` measured 5–8 ns per
  element and a leapfrog over it was 3.8× slower than probing.
- No implicit materialization on `contains`. `materialize` and `try_freeze_with`
  are explicit; `cardinality_bounds` lets callers decide.
- Views borrow (`'a`), like `family::ordered::set_ops`; no owned `'static` views
  that can be stored in a snapshot slot and grow into expression chains.

## 5. Item 6 in more detail: construction without staging

`try_from_iter` and `try_from_encoded` collect every code into a `Vec<W>` before
planning. For explicit policies, an ascending-input constructor can validate
strict ascent and canonicality online and append directly: pushes for sorted,
bit sets for dense (growing the window as the maximum advances), `append` and
`insert_range` for Roaring, per-container builds for chunked. A `from_ranges`
constructor fills containers from inclusive ranges after validating each member's
code, which makes a 10^9-member run buildable without a 8 GB staging vector.
Automatic planning needs the statistics first; Roaring can serve as the universal
staging representation for `auto` (≤ 2 bytes per member) with a conversion at the
end when the plan prefers another candidate.

## 6. Non-goals

- Patricia tries and HAMTs as representations: their sharing wins are covered by
  per-container sharing (item 1) and, later, by arenas; their merge costs are
  comparable to chunked kernels only when both operands share most subtrees.
- Sketches (HyperLogLog, Theta) for cardinality: wrong for an exact set's count;
  only useful as planning estimates across very large sets, which `cardinality_bounds`
  already covers for views.
- C-backed CRoaring: a separately named profile with its own `unsafe` review, as the
  design audit already concluded.

# Should frozen set algebra return views? — lazy evaluation, structural sharing, and the state of the art

Companion to `integer-profile-review.md`. Written before implementing the
review's fixes, because the question changes *what* gets implemented: if `a.union(&b)` returns a
view that references `a` and `b`, the "mixed pairs degrade to Sorted" problem (F1/F2) is no longer
a result-representation problem at all — it becomes a question of *when* and *how* a view is
materialized, and the eager kernels become that materializer rather than the public surface.

---

## 0. Short answer

Yes, but as three separate mechanisms, not one:

| Mechanism | What "maintain references" means | What it buys | Cost it moves |
|---|---|---|---|
| **A. Lazy views** (expression nodes borrowing operands) | `a.union(&b)` is a node `{Union, &a, &b}`; `contains`/`iter`/`cardinality` evaluate on demand | O(1) construction, no intermediate allocations in `A ∩ (B ∪ C) ∖ D`, planner-style reordering, one-shot consumers never pay for a result they read once | `contains` costs one probe per leaf; cardinality is not O(1); lifetimes tie the result to operands |
| **B. Structural sharing** (results share operand storage) | result = `Arc` to an operand when the op is an identity/subset, or shares unchanged *chunks* of operands | O(1) `A ∪ ∅`, O(1) clone, O(touched chunks) for a singleton union — the `SnapshotStore` pattern goes from 3 ms to µs; pointer-equality short-circuits (`A ∩ A' = A'` when chunks are shared) | a chunked representation frost owns (roaring-rs does not expose containers), `Arc` per chunk |
| **C. Lazy evaluation inside materialization** | k-way union accumulates without per-step cardinality; "repair" once | 30 % on Roaring unions per Lemire et al.; fused popcount; windowed SIMD evaluation | nothing visible to the API |

The eager, compact-result kernels from the review are still required: they are what a view
*materializes into*, and `contains`-heavy consumers want a materialized compact set. What the
view layer adds is the choice of when to pay for them, and the ability to pay once for a whole
expression instead of once per operator. Section 4 gives a concrete architecture; section 5 lists
the decisions I need from you before coding.

---

## 1. Precedents — lazy algebra is the norm, not the exception

- **`std::collections::BTreeSet::union`** returns a lazy merge iterator, not a set. frost's own
  `family::ordered::set_ops` does the same for `FrozenOrderedSet` (borrowing, allocation-free, with
  seeking strategies). The integer profile is currently the *only* frost family that materializes on
  every operator — an inconsistency inside the crate.
- **`fst` (BurntSushi)** — frost already wraps it. Its `OpBuilder` collects any number of ordered
  streams and `union()`/`intersection()`/`difference()`/`symmetric_difference()` return *streams*
  over a heap of cursors; nothing is materialized until consumed.
  ([docs](https://docs.rs/fst/latest/fst/raw/struct.OpBuilder.html))
- **Lucene / tantivy `DocSet`** — the search-engine model of a frozen integer set: every posting
  list exposes `advance()` and `seek(target)`; `Intersection` and `Union` are `DocSet`s themselves.
  tantivy's intersection is a leapfrog over the maximum current doc (`go_to_first_doc`), with a
  `seek_danger` fast path for cheap lower bounds, and `fill_buffer(&mut [DocId; 64])` for batch
  evaluation; cardinality is *estimated* from operand sizes for planning
  ([intersection.rs](https://docs.rs/crate/tantivy/0.26.1/source/src/query/intersection.rs),
  [docset.rs](https://docs.rs/crate/tantivy/0.26.1/source/src/docset.rs),
  [size_hint.rs](https://docs.rs/crate/tantivy/latest/source/src/query/size_hint.rs)).
- **Leapfrog (Veldhuizen, LogicBlox)** — k-way intersection of sorted seekable iterators in
  O(N_min · log(N_max/N_min)); the unary case is exactly our problem
  ([arXiv 1210.0481](https://arxiv.org/pdf/1210.0481)). Arntzenius (miniKanren 2025) shows the
  catch for a view *tree*: nesting binary intersections loses the lower-bound propagation that makes
  leapfrog fast, so k-way intersections must be flattened and evaluated with a `seek(bound)`
  interface that propagates bounds even when no element is yielded
  ([arXiv 2510.26016](https://arxiv.org/pdf/2510.26016)).
- **Roaring's own laziness** — "lazy union" skips per-container cardinality maintenance during
  k-way aggregation and repairs once at the end; Chambi et al. measured the eager bit-count at
  ~30 % of union time ([Lemire et al., arXiv 1603.06549](https://arxiv.org/pdf/1603.06549)).
  roaring-rs exposes this as the `MultiOps` trait. CRoaring additionally has a copy-on-write mode
  (`-c` in the bitmap competition harness) in which results share unchanged containers with inputs,
  and a *frozen view* format for zero-copy use.
- **Pilosa/FeatureBase Roaring** — `Freeze()` returns a shallow copy whose containers are shared
  with the parent and marked frozen; `OffsetRange` shares containers too
  ([pkg.go.dev](https://pkg.go.dev/github.com/pilosa/pilosa/roaring)). This is mechanism B at
  container granularity in a production bitmap index.
- **Okasaki & Gill, "Fast Mergeable Integer Maps" (1998)** — big-endian Patricia tries whose
  merge operations exploit shared subtrees: physically equal subtrees are skipped, so unions and
  intersections of related versions cost O(difference) instead of O(size). Modern implementations
  (CEA's `patricia-tree`, Go's `x/tools` VTA trie) add hash-consing for O(1) equality; they are the
  standard structure for program-analysis domains where many related sets are merged constantly
  ([patricia-tree](https://ocaml.org/p/patricia-tree/0.13.0/index.html),
  [x/tools trie](https://godocs.io/golang.org/x/tools/go/callgraph/vta/internal/trie)).

So the integer profile's eager-only API is the outlier. The question is not whether to add views,
but how to make them cheap to *consume* given three physical representations.

---

## 2. What a view needs from the representations: `seek`

Every lazy evaluation strategy above rests on one primitive: `seek(x) → first member ≥ x`
(Lucene `advance`, leapfrog `seek`, Elias–Fano `next_geq`). Its cost per representation:

| Representation | `seek(x)` | Note |
|---|---|---|
| Sorted | exponential search from the cursor, O(log gap) | `set_ops::skip_below` already does this |
| Dense | jump to word `(x−base)/64`, mask low bits, `trailing_zeros`, scan forward over zero words | O(1 + zero words skipped); the zero-word scan is the weak spot for sparse windows (explicit `Dense` policy only) |
| Roaring (roaring-rs) | `Iter::advance_to(x)` | **Measured in the review: 5–8 ns per yielded element; a leapfrog over it was 3.8× slower than probing `contains`** (70 µs vs 18.6 µs on the balanced fixture). roaring-rs's iterator is not a cheap cursor. `rank`/`select` are native but O(containers · log) each. |
| Elias–Fano (proposed N6) | `select₁(high(x)) − high(x)` then a bounded scan, O(1 + log(U/n)) | the reason EF is the compressed list format of choice for leapfrog-style query processing ([Pibiri & Venturini, CSUR 2020](https://arxiv.org/pdf/1908.10598.pdf)) |
| Tree-Encoded Bitmap (Lang et al., SIGMOD 2020) | tree descent, logarithmic | compressed bitmaps with random access designed so that intersection can skip; best space in 7 of 8 real data sets in their evaluation ([paper](https://db.in.tum.de/~lang/papers/tebs.pdf)) |
| Chunked arena (proposed N1) | binary search over chunk keys, then container-local seek | frost would own the containers, so a view can read a 2^16 window's bits directly — the thing roaring-rs cannot give us |

The consequence for design: a view cannot have one evaluation strategy. For Sorted/Dense leaves,
seek-based leapfrog is right; for a Roaring leaf, probing `contains` from the other side is right
(the review's direction rule), and for Roaring-heavy expressions the best path is to *promote*
operands into Roaring and let its container algebra do the work (20× in the review's Dense/Roaring
measurement). tantivy encodes the same reality with specialized scorers per posting-list type.

---

## 3. Windowed evaluation: the hybrid that makes views fast

Element-at-a-time lazy evaluation is what makes "lazy" sound slow. The state of the art does not
evaluate element-wise:

- tantivy's `Union` processes a 64-document *horizon* at a time with a `TinySet` (one `u64`
  bitmap), and `fill_buffer` hands consumers 64 doc ids per call.
- Roaring's algebra is container-wise: the two operands' containers for the same 16-bit key are
  combined with a kernel chosen by their types.

A view over frost's integer profile should therefore evaluate **per 2^16-aligned window**: for
each window key present in any operand, obtain each operand's window as either (a) a 1,024-word
dense slice (Dense leaf, or Roaring bitmap container once frost owns containers), (b) a short sorted
`u16` slice (Sorted leaf restricted by two `partition_point`s; Roaring array container), or (c) a
run list; combine with the kernel for that pair; yield the window's members or count. This is lazy
at the window level (no result allocation, operands untouched) and SIMD-dense inside the window.
It also makes k-way unions natural (OR all operand windows into one 8 KiB scratch bitmap, popcount
once — the Roaring "lazy OR + repair" idea applied window by window) and keeps the `u16` kernels
in L1.

With roaring-rs as a black box this is only partly possible (its iterator is the only window
accessor), which is the strongest argument in this document for the review's N1 proposal: a
frozen chunked arena in which frost owns container bytes. Zig's `zroar` makes the same argument
from the serialization side — its in-memory format *is* the on-disk format, no per-container
allocation, and it reports 2–9× over CRoaring on its own benchmarks (a forum claim, not reproduced
here; [ziggit thread](https://ziggit.dev/t/zroar-serialized-roaring-bitmaps-in-zig-outperforms-croaring-2x-to-9x/17321)).

---

## 4. Proposed architecture

Four layers; the first two are what the review already asked for.

### Layer 0 — eager kernels (review §3.1–3.6)

Promotion matrix, Roaring fallback for over-cap dense unions, carried budgets, fused dense kernels,
same-representation fast paths. Unchanged by this document; they become the *materializer* and
are also what a view falls back to when a window pair has no cheap kernel.

### Layer 1 — `Arc` storage and identity sharing

`Storage<W>` becomes `Arc<StorageInner<W>>` (or `FrozenIntSet` holds `Arc<Storage>`). Then:

- `Clone` is O(1), and "cloning clones storage" leaves the design document.
- Identity results share: `A ∪ ∅`, `A ∩ A`, `A ∖ ∅` return the operand's `Arc`.
- Subset results share when detected *for free*: `A ∩ B` whose count equals `|A|` returns `A`;
  `A ∪ B` whose count equals `|B|` returns `B`. (`intersection_len` is already computed for the
  overflow guard in `union`; make it pay for itself.)
- `SnapshotStore<FrozenIntSet<K>>` stops deep-copying on every `load_full`.

### Layer 2 — views

```rust
/// A lazy set expression over borrowed frozen integer sets.
pub struct IntSetView<'a, K: IntKey> { node: Node<'a, K>, cardinality: AtomicU64 /* u64::MAX = unknown */ }

enum Node<'a, K: IntKey> {
    Leaf(&'a FrozenIntSet<K>),
    Union(Vec<IntSetView<'a, K>>),          // flattened k-way, smallest-first
    Intersection(Vec<IntSetView<'a, K>>),   // flattened k-way, smallest-first
    Difference(Box<IntSetView<'a, K>>, Box<IntSetView<'a, K>>),
    SymmetricDifference(Box<IntSetView<'a, K>>, Box<IntSetView<'a, K>>),
}

impl<'a, K: IntKey> FrozenIntSet<K> {
    pub fn view(&self) -> IntSetView<'_, K>;
}
impl<'a, K: IntKey> IntSetView<'a, K> {
    pub fn union(self, other: impl Into<IntSetView<'a, K>>) -> Self;          // flattens
    pub fn intersection(self, other: impl Into<IntSetView<'a, K>>) -> Self;   // flattens
    pub fn difference(self, other: impl Into<IntSetView<'a, K>>) -> Self;
    pub fn symmetric_difference(self, other: impl Into<IntSetView<'a, K>>) -> Self;
    pub fn contains(&self, key: &K) -> bool;          // one encode, then leaf probes with short-circuit
    pub fn iter(&self) -> impl Iterator<Item = K>;     // windowed evaluation, decode on yield
    pub fn cardinality(&self) -> u64;                  // computed once (windowed count), cached
    pub fn cardinality_bounds(&self) -> (u64, u64);    // O(1): from operand counts, planner-style
    pub fn is_empty(&self) -> bool;                    // short-circuits on the first window
    pub fn materialize(&self, policy: IntSetPolicy) -> Result<FrozenIntSet<K>, IntSetError>;
}
impl<K: IntKey> TryFreezeWith<IntSetPolicy> for IntSetView<'_, K> { /* = materialize */ }
impl<K: IntKey> SetContains<K> for IntSetView<'_, K> {}
impl<K: IntKey> ValueElements for IntSetView<'_, K> {}
impl<K: IntKey> Cardinality for IntSetView<'_, K> {}
```

Design points:

- **Borrowing, like `set_ops`.** The view carries `'a`; this is the frost idiom and keeps views
  allocation-free. A `SharedIntSetView<K>` over `Arc<FrozenIntSet<K>>` is a thin variant for
  `SnapshotStore` composition.
- **Flatten and order.** `a.view().intersection(b).intersection(c)` becomes one 3-way node with
  leaves sorted by cardinality; this is the Arntzenius/leapfrog result (nested binary nodes lose
  bound propagation) and the Roaring "intersect smallest first" rule.
- **Evaluation is per window and per leaf type** (section 3). Dense/Sorted leaves give window
  slices directly; a Roaring leaf is accessed through `contains` when it is a *probe target* and
  through its iterator only when it is the smallest operand of a union; when two or more Roaring
  leaves dominate an expression, `materialize` promotes the others into Roaring and uses native
  container algebra (Layer 0).
- **Cardinality is computed once and cached** in an `AtomicU64` with `u64::MAX` as the
  unknown sentinel (`alloc`-compatible, `Sync`, no lock). `cardinality_bounds` is O(1) and is what a
  caller should use for planning: `|A ∩ B| ≤ min`, `max ≤ |A ∪ B| ≤ sum`, `|A| − |B| ≤ |A ∖ B| ≤ |A|`.
- **No hidden auto-materialization.** The crate's philosophy is observable cost. A view that
  silently materialized after N probes would be a hidden O(n) allocation on a `contains` call.
  Instead: `materialize`/`try_freeze_with` are explicit, and the documentation gives the rule of
  thumb — *views for one-shot consumption and for composing before consuming; materialize for
  repeated membership tests or reuse across operations.*
- **Eager methods stay.** `FrozenIntSet::union` etc. remain and are documented as
  `self.view().union(other).materialize(policy_from_operands)`.

### Layer 3 — chunked arena with per-chunk `Arc` (review N1 + N7)

Once frost owns containers, Layer 2's windowed evaluation reaches Roaring-class data, and Layer 1's
sharing becomes structural: a union touching one chunk of a 4,096-chunk set shares 4,095 `Arc`s.
Okasaki–Gill short-circuits apply at chunk granularity (`Arc::ptr_eq(a_chunk, b_chunk)` ⇒ ∩ and ∪
return the chunk, ∖ and ⊕ return empty). This is the layer that makes versioned workloads
(`SnapshotStore` updates, many sets over one universe) O(changed chunks) in both time and memory.
Hash-consing chunks for O(1) structural equality is possible later but not needed to get the sharing.

---

## 5. Decision matrix

| Workload | Eager (today, fixed) | View (Layer 2) | View + arena sharing (Layer 3) |
|---|---|---|---|
| Iterate `A ∩ B` once | O(n+m) + alloc + drop | O(windows) no alloc — wins | same |
| `contains` ×1,000 on `A ∪ B` | O(n+m) once, then 1,000 × O(1) | 1,000 × 2 probes — loses unless n+m ≫ 2,000 | same |
| `A ∩ (B ∪ C) ∖ D` consumed once | 3 intermediate sets | 0 intermediates — wins | same |
| `current = current ∪ {x}` ×1,000 (snapshots) | 1,000 × O(n) copies | views chain → depth 1,000, unusable | O(1 chunk) each — wins |
| Many sets over one universe, ad-hoc algebra | per-op allocations | fine | shared chunks, pointer short-circuits — wins |
| Cardinality of an expression | O(1) after materialization | O(windows) first time, cached | same |

Recommendation, in order:

1. **Implement Layer 0 and Layer 1 now** (the review's fixes plus `Arc` storage and identity/subset
   sharing). Nothing in the view design is harmed by them, and `SnapshotStore` users get most of the
   practical win (O(1) clone, no deep copy on read) immediately.
2. **Add Layer 2 views** with Sorted/Dense windowed evaluation and probe-based access to Roaring
   leaves; measure against eager on the three workload shapes above (one-shot iterate, repeated
   `contains`, chained expression) and record the crossover.
3. **Then Layer 3** as the frozen chunked arena, which the review already motivated independently
   (scattered `u64`, zero-copy, cheap clone) and which is the only route to windowed evaluation over
   Roaring-class data without roaring-rs exposing containers.
4. **Defer** hash-consing, Patricia tries as a representation (their win is in sharing-heavy merge
   workloads that Layer 3 already covers at chunk granularity), and sketches for approximate
   cardinality (Theta/HLL — only if someone needs `|A ∪ B|` without touching data).

---

## 6. What I need from you before coding

1. **Views borrow (`'a`) or share (`Arc`)?** My default: borrowing views as the primary type,
   mirroring `set_ops`, with an `Arc`-based variant for snapshot composition.
2. **Does `FrozenIntSet::union` stay eager?** My default: yes — keep the eager methods, add
   `view()`; the profile is unreleased so a std-style switch to lazy-by-default is possible, but it
   would make the integer family behave differently from `FrozenOrderedSet::union` (which returns
   an iterator, not a set) in the opposite direction.
3. **Any automatic materialization?** My default: none; explicit `materialize`/`try_freeze_with`
   plus `cardinality_bounds` for callers who want to decide.
4. **Take on the frozen chunked arena (Layer 3) in this pass, or stop after Layer 2?** The eager
   fixes and views are a contained change set (≈ 1.5 k lines with tests); the arena is a new
   representation with its own planner candidate, serialization story, and verification matrix.

If you confirm the defaults I will proceed in the order above and record the view-vs-eager
crossover measurements alongside the review's benchmark tables.

---

## Sources

- Veldhuizen, *Leapfrog Triejoin: a worst-case optimal join algorithm* — https://arxiv.org/pdf/1210.0481
- Arntzenius, *Fair intersection of seekable iterators* (miniKanren 2025) — https://arxiv.org/pdf/2510.26016
- Lemire, Ssi-Yan-Kai, Kaser, *Consistently faster and smaller compressed bitmaps with Roaring* — https://arxiv.org/pdf/1603.06549
- Lemire et al., *Roaring Bitmaps: Implementation of an Optimized Software Library* — https://arxiv.org/pdf/1709.07821
- Pibiri & Venturini, *Techniques for Inverted Index Compression* (ACM CSUR 2020) — https://arxiv.org/pdf/1908.10598.pdf
- Lang, Beischl, Leis, Boncz, Neumann, Kemper, *Tree-Encoded Bitmaps* (SIGMOD 2020) — https://db.in.tum.de/~lang/papers/tebs.pdf
- Okasaki & Gill, *Fast Mergeable Integer Maps* (1998), via the `patricia-tree` OCaml library — https://ocaml.org/p/patricia-tree/0.13.0/index.html — and Go `x/tools` VTA trie — https://godocs.io/golang.org/x/tools/go/callgraph/vta/internal/trie
- tantivy `Intersection`, `DocSet::seek`, size estimation — https://docs.rs/crate/tantivy/0.26.1/source/src/query/intersection.rs , https://docs.rs/crate/tantivy/0.26.1/source/src/docset.rs , https://docs.rs/crate/tantivy/latest/source/src/query/size_hint.rs
- `fst::raw::OpBuilder` lazy set-operation streams — https://docs.rs/fst/latest/fst/raw/struct.OpBuilder.html
- Pilosa Roaring `Freeze`/`OffsetRange` container sharing — https://pkg.go.dev/github.com/pilosa/pilosa/roaring
- CRoaring bitmap competition (copy-on-write flag) — https://awesome.ecosyste.ms/projects/github.com%2Froaringbitmap%2Fcbitmapcompetition
- zroar (flat serialized 64-bit Roaring in Zig; self-reported numbers) — https://ziggit.dev/t/zroar-serialized-roaring-bitmaps-in-zig-outperforms-croaring-2x-to-9x/17321
- Review measurements (Roaring iterator seek cost, Dense→Roaring promotion) — `integer-profile-review.md` §2.2, harness `verification/int-set/revision2/review-probe/`

<!-- Supplied in chat. Preserved as research input, not adopted project instructions. Verification and design decisions are recorded separately. -->

# Lazy Views vs. Structural Sharing for `FrozenIntSet`: Research Findings and a Design for frost-collections

Do not make `union`/`intersection` return lazy views by default: keep set algebra eager, and change *what* it materializes. Make results share storage with their operands at container (chunk) granularity, behind `Arc`, with copy-on-write and pointer-equality shortcuts. Offer lazy expression views only as an explicit, borrowed, opt-in API for one-shot queries such as "count/iterate A∩B∩C without allocating".

## TL;DR

- **The review's three problems are representation and sharing problems, not laziness problems.** The 3.3 ms dense∪singleton blow-up, the deep-copy `Clone`, and the snapshot-store copy every update are all fixed by a chunked, `Arc`-per-container representation (a "persistent Roaring" written in safe Rust). Updating it costs O(#chunks) pointer copies plus one container copy. Lazy views would make the snapshot-store pattern *worse*: N chained `union(&singleton)` views turn `contains` into an O(N) walk.
- **Lazy evaluation pays off *inside* materialization, not as the public result type.** Roaring's lazy union skips per-step cardinality maintenance and repairs once at the end. Chambi et al. measured that maintenance as "a speed penalty of about 30%" in Java, adding that it "effectively disappears" in optimized C. Tantivy and Lucene evaluate expressions through seekable cursors, but they never answer `contains` on a lazy result. Cardinality is either computed exactly by consuming the expression, or estimated with `cost()`/`size_hint()` heuristics for planning.
- **First addition: your own Roaring-style chunked representation, not Elias–Fano.** Use 2^16-value windows with Array/Bitmap/Run containers, each `Arc`-shared and with its own cheap `seek`. This removes the roaring-rs black-box limits (no container sharing; `advance_to` at 5–8 ns/element) and makes per-window mixed-representation evaluation possible. Elias–Fano is a later, memory-driven addition. None of the surveyed Rust EF crates is documented as `no_std`.

## Key Findings

### 1. Lazy / deferred set algebra in production systems

| System | Result representation | `contains` on lazy result? | Cardinality | When it materializes | Measured trade-offs / notes |
|---|---|---|---|---|---|
| Rust `BTreeSet::union`/`intersection` | Borrowing iterator (`Union<'a, T>`) over two operand cursors | No: iterator only | Only by consuming (`count()`) | Caller `collect()`s | Zero allocation, binary only, so k-way needs nesting (see Arntzenius below) |
| `fst` `OpBuilder` | Streams merged over several FSTs (union/intersection/difference/symmetric difference) | No | By consuming the stream | Caller builds a new FST from the stream | Tantivy's term merger relies on the FST union feature (tantivy 0.15 changelog) |
| Lucene `DocIdSetIterator` / `ConjunctionDISI` | Tree of cursors with `nextDoc()`/`advance(target)`; `TwoPhaseIterator` splits a cheap approximation from an expensive confirmation | No; membership is only via `advance` | `cost()` is "generally an upper bound … but may be a rough heuristic, hardcoded value, or otherwise completely" inaccurate | Never, by design; results stream to collectors | `advance` with target ≤ current is undefined. The conjunction leads with the cheapest iterator |
| tantivy `DocSet`/`Scorer` | Cursor trait: `advance`, `doc`, `size_hint`, `seek`, `seek_danger`, `fill_buffer(&mut [DocId; 64])`, `cost`, `count` | No | `size_hint`/`cost` estimates; `count()` consumes | Never; streams | 0.26 sped up non-scored unions using `fill_buffer` plus an optimized `TinySet` (PR #2863), and added `seek_danger` for faster intersections (#2538/#2810). `seek_danger` then caused a false-positive bug inside `BufferedUnionScorer` that was *disabled* in 0.26.2 (#3086/#3088). A separate false-negative bug with nested unions under an intersection was reported in 0.26.0 (#3103). PR #3224 picks `SimpleUnion` when estimated density is ≤128 hits per 4,096 docs (3.125%) and `BufferedUnionScorer` otherwise |
| PostgreSQL `BitmapAnd`/`BitmapOr` | Each plan node *materializes* a `TIDBitmap` (hash of per-page bitmaps) | Yes, because it is materialized | Exact while exact; becomes page-level "lossy" when it exceeds `work_mem` (default 4 MB), forcing a recheck | Always eager per node | Degrading under a memory budget (exact→lossy) is the main idea; since PG 11 the cost model includes the expected lossy fraction |
| Arntzenius 2025 (miniKanren) | Seekable iterators with a *position* plus an idempotent `seek` that includes the current key | Via `seek(x)` and position check | n/a | n/a | Nested binary leapfrog lost bound propagation: `(evens ∩ odds) ∩ evens` over 30M-element arrays took 1.2 s, versus under 1 ms with bound-propagating `Seek` iterators (or after reassociation) |

**Takeaways:**

- No mature system answers `contains` on an *unmaterialized* expression as a primary operation. Search engines don't need it, and Postgres materializes.
- Every system that streams relies on (a) a cost or size estimate to order children, and (b) batch or horizon evaluation to amortize per-element dispatch. Tantivy's 64-doc `fill_buffer` and `TinySet` horizon is the clearest example.
- Tantivy's 2025–2026 `seek_danger` bugs show how hard nested seekable-union semantics are to get right. Expect correctness costs if frost builds a general lazy expression engine.

### 2. k-way intersection over seekable iterators

- **Interface required.** Leapfrog Triejoin and adaptive algorithms (Demaine–López-Ortiz–Munro; Barbay–Kenyon) need `seek(bound)` / `next_geq(x)` plus a current position. Arntzenius's fix makes `seek` idempotent and lets a sub-iterator report a *lower bound* without yielding a key. That way an inner intersection can pass "nothing below x" up to the outer one, and nested binary nodes keep k-way performance.
- **What the experiments say.** Lemire, Boytsov and Kurz (2016) summarize the literature:
  - Ding & König found SvS (smallest-vs-smallest, pairwise) with galloping competitive, and Baeza-Yates and adaptive algorithms *slower*.
  - Barbay et al. found Baeza-Yates ~30% faster on uniform synthetic data, but SvS+galloping ~2× faster than most alternatives on real data (TREC GOV2).
  - Culpepper & Moffat also found SvS+galloping fastest.
  - Lemire et al.'s SIMD intersections (V1, V3, SIMD Galloping) are "often twice as fast as the best non-SIMD algorithms". In their tests on the TREC GOV2 and ClueWeb09 (Category B) collections, "S4-BP128-D4 … uses as little as 0.7 CPU cycles per decoded 32-bit integer".
- **Implication for frost.** Your measurement that leapfrog over roaring-rs `advance_to` was 3.8× slower than probing `contains` is consistent with the literature. When one operand is small, iterate the smallest operand and probe the others (SvS with O(1) or O(log) probes). Leapfrog/seek-based k-way only pays off when *every* operand has a cheap seek. Seek costs differ by representation:

| Representation | Cheap `seek`? | Notes |
|---|---|---|
| Sorted array | Yes | Galloping, O(log d) in the skip distance d |
| Bitmap | Yes | Word scan with `trailing_zeros`, O(gap/64); a summary layer makes it O(1)-ish |
| Elias–Fano | Yes | Select on the high bits |
| roaring-rs | No | Black box: `advance_to` is opaque and 5–8 ns/element |

### 3. Structural sharing and persistence

- **Okasaki & Gill / Patricia tries.** Merges detect physically equal subtrees and return them unchanged. The OCaml `patricia-tree` library documents that `idempotent_inter_filter` "will skip recursive calls to physically equal subtrees" and that physical equality is "preserved as much as possible". Its hash-consed variants make equality and comparison constant-time, at a constant memory and constructor-time overhead.
- **Haskell `Data.IntSet`.** It puts machine-word bitmaps in `Tip` leaves. This "greatly reduce[s] memory footprint and execution times for dense sets", and many operations are O(min(n, W)). This is the same "bitmap leaf under a sharing-friendly tree" pattern recommended for frost, at 64-bit rather than 2^16 granularity.
- **CRoaring copy-on-write.** Shared containers use atomic reference counts. The docs warn that all interacting bitmaps should use the same COW setting, since "interactions between bitmaps with and without COW is unsafe".
- **Sharing is bug-prone in practice.**
  - CRoaring issue #927 (fixed in PR #928, Oct 2026): frozen serialization of COW bitmaps with shared containers was undefined behavior.
  - CRoaring issue #931: `ra_overwrite` left invalid bitmaps on allocation failure in the COW path.
  - CRoaring PR #946: `roaring_bitmap_frozen_view` accepted headers claiming more than 65,536 containers.
  - Java `CopyOnWriteRoaringBitmap` could write into its *source* bitmap because its per-index "shared" flags went stale when containers shifted. PR #874 fixed this by tracking shared containers in an identity set, and PR #885 adds a `PersistentRoaringBitmap`.
  - Lesson: in C and Java, sharing invariants are hand-maintained. In safe Rust, `Arc::make_mut` and `Arc::ptr_eq` enforce them through the type system, which is a strong argument for doing this in frost rather than reaching into CRoaring.
- **Rust persistent collections.** `imbl` `OrdSet` is a B+tree with `Arc`-shared nodes. The `pds` fork (single README source) documents O(1) `ptr_eq` short-circuits for HAMT sets: union returns one copy, difference returns empty, intersection returns one copy. These short-circuits are cheap to add and pay off exactly in the "start from a snapshot, change one thing, combine back" pattern.
- **Granularity that works.** Word-level (Haskell `Tip`), 2^16-value containers (CRoaring COW, Java COW/persistent Roaring), and B-tree nodes (imbl) are all used in production. For a frozen integer set with a dense window type, 2^16-value containers (≤8 KiB bitmaps) are the natural sharing unit. They line up with Roaring's container boundaries and the evaluation window (see §6).

### 4. Lazy evaluation inside materialization

- **Measured cost of cardinality maintenance.** In "Better bitmap performance with Roaring bitmaps" (arXiv 1402.6407), Chambi, Lemire, Kaser & Godin report that computing the cardinality during bitmap-container unions drops throughput to about 500 million words per second: "we suffer a speed penalty of about 30% because we maintain the cardinality". That figure is for the Java version; they note the penalty "effectively disappears when using an equivalent optimized implementation written in C". Lemire et al. (arXiv 1603.06549) therefore use a "lazy union": "the cardinality is set to the flag value '-1', indicating that the cardinality is currently unknown", and "after the final answer is generated, we 'repair it' by computing the cardinality of any bitmap container". They also predict the output container type to avoid conversions; for example, if two arrays' cardinalities sum to more than 4,096, the union is written directly into a bitmap container.
- **Naïve vs. heap-based k-way union.** Lemire et al. found "no clear winner": priority-queue was better on Wikileaks, naïve better on CensusInc/Census1881. In RoaringBitmap PR #211 Lemire states "the PriorityQueue approach is generally not best"; grouping containers by key first ("groupedOr") was much faster than `priorityqueue_or`.
- **Practitioner evidence (single PR, flag).** Apache Pinot PR #19587 reports that switching bitmap folds to RoaringBitmap's lazy `or` plus a single repair made folds 2.2–2.7× faster with ~flat allocation. It deliberately chose `lazyor` over `naivelazyor` because the latter promotes containers to 8 KiB bitmaps on first overlap and "can transiently retain hundreds of MB".
- **roaring-rs caveat.** Issue #377 reports that `MultiOps::union` promotes *any* overlapping array pair to a bitmap with no size check. CRoaring and Java only do this above `ARRAY_LAZY_LOWERBOUND = 1024`. So roaring-rs's k-way lazy union can bloat memory on sparse inputs. That is one more reason to own the container layer.

### 5. Frozen integer set representations: comparison

Bits/element figures are from the cited papers or docs; "—" means no reliable figure was found.

| Structure | Memory / element | `contains` | `seek`/`next_geq` | ∩ / ∪ cost | Rust availability; `unsafe` / `no_std` |
|---|---|---|---|---|---|
| Sorted `u32` array (current) | 32 bits | O(log n) binary search; Eytzinger layout faster for large n (Khuong & Morin) | Galloping O(log d) | SvS/galloping O(m log(n/m)); merge O(m+n) | Trivial in safe `core` |
| Eytzinger / static B-tree layout | 32 bits (+ padding) | Khuong & Morin (arXiv 1509.05053): sorted order with good binary search is best for small n; for larger n "the Eytzinger layout is usually the fastest" | Successor search is natural; *ordered iteration* needs the sorted copy too | Poor for merges | Safe to implement; `aozora_veb` crate exists (unaudited) |
| Dense bitmap window (current) | U/n bits (1 bit when full) | O(1) | O(gap/64) word scan | O(U/64) word ops, auto-vectorizable; fused popcount | Safe `core` |
| Hierarchical bitmap (hibitset) | ≈1 + 1/64 + … bits per universe slot | O(1) | Skips empty ranges through summary layers | Fast whole-set ops via summary pruning | `hibitset` 0.6.4: Rust 2015, last release 2023, **panics above usize^4 (16,777,216 on 64-bit)**; `no_std`/`unsafe` unverified |
| Roaring (2^16 containers) | Array: 16 bits (2c+2 bytes); bitmap: 8 KiB per container (≤16 bits/element at the 4,096 threshold); run: 32 bits per run | Binary search on keys, then O(1) or O(log) | Per container; cheap in principle, but roaring-rs `advance_to` measured 5–8 ns | Container-pair dispatch; lazy union + repair | `roaring` 0.12 (dependency, black box); `croaring` (FFI, not no_std) |
| Elias–Fano | ≤ ⌈log2(U/n)⌉ + 2 bits (Pibiri & Venturini) | Via `next_geq` | O(1) expected via select on high bits | Seek-driven SvS | `vers-vecs` (no unsafe without `simd` except intrinsics; std), `sucds` (pure safe Rust; 64-bit; `no_std` unstated), `sux` (unchecked `unsafe` APIs; heavy dependencies); none documented `no_std` |
| Partitioned EF (Ottaviano & Venturini) | Smaller than EF on clustered data; dense chunks become bitmaps | As EF | As EF, plus a partition directory | Trie-compressed sets were 1.55–3.32× faster than PEF at 1.15–1.72× its space | Rust `RISE` library (arXiv 2606.07187) implements EF/PEF/BIC/StreamVByte; `pefcomp` cut index size ~36% |
| Trie-compressed intersectable sets (Arroyuelo & Castillo) | 0.48–1.01× Roaring's space | Trie descent | Trie navigation | 1.07–1.91× faster than Roaring; adaptive O(kδ lg(u/δ)) | C++ artifact only |
| Tree-Encoded Bitmaps (Lang et al., SIGMOD 2020) | Best compression in 7 of 8 real data sets; up to 1/3 less space | Efficient random access via the tree | Tree navigation | Fast intersection via random access | No Rust implementation found |
| SIMD-BP128 / Stream-VByte / PFor | Few bits/int; S4-BP128-D4 decodes at ~0.7 cycles/int | Block decode needed | Skip blocks via block maxima | Decode-then-intersect; SIMD ~2× faster | Fast variants need SIMD; stable Rust needs `unsafe` `std::arch` (banned) or relies on autovectorization |
| Rank/select bitvector directories | +3.125% (rank) / +3.51% (rank+select) for pasta-flat/cs-poppy; +25% for rank9; +2.297% for orzo | O(1) | O(1) select-based | Bitmap ops | `vers-vecs`, `sucds`, `sux`; see above |
| MPHF side index (PTHash/ptr_hash/PHast) | ~2–3 bits/key, *plus* the stored keys for verification | O(1) hash + verify | None (unordered) | None | `ptr_hash` (3.0 bits/key default; 8–12 ns/key queries; status unverified), `ph` (PHast below 2 bits/key), `quickphf` (`#ǃ[no_std]`, `#ǃ[forbid(unsafe_code)]`, compile-time only) |
| Binary fuse filter | ≈9 bits/entry (BinaryFuse8), FPR ≈2^-8 | Probabilistic: negatives only | None | None | `xorf`: `#ǃ[no_std]` + `alloc`, no nightly; disable the default `uniform-random` feature |
| Patricia trie with bitmap leaves (`Data.IntSet`) | Several words per node; dense-friendly via 64-bit tips | O(min(n, W)) | Tree walk | Merges skip shared subtrees | No mainstream safe Rust equivalent found |
| CUBIT (VLDB 2025) | Bitmap index plus update deltas | — | — | 3–16× throughput and 3–220× lower latency vs. updatable bitmap indexes | For *updatable* indexes; not relevant to a frozen profile beyond its snapshot idea |

On the bitmap-vs-list question, Wang et al. (SIGMOD 2017) compared 9 bitmap and 12 inverted-list compression methods:

- They recommend Roaring "whenever possible" among bitmaps.
- They found the uncompressed bitset dominated by Roaring in almost all cases.
- They found SIMDPforDelta and SIMDBP128 to be the best inverted-list codecs.
- Many RLE bitmaps (WAH, EWAH) can exceed the size of the original list.

### 6. Windowed / blocked hybrid evaluation

The evidence favors per-window evaluation over element-at-a-time laziness:

- Roaring dispatches per 2^16 container pair, with type-specific kernels (array×bitmap in O(|array|), bitmap×bitmap word loops) and output-type prediction.
- Tantivy's union evaluates a 64-document horizon into a `TinySet` and drains it in 64-doc `fill_buffer` batches. It then (PR #3224) picks the union algorithm from estimated density.
- Block-max structures (128-int blocks with block maxima) let intersections skip whole blocks.
- Lemire et al. found schemes that work on ~128-integer blocks with minimal branching to be the fastest.

Mixed representations are handled *per window*: each side presents its window as array, bitmap or run, and a small kernel matrix computes the window result. This is exactly what frost lacks today; a mixed op falls back to a global Sorted result.

### 7. Cardinality of lazy expressions

- **Exact, computed once and cached.** This is right whenever the expression is reused. Fuse it with evaluation: popcount while ANDing words, the way Roaring computes cardinality during bitmap-container ops. In `no_std`, `core::cell::OnceCell` works for `!Sync` views. A `Sync` cache needs an atomic or a `once_cell`-style `race` primitive.
- **O(1) bounds.** max(|A|,|B|) ≤ |A∪B| ≤ |A|+|B| and |A∩B| ≤ min(|A|,|B|). These are enough for planning: choosing the driving operand, choosing the output container type, and pre-sizing allocations. Per-window bounds are much tighter than global ones because windows that don't overlap contribute zero.
- **Sketches (HLL, Theta).** These are wrong for an exact set library's `len()`, but fine for planning across very large sets. Apache DataSketches documents that union keeps the base relative standard error (RSE). Intersections and differences "can have considerably more relative error than a base Theta sketch", and "the source sketch with the smallest theta will dominate the overall resulting error".

## Recommendations for frost-collections

**1. API shape.**

- Keep `fn union(&self, &Self) -> Self` (and friends) **eager** and value-returning.
- Add an explicit borrowed view API: `a.view().and(&b).and(&c)` → `SetExpr<'a, K>`, with `contains`, `iter`, `count`, `is_empty`, `cardinality_bounds()` (O(1)), and `materialize() -> FrozenIntSet<K>`.
- Build the view internally as a *flattened* n-ary AND/OR, never nested binary nodes. Flattening or reassociating is what made Arntzenius's example go from 1.2 s to under 1 ms.
- Do not offer owned, `'static` lazy views that can be stored in a snapshot slot. That is how expression chains grow without bound.

**2. Evaluation strategy per representation.**

- **Intersections:** sort operands by `len()` and drive from the smallest.
  - Probe the others with `contains` when the driver is far smaller. This is your measured winner, and it matches the SvS literature.
  - Switch to per-window kernels when sizes are comparable.
  - Use leapfrog `seek` only among frost-owned representations whose `seek` is cheap. Never route it through roaring-rs `advance_to`.
- **Unions:**
  - Group by 2^16 window key (like "groupedOr").
  - Within a window, OR bitmaps word-wise without tracking cardinality, then repair once.
  - Promote array pairs to a bitmap only above ~1,024 combined elements, unlike roaring-rs `MultiOps`.
- **Dense ∪ small:** clone the window (or, after step 3, `Arc`-share all untouched chunks) and set bits. Never fall back to Sorted when the result fits the dense cap.

**3. Structural sharing (the core fix).**

- **New representation:** a chunked set with a sorted top-level index of `(key: u16 or u48, Arc<Container>)`, where `Container ∈ {Array(Box<[u16]>), Bitmap(Box<[u64; 1024]>), Run(Box<[(u16,u16)]>)}`.
- **Clone:** O(#chunks) `Arc` increments; or wrap the whole index in an `Arc` for O(1).
- **Snapshot-store `union(&singleton)`:** copy the top-level index, `Arc::make_mut` one container, and share the rest.
- **`Arc::ptr_eq` shortcuts:**
  - Per container: A∪A = A, A∩A = A, A∖A = ∅, A⊕A = ∅.
  - At the root: the same shortcuts for whole sets.
  - These are cheap and safe. They pay off only when operands descend from a common snapshot, which is exactly the review's pattern.
- **Roaring stays a black box.** Wrap `RoaringBitmap` in `Arc` so `Clone` is O(1), but treat it as unshareable below the root. Prefer the new chunked representation for any set that participates in algebra.
- **`Arc` availability:** `alloc::sync::Arc` is only available on targets with pointer-width atomics. Gate it, or fall back to `Rc` behind a feature, for `no_std` targets without atomics.

**4. When to materialize.**

- Always on `union`/`intersection`/etc.
- In views: on `materialize()`, or automatically once a view's `count()` or `iter()` has run twice (cache the result).
- Never implicitly on `contains`. A view's `contains` is a short-circuiting AND/OR of operand `contains`, which costs O(k) probes and needs no allocation.

**5. What to add first.**

1. The chunked `Arc`-container representation above. It fixes all three review findings, gives you a cheap per-container `seek`, and is the unit for both sharing and windowed evaluation.
2. `ptr_eq` and empty/full shortcuts.
3. The borrowed `SetExpr` view.
4. Only later, and only if memory per element matters, an Elias–Fano (or partitioned EF) Sorted replacement written in-house. The surveyed crates are std-only, or expose `unsafe` APIs, or carry heavy dependencies.

**6. Constraints honored.**

- No `unsafe`: `Arc`, `make_mut`, `ptr_eq`, boxed fixed-size arrays and `u64::count_ones` are all safe.
- No portable SIMD (nightly) or `std::arch` (requires `unsafe`): write word loops over `[u64; 1024]` that LLVM auto-vectorizes, and enable `popcnt`/BMI2 through target features at the application level.
- `no_std + alloc`: the only std-dependent pieces are an optional `Sync` cardinality cache and third-party succinct crates. Avoid both or feature-gate them.

## Caveats

- **Single-source or vendor numbers:**
  - Pinot's 2.2–2.7× lazy-fold speedup (one PR).
  - The `pds` `ptr_eq` claims (README).
  - Tantivy's 3.125% density threshold (one PR).
  - `ptr_hash` per-key timings (README figures that differ across versions; the paper says 8–12 ns/key).
  - Your own 5–8 ns `advance_to` and 3.8× figures.
  - A blog post (CuriousCoding, "Static search trees: 40x faster than binary search") claiming that at 1 GB input "binary search needs around 1150ns/query, while Eytzinger is 6x faster at 200ns/query", and that Eytzinger is "around 4 times faster" because it can prefetch 4 iterations of cache lines at a time.

  The Roaring 30% cardinality-maintenance penalty, the Lemire SIMD 2× figure, the Wang et al. conclusions, the trie-compressed ratios, the TEB and CUBIT numbers, and the pasta/orzo overheads come from peer-reviewed papers. They are still measured on those authors' hardware and data.
- **Not verified in this pass:** Pilosa/FeatureBase `Freeze()` semantics, Druid's bitmap evaluation, Java `ImmutableRoaringBitmap` internals, CRoaring roaring64's ART layout, zroar's flat-layout claims, block-max WAND details, SuRF, and Inoue et al.'s SIMD intersections. The recommendations don't depend on them.
- **Rust crate facts that are unconfirmed:**
  - The `no_std`/`unsafe` status of hibitset, ptr_hash, boomphf, ph, qwt and sucds was not confirmed from crate roots.
  - Whether `roaring` 0.12 builds as `no_std` should be checked against its Cargo features.
- **Recent bug reports:** the tantivy `seek_danger` bugs and the CRoaring COW bugs are 2025–2026 GitHub reports. They show fragility, not current defects: several are already fixed.


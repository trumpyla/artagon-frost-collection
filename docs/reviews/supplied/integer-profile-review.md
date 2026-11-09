# Frost 0.4.0 — adversarial review of the typed integer profile (`int-set`)

Reviewed: `src/integer/{key,policy,set,storage}.rs` (1,768 lines), `docs/design/integer-sets.md`,
`docs/reviews/integer-design-audit.md`, `docs/integer-benchmarks.md`, `tests/integer_sets.rs`,
`benches/integer_sets.rs`, and the pinned `roaring` 0.12.0 source it delegates to.
Baseline on this host: `cargo build --features int-set --tests` and `cargo test --features int-set --test integer_sets`
pass on Rust 1.99.1 (21 tests). The accompanying probe harness (`verification/int-set/revision2/review-probe/`,
three binaries, path-dependency on `frost`) produced every number below. Numbers are from a
noisy shared cloud host, baseline `x86-64` target unless marked `v3`; treat them as orders of
magnitude, not calibration.

---

## 0. Verdict in one paragraph

The hard parts are right. I could not produce a wrong answer, a panic on valid input, or an
overflow from any representation pair, boundary (`u64::MAX`, `i64::MIN`, signed zero crossing,
`u32` top word), or codec-law violation; the aligned-coordinate dense kernels, `u128` spans,
checked decoding, and the sealed `IntWord` boundary all hold up under attack. The weaknesses are
in the *algebra output policy* and in *wrapper overhead*, and they are large: (1) every
mixed-representation operation — and every dense union that exceeds the 8 MiB window cap —
materializes a **Sorted** result, so one tiny operand converts a 244 KiB bitmap into a 7.8 MiB
array (3.3 ms where 6 µs is attainable) and two 512 KiB dense runs union into 64 MiB where
Roaring needs 1.8 KB; (2) budgets set at construction are not carried by the set, so algebra
silently ignores them in both directions; (3) same-representation `Eq`/`Hash`, Roaring
construction, and Dense/Roaring pairs are 3×–1,700× slower than the native kernels they wrap;
(4) the memory-only planner chooses Roaring for small consecutive runs where Dense is 3× faster
and 80 bytes larger; (5) the `std` requirement is an artifact — `roaring` 0.12 is `no_std + alloc`.
None of these change logical answers; all of them change whether the profile is "high
performance" in the regimes it was built for. Section 4 proposes the frozen/exotic structures
the planner is currently missing.

---

## 1. Findings (ranked)

| # | Finding | Evidence (frost → attainable) | Severity | Where |
|---|---|---|---|---|
| F1 | Mixed-representation algebra always returns Sorted; a tiny operand destroys a compact one. The snapshot-store pattern in `tests/integer_sets.rs` (`current.union(&singleton)`) degrades to Sorted on the **first** update and stays there. | 1 M-member Dense (244 KiB) ∪ Roaring{1}: **3.27 ms → Sorted 7.8 MiB**; clone-window-and-set-bit: **6.4 µs → Dense**. Dense ∖ Sorted{3}: **3.87 ms → Sorted 7.8 MiB**. | **High** (perf + memory; defeats the planner) | `storage.rs` `mixed()`/`merge()` L536–592, `binary()` L467–514 |
| F2 | Dense∪Dense / ⊕ beyond the window cap falls back to **Sorted**, the only representation that is *always* worse than the window it refused to allocate. | two 4 M-member dense runs 2^27 apart (512 KiB each): **53 ms → Sorted 64 MiB**; `RoaringTreemap` of the same set: **1,824 bytes**. | **High** | `storage.rs` L444–449, `Dense::binary` L804–810 |
| F3 | Policy is not retained by the set; `Dense::binary` hard-codes `IntSetPolicy::auto()`. Small budgets are violated upward, large budgets are clamped downward (then hit F2). | two sets built with `with_dense_budget(8)` → union is a **128 KiB Dense**. A 64 MiB-budget user with two 32 MiB windows gets a Sorted result instead of a 40 MiB window. | Medium–High (documented as "growth guard", but it is the user's limit being ignored) | `storage.rs` L804–808 |
| F4 | `PartialEq`/`Ord`/`Hash` walk every element through the `WordIter` enum (and per-element `remaining` bookkeeping) even when both sides share a representation. | Roaring==Roaring, 200 K members: **3.31 ms vs 1.95 µs** native (1,700×). Sorted==Sorted: **1.24 ms** vs a memcmp-class slice compare. | Medium | `set.rs` L335–359, `storage.rs` `Words::next` L879–896 |
| F5 | Roaring candidates are built with `collect()` (+`optimize`) although the input is strictly ascending; `from_sorted_iter` is the O(n) append path. The Auto planner also runs a second full scan (`roaring_score`) after `dedup`. | 200 K members: `collect+optimize` **2.07 ms** vs `from_sorted_iter+optimize` **0.77 ms**; `try_from_iter(.., roaring())` **2.81 ms** vs `sorted()` **0.53 ms**. Bench report: Auto construction 2× explicit sorted. | Medium (one-line fix) | `storage.rs` L60–64, L137–141 |
| F6 | Dense/Roaring pairs probe element-wise. Promoting the dense window with `RoaringBitmap::from_lsb0_bytes` and using the native kernel is **20×** faster; the direction rule also ignores dense word traversal (unlike the sorted rule). | Dense∩Roaring (8,192 vs 5,462): **27.0 µs → 1.30 µs** (native-native: 0.90 µs). Sorted∩Roaring via sorted→bits→`from_lsb0_bytes`: 18.6 → 12.7 µs (1.5×, only when the sorted span is compact). Galloping with `Iter::advance_to` is *slower* (70 µs): do not. | Medium | `storage.rs` L313–326, L499–514 |
| F7 | Planner is memory-only with no speed floor: a 1,000-member consecutive `u32` run scores Roaring 70 vs Dense 152 bytes and picks Roaring. | `contains` ×1,000: **2.94 µs (Roaring) vs 0.91 µs (Dense)**; the saving is ~80 bytes. | Medium (Auto is the `FromIterator` default) | `storage.rs` L144–160 |
| F8 | Dense kernels are popcount-bound at the baseline target (no `POPCNT` in x86-64 baseline; the recorded bench ran with `RUSTFLAGS: null`), while the Roaring control gets `fearless_simd` runtime dispatch. `Dense::binary` also uses a double-checked `at_block` per word and a separate popcount pass. | Dense∩Dense 31,250 words: **27 µs** baseline → **13 µs** `v3`. Dense∪Dense materialized: **89 µs** → fused three-region prototype **71 µs** baseline / **35 µs** `v3`. | Medium (methodology + 2× kernel) | `storage.rs` L711–720, L811–822, L216–228; `docs/integer-benchmarks.md` |
| F9 | Construction always stages every code in a `Vec<W>` (8n bytes) regardless of the target representation; `with_policy` restages; there is no range constructor. A 100 M-member dense set needs 800 MB of staging for a 12.5 MiB result. | structural | Medium (scalability; "high performance frozen" claim) | `set.rs` L64–123, L133 |
| F10 | `int-set` requires `std`, but `roaring` 0.12 is `#![no_std]` with `alloc`; its `std` feature only adds `bytemuck`/`byteorder` and runtime SIMD dispatch. The fixed candidate set would be unchanged under `alloc`. | `roaring-0.12.0/Cargo.toml`, `src/lib.rs:10` | Low–Medium (design simplification) | `Cargo.toml` L48 |
| F11 | API/integration gaps: no `serde`/`borsh`/`schemars`/`arbitrary`/`rayon` adapters for `FrozenIntSet`; no `IntKey` for `u8/u16/i8/i16/char/usize/isize/NonZeroU8/U16`; no `DoubleEndedIterator`; no `rank`/`select`/`range`/`range_cardinality` although all three backends can provide them (Roaring natively); no k-way `union_all` (Roaring `MultiOps`); no by-value `IntoIterator`. | grep `src/adapters` → zero integer impls | Low–Medium | — |
| F12 | Test blind spots: every test asserts *logical* answers only. Nothing asserts the representation or payload of an operation result, nothing asserts that a constructor budget survives algebra, and the proptest windows (≤512 wide) never reach the `Dense::binary → None` fallback or the mixed direction rule at scale. F1–F3 are invisible to the suite by construction. | `tests/integer_sets.rs` | Low (coverage) | — |

### Things I attacked and could not break

- **Coordinate alignment at the edges**: `[u64::MAX-193 … u64::MAX]` vs `[u64::MAX-192 …]`, `i64::MIN`, `i64::MAX`, `u32` top word. `dense_bytes` in `u128`, `base = low & !63`, and the fact that the last word's block start is 64-aligned and ≤ `0xFFFF…FFC0` mean `DenseIter`'s `base + (index-1)*64 + bit` cannot overflow for stored members.
- **`Dense::binary` trimming and empty-window handling**: `Dense::empty()` (`base 0, words []`) composes correctly with every op via `at_block → 0`; Intersection/Difference can never return `None`.
- **Codec-law enforcement**: colliding, reversing, non-canonical, and time-varying codecs are caught at the documented points (`CodecRoundTrip`, `NonMonotonicEncoding`, `NonCanonicalCode`, checked-panic on iteration). The extra `decoded.cmp(&key) != Equal` check catches an `Eq`/`Ord` disagreement that `!=` alone would miss.
- **Representation independence**: `Eq`, `Ord`, `Hash` agree across all three representations (same word stream, cardinality prefix). `is_subset` fast paths (cardinality, bounds) and `separated()` are sound for empty operands.
- **Overflow discipline**: `union_len`/`symmetric_difference_len` subtract overlap before adding; `union()` only pays for an `intersection_len` pass when the naive sum would overflow `u64`.
- **Sealing**: `IntWord: storage::Word` with `pub(crate) trait Word` cannot be implemented or named downstream; `compile_fail` doctests cover both the width and the domain-mixing cases.
- **Fusion and size hints**: `Words`, `DenseIter`, `IntSetIter` are fused; `size_hint` degrades to `(usize::MAX, None)` correctly when the count does not fit.
- **No `unsafe`**, no manual `Send`/`Sync`; `Send + Sync` follow from `PhantomData<K>` and `W::Roaring: Send + Sync`.

---

## 2. What the probes showed

All numbers: median-free single-batch means (`Instant` over N iterations, `black_box`ed), baseline x86-64 unless marked.

### 2.1 Output-representation collapse (F1, F2, F3)

| Case | Result | Time | Payload |
|---|---|---:|---:|
| 1 M-member Dense (window 244 KiB) ∪ Roaring{1} | **Sorted** | 3.27 ms | 7,812 KiB |
| same, prototype: clone window + set one bit | Dense | 6.4 µs | 244 KiB |
| 1 M-member Dense ∖ Sorted{3} | **Sorted** | 3.87 ms | 7,812 KiB |
| `SnapshotStore`-style `current = current.union(&roaring_singleton)` ×3 | Sorted after update 0, 1, 2 | — | — |
| Dense[0, 4 M) ∪ Dense[2^27, 2^27+4 M) (512 KiB each; union window 16 MiB > 8 MiB) | **Sorted** | 53.4 ms | 64 MiB |
| same set as `RoaringTreemap` (`\|` then `optimize`) | Roaring | — | 1,824 B serialized |
| two sets built with `with_dense_budget(8)`, 2^20 apart, union | Dense | — | 128 KiB (budget was 8 B) |

### 2.2 Mixed pairs: probing vs promotion (F6)

Fixture: `base = 1_000_000`, left = 8,192 even offsets, right = 5,462 offsets `3k+1` (the bench's u32 window).

| Strategy | Sorted∩Roaring | Dense∩Roaring |
|---|---:|---:|
| frost 0.4.0 (probe into `contains`) | 18.6 µs | 27.0 µs |
| merge sorted slice against `roaring.iter()` | 52.8 µs | — |
| galloping (`skip_below` + `Iter::advance_to`) | 70.1 µs | — |
| promote sorted → Roaring (`from_sorted_iter`) + native | 39.2 µs | — |
| promote sorted → bits → `from_lsb0_bytes` + native | **12.7 µs** | — |
| promote dense → `from_lsb0_bytes` + native | — | **1.30 µs** |
| native Roaring ∩ Roaring (reference) | 0.90 µs | 0.90 µs |

Skewed (65,536 sorted vs 64 Roaring): frost 2.06 µs, galloping 3.67 µs. Reversed (64 vs 65,536): frost 178 ns, galloping 1.10 µs. Roaring's iterator is ~5–8 ns/element, so anything that iterates it loses to `contains` probing; the current direction rule is the right call for Sorted/Roaring and the only change worth making there is bit-promotion when the sorted span is compact (`dense_bytes(span) ≤ ~2 × n × size_of::<W>()`).

### 2.3 Wrapper overhead (F4, F5)

| Operation (200 K members) | frost | native / reference | ratio |
|---|---:|---:|---:|
| `==` Roaring vs Roaring | 3,312 µs | 1.95 µs | 1,700× |
| `==` Sorted vs Sorted | 1,239 µs | memcmp-class | ≫100× |
| build Roaring: `collect()` + `optimize` | 2,067 µs | `from_sorted_iter` + `optimize`: 772 µs | 2.7× |
| `try_from_iter(.., roaring())` | 2,809 µs | `try_from_iter(.., sorted())`: 530 µs | 5.3× |

### 2.4 Dense kernels and target features (F8)

31,250-word windows (`0..2_000_000` minus multiples of 3 / of 5):

| Kernel | baseline x86-64 | `-C target-cpu=x86-64-v3` |
|---|---:|---:|
| frost `intersection_len` | 27.3 µs | 13.3 µs |
| reference `zip(&).count_ones().sum()` | 29.7 µs | 9.0 µs |
| frost `union` (materialized, incl. cardinality) | 88.8 µs | 80.6 µs |
| prototype three-region fused OR+popcount | 70.7 µs | 36.7 µs |
| reference `zip(\|).collect()` (no popcount) | 8.7 µs | 7.6 µs |

Software popcount is ~1 ns/word; the recorded benchmark compared this against Roaring's
runtime-dispatched SIMD. The comparison in `docs/integer-benchmarks.md` is therefore skewed
against the dense candidate by roughly 2× on counts; the document should record the target
level and, ideally, add a `v2`/`v3` column.

### 2.5 Planner (F7)

`FrozenIntSet::<u32>::from_iter(0..1000)` → `Roaring` (score 70 vs Dense 152 vs Sorted 4,016).
`contains` ×1,000: 2.94 µs Roaring vs 0.91 µs forced Dense. `intersection_len` with `500..1500`:
12.6 ns vs 12.2 ns (equal — Roaring's run container wins that one back).

---

## 3. Design changes, in priority order

### 3.1 Promotion instead of degradation for mixed pairs (fixes F1, F6)

Replace "mixed → Sorted" with a promotion matrix chosen by estimated cost; the output
representation is the promoted operand's. All conversions are O(size of the operand converted)
and need no new unsafe:

| Pair | Convert | Cost | Then |
|---|---|---|---|
| Dense ∘ Roaring | dense → `RoaringBitmap::from_lsb0_bytes(base, bytes)` (per 32-bit partition for `u64`, via `RoaringTreemap::bitmaps()` / `insert`) | O(W) | native kernel, result Roaring |
| Sorted ∘ Roaring | sorted → Roaring via `from_sorted_iter` when `n ≥ roaring.len()/8`; sorted → bits → `from_lsb0_bytes` when the sorted span is compact; otherwise keep probing | O(n) | native kernel |
| Sorted ∘ Dense, small sorted | clone dense window; set/clear bits that fall inside; for ∪/⊕ extend the window only within budget | O(W + m) | result Dense |
| Dense ∘ Sorted, large sorted | restrict the sorted slice to the window with two `partition_point`s (O(log n)), then bit-promote only that sub-slice | O(k + W) | result Dense for ∩/∖ |
| Difference / Union with a small operand of any kind | iterate the **small** operand and edit a clone of the large one | O(large payload + m) | large's representation |

`mixed()` currently iterates the *left* operand for `Difference` regardless of size; that is the
3.87 ms case above. Even without promotion, choosing the iteration side there is a one-line fix.

### 3.2 Dense∪Dense over the cap falls back to Roaring, never Sorted (fixes F2)

When `Dense::binary` returns `None`, promote both windows with `from_lsb0_bytes` and use the
native `|`/`^`. Roaring is the representation *designed* for "two dense islands far apart". Cost
is O(W_a + W_b) instead of O(n_a + n_b) allocation of 8-byte words. Keep Sorted only as the
fallback for the Roaring partition cap.

### 3.3 Carry the budget (fixes F3)

Store `max_dense_bytes` and `max_roaring_partitions` in `Storage` (two `usize`, 16 bytes) and
use `max(left, right)` in `Dense::binary` and in the 3.2 fallback. Alternatively keep the
"growth guard" semantics but compute it from the operands' *own* budgets, not `auto()`. Either
way, document that the result budget is derived from the operands.

### 3.4 Planner: add a speed floor and fuse the statistics pass (fixes F5, F7)

- Prefer Dense whenever its payload is ≤ 4 KiB (one page; always L1-resident) regardless of
  the Roaring score, and in general when `dense_score ≤ 2 × roaring_score`. Memory tie-breaks of
  tens of bytes should not buy 3× slower `contains` and a `BTreeMap`/container indirection.
- Compute `roaring_score` inside the `dedup` pass (it already walks the sorted words) and skip it
  when `dense_bytes + 24 ≤ 64`, the minimum non-empty Roaring score. Expected: Auto construction
  within ~1.2× of explicit Sorted rather than 2×.
- Use `from_sorted_iter` for Roaring candidates (2.7× on construction, F5).

### 3.5 Same-representation fast paths (fixes F4)

`PartialEq`: Sorted/Sorted → slice `==`; Dense/Dense → `base == base && words == words`
(windows are trimmed, so equal sets have equal windows); Roaring/Roaring → native `==`. Mixed:
compare `cardinality` and `bounds` first, then the element stream. `Ord`: same, with
lexicographic fallbacks. `Hash`: feed every representation through identical 64-element
`hash_slice` chunks so that the `write` call sequence is representation-independent for *any*
hasher while Sorted gets slice-speed hashing. Give `Words` a `fold`/`for_each` that dispatches
once per traversal, as `set_ops` already does.

### 3.6 Dense kernels (fixes F8)

Rewrite `Dense::binary` as three slice regions (left-only prefix, overlap via `zip`, right-only
suffix) and accumulate the cardinality in the same pass; drop `Storage::dense`'s separate
popcount. Document `-C target-cpu=x86-64-v2` (POPCNT) in the benchmark report and record the
level in `environment.json`. If runtime dispatch is wanted without crate-local `unsafe`, a
multiversioning dependency can own it; a safe fallback is a carry-save-adder popcount batched over
16 words (~3× over per-word software popcount).

### 3.7 Streaming construction (fixes F9)

Add `from_ascending_encoded(iter, policy)` for explicit policies: validate strict ascent and
canonicality online and append directly to the chosen representation (`Vec<W>` push, bit set,
`RoaringBitmap::append`). Add `from_ranges(iter of RangeInclusive<K>)` that fills Dense words or
uses `insert_range`. Auto can keep the staging path, or sample the first `k` members to plan.

### 3.8 `int-set` under `alloc` (fixes F10)

`int-set = ["alloc", "dep:roaring"]`, `std = [..., "roaring?/std"]`. The candidate set is
unchanged, embedded users get the profile, and runtime SIMD dispatch remains a `std` bonus — which
is exactly how the design document already describes it.

### 3.9 Tests to add (fixes F12)

- For every pair in `every_pair`, assert `result.representation()` and that a
  representation-specific payload estimate never exceeds `2 × min(input payloads) + small` for
  ∩/∖ and `≤ sum` for ∪/⊕.
- A test that a `with_dense_budget(b)` operand never yields a Dense result wider than `b` (or the
  documented derived budget).
- The far-apart dense union asserts `Roaring`, not merely `≠ Dense`.
- Proptest with window widths up to 2^20 and bases up to 2^40 so the fallback path is exercised.

---

## 4. New frozen / exotic integer collections

Each entry names the gap it fills in *this* profile, then costs. "Planner fit" says whether it can
slot in as a candidate behind the existing public type without changing contracts, or needs a
separately named profile (per the design's own rule).

| # | Structure | Fills | Memory | `contains` | Algebra | Planner fit |
|---|---|---|---|---|---|---|
| N1 | **Arena-frozen chunked bitmap** (single allocation Roaring: flat sorted chunk index + descriptor table + container arena) | scattered `u64` prefixes (treemap `BTreeMap` cost), per-container heap allocs, cheap `Arc` clone, zero-copy views | ≈ Roaring, −24 B/container | 1 binary search (Eytzinger/fenced) + container lookup | same kernels as Roaring, contiguous | candidate (replaces/augments Roaring) |
| N2 | **Rank-indexed `FrozenIntMap<K, V>`** = `FrozenIntSet<K>` + `Box<[V]>` + rank directory | there is no integer *map*; the set already is a perfect index | set + 1.5–3% (Poppy) + values | set `contains` + `rank` | key-set algebra reused; columns shareable across maps | new type on top of existing set |
| N3 | **`FrozenRangeSet`** (sorted disjoint intervals) | `[0, 10^12]` is unbuildable today; runs ≫ 16-bit containers | 16 B/run, span-independent | O(log r) | interval sweep O(r_a + r_b) | 4th candidate when `runs ≪ n` (`statistics()` already computes `runs`) |
| N4 | **Sorted + fence index** (every 16th code in a side array; Eytzinger optional) | Sorted's cache-missy binary search in the scattered fallback | +6.25% | ~2 cache misses, branch-free scalar | unchanged (same slice) | drop-in upgrade of Sorted |
| N5 | **Sorted + MPHF side index** (PTHash-class) | O(1) membership *with* ordered iteration for scattered keys | +2.5–3.5 bits/key | 1 hash + 1 probe | merge-based as today | candidate for scattered `u64` |
| N6 | **Elias–Fano / Partitioned EF** | mid-density clustered data where Roaring array containers cost 2 B/member + 32 B/container | n·(2 + ⌈log₂(U/n)⌉) bits; PEF ≈ 1–2 B/member clustered | O(1) `select`, O(log) `contains` via `next_geq` | galloping via `next_geq` | candidate between Sorted and Roaring |
| N7 | **Persistent chunked set** (`Arc` per 2^16 chunk, path-copying) | `SnapshotStore` pattern: O(n) copy per singleton update, F1 collapse | ≈ N1 + 8 B/chunk | as N1 | point updates O(chunk); versions share chunks | separately named *shared* profile (design already anticipates it) |
| N8 | **Borrowed/static storage** (`Repr::Static(&'static [W])`, `Cow`, mmap view of N1) | const tables, zero-allocation sets, zero-copy deserialization | 0 heap | as Sorted/N1 | as today | candidate variant; `const fn` constructors |
| N9 | **Dictionary-compressed set family** (shared sorted key dictionary; each member set is a Dense bitmap over dictionary *ranks*) | many sets over one universe of n distinct keys (tags, ACL-style membership): every set becomes dense and tiny, all algebra is word ops | dictionary once + n/8 B per set | dictionary lookup + bit test | word kernels only | new family type |
| N10 | **Approximate filter** (binary fuse, 8-bit) | pre-filter for mixed ops; membership where 0.4% FP is acceptable | ~9 bits/key | 3 loads | none (or ∧ of filters) | separately named approximate profile |

### N1 — why a *frozen* Roaring is a different structure

`roaring-rs` is a mutable structure: `Vec<Container>` with one heap allocation per container and
a `BTreeMap<u32, RoaringBitmap>` for `u64`. Once frozen, none of that is needed. CRoaring's
*frozen* format is the prior art: a header, a sorted key array, a descriptor array
(`type:2 | cardinality−1:16 | offset:32`), and one arena holding array (`u16·n`), bitmap (1,024 words),
run (`u16` pairs) and *full* (0 bytes) containers. For `u64`, the key array holds 48-bit prefixes,
so "4,096 scattered partitions" is a 64 KiB sorted array and one binary search instead of a
`BTreeMap` walk plus a `Vec` per partition. One `Box<[u64]>` means `Arc<[u64]>` sharing,
`&'static` embedding, `from_bytes` zero-copy, memcpy `Clone`, and a trivially SIMD-friendly layout.
The promotion matrix in 3.1 becomes cheaper still, because `from_lsb0_bytes` is then an in-arena
copy.

### N2 — the set is the index

Every representation already answers `rank(k)` (Roaring natively; Sorted by binary-search index;
Dense with a cumulative-popcount directory per 512 bits). A `FrozenIntMap<K, V>` is therefore
`FrozenIntSet<K>` plus `Box<[V]>` in key order: `get(k) = contains(k).then(|| &values[rank(k)])`.
No hashing, no per-entry key storage, iteration in key order, and several maps can share one
`Arc<FrozenIntSet<K>>` as a common index (columnar frozen tables). Dense's directory is the only
new code; it also gives the set `select(i)`, `nth`, and `DoubleEndedIterator` for free.

### N3 — ranges

`statistics()` already counts runs but no representation exploits a run longer than 65,536.
A `FrozenRangeSet` of `(lo, hi)` pairs is span-independent, makes `from_ranges` trivial, and
turns `universe.difference(&set)` for a contiguous universe into O(r). As a planner candidate it
is scored at `16·runs + 16` and wins exactly where it should.

### N4/N5 — repairing the scattered fallback

The scattered-`u64` fixture is where the profile is "comparable to `FrozenOrderedSet`" and no
better. A fence index (N4) is ~40 lines, safe, and halves the cache misses of the binary search;
an MPHF side index (N5) makes membership O(1) at ~3 bits/key while keeping the sorted array as
the iteration and merge substrate. N5 needs a dependency decision (`ptr_hash`, `boomphf`, or a
crate-local CHD) and a build cost of O(n) expected; it belongs behind an explicit policy first.

---

## 5. Suggested order of work and gates

1. **One-liners, no contract change**: `from_sorted_iter` (F5); `Difference` iterates the small
   side (part of F1); dense-word term in the Dense/Roaring direction rule (F6); record target-cpu
   level in the bench environment (F8). Gate: existing suite + bench before/after.
2. **Same-representation fast paths** (F4) and `Words::fold`. Gate: representation-independence
   tests for `Eq`/`Hash` across all nine pairs (already exist) plus timing rows.
3. **Promotion matrix + Roaring fallback + carried budget** (F1, F2, F3). This changes the
   documented sentence "mixed and sorted paths return sorted codes" and the growth-guard text;
   update `docs/design/integer-sets.md` and the audit table. Gate: the new representation/payload
   assertions in 3.9, proptest with wide windows, bench matrix re-run.
4. **Planner floor + fused statistics** (F7, F5). Gate: `auto_planner_*` tests updated with the
   new expectations and the construction bench.
5. **`alloc`-only `int-set`** (F10) and the isolated feature matrix / embedded build.
6. **Streaming constructors** (F9), then N3/N4 as candidates, then N1/N2 as new types.

Verification notes for the maintainers' guide: the probe harness is three binaries with a path
dependency on `frost`; run `cargo run --release --bin probe{,2,3}` and once more with
`RUSTFLAGS="-C target-cpu=x86-64-v3"`. It validates every prototype against frost's own answer
before timing. No allocator instrumentation was used; payload figures are computed from
cardinality and window size, not measured RSS.

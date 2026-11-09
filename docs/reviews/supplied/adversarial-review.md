# Frost 0.4.0 — adversarial design review and optimization plan

Reviewed: `frost-collections` 0.4.0 (Rust 1.99.1, edition 2024), ~13.7k lines, 120 files.
Baseline verification on this host: `cargo build/test --all-features` passes (215 tests + doctests), `cargo clippy --all-features --all-targets` with `pedantic` is clean, the single `unsafe` module (`adaptive::seq::buf`) re-audited by hand.

Companion artifacts:

- `frost-review-patch.diff` — the two fixes I implemented and verified (adaptive sorted-set algebra; `SnapshotStore` read-path and RMW API), with regressions, benchmark cases, docs and changelog. After the patch: 222 tests pass, clippy pedantic clean, alloc-only and core-only builds pass.
- The probe harness (`/home/claude/probe`) that produced every number below. All numbers are medians of 5 samples on a noisy 2-vCPU cloud host; treat them as orders of magnitude, not calibration.

---

## 0. Verdict in one paragraph

The *architecture* is unusually disciplined: sealed eligibility, explicit plans, honest capability traits, crate-owned iterators, one `unsafe` module with correct invariants, and documentation that refuses to over-promise. I found no memory-safety defect and no semantic bug in the tested paths. The weaknesses are algorithmic and they cluster in exactly the places the docs are proudest of: (1) the "fast frozen set operations" are linear merges that lose to `std::BTreeSet` by **44×–770×** on skewed inputs, and the shipped benchmark only measures equal-size inputs; (2) the adaptive ordered map has **O(n²)** `pop_first`/`extend` cliffs that are policy-reachable; (3) `SnapshotStore::load` clones an `Arc` on every read, which is **17× slower** than `arc-swap`'s guard under only two contending readers; (4) the "frozen" state buys almost nothing algorithmically — `FrozenVec`/`FrozenOrderedMap` deep-clone, cannot be sliced, and `FrozenHashMap` is a shrunk `std::HashMap`. Items (1) and (3) are fixed in the attached patch; (2) and (4) are design proposals below, plus a catalogue of frozen-only structures that would make the "frozen" half of the crate earn its name.

---

## 1. Reproduced findings (ranked)

| # | Finding | Evidence (frost / alternative) | Severity | Status |
|---|---|---|---|---|
| F1 | Sorted-set algebra is a pure linear merge; skewed inputs scan the large side | 64∩65 536: **95 µs vs 2.2 µs** (`BTreeSet`) → 44×; 64∩1 M: **1 290 µs vs 2.0 µs** → 637×; `is_subset` 64⊆1 M: **1 556 µs vs 2.0 µs** → 771× | High (perf; contradicts "fast set operations" positioning; bench omits the regime) | **Fixed in patch** → 3–5 µs, within 2–2.5× of `BTreeSet` |
| F2 | `AdaptiveOrderedMap` sorted variant: `pop_first` is `Vec::remove(0)`, `extend` is per-element insert → O(n²) | drain-by-`pop_first` n=16 384: **41.7 ms vs 0.39 ms** (`BTreeMap`) → 106×; interleaved `extend` n=16 384: **43.2 ms vs 2.2 ms** → 20× | Medium (reachable by any `sorted_up_to(n)` policy; undocumented cliff) | Proposal §3.1 |
| F3 | `SnapshotStore::load` = `load_full()` (Arc clone = contended atomic RMW) | 2 threads × 200 k loads: **47.7 ms vs 3.45 ms** (`ArcSwap::load` guard) → 14–17× | Medium (the type exists for many-reader workloads) | **Fixed in patch**: `load_guard`, `peek` (17× faster), `publish_if_current`, `update` |
| F4 | No read-modify-write publication; docs say "coordinate writers" | lost-update race is trivial to construct | Low–Medium (API gap) | **Fixed in patch** (`update` is RCU; test with 8 writers × 250 updates loses nothing) |
| F5 | Frozen types deep-clone (`derive(Clone)` over `Box<[T]>`), cannot be sub-sliced, and `Arc<FrozenVec<T>>` is a double indirection | structural | Medium (design; the main *reason* to freeze is cheap sharing) | Proposal §3.2 |
| F6 | `FrozenHashMap/Set` = shrunk `std::HashMap` — the frozen state is not exploited | measured: a naive hash-ordered dense layout is *not* a win (see §2.3), so this is a gap, not a defect | Low | Proposal §3.4 (MPHF profile, gated by measurement) |
| F7 | `AdaptiveHashMap::get_or_insert_with` and `AdaptiveOrderedMap::get_or_insert_with` perform the lookup twice (once for the promotion decision, once in the match) | code inspection | Low | 2-line fix each |
| F8 | `FrozenOrderedMap` stores `Box<[(K, V)]>` (AoS): binary search drags `V` through cache; `BTreeSet` beats the flat array 2–3× at 1 M even after galloping | 1 024∩1 M after patch: 179 µs vs 93 µs | Low–Medium | Proposal §3.3 (SoA + sampled index profile) |
| F9 | `SET_OPERATIONS.md` benchmark compares the hash kernel against a strawman (scan large, probe small) and only balanced ordered inputs; the 1 236× headline ratio is against code nobody would write | docs | Low (honesty of claims) | Patch adds skewed cases vs `BTreeSet` and records before/after |

Things I attacked and could **not** break (worth recording so nobody re-spends the effort):

- `InlineBuf`/`InlineIntoIter` ownership: pop-before-read, `take_into_vec` allocates before relinquishing, `Drop` uses slice drop glue (continues on a panicking destructor), ZST and over-aligned paths. No UB candidate found; Miri logs in `verification/` corroborate.
- `range_indices` reproduces `BTreeMap::range` panics exactly, including the empty-map no-validation quirk.
- `FixedBitSet` padding masking (`logical_words`) is correct for `len % 64 == 0`, extra native blocks, and all four binary ops; results have clean padding.
- `Eligible` set: every audited backend is structurally immutable through `&B` (imbl's structural sharing is CoW; `fst` is gated on `D: Eligible`). Live concurrent collections correctly excluded.
- First-representative/last-value laws: `dedup_by` swap trick in `FrozenOrderedMap::from_iter` is correct for runs of length ≥ 3; strict builders return the *input* duplicate, not the stored one.
- Borsh strict decoding: cardinality check catches `Ord`/`Hash` disagreement; Serde reservation is capped at 1 MiB.
- Hash-set algebra `size_hint`s are tight and correct; `fold` specialisations keep `count()` allocation-free.

---

## 2. What the probes showed

### 2.1 Sorted-set algebra (F1) — before/after

| Case | 0.4.0 (linear merge) | `std::BTreeSet` | Patched (adaptive gallop) |
|---|---:|---:|---:|
| intersection_len 64 ∩ 65 536 | 95.3 µs | 2.17 µs | 3.1 µs |
| is_subset 64 ⊆ 65 536 | 123.2 µs | 2.50 µs | 3.0 µs |
| intersection_len 64 ∩ 1 048 576 | 1 290 µs | 2.03 µs | 4.3 µs |
| is_subset 64 ⊆ 1 048 576 | 1 556 µs | 2.02 µs | 4.2 µs |
| intersection_len 1 024 ∩ 1 048 576 | 1 124 µs | 71–93 µs | 179–213 µs |
| intersection_len 65 536 ∩ 65 536 (balanced control) | 85–92 µs | — | 100–108 µs |

Design of the fix (`src/family/ordered/set_ops.rs`):

- **TimSort galloping mode**, not a length-ratio test. A per-step `len_a * 16 <= len_b` check (my first attempt) cost 66 % on the balanced path. Counting consecutive wins per cursor costs one increment and compare, and switches on what the data *does*, so inputs that become skewed mid-traversal adapt without a restart. Balanced overhead settled at ~10–15 %.
- **Gap-estimated first probe**: `lower_bound_gallop(items, key, first_probe = remaining_long / remaining_short)`; if the guess is right it halves the probe count versus classic doubling-from-1, and it degrades to a plain binary search when wrong. Measured 10 % gain over pure exponential search on 1 024∩1 M.
- **Adaptive threshold** (`min_gallop` down to 1 after productive jumps, up to 64 after wasted ones) so persistently skewed data gallops after every single win.
- Applied to intersection/difference iterators, `intersection_len` (hence `union_len`, `difference_len`, `symmetric_difference_len`), `is_disjoint`, `is_subset`. Union and symmetric-difference traversal are output-bound (they must emit ~every member) and stay plain merges.
- Regressions added: a comparison-budget test (`≤ 2·log₂(m)+4` per small member, i.e. far below a linear scan) across all 11 affected entry points using a counting `Ord`; skewed and mixed-regime proptests against `BTreeSet`; exhaustive lower-bound check for every `first_probe`.

Why the remaining 2–2.5× gap to `BTreeSet` at 1 024∩1 M is *not* a comparison-count problem: galloping on a flat 4 MiB array touches ~10 cold cache lines per probe; a B-tree's top levels stay hot across consecutive searches. This is what §3.3 addresses.

### 2.2 `AdaptiveOrderedMap` quadratic paths (F2)

| Case (`sorted_up_to(n)`) | frost | `BTreeMap` | ratio |
|---|---:|---:|---:|
| `pop_first` drain, n = 4 096 | 1.77 ms | 0.098 ms | 18× |
| `pop_first` drain, n = 16 384 | 41.7 ms | 0.39 ms | 106× |
| interleaved `extend`, n = 4 096 | 1.83 ms | 0.51 ms | 3.6× |
| interleaved `extend`, n = 16 384 | 43.2 ms | 2.18 ms | 20× |

With the default threshold (32) neither matters; with any user policy large enough to be interesting, both are quadratic and nothing in the docs says so (the docs say "sorted insertion can shift a suffix" — true, but `pop_first` shifting the whole prefix and `extend` being n inserts are separate cliffs).

### 2.3 Frozen hash layout (F6) — a negative result

I prototyped the obvious "frozen-specific" layout: entries sorted by hash with a `u32` bucket-offset index at load factor 1, hashes stored separately for cheap comparisons.

| n | hit ×4 096 (std SwissTable / dense proto) | miss ×4 096 | memory |
|---|---:|---:|---|
| 1 000 | 0.33 / 0.35 µs | 10.5 / 13.0 µs | 30 KB / 28 KB |
| 100 000 | 11.7 / 17.5 µs | 13.0 / 16.2 µs | 1.95 MB / 2.92 MB |
| 1 000 000 | 17.9 / 40.3 µs | 13.0 / 19.1 µs | 31.2 MB / 28.2 MB |

**The shrunk SwissTable wins or ties everywhere.** Three dependent loads (offsets → hashes → entries) lose to SwissTable's two (control group → slot), and power-of-two bucketing wastes memory at non-power-of-two n. So "frost's frozen hash map is just a HashMap" is a *gap*, not an inefficiency, and the only layout that can beat SwissTable for point lookups is a minimal perfect hash (one dependent access + a small, cache-resident MPHF). That belongs behind an opt-in plan with its own benchmark (§3.4), consistent with the crate's "no hidden switch" rule.

### 2.4 `SnapshotStore` read contention (F3)

2 threads × 200 000 reads of a 64-element `FrozenVec`:

| path | time | ratio |
|---|---:|---:|
| `store.load()` (= `ArcSwap::load_full`, 0.4.0) | 47.7–56.9 ms | 14–17× |
| `ArcSwap::load()` guard (raw) | 3.3–3.5 ms | 1.0× |
| `store.peek(|v| …)` (patch) | 3.45 ms | 1.07× |

On a 2-vCPU host; the gap widens with core count because every `load_full` is a contended `fetch_add` on the same cache line.

---

## 3. Design proposals (not implemented; ordered by value/effort)

### 3.1 `AdaptiveOrderedMap`: `VecDeque` sorted storage + merge-based `extend` (fixes F2)

- Store the sorted variant as `VecDeque<(K, V)>`. `VecDeque::binary_search_by`, `range`, `make_contiguous` and `as_slices` are stable; `insert`/`remove` shift the *shorter* side (average n/4 moves instead of n/2); `pop_front`/`pop_back` are O(1). `freeze()` becomes `Vec::from(deque)` which is O(1) when already contiguous and one rotate otherwise.
- `Extend`: collect the batch, stable-sort by key, dedup (last value wins), then do a single backward in-place merge into the existing storage: O(n + m log m). Same for `FromIterator`-then-promote paths. The same primitive gives `FrozenOrderedMap::merge_with(other)` for free.
- Keep `range_indices` on slices by calling it on `make_contiguous()` only when `as_slices().1` is non-empty (or run it on each half — it is just two `partition_point`s).
- Document the threshold's cost model explicitly: "sorted: O(n) insert/remove, O(1) pop at either end, O(log n) lookup; tree: O(log n) everything."

### 3.2 Make frozen collections cheap to share and slice (fixes F5)

The frozen state is where structural sharing is *safe by construction*, and the crate currently throws that away: `FrozenVec::clone` copies n elements; `Arc<FrozenVec<T>>` (which is what `SnapshotStore` produces) is `Arc → Box → [T]`.

Proposal (additive, as separately named profiles to respect the "one representation per family" rule):

- `SharedFrozenVec<T>` = `{ data: Arc<[T]>, start: u32/usize, len }` — O(1) `clone`, O(1) `slice(range) -> SharedFrozenVec<T>` (the `bytes::Bytes` model), single allocation, `thaw()` = `Arc::try_unwrap` fast path or clone. `SnapshotStore<SharedFrozenVec<T>>` then needs no outer `Arc` at all — publish the value, readers clone it for 1 atomic.
- `SharedFrozenOrderedMap/Set` the same way over `Arc<[(K, V)]>`; `range(..)` can return a *zero-copy frozen sub-map* that keeps every lookup/range method, which is something a `BTreeMap` cannot offer.
- Variant without atomics for single-threaded code: `Rc<[T]>`. Keep these out of `Eligible` (they are families, not backends).

### 3.3 Cache-aware frozen ordered layout (fixes F8, closes the last gap in F1)

- **SoA**: `FrozenOrderedMap { keys: Box<[K]>, values: Box<[V]> }`. Binary search touches only keys; `keys()` becomes a slice; `as_slice() -> &[(K, V)]` has to go (0.5 breaking change) or become `entries() -> impl Iterator`. `range` returns an index pair and zips.
- **Sampled index profile** (`plan::Indexed` → `IndexedFrozenOrderedMap`): keep the ascending array and add a top-level array of every 16th (or `64/size_of::<K>()`-th) key. Lookup = binary search in the sample (hot, L1/L2-resident: 1 M u32 keys → 256 KB) + one cold cache line in the base. This is a static two-level B+-tree ("S+ tree") and is the standard way a sorted array beats a B-tree; expect the 1 024∩1 M gallop case to flip in frost's favour because every gallop probe lands in the hot sample first. Eytzinger layout is the alternative but breaks the ascending-slice contract, so the sampled index is the better fit here.
- Branch-free lower bound over the sample with a fixed iteration count (`len` is a power of two) and `prefetch`-free pure-safe code keeps the "unsafe only in buf.rs" rule.

### 3.4 Opt-in perfect-hash frozen profile (addresses F6 honestly)

`plan::Perfect` → `PerfectFrozenHashMap<K, V, S>`: compute `S::hash_one` per key, build a PTHash/CHD minimal perfect hash over the 64-bit hashes (~2.5–3.5 bits/key, O(n) expected build), store entries densely in MPHF order plus a 8–16-bit fingerprint per slot so misses are rejected without touching the entry. Lookup = hash + MPHF eval (small, cache-resident) + fingerprint check + one entry access + `Eq`. Construction can fail on 64-bit hash collisions among distinct keys → return `Rejected` with the input (fits `TryFreezeWith`). Only ship if the benchmark beats the shrunk SwissTable on *cold* large-n lookups; my warm-cache probe suggests the win is in misses and memory, not hot hits.

### 3.5 Small, mechanical fixes

- `get_or_insert_with` double lookup (F7): compute `linear_pos`/`search` once, promote, then re-search only on the promoted branch.
- `FixedBitSetValues::next` calls the native iterator repeatedly after the first padding index; after one out-of-universe index, set an `exhausted` flag.
- `SetUnion::last()/max()` is O(1) (max of both lasts) — specialise like `count()`.
- `FrozenHashSet::try_from_unique_with_hasher` hashes twice (`contains` + `insert`); with hashbrown as the backing table `entry_ref`/`replace_key` would make it one hash and also enable hash-caching in `AdaptiveHashMap`'s linear entries (`(u64, K, V)` with hash-first comparison for expensive `Eq`). Consider making hashbrown the implementation detail of the std hash families (feature-independent, since std's HashMap already is hashbrown).
- `SetIntersection::new`/`SetDifference::new` could apply `separated()` once (2 comparisons) to empty the cursors immediately; currently only the scalar kernels do.
- `README`/`SET_OPERATIONS.md`: replace the 1 236× strawman hash ratio with the `BTreeSet`/`HashSet` std baselines now in the bench.

---

## 4. More collections the architecture can carry cheaply

These are chosen because the frozen state makes each one *better* than its mutable counterpart, which is the crate's actual thesis; each slots into an existing family/profile/plan pattern.

| Collection | Representation (all safe code) | Why frozen-only wins | Fits as |
|---|---|---|---|
| **Frozen multimap** `FrozenOrderedMultiMap<K, V>` / `FrozenHashMultiMap` | sorted `(K, V)` with `equal_range` via two `partition_point`s; hash: grouped `(K, Box<[V]>)` | `get_all(&k) -> &[V]` is a slice, zero allocation | new family pair; mutable partner = `BTreeMap<K, Vec<V>>` wrapper |
| **Frozen interval map** `FrozenRangeMap<K, V>` | sorted disjoint `(start, end, V)`; `partition_point` on `start`; validated non-overlap at construction | IP/time/Unicode range lookup; construction validation (`Rejected`) is exactly the crate's strict-builder pattern | new family, `TryFreezeWith` |
| **Arena-flattened string set/map** `FrozenStrSet` / `FrozenStrMap<V>` | one `Box<str>` arena + `Box<[u32]>` offsets (+ hash index or sorted order) | n allocations → 1; for 1 M 8-byte strings ≈ 48 MB (`String`s + malloc headers) → ≈ 12 MB; iteration is contiguous | profile of `FrozenHashSet<String>`/`FrozenOrderedSet<String>` via a plan `Flatten` |
| **Jagged frozen sequence** `FrozenJagged<T>` (for `Vec<Vec<T>>`) | flat `Box<[T]>` + offsets | same flattening win; `row(i) -> &[T]` | sequence family profile |
| **Rank/select bitset** `Frozen<FixedBitSet>` + `plan::Ranked` | bits + 1/64 superblock cumulative counts (`u32`/`u64`), optional `select` samples | `rank1(i)`, `select1(k)`, "how many members below x" in O(1); only buildable once bits are fixed | plan on the existing `FixedBitSet` adapter, keeps `ValueElements`/`SetContains` |
| **Elias–Fano sorted integer set** `FrozenEliasFano` | monotone sequence in `2 + ⌈log₂(u/n)⌉` bits/element; high bits as a ranked bitset, low bits packed | ~10× smaller than `Box<[u64]>` for sparse universes; O(1) `select`, O(log(u/n)) predecessor; set algebra by merged `select` scans | profile of `FrozenOrderedSet<u64>` (`plan::Compressed`), complements Roaring (which lacks rank/select/predecessor) |
| **Shared/sliceable frozen sequences and maps** (§3.2) | `Arc<[T]>` + range | O(1) clone and sub-slicing; `SnapshotStore` without outer `Arc` | new profiles |
| **Frozen bimap** `FrozenBiMap<L, R>` | left-sorted `(L, R)` + `Box<[u32]>` permutation sorted by R | two-way lookup with one copy of each element | new family |
| **Perfect-hash map** (§3.4) | MPHF + dense entries + fingerprints | guaranteed single probe; smaller than 7/8-load SwissTable | `plan::Perfect` |
| **Frozen deque** | `VecDeque` → `Compact` already exists | — | already covered; document |

Mutable-side gaps worth closing for symmetry: `MutableOrderedSet`/`MutableHashSet` have no algebra at all (not even `is_subset`), and nothing can intersect a frozen set with a mutable one or with a plain sorted slice. The galloping kernels are already slice-based; exposing them over `&[T]` (`frost::algo::sorted::{intersection, is_subset, …}`) would serve `FrozenOrderedSet`, `MutableOrderedSet` (via `BTreeSet` → not contiguous, skip), `Frozen<Vec<T>>` of sorted data, and user slices with one implementation.

---

## 5. Verification state after the patch

| Check | Result |
|---|---|
| `cargo test --all-features --locked` | 222 passed, 0 failed (was 215; +7 new regressions) |
| `cargo test --no-default-features --features alloc` | 50 passed |
| `cargo test --no-default-features --features alloc,arc-swap-1 --test concurrent` | 7 passed (feature isolation preserved) |
| `cargo build --no-default-features` (core-only) | ok |
| `cargo clippy --all-features --all-targets` (pedantic, `unsafe_code = deny`) | 0 warnings |
| `cargo doc --all-features --no-deps` | 0 warnings |
| `PROPTEST_CASES=3000` on the ordered algebra suite | ok |
| `cargo bench --bench set_operations` | balanced cases within ~15 % of 0.4.0; skewed cases now recorded vs `BTreeSet` |

No new `unsafe`; `unsafe_code = "deny"` is still satisfied outside `adaptive::seq::buf`. The patch is additive on the public API (`SnapshotStore::{load_guard, peek, publish_if_current, update}`); iterator types gained a private field only.

---

## 6. Suggested order of work

1. Land the patch (F1, F3, F4) — verified, additive, closes the two claims most likely to embarrass the crate.
2. F2 (`VecDeque` + merge `extend`) — small, contained, removes an undocumented O(n²).
3. §3.2 shared/sliceable frozen profiles — this is the feature that makes "frozen" mean something beyond "no `&mut` API".
4. §3.3 sampled-index ordered profile — measurable, and it turns the one remaining `BTreeSet` loss into a win.
5. The collections in §4, starting with the multimap, the range map and the flattened string set: they are each under 500 lines on top of existing machinery and have no good std equivalent.
6. §3.4 perfect hashing only after a cold-cache benchmark justifies it.

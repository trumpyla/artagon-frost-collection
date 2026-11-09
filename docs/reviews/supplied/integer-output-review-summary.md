# Supplied integer-output review summary

Provenance: review text supplied by the user in chat on 2026-11-08
(America/New_York). The separate full review and harness mentioned in the text
were not attached to this message. The text below is preserved as supplied;
reproductions and dispositions are recorded separately in
`docs/reviews/integer-output-review.md`.

---

I'll unpack the archive and read through the design, then come back with the review.

Toolchain is in place and the crate builds. Let me run the integer-set tests to establish a baseline while I read the rest of the code.

Both deliverables are ready. The review file follows the same shape as the earlier supplied review in `docs/reviews/supplied/`, so it can drop straight into that folder.

**Bottom line:** no wrong answers, no unsafe, and the arithmetic/alignment hardening holds under attack. The problems are in what the algebra *outputs* and in wrapper overhead — and they're big.

**Top findings (all reproduced in the harness):**

1. **Mixed-representation algebra always degrades to Sorted.** A 244 KiB dense set ∪ a Roaring singleton → 7.8 MiB sorted array in 3.3 ms; clone-and-set-a-bit is 6 µs. Your own `SnapshotStore` test pattern turns Sorted on the first update and never recovers.
2. **Dense∪Dense over the 8 MiB cap falls back to Sorted, not Roaring.** Two 512 KiB dense runs 2^27 apart → 64 MiB; `RoaringTreemap` needs 1.8 KB.
3. **Budgets don't survive construction** — `Dense::binary` hard-codes `IntSetPolicy::auto()`, so an 8-byte budget yields a 128 KiB result, and large budgets get clamped into finding #2.
4. **Same-repr `Eq`/`Hash` is 1,700× slower than native** (per-element enum dispatch); Roaring is built with `collect()` instead of `from_sorted_iter` (2.7×); Dense∩Roaring via `from_lsb0_bytes` promotion is **20× faster** than probing.
5. **Planner picks Roaring for `0..1000`** to save 80 bytes; `contains` is 3× slower than Dense.
6. The recorded bench ran at baseline x86-64 — software popcount for your dense kernels vs. runtime-dispatched SIMD for Roaring. `x86-64-v3` halves dense counts.
7. `int-set` doesn't need `std`: roaring 0.12 is `no_std + alloc`.

One thing I tested and would *not* recommend: galloping Sorted/Roaring with `Iter::advance_to` — roaring's iterator is ~5–8 ns/element and it's slower than your current probing.

**Proposed new structures** (section 4): an arena-frozen single-allocation Roaring (flat chunk index, `Arc`-shareable, zero-copy — the CRoaring "frozen" format idea), a rank-indexed `FrozenIntMap<K,V>` where the set *is* the index, a `FrozenRangeSet` fourth candidate (you already compute `runs`), fence/MPHF indexes to fix the scattered-u64 fallback, Elias–Fano, a persistent chunked set for the snapshot pattern, static/borrowed storage, and a dictionary-compressed set family.

If you want, next step is a verified patch for the one-liners and the promotion matrix (items 1–4), same format as the previous review bundle.

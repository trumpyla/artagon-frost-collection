# Adversarial review of the integer-profile implementation

> Supplied revision-2 review, preserved as historical evidence. The final merge
> decisions and corrections are in [the integration review](revision2-integration-review.md).

This review attacked the implemented `FrozenIntSet` profile (revision 1), after the
[design audit](integer-design-audit.md) had shaped it. The supplied review bundle
under [`supplied/`](supplied/README.md) holds the full text with its probe
measurements, the exploration of lazy views and structural sharing that followed,
and the research report on the state of the art. This page records the findings,
their disposition in revision 2, and what remains open.

## Decision

Keep set algebra eager and value-returning. Fix what it materializes: promote one
operand into the other's representation or edit a copy of the larger operand, share
storage behind an `Arc`, add chunked storage whose containers are shared between
versions, and carry construction limits into results. Offer lazy expression views
only as a future explicit, borrowed API for one-shot queries; never as the result
type of `union` or `intersection`, and never with implicit materialization on
`contains`. Both the exploration and the independent research arrived at this
split: the review's three main problems were representation and sharing problems,
and element-at-a-time lazy evaluation over the Roaring iterator measured slower
than probing.

## Findings and disposition

| Severity | Finding | Evidence (revision 1) | Resolution (revision 2) |
| --- | --- | --- | --- |
| High | Every mixed-representation operation materialized a sorted array; one tiny operand destroyed a compact one, and the snapshot pattern (`current.union(&singleton)`) turned sorted on the first update. | 244 KiB dense ∪ Roaring{1} → 7.8 MiB sorted in 3.3 ms; 6 µs attainable | Promotion matrix in [`storage.rs`](../../crates/frost-collections-integer/src/integer/storage.rs): dense windows are copied and edited for small operands, promoted with `from_lsb0_bytes` for balanced Roaring pairs, Roaring copies are edited for small sorted operands. Measured 83× (dense ∪ singleton), 116× (dense ∖ sorted{4}), 18–39× (dense/Roaring pairs). |
| High | Dense ∪ dense beyond the window cap fell back to sorted, the only representation always larger than the window it refused. | two 512 KiB islands → 64 MiB sorted in 53 ms; Roaring 1.8 KB | Both windows are promoted to Roaring and combined natively (118×). |
| Medium–High | Budgets were not retained; `Dense::binary` hard-coded the default policy, violating small budgets upward and clamping large ones. | 8-byte budget produced a 128 KiB result | Sets carry their limits (`policy()`); results inherit the larger operand limit; result windows use `max(budget, operand payloads)`. Tested in `budgets_are_inherited_by_results`. |
| Medium | Same-representation `Eq`/`Ord`/`Hash` walked every element through the iterator enum. | Roaring == Roaring 1,700× slower than native | Native comparisons per representation, allocation-identity check first, chunked hashing. Measured 53–1,950×. |
| Medium | Roaring candidates were built with `collect()`; automatic planning ran a second full scan. | 2.7× slower than `from_sorted_iter` | Bulk `append` for sparse stretches and `insert_range` for runs of ≥ 32, then run optimization. Roaring construction 1.5–10.7× faster, automatic 1.2–6.6×. |
| Medium | Dense/Roaring pairs probed element-wise; the direction rule ignored dense word traversal. | 27 µs where promotion took 1.3 µs | Work-estimate strategy (`Promote`, `IterateDense`, `IterateRoaring`) shared by counts, predicates and materialization. |
| Medium | Memory-only planner chose Roaring for small consecutive runs. | `contains` 3.2× slower than dense to save 80 bytes | 4 KiB dense floor in `plan()`. |
| Medium | Dense kernels used a double-checked per-word accessor and a separate popcount pass; the recorded benchmark ran without POPCNT while Roaring used runtime SIMD. | 2× on counts at `x86-64-v3` | Three-region slice kernels with the count fused into the pass (2.2–2.9× on materialized dense operations at the baseline target). `target-cpu` is documented; runtime dispatch without crate-local `unsafe` remains open. |
| Medium | Construction stages all codes in a vector; no range constructor. | 100 M-member dense set needs 800 MB staging | **Open.** Recorded in the [proposal](../design/proposals/integer-collections.md) as streaming and range constructors. |
| Low–Medium | `int-set` required `std` although `roaring` 0.12 is `no_std + alloc`. | manifest | `int-set = ["alloc", "dep:roaring"]`; `std` forwards `roaring?/std`. Built for `thumbv7em-none-eabihf`. |
| Low–Medium | No `IntKey` for narrow and address-sized integers, `char` or small `NonZero` types; no adapters; no `rank`/`select`/range API. | — | `u8`, `u16`, `i8`, `i16`, `char`, `usize`, `isize`, `NonZeroU8`, `NonZeroU16` added with checked decoding. Adapters, rank/select and ranges remain open (proposal). |
| Low | Tests asserted logical answers only; representation collapse was invisible. | — | Tests assert result representations and payload bounds, budget inheritance, storage sharing, container sharing, far-apart dense union → Roaring, two-cluster proptests over four representations, narrow-key round trips. |

## What was attacked and held

Coordinate alignment at `u64::MAX`, `i64::MIN` and the `u32` top word; `u128` spans;
dense iterator overflow at the last word; codec-law enforcement for colliding,
reversing, non-canonical and time-varying codecs; representation-independent
equality and hashing; `separated()` and subset fast paths for empty operands;
overflow discipline in union counts; the sealed `IntWord` boundary; iterator fusion
and size hints; absence of `unsafe`. None produced a wrong answer or a panic on
valid input.

## Measurement notes

Two-binary benchmark comparisons on the review host moved unchanged `std::BTreeSet`
control loops by up to 2× through code placement alone. The before/after claims
above therefore come from a same-binary harness that links both versions of the
profile, validates every answer and alternates measurements; its CSV, the three
repository benchmark runs and the harness source are under
[`verification/int-set/revision2/`](../../verification/int-set/revision2/README.md),
and [the benchmark report](../../crates/frost-collections-integer/docs/integer-benchmarks.md) explains the protocol and
lists the cases that remained slower.

## Open items

- Streaming and range constructors (no staging vector for explicit policies).
- Serde/Borsh/Schemars/Arbitrary/Rayon adapters for `FrozenIntSet`.
- `DoubleEndedIterator`, `rank`, `select`, range queries, k-way `union_all`.
- Runtime POPCNT/SIMD dispatch for dense kernels without crate-local `unsafe`.
- Explicit borrowed expression views with per-window evaluation.
- A frozen chunked *arena* (single allocation, zero-copy) and the other collections
  in the [proposal](../design/proposals/integer-collections.md).

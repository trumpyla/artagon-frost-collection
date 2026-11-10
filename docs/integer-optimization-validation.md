# Integer and family optimizations: validation and measurements

Date: 2026-11-10. Baseline: `main` at `22697a8`. This record covers the series
that the [changelog](../CHANGELOG.md) lists under three Unreleased sections:
integer profile optimizations, family construction and iterator costs, and
emptied adaptive tree ranges. The design decisions are in
[decisions](design/decisions.md).

The series changes costs and fixes three defects. It adds no public item,
feature, dependency, representation candidate, or policy limit, and it does not
change the Hash event stream.

## What the series contains

| Change | Status | Evidence |
| --- | --- | --- |
| `fold` and `count` on `IntSetIter` and `EncodedValues` | Delivered | 1.7x to 17x on `fold` traversal; `for` loops unchanged |
| Sorted result path: specialized merges, segment copies for skewed operands, partition bound from cached bounds, early survey exit | Delivered | 1.8x to 8x on skewed Sorted updates; balanced merges within the control variation |
| Construction: classify staged codes, bitmap sort for dense unsorted input | Delivered, second design | 3.1x to 4.9x on dense shuffled input; ascending input at or above the baseline |
| Cached native Roaring statistics | Delivered | Identity shortcuts on a large Roaring operand: 36 us to 23 ns. Each storage root grows by 8 or 16 bytes |
| Stride probe in sorted Roaring construction | Delivered | 1.12x to 1.34x, inside the control variation; not claimed as a gain |
| `fold` on the ordered union and difference cursors | Delivered | 2.1x to 7.6x when one operand ends early; balanced inputs unchanged |
| Result sharing on the scattered and empty result paths | Delivered (defect) | 247 deviations before, none after |
| `AdaptiveOrderedMap::range` on an emptied tree | Delivered (defect) | Failing test before the change |
| Bulk extension, bulk ordered collection, iterator end queries | Delivered | 1.6x to 24x; `last` and `max`: 0.5 ms to 4 ns |
| Hash extension on a nonempty table (follow-up) | Delivered (defect) | Hash calls and capacity equal the standard collections; 2x hash calls before |
| Direct merge for Chunked Array pairs | Withdrawn | 17% to 35% slower than the baseline for union, difference, and symmetric difference on three datasets |
| Order tracking while staging (first construction design) | Replaced | 7% to 24% slower than the baseline on ascending input |
| Hash event stream | Not changed | A faster stream changes a recorded contract |
| Allocation-free empty set | Not changed | `FrozenIntSet::new` allocating its header is a recorded contract |

## Tests added

Seventeen tests, each with an `unreviewed` catalog record:

- `tests/integer_sets.rs`: `public_iterator_folds_resume_after_partial_consumption`,
  `construction_normalizes_every_arrival_order_identically`,
  `construction_reports_contract_errors_on_every_path`,
  `scattered_sorted_results_share_an_equal_operand`,
  `empty_results_share_an_eligible_empty_operand`. The existing cursor helper
  also checks `fold` after every partial consumption.
- `src/integer/storage.rs`: `sorted_kernel_matches_member_algebra_across_shapes`,
  `early_survey_exit_selects_the_same_backend_as_a_complete_survey`,
  `partition_span_bounds_the_occupied_partitions`,
  `cached_native_statistics_match_a_fresh_summary`.
- `src/integer/storage/word.rs`:
  `sorted_roaring_builder_matches_sorted_import_across_run_shapes`,
  `sorted_roaring_builder_is_independent_of_run_alignment`.
- `tests/ordered_set_ops.rs`:
  `folds_match_stepwise_traversal_for_merge_and_seek_kernels`.
- `tests/contracts.rs`:
  `adaptive_ordered_range_on_an_emptied_tree_matches_the_sorted_form`,
  `bulk_extension_keeps_representatives_policies_and_partial_progress`,
  `collected_ordered_families_match_entry_by_entry_insertion`,
  `iterator_end_queries_match_a_complete_traversal`,
  `hash_extension_reserves_like_the_standard_collections`.

The construction tests compare every arrival order, including a codec that
violates the order law, with a reference that implements the two-pass contract.
The range test, the two sharing tests, and the hash extension test failed
before their fixes.

## Independent differential review

A separate review wrote a seeded differential suite that compares
`FrozenIntSet` with `BTreeSet<u64>` models of the encoded codes. It covers all
16 representation pairs, twelve key types, domain extremes, large dense
windows, every pair of Chunked container classes, chunk sharing, codec call
counts in bulk paths, and long update walks. The suite is a scratch harness of
about 2,600 lines that runs for 23 minutes; it is not part of this series.

- On `main` it found no wrong answer. It recorded results that equal an
  eligible operand but do not share its root: 247 in a directed sweep. That is
  the sharing defect in the table above.
- On this series it passed with 15 of 15 tests and no sharing deviation. The
  log of the run on the final commit is
  [review-differential-on-branch.log](../verification/integer-optimizations/results/review-differential-on-branch.log).

The review of the integer profile ended before it wrote a report, so its
performance observations, if any, are not available. The review of the other
modules completed; its findings are below.

## Measurement method

The harness in `verification/integer-optimizations/ab-harness` links two
revisions of the crate into one executable: the baseline under a renamed
package, and the working tree. Each case first checks its answer against the
standard library. The two implementations then run alternately on identical
data. The reported time is the minimum of the per-round medians of five samples.
Allocation, construction, and destruction of results are inside the timed
region for materializing cases.

Host: 2 virtual CPUs (Intel Xeon, 2.10 GHz, AVX2 and AVX-512F reported), Linux
6.18, Rust 1.99.1, release profile with thin LTO and one codegen unit, default
target CPU. Runs were pinned to one CPU with no other job running.
[environment.json](../verification/integer-optimizations/environment.json)
records the details.

### Control and limits

A control run compares commit `82f4d5c` with itself. That commit is the final
tree before the score field was narrowed. Identical source on both sides gave
ratios from 0.62 to 1.72 across 630 cases; 109 cases were outside
0.90 to 1.10 and 42 were outside 0.80 to 1.25. The median was 1.00. Two copies
of the same function at different addresses, and operands at different heap
positions, are enough to produce that spread on this host.

Therefore:

- A ratio below about 1.5 for one case is not evidence of a change. This record
  claims a gain only where both runs show at least 1.5x and the control for
  that case is near 1.
- The figures are for this host and these workloads. They are not universal
  speedups, and no threshold in the code was tuned from them.
- The run is same-host and same-day. Nothing here is a regression budget.

## Results

Two runs of nine rounds each, baseline `22697a8` against the final tree
(`90f2a4f`). Times are from the first run. "Ratio" is baseline time over new
time for both runs. Baseline times for one case differ between runs and
executables by up to a factor of two on this host; read the ratios, not the
absolute times.

| Case | Baseline | New | Ratio |
| --- | ---: | ---: | --- |
| `IntSetIter::fold` sum, Sorted, 200k members | 465 us | 28 us | 16.7, 17.4 |
| `EncodedValues::fold` sum, Sorted | 467 us | 29 us | 16.3, 16.8 |
| `fold` sum, Chunked | 1,131 us | 139 us | 8.1, 11.5 |
| `fold` sum, Dense | 1,003 us | 337 us | 3.0, 2.7 |
| `fold` sum, Roaring | 1,568 us | 897 us | 1.7, 2.2 |
| `for` loop sum, all four representations | | | 0.88 to 1.20 |
| Sorted union, one operand much smaller (`u32.skew`) | 94 us | 12 us | 8.0, 8.1 |
| Sorted difference, same shape | 375 us | 150 us | 2.5, 2.4 |
| Sorted symmetric difference, same shape | 441 us | 180 us | 2.4, 1.8 |
| Sorted 200k union with 1 member | 836 us | 222 us | 3.8, 3.7 |
| Sorted 200k union with 100 members | 857 us | 244 us | 3.5, 3.5 |
| Sorted 200k difference with 1 member | 709 us | 225 us | 3.1, 3.3 |
| Construction, u32 dense shuffled, Sorted policy | 3,710 us | 769 us | 4.8, 4.9 |
| Construction, u32 dense shuffled, automatic policy | 4,277 us | 1,277 us | 3.4, 3.1 |
| Construction, u64 sparse ascending, automatic policy | 1,087 us | 708 us | 1.5, 1.5 |
| Roaring union with an empty set (identity shortcut) | 36 us | 23 ns | about 1,500 to 2,100 |
| Roaring union with its own root | 66 us | 30 ns | about 1,600 to 2,200 |
| Ordered `difference().fold`, 1 excluded key | 132 us | 17 us | 7.6, 7.1 |
| Ordered `union().fold`, 1 added key | 36 us | 17 us | 2.1, 2.5 |
| `InlineVec::extend_from_slice`, spilled, 1 MiB of bytes | 986 us | 40 us | 24.4, 22.7 |
| `InlineVec::extend_from_slice`, spilled, u64 | 152 us | 43 us | 3.5, 3.7 |
| `MutableHashSet::from_iter`, 200k | 6.8 ms | 3.5 ms | 1.9, 1.9 |
| `AdaptiveHashMap::from_iter`, 200k | 6.6 ms | 3.5 ms | 1.9, 2.1 |
| `MutableOrderedMap::from_iter`, 200k sorted | 17.2 ms | 1.8 ms | 9.7, 11.6 |
| `MutableOrderedMap::from_iter`, 200k random | 30.6 ms | 9.2 ms | 3.3, 3.3 |
| `MutableOrderedSet::from_iter`, 200k sorted | 16.3 ms | 1.5 ms | 10.7, 10.7 |
| `MutableOrderedSet::from_iter`, 200k random | 28.4 ms | 6.0 ms | 4.7, 5.1 |
| `MutableOrderedMap::iter().last()`, 200k | 515 us | 4 ns | full traversal to one descent |
| `MutableOrderedMap::keys().max()`, 200k | 631 us | 5 ns | full traversal to one descent |
| `MutableOrderedMap::iter().nth(100k)` | 342 us | 215 us | 1.6, 1.8 |

The control ratio for each row above is between 0.79 and 1.15, with two
exceptions: 1.27 for the Dense `fold` and 1.53 for dense shuffled construction
under the automatic policy. Both gains remain well above their control.

Geometric means over the dataset and policy combinations of each general
operation (29 or 95 cases each), for the two runs and the control:

| Operation | Run 1 | Run 2 | Control |
| --- | ---: | ---: | ---: |
| `construct` | 1.15 | 1.16 | 1.03 |
| `snapshot_update` | 1.14 | 1.15 | 1.00 |
| `symmetric_difference_materialize` | 1.08 | 1.07 | 1.02 |
| `union_materialize` | 1.08 | 1.08 | 0.98 |
| `difference_materialize` | 1.06 | 1.05 | 1.02 |
| `intersection_materialize` | 1.01 | 1.02 | 1.01 |
| `clone` | 1.01 | 0.99 | 1.00 |
| `is_subset` | 0.99 | 0.99 | 1.06 |
| `equality` | 0.99 | 0.95 | 1.02 |
| `intersection_count` | 0.98 | 0.99 | 1.01 |

### No measurable change

- `for` loops and `collect` over a set. They call `next`, which this series
  does not change.
- Balanced Sorted merges, balanced ordered-cursor `fold`, and `collect` from the
  ordered cursors.
- Dense/Dense, Roaring/Roaring, and Chunked/Chunked algebra, `clone`,
  `is_subset`, and `intersection_len`.
- Construction from ascending or descending input: 1.06 to 1.33 in the series
  cases. That is at or above the baseline but inside the range the control
  shows for identical code, so it is not claimed as a gain.
- Sorted Roaring construction with the stride probe: 1.12 to 1.34 across five
  shapes, with the control at 1.05 to 1.11. Not claimed as a gain.
- The 64-step Roaring update chain. An earlier single-round reading of 1.34
  did not repeat.

### Slower observations

Eleven cases were at or below 0.87 in both final runs. Six are `equality`
cases, at 0.72 to 0.85: for example Sorted equality on the `i64.across_zero`
dataset, 1.5 us against 1.8 us. The source of those comparisons is the same on
both sides: a slice comparison for Sorted and the derived comparison for
Dense. The times fall into two modes that also appear for the baseline itself
in different executables (1.1 us and 1.6 us for the same Sorted case), and the
control shows 0.71 and 1.34 for two Sorted equality cases with identical
code. The evidence points to operand placement, not to this series, but this
host cannot settle it. The geometric mean for `equality` over 29 cases is
0.99 and 0.95.

One contributing cost was found and reduced. The cached Roaring statistics
first held the score as `u128`, which raised the alignment of every storage
root to 16 bytes and its size from 64 to 96 bytes (32-bit words) and from 72
to 112 bytes (64-bit words). With the score in 64 bits the roots are 72 and 88
bytes: 8 and 16 bytes more than the baseline, for every set, including the
empty set. The `equality` mean was 0.92 and 0.93 before that change.

The other five cases are mixed-representation `intersection_len` and
intersection cases on the `u32.skew` and `u32.runs` datasets, at 0.48 to 0.85.
This series does not change those kernels, and the control for the lowest
one is 1.72.

Two designs measured slower than the baseline and are not in the final tree.
Their raw results are kept under
`verification/integer-optimizations/results/superseded-ce24b6c/`. The two full
runs at `82f4d5c` are under `results/at-82f4d5c/`.

- The direct merge for Chunked Array pairs: 0.65 to 0.83 of baseline speed for
  union, difference, and symmetric difference on the `i64.across_zero`,
  `u64.scattered_prefixes`, and sparse-container datasets, in 15 rounds, with
  the control for those cases between 1.00 and 1.20.
- Order tracking while staging: 0.76 to 0.93 on ascending input under the
  Sorted and Dense policies, with the control between 0.92 and 1.07.

A stale-baseline defect in `prepare-baseline.sh` was found during this work.
`git archive` gives files their commit time, and Cargo treated the older
sources as unchanged, so a switch back to an older baseline kept the newer
one. The script now extracts with the current time. Runs taken with the stale
baseline were discarded.

## Gates

All commands ran on the final tree on the host above.

| Command | Result |
| --- | --- |
| `make check` | Exit 0. Formatting, document links, Clippy with `-D warnings` on all targets and features, rustdoc, tests, examples, catalog, and benchmark inventory. 390 tests passed and none failed across 53 result lines, which include doctests and the catalog tool's own tests. |
| `make integers` | Exit 0. Integer tests without default features, in release with `arc-swap-1`, and the downstream codec crate. |
| `make embedded` | Exit 0. `thumbv7em-none-eabihf` and `thumbv6m-none-eabi`. |
| `make profiles` | Exit 0. Core-only, alloc-only, and default profiles. |
| `make features` | Exit 0. Each optional feature alone. |
| `make examples` | Exit 0. |
| `make package` | Exit 0. Build and test of the extracted package. |

Not run, because the tool is not available on this host:

- `make miri`. The Miri component is not available for the 1.99.1 toolchain
  here. This series adds no `unsafe` and does not change `src/adaptive/seq/buf.rs`.
  `InlineVec::extend_from_slice` calls `Vec::extend_from_slice` in safe code.
- `make shell-check`. ShellCheck, shfmt, and Bats are not installed. The series
  adds one shell script and one Make target; neither has been linted.
- Loom, Kani, coverage, mutation, and fuzz targets. The series does not touch
  the snapshot protocol.

`make verify` is `check`, `profiles`, `features`, `integers`, `examples`, and
`package`. Each ran as its own command.

## Follow-up after delivery

A later review of the map and set modules found that `MutableHashMap::extend`
reserved the whole size hint on a nonempty table, and that this series had
copied that rule into `MutableHashSet::extend` and `AdaptiveHashMap::extend`.
Extending with keys already present doubled the table. The three paths now
reserve as the standard collections do. The comparison results above predate
this change; the `from_iter` cases start from an empty table and take the same
path as before. `make check` passed again after the change. The other gates
were not repeated for it.

## Findings not changed

From the review of the non-integer modules. Each is a cost, not a wrong answer,
except where noted.

| Finding | Reason it is not changed here |
| --- | --- |
| `InlineBuf::clone` copies element by element; the review measured it 2x to 55x slower than `Vec::clone` for large spilled buffers. | The code is in the crate's only `unsafe` module. A change needs an ownership review and Miri, which was not available. |
| `InlineBuf::from_iter` and `from_vec` move elements one at a time. | Same module, same requirement. |
| Inline `retain` visits the inline capacity, not the length. | Same module, same requirement. |
| `MutableOrderedMap::append` measured 2.7x to 3.7x slower than `BTreeMap::append` when both maps have similar sizes. | A fix needs a size-ratio gate between per-key insertion and a bulk rebuild. One host's timings do not justify a threshold. |
| `AdaptiveFrozenHashMap::thaw` keeps the frozen table's capacity in linear form; the review reported 47x slower lookups in one large case. | The representation after `thaw` is a policy decision. |
| `MutableOrderedMap::range` and `MutableOrderedSet::range` panic on inverted bounds for a tree emptied by removal. | Documented as native `BTreeMap::range` behavior. The adaptive map, which documents the opposite, is fixed. |
| The borsh adapter orders keys with `>=` where the family uses `Ord::cmp`. | Reported only. It matters for key types whose `PartialOrd` and `Ord` disagree. |
| `Arbitrary::size_hint` bounds are loose. | Reported only. |

From the later review of the map, set, and sequence modules. The figures come
from a separate probe executable on the same host, with seven repetitions and
the minimum reported. They were not taken with the comparison harness.

| Finding | Reason it is not changed here |
| --- | --- |
| `FrozenOrderedMap::get` with `String` keys measured 1.1x to 1.7x slower than `BTreeMap::get`, for hits and misses, at 1,000 and 100,000 entries and three key shapes. A plain `binary_search_by` over the same vector measured the same. With `u64` keys the frozen map measured 0.43x to 0.79x of the tree's time. | The cost belongs to binary search over entries whose keys need a pointer dereference. A different layout is a representation decision, and `as_slice` exposes the current one. No benchmark in the repository covers this lookup. |
| `MutableOrderedMap::try_from_unique` inserts entry by entry with two descents each. It measured 6.1x (random) and 14.3x (sorted) slower than `FrozenOrderedMap::try_from_unique` followed by `thaw`, at 200,000 entries. | The two builders report different duplicates: the first in input order, and the second occurrence of the smallest duplicated key. A bulk build must keep the documented one. |
| `extend` into an empty `MutableOrderedMap`, `MutableOrderedSet`, or `AdaptiveOrderedMap` measured 3.3x to 8.8x slower than `from_iter` with the same input. `BTreeMap::extend` has the same cost. | An empty receiver could take the bulk path. It is an addition, not a correction. |
| `AdaptiveFrozenHashMap::from_iter` builds a hash table before it selects the linear form. Up to 8 entries it measured 1.3x to 3.1x slower than building through `AdaptiveHashMap`. Above the threshold the order reverses: 2.1x at 9 entries. | A fix needs a builder that stages up to the threshold before it chooses. |
| `nth_back` is not forwarded by the borrowed iterators of `FrozenOrderedMap`. `iter().rev().nth(100_000)` measured 47 us; `nth`, `last`, and `count` on the same iterator measured 1 ns to 3 ns. | Small. `SortedIter`, `Keys`, `Values`, and the `double` flag of `wrap_iter!` need the override. |
| `InlineVec::truncate` and `clear` drop inline elements from the back, and spilled elements from the front, as `Vec` does. | The order is observable, but a forward drop needs new `unsafe` in the inline buffer and Miri. |
| The strict hash builders hash each key twice: a membership test, then the insertion. | The standard collections return no rejected key from an occupied entry. The strict builder measured 0.92x of the lenient one's time, so the cost is small. |

The Hash stream costs about 2.5 times a plain `write_u64` loop for a large
scattered set with SipHash: 518 us against 204 us per 100,000 members in a
separate probe. The one-byte tag before the members leaves each following
8-byte write unaligned in the hasher's buffer. A `u64` tag or block writes
would remove the cost and change the event stream.

## Reproduce

```sh
cd verification/integer-optimizations/ab-harness
./prepare-baseline.sh 22697a8
cargo build --release
taskset -c 1 target/release/frost-integer-optimizations-ab 9 > run.csv
AB_FILTER=construct,fold taskset -c 1 target/release/frost-integer-optimizations-ab 15
```

For the control, prepare the baseline from the revision under test.

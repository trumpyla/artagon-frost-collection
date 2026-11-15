# Integer output benchmark comparison

> Historical evidence for the source revision named below. The current merged
> implementation is documented in [revision-2 integration validation](../../../docs/revision2-integration-validation.md).

The final implementation preserves compact output in the reviewed workloads and
removes large per-member costs from equality and long-run hashing. The retained
measurements also show regressions in balanced Sorted-minus-Roaring difference,
several member-hashing cases, and some very small intersections. This report
describes the completed integer-output correction, not the later lazy-view or
structural-sharing proposals.

The source of every final timing below is
[comparison.csv](../../../verification/integer-output/comparison.csv), also provided as
[JSON](../../../verification/integer-output/comparison.json). Raw samples, logs,
compiler records, input checks, source hashes, comparison lockfiles, and the
complete prior source archive are in
[the evidence directory](../../../verification/integer-output/README.md). Correctness
and release-gate scope are documented separately in
[integer output validation](integer-output-validation.md).

## Method and interpretation

The same [public-API harness](../benches/integer_output.rs) and
[timing support](../../frost-collections-test-support/src/bench.rs) were compiled against the previously
delivered full archive and the final candidate. The comparison uses Rust
**1.99.1**, LLVM **23.1.1**, edition 2024, on Linux x86-64, with release
optimization level 3, LTO disabled, 16 codegen units, and unwinding panics.
The standalone manifests enable the same `std`, `int-set`, and `roaring-0_12`
features. All **27 resolved dependency pins**, including versions, sources and
checksums, match between versions; Roaring remains **0.12.0**. The
[metadata](../../../verification/integer-output/bench-metadata.json),
[compiler record](../../../verification/integer-output/rustc.txt), and
[comparison script](../../../verification/integer-output/compare.py) record the setup.

Two compiler profiles are kept separate:

| Label in this report | Compiler flags |
| --- | --- |
| Baseline | `-C target-cpu=x86-64` |
| v3 | `-C target-cpu=x86-64-v3` |

Timed binaries ran serially on CPU 0, in **before / after / after / before**
order within each profile, after compilation completed. There are **115 cases
per execution, eight executions, and seven retained samples per case:
115 × 8 × 7 = 6,440 samples**. Each version/profile/case therefore has **14
samples**, pooled across its two runs. The tables report their pooled median,
not the median of two run medians. Both run medians and all raw samples remain
available; no slower samples were discarded. These are descriptive warm-cache
wall-clock observations, not confidence intervals or results from multiple
machines.

The timer calibrates batches to at least approximately four milliseconds before
recording seven per-operation sample averages. Inputs and complete-answer checks
are outside timing; materialized output allocation and destruction are inside.
The 32-update case includes cloning the starting value, then deriving 32 updated
values. It does **not** time `SnapshotStore` atomic publication or concurrent
contention. Membership rows time the whole **1,024-query batch**. Construction
rows include the operations named in their labels; Frost validation and
normalization are included. See the
[reproduction instructions](../../../verification/integer-output/README.md) for exact
commands and the boundaries of each native control.

## Compact output: exact payload evidence

The timing harness's representation observations are preserved in the
[before](../../../verification/integer-output/before-baseline-verify.log) and
[after](../../../verification/integer-output/after-baseline-verify.log) verification
logs. A separate candidate-only
[payload reproducer](../../../verification/integer-output/payload.rs) asserts actual
contiguous buffer lengths through `contiguous_payload_bytes()`; its results are
in [payload-check.csv](../../../verification/integer-output/payload-check.csv).

| Workload | Before | Final candidate |
| --- | --- | --- |
| One million alternating u64 values, union a Roaring singleton | Sorted, **8,000,008 payload bytes** | Dense, **250,000 exact payload bytes** |
| Same input after 32 distinct singleton additions | Sorted, **8,000,256 payload bytes** | Dense, **250,000 exact payload bytes** |
| Two singleton Dense sets constructed with an 8-byte Dense budget, far apart | Dense, **131,080 minimum payload bytes** | Sorted, **16 exact payload bytes**, retaining the **8-byte Dense budget** |
| Two dense runs, each 4,194,304 members, separated by `2^27` | Sorted union, **67,108,864 payload bytes** | Roaring union, **8,388,608 members**; contiguous payload accessor returns `None` |

Both distant Dense inputs have **524,288 exact payload bytes** in the candidate.
Their matching native Roaring union has **1,824 serialized bytes**. That is a
wire-format observation of the native control, **not a resident-memory
measurement of either native or Frost output**. `None` likewise does not mean
zero allocated bytes. Sorted sizes are cardinality times the encoded word
width, excluding wrappers and allocator overhead. The historical API exposes
only bounds for Dense, so its 131,080-byte figure is a minimum computed from
aligned bounds. The payload limit governs the Dense candidate, not the Sorted
fallback, staging allocations, or total peak memory.

The balanced Sorted/Roaring workload also changes output: its union goes from
a 12,000,000-byte Sorted payload to Roaring; its Sorted-minus-Roaring difference
goes from a 4,000,000-byte Sorted payload to Dense with a 250,000-byte minimum
window. That compact difference output comes with a measured time regression,
reported below. The shared harness does not measure its peak allocation.

## Algebra and repeated updates

Times are **microseconds per operation**. Speedup is before divided by after,
within the same compiler profile.

| Workload | Baseline before | Baseline after | Speedup | v3 before | v3 after | Speedup |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Dense union Roaring singleton | 1,025.254 | 4.805 | 213.39× | 1,017.352 | 4.755 | 213.95× |
| Dense XOR absent Roaring singleton | 1,033.846 | 4.783 | 216.16× | 1,010.025 | 4.842 | 208.59× |
| Dense difference present Roaring singleton | 4,072.534 | 4.790 | 850.15× | 3,976.442 | 4.686 | 848.65× |
| 32 updates, including initial clone | 19,084.433 | 154.668 | 123.39× | 17,040.531 | 153.963 | 110.68× |
| Union of distant Dense runs | 35,586.224 | 348.385 | 102.15× | 35,109.079 | 231.773 | 151.48× |
| Balanced Sorted/Roaring union | 13,477.150 | 9,159.849 | 1.47× | 13,062.888 | 6,332.706 | 2.06× |
| Balanced Roaring-minus-Sorted difference | 36,214.197 | 6,316.899 | 5.73× | 35,539.779 | 8,040.606 | 4.42× |
| Dense/Roaring overlap count | 6,739.167 | 84.031 | 80.20× | 6,917.496 | 64.755 | 106.83× |
| Dense/Roaring overlap materialization | 7,339.754 | 119.468 | 61.44× | 7,499.006 | 67.382 | 111.29× |

The tiny-update result is close to the raw Dense clone-and-set control in this
workload: **4.847 µs** baseline and **5.154 µs** v3 in the final executions.
It remains a payload copy; this revision does not implement persistent shared
containers. The native Roaring clone-and-insert control is **5.896 / 6.053 µs**.

Distant Dense union still has substantial conversion work. A native union of
already prepared Roaring inputs is **6.881 / 6.803 µs**, while Frost starts with
Dense operands and produces a permitted compressed result. Comparing those
times does not isolate wrapper overhead.

### Promotion controls have a narrower input contract

For the overlap workload, the final native
`promotion_and_materialize` control takes **46.801 µs** baseline and **23.482 µs**
v3. Its timing includes `from_lsb0_bytes` and native intersection, but its input
byte vector was **built before timing**. Frost starts with Dense u64 words and
also constructs the byte buffers during the timed operation, then applies its
result metadata and policy handling. Consequently the remaining **2.55× / 2.87×**
time ratio is **not a measurement of pure wrapper overhead**. The same
prebuilt-byte qualification applies to `promotion_and_count`; the separately
named `cached_promotion_count` excludes promotion altogether. The exact setup
is visible in `dense_roaring_intersection` in the common harness.

This distinction does not negate the avoided per-member path. It limits how
precisely this harness can attribute the remaining difference. Dense/Roaring
count may now allocate temporary promotion storage; count returning a scalar
does not imply allocation-free execution.

## Equality, hashing, construction and membership

### Equality and long runs

Equal values are independently cloned before timing, rather than comparing a
set with itself. Same-representation equality now uses contiguous or native
comparisons. For a million consecutive u64 values:

| Equality representation | Baseline before → after (µs) | v3 before → after (µs) |
| --- | ---: | ---: |
| Sorted | 4,060.018 → 206.887 | 4,921.060 → 198.818 |
| Dense | 4,227.154 → 2.234 | 4,570.155 → 2.188 |
| Roaring | 17,419.457 → 0.060 | 17,862.961 → 0.059 |

The extreme Roaring improvement is specific to run-compressed equal operands
whose native representation is very small. It is not a constant-time equality
claim for arbitrary million-member sets. For one million alternating values,
final Roaring equality takes **4.747 / 4.667 µs**; for 2,048 scattered upper
prefixes, it takes **15.650 / 14.573 µs**.

Hash can prove that an entire set is one contiguous run from cached cardinality
and bounds, then hash that canonical description. A million-member run hashes
in approximately **24–25 ns** in each representation/profile, down from
millisecond traversal. Eight runs totaling 32,768 values also benefit:

| Logical Hash representation, eight runs | Baseline before → after (µs) | v3 before → after (µs) |
| --- | ---: | ---: |
| Sorted | 162.870 → 32.163 | 145.368 → 27.926 |
| Dense | 157.318 → 1.372 | 145.410 → 1.248 |
| Roaring | 364.895 → 0.301 | 349.167 → 0.306 |

Hash format selection depends on logical content so equal sets make the same
Hasher calls across representations. It uses a small member stream or at most
16 maximal runs, falling back to members when the bounded scout finds a
seventeenth run. It does not promise stable hash bytes across crate versions.
The comparisons above and below time the public Hash operation on the same
logical sets before and after, with each version's protocol.

Roaring 0.12 has no native `Hash`. The harness's custom native member hash and
raw buffer hashes are explicitly different protocols. Hashing a Dense buffer's
words or a Sorted slice in bulk is not a valid interchangeable semantic
baseline for cross-representation Frost hashing.

### Validated construction and small-set membership

| Workload | Baseline before → after (µs) | v3 before → after (µs) |
| --- | ---: | ---: |
| Forced Roaring build, million-value run | 13,186.089 → 8,182.894 | 12,167.445 → 8,301.295 |
| Forced Roaring build, million alternating values | 15,671.092 → 8,766.002 | 14,304.297 → 8,348.326 |
| Forced Roaring build, 2,048 upper prefixes | 272.197 → 242.396 | 280.925 → 251.640 |
| Auto `0..1000`, batch of 1,024 queries | 2.870 → 1.125 | 2.705 → 1.131 |

Validated construction is **31.77–44.06% faster** on the million-member cases;
the sparse-prefix case improves about 10–11%. Frost still performs codec checks
and normalization. For example, native sorted-iterator construction of the
million-value run is **6,187.789 / 6,215.207 µs** in the final executions, so
these controls do not support a claim that validation is free.

Auto now chooses Dense for `0..1000`, using the documented 128-byte small-saving
preference. The observed batch improves **2.55× / 2.39×**. This single query mix
does not establish a general workload-optimal threshold.

## Remaining regressions and variation

The table includes every non-skewed Frost case whose pooled time increased by
at least 10% in either profile. Changes are percentages of the before time;
positive values mean slower. Tiny skewed cases are reported separately below.

| Workload | Baseline before → after (µs) | Change | v3 before → after (µs) | Change |
| --- | ---: | ---: | ---: | ---: |
| Balanced Sorted-minus-Roaring difference | 7,945.974 → 9,475.134 | +19.24% | 7,687.652 → 9,644.864 | +25.46% |
| Million alternating values, Sorted Hash | 4,706.751 → 5,396.043 | +14.64% | 4,320.019 → 5,283.969 | +22.31% |
| Million alternating values, Dense Hash | 4,496.667 → 5,949.480 | +32.31% | 4,603.800 → 5,458.831 | +18.57% |
| Seventeen runs / 69,632 members, Sorted Hash | 332.477 → 425.908 | +28.10% | 294.040 → 427.379 | +45.35% |
| Seventeen runs / 69,632 members, Dense Hash | 326.704 → 406.242 | +24.35% | 334.248 → 379.414 | +13.51% |
| Sparse local 2,048 values, Sorted Hash | 9.619 → 10.997 | +14.33% | 9.183 → 10.910 | +18.81% |
| Sparse upper-prefix 2,048 values, Sorted Hash | 9.676 → 11.128 | +15.00% | 9.026 → 11.070 | +22.65% |
| Dense/Dense overlap count | 23.462 → 23.749 | +1.22% | 6.031 → 8.557 | +41.88% |

Compactness and execution time therefore improved together in many cases, but
not all. The balanced difference now produces a compact Dense result while
taking longer. The hash selector substantially improves long runs and avoids
unbounded run scouting, but the fallback still has costs. In particular, a
Sorted seventeen-run input requires inspecting the first sixteen runs before
switching to a full member stream. The bound is on runs, not the number of
members inspected inside accepted runs. The measured slowdown remains visible
rather than being removed by selecting only the favorable eight-run case.

The v3 Dense count needs special caution. The two before-run medians are
**6.035 and 5.614 µs**; the two after-run medians are **8.585 and 5.923 µs**.
All seven samples in the first after run fall between **8.556 and 8.620 µs**,
while the second ranges from **5.670 to 8.565 µs**. Pooling gives the reported
8.557 µs. This is an observed regression with substantial run variation, not
evidence of a stable 41.88% slowdown on every execution. No samples were removed
to make the pooled result resemble the faster run.

### Very small skewed intersections

These cases intersect a million-member Sorted input with two Dense or Roaring
members. Times are **nanoseconds**, and include materialized-result allocation
where indicated. Both operand orders remain measured.

| Two-member representation and operation | Baseline before → after (ns) | v3 before → after (ns) |
| --- | ---: | ---: |
| Dense, Sorted first, count | 49.537 → 52.105 | 51.889 → 49.587 |
| Dense, Sorted first, materialize | 81.645 → 101.158 | 108.968 → 95.740 |
| Dense, compressed first, count | 53.087 → 54.638 | 48.847 → 80.672 |
| Dense, compressed first, materialize | 78.773 → 97.774 | 79.944 → 122.201 |
| Roaring, Sorted first, count | 101.041 → 101.956 | 92.203 → 123.268 |
| Roaring, Sorted first, materialize | 162.431 → 184.324 | 157.706 → 180.904 |
| Roaring, compressed first, count | 99.849 → 100.332 | 114.356 → 92.005 |
| Roaring, compressed first, materialize | 162.206 → 183.241 | 158.147 → 176.629 |

Baseline materialization is about **13–24% slower** across these cases. In v3,
the Dense/compressed-first count and materialization increase **65.15%** and
**52.86%**, while other directions improve. These are material relative changes
at tens of nanoseconds, with environmental sensitivity also visible in the
unchanged raw binary-search control (**42.160 → 60.388 ns**, +43.23%, in v3).
The native controls are retained to expose this variation; it should neither
be attributed entirely to Frost nor used to dismiss every regression.
Separate operation-count regression tests establish that the large Sorted
input is not scanned; wall-clock samples do not prove that complexity property.

## CPU profile effects

The final raw Dense AND/popcount control measures **23.558 µs** baseline versus
**5.731 µs** v3. The final native cached-promotion count measures **24.010 µs**
versus **6.027 µs**. These observed profile differences explain why the earlier
review's comparison needed explicit CPU flags. They do not establish which
individual instruction accounts for every difference; the harness is not an
assembly profiler.

Within a profile, Frost and native controls receive the same compiler flags.
That does not disable the dependency's supported runtime CPU selection when
`std` is enabled. The runs are not measurements of the alloc-only embedded
profile or other processors. Compiler-profile differences must not be pooled
into one speedup figure. The Dense count variation described above also makes
its single pooled baseline-to-v3 ratio a poor standalone estimate.

## The first measured implementation is preserved

The [first-pass directory](../../../verification/integer-output/first-pass/README.md)
contains its source snapshot, exact 91-case harness, raw samples, locks and
comparison. It is separate from the final **115-case** run. First-pass timings
are not additional samples of the final implementation.

The first pass already fixed the large output expansions and same-representation
equality, but its Dense/Roaring materialization still took **5,016.369 µs**
baseline and **4,867.981 µs** v3. The native conversion/materialization controls
took **46.759 / 23.547 µs**, with the same prebuilt-byte qualification described
above. Source inspection found that a legal compact native result was being
replanned into Dense through per-member work. The final Auto path can retain a
legal native Roaring result when its estimate is competitive with Sorted;
the final corresponding times are **119.468 / 67.382 µs**.

The first run-only hash protocol also regressed. Baseline alternating Dense
Hash rose **48.48%**, sparse-local Roaring Hash **58.85%**, and sparse-prefix
Sorted Hash **36.73%**. In v3, sparse-local Roaring Hash rose **123.78%**.
Those measurements prompted the content-derived member/run selector and its
bounded scout. The final sparse-local Roaring result is **22.168 µs** baseline
and **22.075 µs** v3, compared with first-pass candidate times of
**48.742 / 49.361 µs**. These are observations from separate experiments, not a
new paired confidence estimate. The remaining member-hash regressions are
reported above.

The measurements support the implemented compact-output and dispatch fixes
under the recorded workloads. They also identify concrete future work:
balanced difference conversion costs, member hashing after the run-selection
boundary, and allocation/dispatch overhead in tiny materialized intersections.
They provide no performance evidence for the unimplemented lazy-view,
persistent-container, flat-arena, or new succinct representations.

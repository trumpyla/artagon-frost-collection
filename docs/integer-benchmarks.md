# Integer-profile benchmark evidence

> Historical evidence for the source revision named below. The current merged
> implementation is documented in [revision-2 integration validation](revision2-integration-validation.md).

> Historical results before the output-policy corrections. See the current
> [output benchmark comparison](integer-output-benchmarks.md) for new measurements.

The measurements led to one targeted implementation change: mixed intersections
now compare scan and lookup work before choosing a probe direction. On the
large-base fixture, Sorted/Dense intersection counts improved by **10.9 / 10.7 times**, and Sorted/Roaring counts by **4.0 / 4.0 times**, across two matched runs.
Other observed timings became slower. The complete before/after data remain
available; this is not a universal speedup or a zero-regression claim.

The automatic representation planner chose dense storage for local windows,
Roaring for run/skew fixtures, and sorted storage for scattered upper prefixes.
It remains a deterministic memory-oriented heuristic with explicit overrides.
Its representation thresholds were not tuned to this machine's timings.

## Reproduction and evidence

Two initial runs and two revised runs executed serially on CPU 0 of an AMD EPYC
9V74 host, using Rust 1.99.1 and Cargo's release defaults with `--features int-set`.
No other project build, test, or benchmark ran during these measured
executions. Inputs are reused and warm. Each batch calibrates to at least four
milliseconds, then records seven samples in execution order. All numeric tables
below use **median nanoseconds per operation, run 1 / run 2**. These observations
are not confidence intervals or isolated hardware-cycle measurements. Execution
order was two initial runs followed by two revised runs; it was not randomized
or counterbalanced between implementations.

```sh
cargo bench --locked --features int-set --bench integer_sets --no-run
taskset -c 0 target/release/deps/integer_sets-65df4a1a9fb4a10c
```

Cargo prints the build-specific executable name. The [execution record](../verification/int-set/bench-execution.json)
contains exact commands, successful exit codes, and executable/source hashes.
The [environment record](../verification/int-set/environment.json) records the
compiler, CPU, flags, dependencies and affinity. The [benchmark source](../benches/integer_sets.rs)
uses the common [timing support](../benches/support/mod.rs).

Both revised runs exited successfully with **213 distinct cases and 1,491 raw
samples each**: 135 same-policy operation cases, 21 BTreeSet/FrozenOrderedSet
controls, 10 native Roaring controls, 36 mixed-pair cases, and 11 construction
cases. Each initial run contains 195 cases and 1,365 samples. Exactly 195 case
names match across revisions; 18 newly added skewed mixed-pair cases have no
initial measurement. They are labeled new rather than used for a before/after
claim. Materialized answers and constructed contents are checked against full
independent BTreeSet sequences before timing; scalar answers are checked too.

- Revised [run 1 CSV](../verification/int-set/bench-after-1.csv) and [representation/budget log](../verification/int-set/bench-after-1.log).
- Revised [run 2 CSV](../verification/int-set/bench-after-2.csv) and [representation/budget log](../verification/int-set/bench-after-2.log).
- [Revised paired summary, including min/max](../verification/int-set/bench-summary.csv) and [all 195 matched cases plus 18 new cases](../verification/int-set/bench-comparison.csv).
- Initial [run 1 CSV](../verification/int-set/mixed-probe-before-1.csv), [run 2 CSV](../verification/int-set/mixed-probe-before-2.csv), and [execution hashes](../verification/int-set/mixed-probe-before-execution.json).
- [Complete initial source files and reproduction instructions](../verification/int-set/baselines/README.md).
- [Revised optimized build log](../verification/int-set/bench-build-after.log).

Dataset generation, operand construction, and reference answers are outside
operation timing. Native Roaring operands are optimized before timing, matching
Frost's construction-time optimization. Materialization includes output
allocation and destruction. Construction includes validation, normalization,
representation construction and destruction; input fixtures are already ascending.
These cases do not measure shuffled or duplicate-heavy ingestion. No allocator
instrumentation measures resident bytes or allocation counts.

## Dataset coverage

The u32 window has 8,192 left and 5,462 right members near one million. The u64
window uses those cardinalities near 10^15, with bases offset by 79 and different
aligned bitmap coordinates. The UserId fixture wraps that u64 dataset. Signed
values cross zero, spanning two upper-32-bit encoded partitions. Each run
fixture contains four 1,024-member consecutive ranges in separate 16-bit
containers. The skew fixture compares 65,536 consecutive members with 64
regularly spaced members. The scattered fixture has 4,096 members in 4,096
distinct upper-32-bit partitions on each side.

Forced dense construction is ineligible for scattered prefixes: its aligned
payload would require about 230 PB and 460 PB for the two inputs, against the
default 8 MiB limit. These rejections are logged and not timed. The forced Roaring
cases remain eligible, exactly meeting the default 4,096-partition cap.

## The measured mixed-direction change

Initially, every mixed intersection probed from the smaller-cardinality operand.
For Sorted/Dense with 8,192 sorted members and 5,462 dense members, that meant
thousands of binary searches. Scanning the slightly larger sorted operand with
constant-time dense membership performs much less work on this fixture.

The revised sorted/compressed direction estimate selects the sorted scan when
`n <= m * bit_width(n) + scan_words`, using u128 arithmetic. Here n is the sorted
cardinality, m the compressed cardinality, and scan_words is the dense bitmap's
word count or zero for Roaring. The dense term accounts for empty-word traversal.
Extreme cardinality skew can still select compressed-side probing. Counts,
disjointness, and materialized intersections share this decision. No new logical
key decoding or materialization is introduced into count queries.

| Large-base operation | Before | After | Before / after |
| --- | --- | --- | --- |
| Sorted/Dense count | 63,848.7 / 62,157.7 | 5,875.8 / 5,795.5 | 10.87× / 10.73× |
| Sorted/Dense materialization | 65,387.5 / 63,911.4 | 7,417.9 / 7,175.9 | 8.81× / 8.91× |
| Sorted/Roaring count | 95,411.6 / 92,942.9 | 23,826.8 / 23,086.8 | 4.00× / 4.03× |
| Sorted/Roaring materialization | 104,361.2 / 102,224.8 | 25,791.1 / 24,759.3 | 4.05× / 4.13× |

This is a work estimate, not a model of CPU cycles. Roaring's partition/container
lookup costs, memory locality, and branch behavior are not fully modeled.
Dense/Roaring pairs still use the smaller member count. The measured gain applies
to the named cases; disjointness uses the revised rule but has no separate timing
row in this benchmark.

## Revised intersection counts

| Dataset | Auto representation | Auto | Forced sorted | FrozenOrderedSet | Native Roaring | BTreeSet |
| --- | --- | --- | --- | --- | --- | --- |
| u32 local window | Dense | 170.7 / 154.5 | 8,505.7 / 8,080.0 | 7,649.9 / 8,407.9 | 783.4 / 762.9 | 22,661.6 / 22,507.7 |
| u32 skew (65,536:64) | Roaring | 40.0 / 49.4 | 2,730.7 / 2,703.2 | 2,686.9 / 2,760.2 | 39.5 / 35.4 | 1,156.1 / 1,160.2 |
| u32 run clusters | Roaring | 23.5 / 24.0 | 4,610.1 / 4,643.8 | 4,475.0 / 4,694.7 | 21.2 / 22.0 | 14,466.7 / 13,158.2 |
| u64 large-base window | Dense | 201.0 / 200.6 | 9,411.7 / 9,997.4 | 10,359.0 / 10,071.2 | 955.1 / 772.7 | 20,507.8 / 19,691.0 |
| u64 scattered prefixes | Sorted | 3,941.0 / 3,514.6 | 3,496.8 / 3,601.7 | 3,522.6 / 3,550.2 | 110,375.7 / 108,690.6 | 10,472.5 / 10,301.7 |
| i64 across zero | Dense | 203.3 / 200.3 | 8,670.7 / 9,119.7 | 8,334.5 / 8,776.1 | — | 22,855.0 / 22,837.4 |
| UserId large-base window | Dense | 198.5 / 194.5 | 10,816.7 / 9,911.7 | 9,079.8 / 7,701.8 | — | 23,352.4 / 22,605.2 |

Large absolute IDs still benefit from a small local window. Scattered prefixes
create many tiny child bitmaps: here the automatic sorted choice counts in
3.9 / 3.5 µs while native Roaring takes 110.4 / 108.7 µs. It is comparable to the
ordinary FrozenOrderedSet control in that fixture, and slightly slower in the
first run. This prevents a claim of superiority to every existing Frost family.

Native controls also expose wrapper overhead. Run-cluster native Roaring count
costs 21.2 / 22.0 ns, compared with 23.5 / 24.0 ns for the automatic profile.
Bounds checks, representation dispatch, and a typed public API do not make
every operation faster than directly using the selected native backend.

## Revised intersection materialization

| Dataset | Auto | Forced sorted | Forced dense | Forced Roaring | Native Roaring | BTreeSet |
| --- | --- | --- | --- | --- | --- | --- |
| u32 local window | 752.1 / 678.8 | 15,861.3 / 14,883.4 | 718.9 / 683.5 | 3,074.5 / 2,962.3 | 2,955.9 / 2,909.5 | 78,797.4 / 79,027.1 |
| u32 skew (65,536:64) | 116.6 / 183.0 | 3,119.7 / 3,011.9 | 2,632.9 / 2,544.8 | 127.3 / 119.2 | 105.1 / 103.4 | 2,686.2 / 2,695.6 |
| u32 run clusters | 236.3 / 245.4 | 10,477.7 / 10,437.9 | 7,723.1 / 7,926.9 | 234.2 / 260.5 | 231.1 / 265.3 | 56,271.3 / 56,634.5 |
| u64 large-base window | 755.6 / 739.0 | 16,577.0 / 15,519.4 | 747.6 / 719.6 | 3,176.0 / 3,113.1 | 5,395.4 / 3,214.1 | 91,087.6 / 80,860.2 |
| u64 scattered prefixes | 8,886.6 / 9,771.7 | 9,272.7 / 9,248.4 | — | 605,017.2 / 580,704.5 | 584,614.1 / 588,242.1 | 53,381.4 / 52,213.1 |
| i64 across zero | 763.7 / 732.2 | 16,879.5 / 18,714.5 | 722.3 / 703.3 | 4,093.9 / 4,247.5 | — | 58,941.8 / 60,841.2 |
| UserId large-base window | 715.8 / 724.9 | 16,393.8 / 16,155.7 | 729.8 / 756.4 | 3,080.0 / 3,132.5 | — | 58,912.5 / 58,365.3 |

These compare complete results, including allocation and drop. Dense pair
kernels retain a bounded local bitmap; mixed and sorted paths materialize sorted
words. Native Roaring kernels retain Roaring storage. Output representation can
therefore affect both execution cost and later operations.

## Other materialized operations

All four operations are validated and timed for every eligible policy. Revised
automatic-profile union, difference, and XOR medians are:

| Dataset | Union | Difference | Symmetric difference |
| --- | --- | --- | --- |
| u32 local window | 720.8 / 770.1 | 720.1 / 706.4 | 732.1 / 719.5 |
| u32 skew (65,536:64) | 224.2 / 220.7 | 1,173.5 / 1,115.7 | 1,022.3 / 985.7 |
| u32 run clusters | 140.4 / 136.3 | 144.1 / 142.9 | 377.2 / 375.3 |
| u64 large-base window | 795.3 / 720.4 | 818.1 / 745.0 | 758.7 / 717.1 |
| u64 scattered prefixes | 7,503.1 / 6,627.8 | 7,945.4 / 7,969.4 | 7,481.0 / 7,065.1 |
| i64 across zero | 735.5 / 727.1 | 782.7 / 722.8 | 740.7 / 713.6 |
| UserId large-base window | 717.8 / 722.0 | 793.1 / 772.9 | 729.7 / 721.5 |

Every explicit policy's results remain in the raw CSVs. Result cardinality and
representation differ by operation; a scalar intersection improvement does not
establish the same improvement for union, difference, or XOR.

## Revised mixed-representation matrices

The large-base u64 matrices keep the same left/right logical datasets. Rows name
the left representation (8,192 members), columns the right (5,462 members).
Changing the pair assigns different storage to those datasets; it is not merely
swapping the same physical operands. All nine ordered pairs are validated.

### Large-base intersection counts

| Left / right | Sorted | Dense | Roaring |
| --- | --- | --- | --- |
| Sorted | 10,199.7 / 10,174.2 | 5,875.8 / 5,795.5 | 23,826.8 / 23,086.8 |
| Dense | 4,059.6 / 3,925.4 | 214.3 / 168.7 | 37,220.7 / 37,460.4 |
| Roaring | 15,934.5 / 15,728.8 | 17,286.1 / 17,510.4 | 797.9 / 703.3 |

### Large-base materialized intersections

| Left / right | Sorted | Dense | Roaring |
| --- | --- | --- | --- |
| Sorted | 16,438.6 / 14,830.9 | 7,417.9 / 7,175.9 | 25,791.1 / 24,759.3 |
| Dense | 5,532.7 / 5,231.7 | 749.3 / 1,235.5 | 42,341.7 / 41,732.6 |
| Roaring | 18,643.2 / 18,291.0 | 20,287.5 / 19,873.9 | 3,314.8 / 3,111.2 |

Mixed storage remains slower than same-backend bitmap operations on this
fixture. An application repeating many operations may explicitly replan with
`with_policy`, accepting its rebuild cost. The API does not silently rebuild
operands or rerun the planner after every native result.

### Newly measured skewed intersection counts

These rows use 65,536 left members and 64 right members. They retain extreme
skew as a separate control for the direction estimate. They are new measurements,
not an empirical before/after speed claim.

| Left / right | Sorted | Dense | Roaring |
| --- | --- | --- | --- |
| Sorted | 2,770.8 / 2,820.4 | 2,299.6 / 2,299.3 | 1,621.4 / 1,705.4 |
| Dense | 89.6 / 52.1 | 573.0 / 785.0 | 246.3 / 242.6 |
| Roaring | 185.4 / 187.7 | 628.8 / 629.7 | 39.8 / 43.7 |

For the large sorted side, the estimate retains probing from the 64-member
compressed operand instead of scanning all 65,536 sorted members. The CSVs also
contain materialization for all nine skewed pairs.

## Construction cost

| Dataset | Auto | Sorted | Dense | Roaring |
| --- | --- | --- | --- | --- |
| u32 local window | 33,420.3 / 32,914.2 | 16,146.4 / 15,605.6 | 24,095.8 / 24,652.9 | 99,836.5 / 101,240.8 |
| u64 large-base window | 40,377.8 / 39,964.2 | 16,804.5 / 16,873.7 | 25,824.9 / 26,315.5 | 136,245.0 / 121,774.8 |
| u64 scattered prefixes | 22,304.4 / 22,168.6 | 8,814.5 / 8,899.5 | — | 612,324.5 / 598,520.0 |

Automatic construction is slower than explicit sorted construction on these
already ordered inputs. For scattered prefixes, auto costs 22.3 / 22.2 µs while
explicit sorted costs 8.8 / 8.9 µs, despite the same final representation. Auto
scans container/run/prefix statistics; explicit sorted and dense plans skip the
Roaring statistics scan. Known distributions or construction-heavy workloads
can use an explicit policy. `with_policy` also rebuilds; it is not a free cast.

## Slower observations and uncertainty

The complete comparison retains slower cases, including:

| Case | Before | After |
| --- | --- | --- |
| Large-base sorted/sorted count | 8,485.2 / 8,325.6 | 10,199.7 / 10,174.2 |
| Scattered auto materialization | 7,661.0 / 7,676.9 | 8,886.6 / 9,771.7 |
| Large-base Roaring/Sorted materialization | 16,351.5 / 16,513.8 | 18,643.2 / 18,291.0 |
| Large-base dense/dense materialization | 679.6 / 724.5 | 749.3 / 1,235.5 |

Timing variation is also visible when identical representations are selected.
In the initial run, automatic u32-window materialization measured 1,157.0 / 700.2
ns, while explicit dense measured 661.1 / 1,152.1 ns. The algorithm did not change
between those two executions. CPU affinity and repeated samples do not eliminate
host variation, code placement, or allocation effects. Adding the skew cases
also changes the later cases' position in the complete run. These experiments
do not isolate the cause of every slower observation or establish statistical
significance. Preserve them when deciding whether a particular workload benefits.

The targeted direction change removes a clear category of repeated binary
search work and shows a large repeatable gain on its motivating fixtures. It
does not justify claiming that the entire revised crate is uniformly faster.

## Scope of the result

Keep the bounded memory-oriented planner, explicit overrides, and fixed
sorted/dense/Roaring profile. These measurements support the implemented probe
direction refinement without requiring a C backend or a new unsafe SIMD layer.
The codec tests separately establish that bulk operations stay on encoded words;
the benchmark alone cannot prove that invariant.

This experiment does not establish allocation counts, resident memory,
cache-cold or NUMA behavior, arbitrary distributions, shuffled ingestion, other
CPUs, 32-bit throughput, concurrent throughput, or a universal fastest backend.
Cross-target, concurrency, bounded-domain, and production-API tests are distinct
evidence in the [integer validation report](integer-validation.md).

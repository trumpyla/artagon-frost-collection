# Benchmarking Frost

Benchmarks are optimized executables with calibrated batches and retained raw
samples. They assert observable answers before timing. They are evidence for
the stated inputs on the measured host, not pass/fail correctness gates.

```sh
cargo bench --locked --all-features --bench ordered_sets
cargo bench --locked --all-features --bench adaptive_maps
cargo bench --locked --all-features --bench snapshots
cargo bench --locked --all-features --bench set_operations
cargo bench --locked --all-features --bench performance_review
cargo bench --locked --features int-set --bench integer_sets
cargo bench --locked --features int-set,std --bench integer_output
```

`ordered_sets` covers ratio boundaries, balanced input, prefix/tail clusters,
distributed members, and mixed distributions through count and iterator paths.
`adaptive_maps` distinguishes construction from drain and extension work and
shows explicit policy thresholds. `snapshots` compares owned and guarded reads
with persistent synchronized workers. `set_operations` uses native set controls;
`performance_review` preserves the earlier 140-case corpus.
`integer_sets` compares fixed integer representations, all four materialized
operations, mixed intersections, and construction on identical logical inputs.
It includes dense local windows, long runs, scattered upper prefixes, signed
values, and application IDs. Its current construction inputs are sorted;
those cases do not measure shuffled-ingest sorting throughput.

`integer_output` measures mixed output materialization, distant dense windows,
native equality, representation-independent Hash on several data shapes,
sorted Roaring builders, byte promotion and small dense membership. The
[comparison report](../integer-output-benchmarks.md) describes the bundled
baseline archive and identical-harness reproduction script. It compares baseline
x86-64 and x86-64-v3 separately; changing the target CPU is not a source-level
speedup and must not be hidden in the aggregate.

## Measurement contract

Record Rust, target, OS, CPU, affinity, Cargo profile, RUSTFLAGS, and dependency
lockfile. Use the same harness and flags for both implementations. For a
comparison, run before/after/after/before serially, without concurrent builds or
other benchmarks. On Linux, `taskset -c 0` can pin single-threaded comparisons;
do not pin all contending-reader workers to one CPU when measuring multicore
contention. Include thread counts and affinity for that workload separately.

Validate cardinalities and checksums before timing. Consume results through
`black_box`. State whether setup, allocation, destruction, and thread startup
are included. For mutation benchmarks, restoring input is setup; destroying a
returned result is part of the cost only when the stated operation includes it.
Thread startup and join overhead must not masquerade as snapshot read cost.

Use the standard collection's native operation as the primary control. A
naive scan may explain an algorithmic difference but cannot stand in for native
intersection. Count actual lookups rather than labeling every row by a fixed
number when a short collection supplies fewer keys. Assert that miss probes
really miss. Do not infer allocated bytes from capacity without accounting for
the backend's buckets, control bytes, alignment, and allocator.

Keep individual samples, median/min/max, baseline regressions, and input shapes.
Noise, inlining and code layout affect nanosecond cases; repeated measurements
help assess them but do not establish cross-platform superiority. Do not add
wall-clock speed assertions to CI. Comparison-count bounds belong in regression
tests when they express the intended algorithmic complexity.

## Isolation from coverage, sanitizers, and policy thresholds

- Frost benchmarks use custom `harness = false` executables backed by
  [`benches/support/mod.rs`](../../benches/support/mod.rs) (not Criterion).
  Setting `FROST_BENCH_LIST=1` lists registered benchmark cases without timing
  loops so `make catalog-inventory` can verify every benchmark executable against
  [`verification/metadata/bench-inventory.json`](../../verification/metadata/bench-inventory.json)
  via [`verification/catalog`](../../verification/catalog/Cargo.toml).
- Never collect timing samples from binaries built with `-C instrument-coverage`
  (`cargo-llvm-cov`), sanitizer instrumentation (`cargo-fuzz` / AddressSanitizer),
  Miri, or Loom, and never run benchmarks concurrently with `make check`,
  `make coverage`, `make mutants`, or `make fuzz-smoke`.
- Keep answer validation outside the timed loop using `measure` and
  `measure_with_setup` in [`benches/support/mod.rs`](../../benches/support/mod.rs).
  State explicitly whether input construction, clone/thaw staging, result
  destruction, and worker-thread lifecycle are inside or outside the timed
  region.
- Do not change adaptive crossover defaults (`8` entries for `AdaptiveHashMap`,
  `32` entries for `AdaptiveOrderedMap`) or `IntSetPolicy` planner constants
  (`128`-byte Dense preference, `8 MiB` dense budget, `4,096` Roaring partitions)
  from a single host's timings alone. Threshold changes require cross-architecture
  evidence, counterworkloads (including oscillation and skewed distributions),
  and updates to the [verification guide](../verification.md),
  [testing standards](testing.md), and the
  [verification strategy report](../research/reports/rust-verification-strategy.md).

Historical results and their environment are under `verification/history/` and
[the previous review](../reviews/performance-review.md). The prior production
candidate's results are under `verification/results/`; integer-profile results
are under `verification/int-set/`. Historical output-policy comparisons are under
`verification/integer-output/`, including the explicitly preserved first pass.
[Current validation](../revision2-integration-validation.md)
states which runs were actually completed for this revision.

## Reviewed-main versus revision-2 integration

The current same-executable comparison lives in
`verification/revision2-integration/ab-harness/`. Its checked-in baseline fixture
contains the actual reviewed main source; only Cargo package/library names were
changed to let both implementations coexist. Both use the same integer/std
features and dependency versions. Run it serially after verification workers
finish, retain stdout and stderr, and report medians and regressions rather than
selecting the fastest round. See the current validation report for the exact
executed command, source digests, host, and results.

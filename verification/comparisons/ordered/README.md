# Ordered strategy comparison inputs

This standalone audit harness preserves the exact two internal implementations
used for the integration decision. `prior.rs` is the previous optimized kernel;
`incoming.rs` is the review bundle's proposed replacement. These are historical
inputs, not modules of the production crate. Do not edit them to follow current
code changes. Cargo does not compile this directory, and `verification/**` is
excluded from the package.

From the repository root, on the recorded toolchain:

```sh
rustc --edition=2024 -C opt-level=3 -C codegen-units=1 -C lto=thin \
  verification/comparisons/ordered/main.rs -o /tmp/frost-ordered-comparison
taskset -c 0 /tmp/frost-ordered-comparison > /tmp/ordered-comparison.csv
```

Use an available isolated CPU instead of `0` where needed. Run without competing
builds or tests. Both implementations are compiled into the same executable and
each workload runs in both orders. All answers are checked against `BTreeSet`.
The sample columns record warm wall-clock nanoseconds per operation in execution
order. The local host is noisy; minima/maxima and the two trial medians are
retained so a single ratio does not hide variability.

Recorded [results](../../results/ordered-strategy-comparison.csv) and
[environment/source hashes](../../results/ordered-strategy-environment.json)
support the [integration assessment](../../../docs/reviews/incoming-assessment.md).
The current public API workload lives in `benches/ordered_sets.rs`.

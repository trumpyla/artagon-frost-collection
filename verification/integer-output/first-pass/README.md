# First measured implementation: retained evidence

This directory preserves the first promotion/limits/equality/hash implementation
measured during this review. It is historical evidence, not the final candidate.
The measurements exposed additional costs that motivated the next implementation
pass: automatic replanning of legal Roaring results into Dense, and run-based
hashing overhead on sets with many short runs.

Eight complete executions ran serially on CPU 0: before/after/after/before under
`-C target-cpu=x86-64`, then the same order under `-C target-cpu=x86-64-v3`.
Each execution contains 91 cases with seven samples per case. Both runs are
retained, including native controls and slower observations. `comparison.csv`
records each run's median and the median pooled across its fourteen samples;
the pooled value is descriptive, not a statistical confidence interval.

Selected baseline-CPU observations, from the pooled medians:

| Workload | Before | First implementation |
|---|---:|---:|
| Million-member Dense union Roaring singleton | 1.019 ms | 4.586 µs |
| 32 mixed updates, including initial clone | 18.38 ms | 149.7 µs |
| Distant dense-run union | 33.62 ms | 336.5 µs |
| Equal million-member Dense run | 4.281 ms | 2.263 µs |
| Dense/Roaring overlap count | 6.532 ms | 87.07 µs |
| Dense/Roaring overlap materialization | 7.504 ms | 5.016 ms |
| Native conversion plus overlap materialization control | 45.42 µs | 46.76 µs |
| Alternating million-member Dense logical Hash | 4.707 ms | 6.989 ms |
| Sparse local Roaring logical Hash | 30.68 µs | 48.74 µs |
| Balanced Sorted/Roaring difference | 7.730 ms | 8.813 ms |

The native materialization control showed that the remaining roughly 5 ms
Frost materialization cost was not inherent to Dense-to-Roaring conversion.
The Hash results showed that maximal-run encoding should be bounded or selected
by data shape; it was highly effective on long runs and slower on sparse or
alternating values. Those observations are retained even after subsequent fixes.

`measured-candidate-source.zip` contains all library Rust sources, the Cargo
manifest, lockfile, toolchain file, and benchmark/support files needed to identify the
measured candidate. Its source hashes were checked against `bench-metadata.json`
when saved, with no differences. For a full project tree, overlay those files
onto the complete baseline archive in the parent directory. `integer_output.rs`
and `support.rs` also preserve the exact measured harness separately. The
comparator, before/after locks, build logs, verification logs, and raw timing
files are included so the provenance is inspectable.

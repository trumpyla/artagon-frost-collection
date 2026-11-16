---
name: frost-perf-triage
description: Finds the cause of a Frost slowdown with the repository's benchmark harness and a profiler, and reports it with evidence. Use when a benchmark got slower, a change may cost speed, or someone asks why a code path is slow.
---

Find what costs the extra time, prove it with measurements, and report it.
The measurement contract in `docs/maintainers/benchmarks.md` governs every step.

1. Name the workload. Pick the benchmark and case that show the slowdown:
   `FROST_BENCH_LIST=1 cargo bench --locked --all-features --bench <name>`
   lists the cases without timing. The commands per benchmark are in
   `docs/maintainers/benchmarks.md`.
2. Reproduce it. Run the benchmark serially, before/after/after/before, with the
   same toolchain, profile and flags. Run nothing else at the same time. Record
   Rust, target, OS, CPU, affinity, profile, RUSTFLAGS and the lockfile. For the
   integer package, `make integer-ab` runs the checked-in A/B harness.
3. Decide if it is real. Compare medians and the raw samples of all four runs.
   A difference smaller than the spread between runs of one version is noise:
   report it as not shown, and stop.
4. Locate the cost. Build the benchmark without running it:
   `cargo bench --locked --all-features --bench <name> --no-run`
   (the output names the executable). Then profile that executable:
   - Linux or macOS: `samply record --save-only -o target/<name>.json.gz -- <executable>`.
   - Linux only, for an exact count: `valgrind --tool=callgrind <executable>`,
     then `callgrind_annotate --auto=no --threshold=95 callgrind.out.<pid>`.
     Calibrated batches run far slower under Valgrind; use it for short benchmarks.
   Profile the old and the new build the same way. Compare the top functions.
5. Map the function to the change with `git diff` and its callers. Write one
   hypothesis in one sentence.
6. Change one thing. Repeat step 2. Keep every run, slower ones included.
7. Report: workload and case, environment, medians and spread before and after,
   the profile lines that show the cost, the cause, and what you did not check.
   Report a measured workload, not a general speedup.

Do not:

- add a timing assertion or a CI speed gate;
- change an adaptive crossover or `IntSetPolicy` constant on one host's timings;
- time a build that has coverage, sanitizer, Miri or Loom instrumentation;
- run a benchmark at the same time as `make check`, `make coverage`, `make mutants`
  or `make fuzz-smoke`.

---
applyTo: "crates/*/benches/**/*.rs,crates/frost-collections-test-support/src/bench.rs"
---

- Benchmarks follow `docs/maintainers/benchmarks.md`. Flag a benchmark that
  times before it validates its answer, or that has no native standard-library
  control.
- Flag a benchmark that does not say whether setup, allocation, destruction or
  thread startup is timed.
- A changed case list needs an updated benchmark inventory
  (`make catalog-inventory`).
- Flag any timing assertion in a test or any speed gate in CI.

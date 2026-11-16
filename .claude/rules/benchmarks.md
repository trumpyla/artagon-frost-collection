---
paths:
  - "crates/*/benches/**/*.rs"
  - "crates/frost-collections-test-support/src/bench.rs"
---

Benchmarks follow `docs/maintainers/benchmarks.md`. Validate the answer before
timing, use the standard collection's native operation as the control, and say
whether setup and destruction are timed. A changed case list needs
`make catalog-inventory`. Add no CI speed gate. For a slowdown, use the
`frost-perf-triage` skill.

# Testing and benchmark work for agents

This file routes test, benchmark and performance tasks to the rules that
govern them. Read it when a task adds or changes a test, a benchmark, or the
speed of a code path. It adds no rule of its own except under
[Agent limits](#agent-limits); the linked files are the authority.

## Where the rules are

| Topic | Read |
| --- | --- |
| Target layout, seeds, coverage, mutation, gate tiers | [testing.md](testing.md) |
| Benchmark measurement contract and isolation | [benchmarks.md](benchmarks.md) |
| Test and benchmark metadata, obligation mapping | [CRATES.md](../../CRATES.md) |
| Catalog and benchmark inventory commands | [verification/catalog/README.md](../../verification/catalog/README.md) |
| Semantic and safety contracts a test protects | [implementation-contracts.md](implementation-contracts.md) |
| Which gates to run before review | the `make-gates` skill |

## Tasks

| Task | Do | Done when |
| --- | --- | --- |
| Add a regression or property test | Put it in `crates/<owning package>/tests/`. Compare with the standard library or enumerate a small domain. Commit any `.proptest-regressions` file. | `make check` passes, and the new cases are in the catalog as `unreviewed` |
| Cover a collection type for set operations | Use the `frost-test-change` skill, section B | Every set type runs the same checks |
| Add or change a benchmark | Use the `frost-test-change` skill, section C | `make catalog-inventory` passes |
| Investigate a slowdown | Use the `frost-perf-triage` skill | A measured cause, or an open question with its evidence |
| Prepare for review | Use the `make-gates` skill | Every gate run is reported with its result |

## Choosing a technique

| If the code under test… | Use | Example in this repository |
| --- | --- | --- |
| must agree with a standard collection | A differential property test with proptest | `crates/frost-collections-core/tests/ordered_set_ops.rs` |
| has a small finite input domain | Exhaustive enumeration | `crates/frost-collections-core/tests/exhaustive_collections.rs` |
| keeps state across operations | proptest-state-machine against a model | `crates/frost-collections-core/tests/formal_models/stateful.rs` |
| publishes snapshots across threads | Deterministic barriers; the Loom lane only on request | `crates/frost-collections-concurrent/tests/publication_model.rs` |
| touches `buf.rs` | The Miri lanes, only on request | `.claude/rules/unsafe-boundary.md` |
| decodes integer codes | Seed replay with `make fuzz-check` | `crates/frost-collections-integer/fuzz` |

## Agent limits

- Add no test or benchmark dependency without a maintainer decision. rstest,
  rstest_reuse, divan, gungraun, criterion and linkme were proposed outside this
  repository; none is adopted. `proptest`, `proptest-state-machine` and `exhaust`
  are already dev-dependencies of `frost-collections-core`.
- Add no timing assertion, sleep or speed gate to a test or to CI.
- Change no adaptive crossover or `IntSetPolicy` constant on one host's timings.
- Run no Tier 2 or Tier 3 lane (`make miri`, `make kani`, `make mutants`,
  `make fuzz-smoke`, `make loom`) unless the maintainer asks.
- Report each command you ran with its result. Report a gate you did not run as
  open, never as passed.

## Platforms

CI runs on Linux only. Maintainers run locally on Linux and macOS.

| Tool | Linux | macOS |
| --- | --- | --- |
| `samply` sampling profiler | Yes | Yes (also records off-CPU waits) |
| `valgrind --tool=callgrind`, `callgrind_annotate` | Yes | No (Valgrind does not support current macOS) |
| `perf` | Yes | No |
| `taskset -c 0` pinning | Yes | No |

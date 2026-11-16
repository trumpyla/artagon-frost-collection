---
name: frost-test-change
description: Adds a Frost test, extends set-operation checks to another collection type, or adds a benchmark, with the catalog and inventory records the change needs. Use when a change adds or edits files under crates/*/tests or crates/*/benches.
---

Read `docs/maintainers/agent-testing.md` first. It lists the rule files.

## A. Add a regression or property test

1. Put the test in `crates/<owning package>/tests/`.
2. Check against the standard library, or enumerate a small domain. Use only
   existing dev-dependencies.
3. No sleep and no timing assertion. Use barriers for threads.
4. Commit any new `.proptest-regressions` file.
5. Add the catalog cases:
   `cargo run --locked --manifest-path verification/catalog/Cargo.toml -- scaffold --root .`
   New cases are `unreviewed`. Do not mark one `curated`; a maintainer reviews it.
6. Map each changed public obligation to an asserting test, as `CRATES.md` requires.

## B. Run the same set-operation checks on every set type

1. Start from `crates/frost-collections-core/tests/ordered_set_ops.rs`. Its
   `check_against_standard` compares each operation with `std::collections::BTreeSet`.
2. Write the check once, generic over the capability the types share (the
   traits in `crates/frost-collections-core/src/read.rs` and `write.rs`, or a
   small test-side adapter trait for inherent methods). Sort the output of
   unordered types in the adapter.
3. Assert agreement with the standard set and the laws: union and intersection
   commute, |A ∪ B| + |A ∩ B| = |A| + |B|, subset and disjoint agree with the
   intersection, and union and intersection keep the left representative.
4. Run it for each type with one line per type in a `macro_rules!` type matrix.
5. Feed it three ways: named edge cases, a proptest generator with a small value
   range (for example `-50..50`), and every subset pair of a small universe.
6. Prove the suite can fail: break one type on purpose and confirm that only
   that type's tests fail. Do not commit the break.

## C. Add or change a benchmark

1. Use `measure` or `measure_with_setup` from
   `crates/frost-collections-test-support/src/bench.rs`. Validate the answer
   before timing. Consume results through `black_box`.
2. State whether setup, allocation, destruction and thread startup are inside
   the timed region.
3. Use the standard collection's native operation as the control.
4. Update the inventory: capture `FROST_BENCH_LIST=1` output to a file, then run
   `cargo run --locked --manifest-path verification/catalog/Cargo.toml -- inventory-scaffold <bench target path> <file> --root .`
5. Run `make catalog-inventory`.

## Done when

The `make-gates` skill reports `make check` passed, and your report lists every
command you ran and every gate you did not run.

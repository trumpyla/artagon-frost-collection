# Isolated fuzz harnesses (`frost-fuzz`)

`verification/fuzz` is an isolated Cargo workspace (`[workspace]`) excluded from
the published `frost-collections` package (`exclude = ["verification/**"]` in
[`Cargo.toml`](../../Cargo.toml)). It keeps `libfuzzer-sys` and fuzzing
binaries out of the library dependency graph while exercising the real public
APIs against independent reference oracles.

## Harnesses and oracles

| Target | Production APIs exercised | Independent oracle and invariants |
| --- | --- | --- |
| `ordered_set_algebra` | `MutableOrderedSet`, `FrozenOrderedSet`, `MutableOrderedMap`, `FrozenOrderedMap`, borrowed set algebra (`union`, `intersection`, `difference`, `symmetric_difference`), scalar counts (`*_len`), predicates (`is_subset`, `is_superset`, `is_disjoint`), and partial iterator consumption (`next`, `size_hint`, `count`, `fold`) | `std::collections::BTreeMap<i16, u8>` and `BTreeSet<i16>` tracking explicit representative tags (`RepKey { id, representative }`): first representative wins on construction, last map value wins without replacing the stored key, left representative wins in `union` and `intersection`, and `is_subset`/`is_superset`/`is_disjoint` match `BTreeSet`. |
| `integer_set_algebra` | `FrozenIntSet<i32>`, `TryFreezeWith<IntSetPolicy>`, all five policy modes (`Auto` plus four explicit representation candidates: `Sorted`, `Dense`, `Roaring`, `Chunked`), bounded `IntSetLimits`, materialized algebra (`*_with_policy`), scalar counts (`*_len`), predicates (`is_subset`, `is_superset`, `is_disjoint`), iterator `fold`, `Eq`, and `Hash` | `std::collections::BTreeSet<i32>` membership and predicate oracle plus an exact `Hasher` event-stream recorder verifying that `FrozenIntSet` equality and `Hash` event sequences remain identical across representations. |

## Execution modes

1. **Deterministic corpus and boundary replay (`make fuzz-check`, stable Rust 1.99.1)**:
   ```sh
   cargo test --manifest-path verification/fuzz/Cargo.toml --target-dir target/fuzz --lib --locked
   ```
   Verifies that the checked-in seed files under `verification/fuzz/corpus/`
   decode to non-empty overlapping sets (with duplicate keys for ordered sets),
   replays direct typed edge cases and all `5 x 5 x 5 = 125` integer policy-mode
   triples under `run_integer_case_strict` (asserting `productive_combinations == 125`
   and exact `IntSetRepr` for forced policies with zero `Sorted` construction
   fallback or output-policy skipping), tests ineligible `IntSetLimits`
   rejection (`integer_set_ineligible_policy_limits_are_rejected_and_preserve_input`),
   and runs bounded synthetic sweeps without requiring a nightly compiler or
   sanitizer runtime.

2. **Coverage-guided libFuzzer smoke (`make fuzz-smoke`, diagnostic nightly)**:
   ```sh
   cargo +nightly-2026-11-08 fuzz run --fuzz-dir verification/fuzz \
     --target-dir target/fuzz ordered_set_algebra \
     target/fuzz-corpus/ordered_set_algebra \
     verification/fuzz/corpus/ordered_set_algebra -- -runs=64 -max_total_time=10
   ```
   Requires `cargo-fuzz` (`0.13.2`), `arbitrary` (`1.5.0` in
   [`Cargo.lock`](Cargo.lock)), and the diagnostic nightly toolchain
   (`nightly-2026-11-08`). Passing a bounded smoke run checks harness linkage
   and corpus replay; it is not an exhaustive proof.

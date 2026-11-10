# Mutation testing and survivor triage

Mutation testing evaluates whether Frost's test suite detects small semantic
faults injected into `src/**/*.rs` (such as replacing `<` with `<=`, inverting
boolean conditions, or returning early defaults). High line coverage without
mutation testing can still leave weak assertions.

Configuration lives in [`.config/mutants.toml`](../../.config/mutants.toml) and
is invoked through `make mutants`.

## Denominator and mutation score definition

For a mutated scope `S`, `cargo-mutants` classifies each generated mutant into:

- **Killed (`caught`)**: at least one test failed when the mutation was applied.
- **Survived (`missed`)**: all selected tests passed despite the mutation.
- **Timeout (`timeout`)**: a test exceeded `max(minimum_test_timeout, timeout_multiplier * baseline)` (treated as detected infinite-loop/progress failure, but inspected separately so slow tests are not confused with caught hangs).
- **Unviable (`unviable`)**: the mutant failed to compile (excluded from the denominator).

$$\text{Mutation Score}(S) = \frac{\text{Killed}}{\text{Killed} + \text{Survived} + \text{Timeout}}$$

## Priority tiers and target thresholds

| Tier | Modules | Target score | Rationale |
| --- | --- | --- | --- |
| Tier 1: Critical invariants & unsafe ownership | `src/adaptive/seq/buf.rs`, `src/concurrent/publication.rs`, `src/family/ordered/set_ops.rs`, `src/integer/{key,policy,set,storage,dense,chunked}.rs`, `src/backends/bitmap.rs` | `>= 90%` (0 unexplained survivors in `buf.rs` and `publication.rs`) | Off-by-one, padding-mask, representative-choice, or pointer-identity mutations here directly violate safety or core algebraic laws. |
| Tier 2: Mutable/frozen families & adaptive policies | `src/family/**`, `src/adaptive/**`, `src/frozen.rs`, `src/plan.rs`, `src/concurrent/snapshot.rs` | `>= 85%` | Entry APIs, duplicate policies, promotion thresholds, and freeze/thaw round-trips. |
| Tier 3: Native backend & framework forwarding | `src/backends/**` (excluding `bitmap.rs`), `src/adapters/**`, `src/iter.rs`, `src/read.rs`, `src/write.rs` | `>= 75%` | Thin delegation layers where capacity hints or `Debug`/`Display` formatting produce benign survivors. |

## Survivor triage taxonomy

Every surviving mutant in a reviewed run must be recorded in
[`survivor-triage.json`](survivor-triage.json) under one of five dispositions:

1. `test-gap`: The mutant changes observable behavior promised by the contract.
   **Action**: Add a deterministic differential, boundary, or representative test
   that kills the mutant; never mark a real contract violation as acceptable.
2. `equivalent-mutant`: The mutated code is semantically identical to the
   original on all reachable inputs (for example, changing `<` to `<=` at a
   threshold boundary where both branches compute the same value).
3. `performance-fast-path`: The mutant disables an optimization shortcut (such
   as an `Arc::ptr_eq` self-operation fast path or a capacity pre-reservation
   hint) while the fallback path computes the exact same logical result.
   **Action**: Where observable (such as allocation identity sharing in
   `FrozenIntSet` or comparison counts in `tests/ordered_performance.rs`), add
   an identity or operation-count assertion; if purely internal (`Vec::with_capacity`
   hint), record the justification.
4. `defensive-overflow-guard`: Checked arithmetic or invariant fallback that is
   unreachable on 64-bit hosts without a multi-exabyte allocation, covered
   instead by 32-bit Miri (`--target i686-unknown-linux-gnu`) or Kani harnesses.
5. `diagnostic-only`: `fmt::Debug` or `fmt::Display` formatting strings not
   part of a serialized wire contract.

## Current host status

On the baseline validation host (`aarch64-apple-darwin`, Rust `1.99.1`),
`cargo-mutants` is **UNAVAILABLE** (not installed). Running `make mutants`
checks `"$(CARGO)" mutants --version` first and exits non-zero immediately rather
than reporting a skipped or fabricated pass. Mutation score is therefore
recorded as **UNMEASURED** at baseline `6ab0918dbbf655894d8d3176b81091bd28899c5f`.

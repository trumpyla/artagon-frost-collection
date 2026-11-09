# Integer profile, revision 2 — measurement record

Evidence for the promotion matrix, shared storage, chunked representation,
inherited budgets, and `alloc`-only feature described in
`docs/integer-benchmarks.md` (revision 2 section) and `CHANGELOG.md`.

- `bench-1..3.csv` / `.log` — three complete runs of `benches/integer_sets.rs`
  on the revised crate (374 cases each; logs record representations, payloads
  and shared-container counts).
- `baseline-binary-1..3.csv` — three runs of the 0.4.0 integer-profile bench
  binary, interleaved with the revised binary. Unchanged control loops
  (`btree`, `native_roaring`, `frozen_ordered`) moved by up to 2× between the
  two binaries on this host, so this pair is kept as a record of that layout
  effect rather than used for the before/after table.
- `ab-same-binary.csv` — the comparison used in the report: both versions of
  the integer profile compiled into one executable (`ab-harness/`), every case
  validated, old and new measured alternately for three rounds; columns are the
  minimum per-round median for each version, their ratio, and the result
  representation each version produced.
- `ab-harness/` — the harness source. It needs the 0.4.0 tree copied next to it
  as `../frost-baseline` with `[package] name = "frost-baseline"` and
  `[lib] name = "frost_baseline"`; see its `Cargo.toml`.
- `review-probe/` — the probe programs from the adversarial review that first
  demonstrated the representation collapse, the ignored budgets, the equality
  cost and the dense/Roaring promotion gain.
- `environment.json` — host, toolchain, flags, affinity and protocol.

Run on a shared 2-vCPU host without POPCNT enabled at compile time; treat
ratios below about 1.3× or above about 0.75× on sub-microsecond cases as noise.

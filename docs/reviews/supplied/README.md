# frost-collections 0.4.0 — adversarial review bundle

- `FROST_ADVERSARIAL_REVIEW.md` — findings, measurements, design proposals, new-collection catalogue.
- `frost-review-patch.diff` — the verified patch (adaptive sorted-set algebra; `SnapshotStore` read path + RMW API).
  Apply with `cd frost && patch -p1 < ../frost-review-patch.diff`.
- `frost-patched/` — the original tree with the patch already applied (222 tests pass, clippy pedantic clean).
- `probe-harness/` — the benchmark harness that produced every number in the report.
  Run with `cargo run --release` from inside it; its `Cargo.toml` points at `../frost`, so place it
  next to a `frost/` directory (e.g. rename `frost-patched` to `frost`, or edit the path).

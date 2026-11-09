# frost-collections 0.4.0 — adversarial review bundle

- `FROST_ADVERSARIAL_REVIEW.md` — findings, measurements, design proposals, new-collection catalogue.
- `frost-review-patch.diff` — the verified patch (adaptive sorted-set algebra; `SnapshotStore` read path + RMW API).
  Apply with `cd frost && patch -p1 < ../frost-review-patch.diff`.
- `frost-patched/` — the original tree with the patch already applied (222 tests pass, clippy pedantic clean).
- `probe-harness/` — the benchmark harness that produced every number in the report.
  Run with `cargo run --release` from inside it; its `Cargo.toml` points at `../frost`, so place it
  next to a `frost/` directory (e.g. rename `frost-patched` to `frost`, or edit the path).

## Revision 2 inputs (historical evidence)

The integration also preserves `integer-profile-review.md`,
`lazy-views-exploration.md`, and `lazy-views-research-report.md` from the supplied
revision archive. Their proposals and measurements describe that input. The
current merge decision is `../revision2-integration-review.md`; the original two
patches and their digests are retained under
`verification/revision2-integration/input/` from the repository root.

## Collection algebra research inputs

- [Frost1](frost-collection-algebra-research-1.md) preserves `Frost1.md.md`.
- [Frost2](frost-collection-algebra-research-2.md) preserves `Frost2.md.md`.

Both files are unmodified supplied research, including unresolved citation
tokens and illustrative APIs. Frost2's sandbox research-template link refers
to an artifact that was not supplied. Neither report establishes a current API,
verified dependency claim, or implemented workspace split.

The [collection algebra synthesis](../../design/collection-algebra.md) records
their proposed contracts, the single-library constraint, and implementation
gates. The [attachment record](../../../verification/attachment-integration-2026-11-09/README.md)
identifies all five supplied inputs and separates imported evidence from checks
executed on this checkout.

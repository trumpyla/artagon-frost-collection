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

## Collection algebra and computed-set research inputs

| File | Original name | SHA-256 | What it is |
| --- | --- | --- | --- |
| [Frost1](frost-collection-algebra-research-1.md) | `Frost1.md.md` | `543d67b2f7c3724f9c96849ea77c8dd24c3a36792395c46e6050d1383ebd8f6d` | Extended collection-algebra survey with architecture proposals |
| [Frost2](frost-collection-algebra-research-2.md) | `Frost2.md.md` | `046cdaf91f7f2912ce966835951d00dd42ae230f5a76914b5eeb5b77f8862ad3` | Overlapping survey, implementation sketches, and a selected URL bibliography |
| [Computed-sets summary](frost-computed-sets-summary.md) | `Frost3.md.md` | `6f39fdbcf2d7816a3ea33eb40e371be7115b1b452cf040a7f8fe84a60de8177c` | Summary and delivery note for a computed-sets report; **not** the full report |
| [Research handoff archive](frost-research-handoff.zip) | Supplied handoff ZIP | `605767df09f7a631f7c27951c38054216e63de8ece55f9ca4d3f6c7fc65ece0f` | Integration instructions, draft README text, five template starters, and the three reports above |

All four files are unmodified supplied material. The three reports keep their
original opaque citation markers, session-local links, and illustrative APIs;
these files are the only Markdown documents in the repository allowed to
contain them. They are evidence, not instructions or current project policy.
Frost1 and Frost2 overlap; neither report, nor the summary, establishes a
current API, a verified dependency claim, or an implemented workspace split.

The reports and summary refer to artifacts that were not supplied and were not
recovered: a standalone collection-algebra research document, an earlier
research template, a full computed-sets report, its template, and its ZIP. No
standalone algebraic-foundations report was identified. The summary's
statements that those files were created, and its execution claims, are not
evidence that they exist. The [integration audit](../../research/integration-audit.md)
tracks each gap.

The [research index](../../research/README.md) holds the curated, cited
derivatives. The [collection algebra guide](../../design/collection-algebra.md)
records their proposed contracts, the single-library constraint, corrections to
the sketches, and implementation gates. The
[attachment record](../../../verification/attachment-integration-2026-11-09/README.md)
identifies the five inputs of the earlier integration and separates imported
evidence from checks executed on that checkout.

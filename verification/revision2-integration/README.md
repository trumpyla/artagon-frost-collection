# Revision 2 integration evidence

This directory records the selective merge of the four supplied inputs into the
reviewed main source. Current commands, results, and costs are summarized in
`../../docs/revision2-integration-validation.md`; review decisions are in
`../../docs/reviews/revision2-integration-review.md`.

- `before/` contains reproduced original defects, baseline/incoming checks,
  toolchain metadata, and input digests. These are not final-source passes.
- `input/` retains both supplied patches verbatim. The code-only patch is a
  subset of the full patch; they must not be applied one after the other.
- Named current `.log` files retain stdout/stderr, and status/manifest JSON
  records exact command outcomes and source digests where supplied. The final
  native, package, affected-feature, embedded, Miri, and benchmark runs cover
  the final production bytes. The full feature matrix and independent policy
  oracle predate the final two-file optimization-placement refinement; the
  validation report distinguishes those scopes.
- `first-pass/` preserves the earlier complete measurements and the two source
  files before that refinement. No per-case best-result selection was applied.
- `final-documentation-patch-checks.json` records final formatting, local links,
  whitespace, and clean patch applicability against the reviewed attachment.
- `ab-harness/` compiles reviewed main and integrated source into one executable.
  `reviewed-baseline/` retains byte-identical source from the reviewed input;
  only Cargo package/library names were changed for co-compilation.
- The automatic constructor retains main's planner. The incoming revision's
  4 KiB unconditional Dense preference and larger inherited budgets were not
  adopted. The harness is therefore measuring the integrated design, not the
  supplied revision unchanged.

Historical evidence elsewhere in `verification/` remains intact and must not be
presented as validation of this final source. Build directories and compiled
binaries are excluded from the delivered archive.

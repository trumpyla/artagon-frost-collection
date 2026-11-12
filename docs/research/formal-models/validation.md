---
id: FROST-RESEARCH-FORMAL-VALIDATION
status: verification-record
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Validation and delivery boundaries

Baseline: `27b14399b0245df2a018fd5eb6f24f47ce7c2dbe`. Date: 2026-11-12.
This record concerns the formal documentation/artifact change, not the Rust
library's release verification.

## Executed mathematics

```sh
node docs/research/formal-models/evidence/model-checks.mjs
```

Executed with Node v22.16.0. The deterministic model has **258,624
assertions**, **99,118 counted finite cases**, 14 groups,
33 explicit counterexamples and six deliberately incorrect
model variants rejected. Two consecutive runs produce identical JSON output.
The [result record](evidence/model-results.json) states every bounded domain.
No Frost source is imported or executed by this model. Assertions count checks,
not Rust tests, production coverage, independent reviewers, or unbounded proofs.

## Source and document checks

[Baseline evidence](evidence/baseline.json) records exact current-main Git blob
IDs for all six modified existing files. The current read, frozen-wrapper, and
IntKey source blobs were also matched against the available source snapshot.
No complete fresh clone was available: the connector supplied current metadata
and content, while direct Git/network access in the container was unavailable.

All 12 existing executable Rust/compile-fail README blocks remain unchanged.
The AGENTS symlink is real, relative and resolves to the metadata-bearing entry
point. Every existing implementation/safety/testing rule section was retained
in the linked implementation-contracts guide. CRATES, production Rust, Cargo,
Make, CI, existing tests/benchmarks and Beads state are unchanged.

Structural checks cover unique metadata IDs, registered law definitions, valid
JSON/YAML, fenced blocks, and new or changed local Markdown references and
anchors. Untouched target paths are checked against observed main paths; their
contents and every inherited link are not re-audited. See the delivery's
`artifact-validation.json` for exact scope and counts. External URLs are listed
as sources, not represented as an exhaustive live-link crawl.

## Patch and archive checks

The delivery builds a Git diff against the exact verified-file baseline, checks
whitespace, applies it to a separate scratch copy, compares all resulting bytes
and the symlink target, checks reversal, and tests ZIP integrity/checksums.
The delivery-level `patch-validation.json` identifies the exact patch hash and
results. No GitHub write or repository commit is part of this task.

Use the Git patch rather than copying the overlay to preserve mode 120000 for
AGENTS.md. A target checkout must still check its own revision and unrelated work.

## Unavailable / not executed

`rustc`, `cargo`, and `rustup` are unavailable in this runtime. No Rust compilation,
doctests, make check, Clippy, Miri, Kani, Loom, cargo-mutants or benchmarks were run.
Do not infer their success from prior repository logs or these finite models.
No independent model reviewer or machine proof assistant was used. Full linked
research reports not recovered earlier remain missing.

## Required destination checks

From a complete checkout of the reviewed revision, preserve unrelated edits and
run `git apply --check` before application, then `git diff --check` and the
required `make check` gate. Review CRATES catalog/feature/ownership requirements
for any later production implementation. Do not lower the toolchain or weaken
checks to obtain a passing result. A newer main needs conflict reconciliation,
not an old source-tree overwrite.

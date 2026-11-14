---
id: FROST-RESEARCH-FORMAL-VALIDATION
status: verification-record
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Validation and delivery boundaries

Sections up to "Required destination checks" are the delivery's own record,
written before integration. They describe the delivery, not this branch.
[Destination results](#destination-results-2026-11-12) at the end records what
ran in a complete checkout and what changed afterwards.

Baseline: `27b14399b0245df2a018fd5eb6f24f47ce7c2dbe`. Date: 2026-11-12.
This record concerns the formal documentation/artifact change, not the Rust
library's release verification.

## Executed mathematics

```sh
node docs/research/formal-models/evidence/model-checks.mjs
```

Executed with Node v22.16.0. The delivered model had 258,624 assertions,
99,118 counted finite cases, 14 groups and 33 counterexamples. Its "six
deliberately incorrect model variants" were comparisons of constants, not
mutations of the model. The model was revised at integration; current numbers
are in the destination results below. Two consecutive runs produce identical
JSON output.
Its result record and the script were removed on 2026-11-13 when the models
moved to Rust tests; see the last section. No Frost source is imported or executed by this model. Assertions count checks,
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
in the linked implementation-contracts guide. In the delivery, CRATES,
production Rust, Cargo, Make, CI, existing tests/benchmarks and Beads state were
unchanged; integration then edited CRATES.md references and seven catalog
`source` strings, as recorded below.

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

## Destination results, 2026-11-12

Run in a complete checkout of `27b1439` with Rust 1.99.1 and Node v22.23.4.

| Check | Result |
| --- | --- |
| Patch SHA-256 against the delivery record | Match (`4e6ad3a2…609e2d0`) |
| `git apply --check`, `git apply`, `git diff --check` | Passed |
| Delivered model rerun compared with delivered results | Byte-identical |
| `make check` on the delivered patch | Passed |
| `make package` with the symlink | Passed; Cargo packages `AGENTS.md` as a regular copy of its target |
| `make fuzz-check` | Passed (3 deterministic regressions) |
| `make check` after all corrections | Passed (1,168 repository-local links) |
| Revised Node model (2026-11-12, since removed) | 415,592 assertions, 145,463 cases, 13 groups, 32 computed counterexamples, 5 illustrations; deterministic |
| Its mutant runner (since removed) | 18 of 18 killed |

Not run: `make verify`, the feature matrix, embedded builds, Miri, Kani, Loom,
`cargo-mutants`, the libFuzzer smoke, and benchmarks. The change touches no
source, feature, or dependency.

Files changed beyond the delivery's six: `CRATES.md`,
`docs/design/crate-layout.md`, `docs/design/decisions.md`,
`docs/research/templates/research-prompt.md`,
`docs/research/synthesis/foundations-and-engine-roadmap.md`, `CHANGELOG.md`,
and `verification/metadata/catalog.json` (seven `source` strings; no catalog ID
changed). `src/`, `tests/`, `benches/`, Cargo manifests, the Makefile, CI,
scripts, and Beads state are untouched. The
[destination review](../../reviews/value-domain-algebra-destination-review.md)
lists every correction.

## Rust models, 2026-11-13

The Node model and its mutant runner were replaced by the Rust test target
[`tests/formal_models`](../../../crates/frost-collections-core/tests/formal_models/main.rs). It has 42 cases,
each a curated catalog record under `C-ALGEBRA-FORMAL-MODELS`. The textual
mutants became wrong-definition tests inside the target. Seven cases run real
Frost types.

| Check | Result |
| --- | --- |
| `cargo test --locked --test formal_models` | 42 passed |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed |
| `make catalog` | Passed; 387 case records, 64 curated, 12 contracts |

On the same day the hand-written generators were replaced by `exhaust`,
`itertools`, `proptest` and `proptest-state-machine`, and four cases were added:
an independent enumeration of partial bijections, exhaustive and sampled
operation sequences on the real mutable set families, and a sampled union law
whose wrong rule must fail with a shrunk counterexample.

| Check | Result |
| --- | --- |
| `cargo test --locked --test formal_models` | 46 passed |
| `make catalog` | Passed; 391 case records, 68 curated, 12 contracts |

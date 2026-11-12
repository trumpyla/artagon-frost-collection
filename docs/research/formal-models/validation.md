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
| Revised model | 415,592 assertions, 145,463 cases, 13 groups, 32 computed counterexamples, 5 illustrations; deterministic |
| Model mutants | 18 of 18 killed |

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

## Prior-art integration results, 2026-11-12

Run in a complete checkout of the branch at `89a80b6` (based on `27b1439`)
with the prior-art integration applied as uncommitted edits, Rust 1.99.1 and
Node v22.23.4. The integrator is one AI session, which also ran the survey.

| Check | Result |
| --- | --- |
| `make check` | Passed. Its link step verified 1,238 repository-local links. It started before the last documentation edits of the integration; `make doc-links` was rerun after them (next row) |
| `make doc-links` after all edits | Passed (1,244 links) |
| `git diff --check` | Clean |
| `model-checks.mjs` and `model-mutants.mjs` rerun | Byte-identical to the recorded results; neither file was changed |
| `prior-art-checks.mjs` | 88,986 assertions, 48,922 cases, 5 groups, 8 computed counterexamples; two runs identical; a run from the repository path matches the recorded results |
| `prior-art-mutants.mjs` | 8 of 8 caught |
| Corpus manifest against its schema | Valid (checked with the Python `jsonschema` package) |
| Raw input hashes, byte and line counts | Match for all five raw inputs |
| Report front matter against its manifest entry | Equal |
| Report sections against the landscape template | All thirteen present, in order |
| Law IDs | 31 in the catalog, 31 in the proof ledger, each bracketed in its chapter |
| Source ids used in changed documents | All defined in a source catalog |
| Anchors of links in changed documents | All resolve |
| Supplement manifest | Regenerated; every listed hash and size matches |

The structural rows were checked with throwaway scripts that are not kept in
the repository.

External sources: fifteen pages were fetched and read through a text
extraction on 2026-11-12. Four pages could not be read: the Isabelle
Collections Framework paper (client error), two Hackage pages (automated
fetching disallowed), and one Verus issue (rate-limited twice). The
[source catalog](../SOURCES.md#prior-art-survey-sources-2026-11-12) records
each. Two papers were read only to the first 100,000 characters.

Not run: `make verify`, the feature matrix, embedded builds, Miri, Kani, Loom,
`cargo-mutants`, the libFuzzer smoke, and benchmarks. The change touches no
source, feature, or dependency. No person reviewed the mathematics, and no
proof assistant or solver was used. The stability theorem and the
non-axiomatizability result are cited, not proved.

Files changed by the integration: the algebra chapters except `frozen.md`,
`collection-models.md` and `rust-integration.md`; `catalog.json`;
`docs/design/decisions.md`; the research index, manifest, source catalog, raw
index and synthesis; this supplement's index, sources, prompt, manifest and
this record; `docs/README.md`; `docs/reviews/supplied/README.md`; and
`CHANGELOG.md`. New files: the curated report, the two raw inputs, and the
four `prior-art-*` evidence files.

### Replay onto the updated branch before push

Before the push, the remote branch had gained one commit, `e1fd245`, a
crate-split proposal that changes three documentation files and none of the
files above except `docs/README.md`. The integration commit was replayed onto
it without a conflict. The table above describes the run before the replay.
After the replay:

| Check | Result |
| --- | --- |
| `make check` | Passed on the replayed tree; its link step verified 1,251 repository-local links |
| `git diff --check` against `e1fd245` | Clean |
| `prior-art-checks.mjs`, `prior-art-mutants.mjs`, `model-checks.mjs` | Byte-identical to the recorded results |
| Supplement manifest | Regenerated; `docs/README.md` was the one stale entry, and it was already stale at `e1fd245` |

The other structural rows were not rerun; the replay changed none of their
inputs.

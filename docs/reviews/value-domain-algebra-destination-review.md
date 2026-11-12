---
id: FROST-ALG-DESTINATION-REVIEW
status: review
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Destination review: value domains and collection algebras

Reviewed on 2026-11-12 while applying the delivery
`frost-value-domain-algebras.patch` to a complete checkout of `27b1439`. The
delivery's [own review](value-domain-algebra-review.md) was four passes by its
author, without a Rust toolchain or a full checkout. This record covers what
that review could not.

## Method and independence

Three readers worked from separate contexts and did not see each other's
conclusions before reporting:

| Reader | Scope | Method |
| --- | --- | --- |
| Integrator | Patch integrity, the rules restructure, existing-file hunks, metadata, provenance, project gates | Read every delivered file; ran the gates listed in the [validation record](../research/formal-models/validation.md#destination-results-2026-11-12) |
| Fact reviewer | Every statement about current source, documents, tracker and evidence | Read-only comparison with `src/`, `tests/`, `Cargo.toml`, design documents and Beads state |
| Mathematics reviewer | Every law, bound, closure claim and counterexample; the finite model | Argument plus its own brute-force scripts on small domains; 16 textual mutations of the model |

All three are runs of one AI model. They are independent of the delivery's
author and of each other's context, not independent in kind. No person, proof
assistant, or solver reviewed the mathematics.

## Result

No stated law, bound, closure claim, or chapter counterexample was found false.
No Rust item is presented as implemented when it is not, and every checked
statement about current source behaviour is correct. The defects were in the
evidence layer, in provenance, and in what the rules restructure left stale.

## Findings and dispositions

| ID | Severity | Finding | Disposition |
| --- | --- | --- | --- |
| D01 | high | The model's "adversarial mutant detection" compared constants and altered no model function. Deleting it changed only the assertion total. | Removed. `model-mutants.mjs` now applies 18 textual mutations in child processes; all are caught. |
| D02 | high | Seven real mutants were accepted by the delivered model: right-representative intersection, delta replay that ignores deletions, delta replay replaced by an unrelated fold, checked BiMap union that never conflicts, bag min as a product, monus as the left count, and a mislabelled alignment tag. | Added the checks that catch each: pinned left rule, a last-event-wins replay reference, union success against validity of the three-way union, scalar identities tying min, max, monus and absolute difference together, and pinned alignment output. |
| D03 | medium | Nine of 33 counterexamples were two hand-written literals. Two more did not test their stated law. | Four are now computed from model functions. Five remain and are labelled `illustration` in the results and the catalog. The two mis-stated ones are computed. |
| D04 | medium | Catalog evidence overstated: `FM-PREFIX` cited a model with no stream; `DOM-EQ` looped over relations that are equivalences by construction; `BAGREL-COMP` used weights 0 and 1 only; the "relation matrices versus composition formulas" cross-check did not exist; `DOM-IMAGE` tested necessity for intersection only. | Evidence strings corrected in the proof ledger and catalog. Added the Boolean-versus-weighted cross-check, weights 0..2, and per-operation necessity. `FM-PREFIX`, `DOM-EQ` and `SEQ-EFFECT` are stated as having no executable check. |
| D05 | medium | Headline counts read as evidence strength. About 28,000 assertions were identities of JavaScript's bit operators and about 15,000 could not fail. | Removed assertions that could not fail. The documents now say the count is not a coverage measure. |
| D06 | medium | `AGENTS.md` became a symlink to a short entry, so the contracts were no longer in the file agents load, and the entry did not mention `make check` or the unsafe boundary. | Kept the symlink as delivered. The entry now states the rules that hold for every change inline. |
| D07 | medium | The move left stale references: seven contract `source` strings in the verification catalog, three passages in `CRATES.md`, one each in the crate-layout design, the root README and the research prompt template. The catalog tool only checks that the path is a file, so it still passed. | Repointed to `docs/maintainers/implementation-contracts.md`. No catalog ID changed. Two links in the historical lazy-views audit still target `AGENTS.md` and are left as written. |
| D08 | medium | The entry and rule documents said "preserve current APIs". The maintainer decided on 2026-11-12 that API compatibility is not a constraint. | Recorded in [design decisions](../design/decisions.md#api-compatibility-is-not-a-constraint-2026-11-12). Entry, checklist, documentation rules, catalog and chapters restated. Semantic contracts, feature additivity and unsafe confinement are kept; that reading is the integrator's and is flagged for the maintainer. |
| D09 | medium | The input inventory lists 199 documents with `delivery_copy` paths. Those paths are members of the delivery archive, which is not in the repository; 150 of the 199 are not tracked here. | Stated in the supplement, with the archive's size and SHA-256. The archive is not committed. |
| D10 | low | `domains.md` repeated one paragraph three times. | Fixed. |
| D11 | low | The new section in `set-algebra.md` sat above the document's status line. | Moved below the introduction. |
| D12 | low | Chapters referred to "the session", an "earlier 13-package catalog" and "CI failures observed earlier". | Replaced with repository referents and the verifiable CI fact. |
| D13 | low | `rust-integration.md` called plural package names a recommendation. The decision record adopts no rule and the synthesis proposes singular. | Corrected. |
| D14 | low | Six new question IDs overlapped corpus questions without saying so. | Each now names the corpus question it refines. They stay local to the supplement. |
| D15 | low | Location and term slips: "root ordered module" for `src/family/ordered/set_ops.rs`; "adapters" for the native `VecDeque`/`LinkedList` backends; "8 distinct archive byte streams" for eight nested archives. | Corrected. |
| D16 | low | The bag chapter said every non-additive operation can newly overflow an aggregate total. That holds only for max and absolute difference. | Qualified. |
| D17 | low | Representative-level absorption and distribution were not covered. Two orientations change the selected object. | Added to the set chapter and the rewrite table, with computed counterexamples `CX-REP-ABSORPTION` and `CX-REP-DISTRIBUTE` and exhaustive checks of the orientations that hold. |
| D18 | low | The conversion law said when image fails to preserve intersection, difference and XOR, but not in which direction. | Added the inclusions and their consequence for a grant-and-deny rule, checked for all functions on three keys. |
| D19 | low | Counterexample IDs existed only in the script and the delivery's review. | Registered in the catalog with their kind. |
| D20 | low | "Materialized answer" was redefined in the root README but not in the research synthesis. | Synthesis aligned. |
| D21 | low | The delivery's validation record describes the delivery; several of its statements are false for this branch. | Marked as the delivery's record and followed by destination results. |

## Confirmed, with no change needed

- The patch hash matches the delivery record and the patch applies to `27b1439`.
- The six recorded baseline blob IDs and the three source blob IDs match.
- `implementation-contracts.md` equals the previous `AGENTS.md` apart from its
  header and re-prefixed link targets.
- The delivered model reran byte-identically before it was revised.
- Every capability, trait, plan and seal named as existing does exist with the
  stated role. Every bag, multimap, BiMap, view and domain name is absent from
  `src/` and is labelled proposed or candidate.
- Chapter mathematics confirmed by independent brute force includes: image and
  preimage laws for all functions on three and four keys; map merge
  associativity against all 81 resolvers on two values; closure, local
  identities and checked-union behaviour of all 34 partial bijections on three
  keys; the knowledge-bound rules, which are exact for independent operands;
  and the bag, sequence and relation laws.

## Found outside the delivery

These predate the delivery and are not fixed here.

- **Stale fuzz oracle.** `src/integer/storage.rs` can return an empty Chunked
  operand under an Auto output policy, which the 2026-11-10 decision record
  describes as intended sharing. `verification/fuzz/src/lib.rs` asserts that
  empty non-forced outputs are Sorted. `make fuzz-check` passes; the libFuzzer
  smoke was not run here.
- **Scheduled workflow.** On main at `27b1439` the push workflow passes. The
  scheduled "Tiered verification and coverage" workflow fails in
  `nightly-deep-verification` (step: bounded libFuzzer smoke targets) and
  `nightly-mutation` (step: mutation suite). Whether the stale oracle causes the
  first failure was not established.

## Open

- The symlink trades a complete auto-loaded rule file for a short one. On a
  checkout without symlink support `AGENTS.md` is a one-line file holding the
  target path. Reverting the commit that introduced it restores the regular
  file without touching the algebra documents.
- The delivery archive is not retained, so 150 inventoried inputs cannot be
  checked from this repository.
- The 228 KB input inventory is included in the Cargo package, because the
  manifest excludes only `verification/**` and `docs/reviews/supplied/**`.
- Nothing here is accepted design. The chapters remain proposed contracts, and
  no production API exists for them.
- Not run: `make verify`, the feature matrix, embedded builds, Miri, Kani, Loom,
  `cargo-mutants`, the libFuzzer smoke, and benchmarks.

[Formal index](../design/algebra/README.md) ·
[Delivery review](value-domain-algebra-review.md) ·
[Validation](../research/formal-models/validation.md)

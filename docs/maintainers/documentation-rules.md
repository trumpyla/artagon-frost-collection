---
id: FROST-RULES-DOCUMENTATION
status: maintainer-guidance
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Documentation and research update rules

[CRATES.md](../../CRATES.md) remains authoritative for layout, dependencies,
API obligations, coverage and metadata. Existing published types and selected
naming decisions are not superseded by an imported research report.

## Formal-first changes

For every new algebra operation, state: input/output value and key domains;
validity and collection invariants; mathematical denotation; closure or checked
failure; representative, multiplicity and order rules; ownership and recovery;
callback effects; source context/completeness; and CPU/memory/I/O assumptions.
Then define type-specific refinements and a reference model. A semantic trait
implementation is not a compiler-checked proof, and never licenses unsafe code.

Update the [formal catalog](../design/algebra/catalog.json), relevant domain
chapter and [terminology](../design/algebra/terminology.md) together. Use stable
law IDs. Distinguish proof arguments, finite checks, implementation tests,
solver/model-checker evidence, and performance data. Record counterexamples to
rejected rewrites, including changes in definedness, errors or representatives.

## Co-update rules

- Public behavior: rustdoc and asserting tests/catalog records, changelog and
  design decisions as current project policy requires. This documentation-only
  proposal must not pretend to have changed a production API.
- Formal proposal: domain chapter, operation table, cross-domain conversions,
  law ledger and affected README/index entries. Keep the current source mapping.
- Names: preserve main's `SeqMultiMap`, `SetMultiMap`, `BagMultiMap`, existing
  capabilities, and distinct integer proposal names. Mark new candidates as such.
- Ownership: distinguish frozen, materialized, shared, persistent, borrowed and
  revision-bound. Do not claim materialized references are lifetime-independent.
- Crate layout: update the decision under `frost-wlt.1` and authoritative standards
  when approved; do not treat algebra ownership as approval to move inherent methods.

## Research workflow

Use the existing [research prompt](../research/templates/research-prompt.md),
[landscape template](../research/templates/landscape-report.md),
[deep-dive template](../research/templates/deep-dive-report.md), and
[subtopic handoff](../research/templates/subtopic-handoff.md). A landscape must
produce specific follow-up questions, assumptions, evidence gaps and acceptance
criteria; a child report links back and feeds decisions into its parent.

Preserve originals with hashes. Register curated derivatives, sources, versions,
retrieval date, conflicts and missing artifacts. No opaque chat citations or
session-local URLs in active guidance. Use portable primary references, and
label unverified mappings or newly authored replacements honestly. Never promote
an illustrative snippet to a compiled API or a report into an accepted design.
The new formal supplement uses its own manifest without rewriting older evidence.

## Review and validation

Check local links/anchors, metadata, exact input hashes, naming drift, examples,
and scoped diff. Adversarial review must challenge quotient congruence, closure,
partial-operation definedness, ownership, effects, overflow, source consistency,
and counterexamples. State whether reviewers are independent; several passes by
one author are not independent-agent review. Run the actual required project
gates; unavailable Rust/Miri/Kani/Loom checks remain unrun, not inferred from
mathematical scripts. Keep CI remediation and source optimization outside a
pure documentation patch unless specifically authorized.

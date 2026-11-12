---
id: FROST-RESEARCH-FORMAL-SUPPLEMENT
status: research
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Value domains and algebra: research supplement

Status: new, source-reconciled formal design, not a recovered Deep Research
report or an implementation. Baseline: `27b14399b0245df2a018fd5eb6f24f47ce7c2dbe`.

## Executive conclusion

Use a small semantic hierarchy: valid values; key equivalence; collection
carrier; representation invariant; operation denotation and partiality; observable
contract; and evaluation context. Freezing refines access/ownership over a
carrier. It does not create a universal algebra or prove deep immutability.
Prefer refined Rust types and checked construction before adding optional source
bindings. Existing Frost families do not acquire mandatory domain parameters.

## Artifacts

| Artifact | Purpose |
| --- | --- |
| [Formal index](../../design/algebra/README.md) | Abstract model first, then each collection |
| [Source-to-contract map](source-map.md) | Reconciliation of session inputs and current source |
| [Input inventory](input-inventory.json) | 199 unique textual documents, 345 recorded occurrences, 8 distinct archive byte streams |
| [Primary sources](SOURCES.md) | Checked primary contracts, definitions and limits |
| [Adversarial review](../../reviews/value-domain-algebra-review.md) | Four same-author passes and concrete dispositions |
| [Proof ledger](../../design/algebra/proof-obligations.md) | Proof arguments, finite checks and unexecuted implementation gates |
| [Model source](evidence/model-checks.mjs) / [results](evidence/model-results.json) | Reproducible finite mathematical checks; no production Rust imported |
| [Validation](validation.md) | Commands, patch checks and unavailable gates |
| [Research prompt](research-prompt.md) | Focused child investigations using the existing project templates |
| [Supplement manifest](manifest.json) | This supplement's artifact/contract inventory, separate from historical corpus metadata |

## Inputs and completeness

"Session" in this supplement means the authoring conversation that produced the
delivery `frost-value-domain-algebras.zip` (1,296,927 bytes, SHA-256
`a4e4f7c8894f578932d3acc8de39077c59cdd53a412b3abde386c7137ee85559`). That
archive is **not retained in this repository**. Each `delivery_copy` path in the
[input inventory](input-inventory.json) names a member of that archive, not a
repository file. When the inventory was checked against this repository on
2026-11-12, 49 of its 199 documents were byte-identical to a tracked file at
`27b1439`; the other 150 (older revisions and unmerged patches) can be verified
only against the archive.

All mounted original reports, handoff instructions, prior reconciled archives,
formal patches, and relevant repository snapshots were inventoried. Identical
bytes are deduplicated in the inventory. Source-control object databases and
build directories are excluded. The word “inventoried” means read/routed by
content and provenance; it does **not** certify every historical benchmark,
external citation, or unrelated document sentence.

Frost1 and Frost2 overlap. Their original bytes remain evidence under the
repository's supplied-input paths. Frost3 is a delivery summary. Separate full
computed-set/foundations deliverables referred to by session-local links have
not been established as available files. The new formal documents are explicitly
new synthesis, not those unrecovered reports. Original archives retain their
own dated evidence and cannot validate this patch.

## Model evidence

Run from a complete repository after applying the patch:

```sh
node docs/research/formal-models/evidence/model-checks.mjs
```

The script uses only Node's standard assertion library, no network/dependencies,
no filesystem writes and no Rust imports. Node is optional research tooling,
not a new Make/build/CI prerequisite. Current result: **258,624
assertions**, **99,118 counted cases**, 14 groups,
33 explicit counterexamples and six deliberately incorrect
model variants detected. Small integer arithmetic stays exactly representable
in JavaScript Number; it is not a simulation of every u64/usize overflow path.

The checks cover logical membership, representatives, conversion/closure,
map presence and joins, bag counts, sequence occurrences, relation/BiMap
composition, incomplete knowledge, and ordered deltas. They cannot establish
Rust borrow safety, drop/panic behavior, full-domain correctness, performance,
actual source consistency, or generic trait coherence. No independent reviewer
or solver proof is claimed.

## Handoff to implementation

Accept the formal contracts explicitly, then implement one small domain/view
with a reference implementation and real Rust obligations under CRATES. Existing
methods need not stay source-compatible (see the
[decision of 2026-11-12](../../design/decisions.md#api-compatibility-is-not-a-constraint-2026-11-12));
a breaking change records its reason. Treat runtime dictionary
bindings and fallible durable cursors as separate adapter work. Measure terminal-
specific workloads only after a correct semantic baseline exists. On main at
`27b1439` the push workflow passes and the scheduled "Tiered verification and
coverage" workflow fails in its `nightly-deep-verification` and
`nightly-mutation` jobs (runs of 2026-11-10 to 2026-11-12). That failure
predates this supplement and is separate remediation work.

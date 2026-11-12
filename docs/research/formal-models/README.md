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
| [Input inventory](input-inventory.json) | 199 unique textual documents, 345 recorded occurrences, 8 nested archives examined |
| [Primary sources](SOURCES.md) | Checked primary contracts, definitions and limits |
| [Adversarial review](../../reviews/value-domain-algebra-review.md) | Four same-author passes and concrete dispositions |
| [Proof ledger](../../design/algebra/proof-obligations.md) | Proof arguments, finite checks and unexecuted implementation gates |
| [Model source](evidence/model-checks.mjs) / [results](evidence/model-results.json) | Reproducible finite mathematical checks; no production Rust imported |
| [Model mutants](evidence/model-mutants.mjs) / [results](evidence/model-mutants-results.json) | Which wrong definitions the model's checks catch |
| [Prior-art report](../reports/value-domain-algebra-prior-art.md) | Established theory per construct, per-source check levels, corrections to the survey, and the four laws added from it |
| [Prior-art checks](evidence/prior-art-checks.mjs) / [results](evidence/prior-art-checks-results.json) | Finite checks for `BAG-MONUS`, `BIMAP-COMPAT`, `CLOSURE-STABLE` and `CTX-KEY`; separate from the model and its totals |
| [Prior-art check mutants](evidence/prior-art-mutants.mjs) / [results](evidence/prior-art-mutants-results.json) | Which wrong definitions those checks catch |
| [Destination review](../../reviews/value-domain-algebra-destination-review.md) | Independent review at integration, corrections, and the gates that ran |
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
only against the archive. The same holds for the delivery's
`artifact-validation.json` and `patch-validation.json`, which the
[validation record](validation.md) cites. The inventory also lists two
`recovered-research` export files from which none of the 199 documents
originates; they do not change the statement below that the full reports remain
unrecovered.

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
not a new Make/build/CI prerequisite. Current result: **415,592 assertions** over **145,463 counted cases** in 13 groups, with
32 computed counterexamples and 5 illustrations.
An illustration records two hand-written values that differ; it is not a model
result. Many assertions are identities of JavaScript's native bit operators, so
the count is not a coverage measure. Small integer arithmetic stays exactly
representable in JavaScript Number; it is not a simulation of every u64/usize
overflow path.

A separate runner applies 18 textual mutations to the model and reports which
checks catch them:

```sh
node docs/research/formal-models/evidence/model-mutants.mjs
```

All 18 are caught in the [recorded run](evidence/model-mutants-results.json).
The delivered model had a section called "mutant detection" that compared
constants and changed no model function; the destination review found seven real
mutants that it accepted, replaced that section, and added the checks that now
catch them. See the [destination review](../../reviews/value-domain-algebra-destination-review.md).

The laws added with the prior-art survey of 2026-11-12 have their own script,
so the model's recorded totals above are unchanged:

```sh
node docs/research/formal-models/evidence/prior-art-checks.mjs
node docs/research/formal-models/evidence/prior-art-mutants.mjs
```

Recorded result: **88,986 assertions** over **48,922 counted cases** in 5
groups, with 8 computed counterexamples; 8 of 8 mutants caught. The script
checks monus identities on counts 0..6, compatibility on all 34 partial
bijections on three keys, instances of the stability theorem, the
three-valued reading of the knowledge bounds, and a toy model of a result
cache. It does not prove the cited stability theorem, and its cache model is
not a model of any engine. The same limits as the model apply.

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
`nightly-mutation` jobs (runs of 2026-11-11 and 2026-11-12; the failing steps
are the bounded libFuzzer smoke targets and the mutation suite). That failure
predates this supplement and is separate remediation work.

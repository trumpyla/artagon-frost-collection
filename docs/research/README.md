# Frost research corpus

This directory holds the research that informs Frost's design: supplied
reports preserved as evidence, curated derivatives with corrections and
portable citations, a decision-oriented synthesis, reusable research
templates, and ready-to-run prompts for the next focused investigations.
Nothing here is an accepted design or an implemented API unless its status
says so. The [documentation index](../README.md) lists every design,
verification, and review document; the [crate README](../../README.md)
explains the rationale and the implemented library.

The current [naming dispositions](../design/collection-algebra.md#naming-dispositions)
resolve selected research spellings without implementing the proposed types.

Baseline for every artifact: commit `82becb998af5627cc1e58118ae01515d8a3559ca`,
imported or written on 2026-11-09. The [machine manifest](manifest.json)
(validated against [its schema](manifest.schema.json)) records ids, hashes,
status, and relationships for scripts; this page is the human index.

## Status labels

| Label | Meaning |
| --- | --- |
| implemented | Behaviour present in inspected source at the baseline, with tests |
| proposal | A design idea with no implementation; may contain pseudocode |
| research | Imported or curated research; evidence quality stated per claim |
| historical evidence | A record of a past check or input; not current policy |
| planned task | A question or task with a stable id and no result yet |

Research status per artifact: `imported` (byte-exact input), `curated`
(corrected derivative or new synthesis), `reviewed`, `superseded`.
Source validation: `pending`, `partial`, `verified`.

## Artifacts

| Artifact | Kind | Status | Purpose |
| --- | --- | --- | --- |
| [Raw inputs](raw/README.md) | index | imported; historical evidence | Where the three byte-exact inputs and the handoff archive live, with hashes and warnings |
| [Source catalog](SOURCES.md) | catalog | partial | Stable `SRC-…` ids with versions, sections, access dates, what each supports, and limits; input-supplied bibliography marked unverified; gap table |
| [Citation map](citation-map.json) | machine | pending | Every opaque export marker group, scoped to its input, with unresolved status and editorial support ([schema](citation-map.schema.json)) |
| [Integration audit](integration-audit.md) | audit | curated | Missing artifacts and recovery attempts, the hazard decision matrix, the n-ary bound derivation, executed checks, evidence gaps, verification updates |
| [Collection algebra survey](reports/collection-algebra-survey.md) | report, landscape | curated; partial | Curated from Frost1: domains and laws, representative identity with a compiled corrected example, ecosystem evidence with status, corrected sketches, per-claim ledger |
| [Collection algebra implementation notes](reports/collection-algebra-implementation-notes.md) | report, landscape | curated; partial | Curated from Frost2: versions and licenses as dated claims, target-restriction correction, output modes, n-ary strategies, verification mapped to Make targets |
| [Computed sets summary](reports/computed-sets-summary.md) | summary | curated; partial | Curated from Frost3; the full report it describes was not supplied |
| [Foundations and engine roadmap](synthesis/foundations-and-engine-roadmap.md) | synthesis, landscape | curated (new synthesis); partial | Executive map, taxonomy, claim ledger, conflicts, alternatives, current-versus-planned matrix, prioritized questions, roadmap |
| [Master research prompt](templates/research-prompt.md) | template | curated | Launch a landscape, deep-dive, or validation task without this conversation |
| [Landscape report template](templates/landscape-report.md) | template | curated | Required sections and ledger for an overview |
| [Deep-dive report template](templates/deep-dive-report.md) | template | curated | Required sections for a deep dive or validation |
| [Subtopic handoff template](templates/subtopic-handoff.md) | template | curated | Hand one question to a child researcher with stable ids |
| [Review checklist](templates/review-checklist.md) | template | curated | Review any research or rationale change |
| [Prompt: algebraic foundations](prompts/algebraic-foundations.md) | prompt | planned task `Q-ALG-1` | Law tables, counterexamples, rewrite permissions per domain and terminal |
| [Prompt: Frozen API contracts](prompts/frozen-api-contracts.md) | prompt | planned task `Q-FROZEN-1` | A written standard for new frozen families and wrappers |
| [Prompt: computed sets and durable execution](prompts/computed-sets-and-durable-execution.md) | prompt | planned tasks `Q-CSET-1`, `Q-NARY-1` | Evaluation model, revisions, overlays, budgets, cursors, n-ary cost model |
| [Prompt: authorization-engine integration](prompts/authorization-engine-integration.md) | prompt | planned task `Q-AUTHZ-1` | Data contracts at the engine boundary; influences sourced or marked influence-only |

Questions `Q-MAP-1`, `Q-BAG-1`, `Q-MM-1`, `Q-BIMAP-1`, and `Q-PREFIX-1` have
launch blocks under their own headings in
[synthesis section 8.1](synthesis/foundations-and-engine-roadmap.md#81-seeded-follow-up-blocks),
which `manifest.json` references as their briefs; a dedicated prompt can be
written with the subtopic handoff template once a question's dependencies are
answered.

## Relationships

```text
raw inputs (docs/reviews/supplied)  --curated into-->  reports/
reports/ + inspected source          --synthesized-->   synthesis/foundations-and-engine-roadmap.md
synthesis questions Q-…              --handed off-->    prompts/ (made with templates/subtopic-handoff.md)
child reports (future)               --feed back-->     synthesis sections 3, 4, 7, 8 and manifest.json
SOURCES.md ids                        --cited by-->      every curated file; citation-map.json records raw markers
```

A child report never approves a design; design proposals live under
`docs/design`, and implementation status comes only from source and tests.

## Reading paths

**New contributor.** Crate README rationale → [Frozen API guide](../design/frozen-api.md)
→ [design philosophy](../design/philosophy.md) → this index → the
[synthesis](synthesis/foundations-and-engine-roadmap.md) sections 1, 2, and 7.

**Frozen API implementer.** [Frozen API guide](../design/frozen-api.md) →
[architecture](../design/architecture.md) sections 1-3 and 6 →
[snapshot contract](../design/snapshots.md) → the tests named in the source
catalog (`SRC-FROST-TESTS`) → [prompt Q-FROZEN-1](prompts/frozen-api-contracts.md).

**Algebra or planner implementer.** [Set operations](../set-operations.md)
(implemented) → [general set algebra](../design/set-algebra.md) (proposal) →
[collection algebra synthesis](../design/collection-algebra.md) → the
[survey](reports/collection-algebra-survey.md) and
[implementation notes](reports/collection-algebra-implementation-notes.md) →
[prompt Q-ALG-1](prompts/algebraic-foundations.md) → the n-ary derivation in
the [audit](integration-audit.md).

**Authorization-engine integrator.** Synthesis section 6 →
[computed sets summary](reports/computed-sets-summary.md) →
[snapshot contract](../design/snapshots.md) → [integer profile](../design/integer-sets.md)
for encoded ids → [prompt Q-CSET-1](prompts/computed-sets-and-durable-execution.md)
→ [prompt Q-AUTHZ-1](prompts/authorization-engine-integration.md).

**Researcher starting a subtopic.** This index → synthesis section 8 (pick a
`Q-…` id) → its prompt, or write one with the
[subtopic handoff template](templates/subtopic-handoff.md) → the
[master prompt](templates/research-prompt.md) → the report template for the
mode → register the result in `manifest.json` and run the
[review checklist](templates/review-checklist.md).

## Adding an artifact

1. Choose a stable id: `frost-research-<kind>-<slug>`; questions are
   `Q-<AREA>-<n>`; sources are `SRC-<ORIGIN>-<DETAIL>`.
2. Put the metadata block (the keys in `manifest.schema.json`) in the file's
   front matter and add the same entry to `manifest.json`.
3. Cite only catalog ids or portable references. No opaque export markers, no
   session-local links. Raw inputs are the documented exception and live
   under `docs/reviews/supplied/`, never here.
4. Label every snippet: compiled and run, compiled only, or pseudocode.
5. Run the structural checks listed in the audit and the review checklist.

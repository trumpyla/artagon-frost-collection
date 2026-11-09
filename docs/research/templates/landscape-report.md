---
id: frost-research-template-landscape
kind: template
level: landscape
research_status: curated
implementation_status: not-applicable
source_validation: not-applicable
input_files: [docs/reviews/supplied/frost-research-handoff.zip]
input_members: [frost-research-handoff/templates/landscape-report.md]
input_sha256: [605767df09f7a631f7c27951c38054216e63de8ece55f9ca4d3f6c7fc65ece0f]
origin_created_at: unknown
imported_at: 2026-11-09
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca
parent_ids: [frost-research-template-prompt]
related_ids: [frost-research-template-deep-dive, frost-research-template-subtopic-handoff]
supersedes: []
open_questions: []
---

# Landscape report template

Copy everything below the horizontal rule into the new report. Keep the
front-matter keys; they are the same keys the corpus manifest records. Replace
every `REPLACE`. Leave `unknown` where a value is genuinely unknown.

---

```yaml
id: REPLACE
kind: report
level: landscape
research_status: draft        # draft | imported | curated | reviewed | superseded
implementation_status: proposed   # proposed | implemented | mixed | not-applicable
source_validation: pending    # pending | partial | verified
input_files: []
input_sha256: []
origin_created_at: ACTUAL_DATE_OR_unknown
imported_at: ACTUAL_DATE
repo_baseline: FULL_COMMIT_HASH
parent_ids: []
related_ids: []
supersedes: []
open_questions: []
```

# TITLE

## Executive synthesis

The decision, the main recommendation, the strongest evidence for it, the key
uncertainty, and the immediate next step. One screen.

## Context and current implementation

Repository revision; modules, features, tests, and documents actually
inspected; user requirements; evidence that was unavailable. Distinguish
implemented mechanisms from target architecture in a two-column table.

## Scope and non-goals

Audience, workloads, logical domains, constraints, and explicit exclusions.

## Taxonomy and formal vocabulary

Define each logical domain before comparing operation names. Include an
operation / law / observable-semantics table with columns for equality
relation, representative rule, order, multiplicity, failure or overflow, and
callback effects.

## Evidence and ecosystem map

Primary-source inventory with `SRC-…` ids, a versioned capability matrix,
conflicting sources, and what each option does not provide.

## Candidate architectures and tradeoffs

Status quo, conservative extension, specialization, and rejected alternatives.
State decision criteria, not arbitrary scores.

## Cross-cutting contracts

Representatives; ordering; ownership; errors and overflow; mutable, frozen,
persistent, snapshot, and materialized distinctions; concurrency; feature and
coherence constraints; security boundaries.

## Research gaps and decision dependencies

List what the evidence does not settle. Order dependent questions so that a
later question never requires an answer the earlier one has not produced.

## Prioritized research questions

| Question ID | Decision to unlock | Why existing evidence is insufficient | Depends on | Child brief | Acceptance criteria |
| --- | --- | --- | --- | --- | --- |
| REPLACE | REPLACE | REPLACE | REPLACE | REAL_FILE_OR_pending | REPLACE |

Each child brief is a real file produced with `subtopic-handoff.md`, or the
word `pending` with the reason.

## Recommendation and roadmap

Near-term acceptance gates, deferred questions, expected owners, and the
conditions that would reverse the recommendation. No crate creation, API
claim, or implementation status that the checkout does not show.

## Claim and evidence ledger

| Claim ID | Claim | Class | Source or derivation | Scope and confidence | Open validation |
| --- | --- | --- | --- | --- | --- |
| REPLACE | REPLACE | implemented-and-inspected / documented-upstream / derived / proposal / experiment-planned / experiment-executed / historical / unverified | REPLACE | REPLACE | REPLACE |

## References

Portable references only: version, date, section, and primary-source link.
No opaque export markers and no session-local links.

## Change and lineage record

Parent links, children, corrections, superseded claims, and the next review
trigger.

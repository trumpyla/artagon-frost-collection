---
id: frost-research-template-deep-dive
kind: template
level: deep-dive
research_status: curated
implementation_status: not-applicable
source_validation: not-applicable
input_files: [docs/reviews/supplied/frost-research-handoff.zip]
input_members: [frost-research-handoff/templates/deep-dive-report.md]
input_sha256: [605767df09f7a631f7c27951c38054216e63de8ece55f9ca4d3f6c7fc65ece0f]
origin_created_at: unknown
imported_at: 2026-11-09
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca
parent_ids: [frost-research-template-prompt]
related_ids: [frost-research-template-landscape, frost-research-template-subtopic-handoff]
supersedes: []
open_questions: []
---

# Deep-dive report template

Copy everything below the horizontal rule into the new report. A validation
report uses the same headings with the substitutions described in the master
prompt. Replace every `REPLACE`.

---

```yaml
id: REPLACE
kind: report
level: deep-dive            # deep-dive | validation
research_status: draft
implementation_status: proposed
source_validation: pending
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

## Executive decision brief

Recommendation, engineering consequences, strongest evidence, important
limitations, and alternatives still viable.

## Research question and inherited context

Decision to unlock; parent question and claim ids; assumptions independently
revalidated (with the result of revalidation); scope and non-goals; repository
baseline, toolchain, and the inspected source paths.

## Formal model

Domains, universes, relations, operations, identities, cardinality,
equivalence, and closure. State error, partiality, overflow, and snapshot
semantics. Give one counterexample for each tempting invalid generalization
(for example: membership-equivalent rewrite that changes representatives;
lazy evaluation assumed to be bounded; frozen assumed to be consistent).

## Laws and optimizer permissions

| Transformation | Preconditions | Membership effect | Representatives / order / effects | Evidence | Disposition |
| --- | --- | --- | --- | --- | --- |
| REPLACE | REPLACE | REPLACE | REPLACE | REPLACE | allowed / allowed-for-terminals / forbidden |

## Public API and ownership

Actual versus proposed signatures; borrowed queries; lifetimes; operation-
specific bounds; stored references versus decoded or lending outputs; error
types and what they recover; construction, freeze, thaw, and extraction;
feature and `no_std` boundaries. State the compile status of every snippet:
**compiled and run**, **compiled only**, or **pseudocode**. Record the
command and toolchain for anything compiled.

## Implementations and physical plans

At least a simple oracle, a conservative implementation, and the relevant
optimized alternatives. Supply pseudocode, data layouts, invariants, and
dependency direction. Describe static/dynamic dispatch and coherence
constraints where applicable.

## Complexity and resource model

Define every variable: source count `k`, input sizes, key length, data skew,
output size, block size, buffer budget. Separate worst-case, expected, and
amortized CPU; allocations; resident memory; I/O, seeks, or network;
cancellation and backpressure. State workspace separately from output. Do not
hide a full traversal behind a lazy API, and do not transfer a bound
documented for one algorithm to a different one.

## Consistency, failure, and threat analysis

Invalid inputs; callback panics and effects; stale or incomplete indexes;
domain mismatches; retries; revocation; snapshot compatibility; resource
exhaustion. State the boundary of the library's responsibility.

## Verification plan and executed evidence

Independent model; property and exhaustive tests; representative, drop, and
lifetime tests; fault injection; applicable Miri, Loom, or Kani scope.
Separate proposed checks from executed commands and results. State proof
bounds and unmodeled dependencies.

## Benchmark design and results

Workloads, controls, data generation, warm and cold states, architecture,
toolchain, flags, repetitions, raw outputs, correctness checks, and timing
boundaries. Mark unrun scenarios **PLANNED**. Include counterworkloads and
regressions, not only favorable cases.

## Decision and integration roadmap

Chosen alternative, rejected alternatives, acceptance gates, public
compatibility, owner/module proposals, and follow-up decisions. No claimed
production implementation without source evidence.

## Evidence ledger and bibliography

Each important external fact maps to a versioned primary source with a
`SRC-…` id. Derivations state assumptions. Contradictions and missing evidence
are visible.

## Parent feedback and child work

Changes proposed to the parent synthesis, and self-contained handoffs for the
remaining questions. Preserve stable ids and uncertainty.

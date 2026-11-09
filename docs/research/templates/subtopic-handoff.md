---
id: frost-research-template-subtopic-handoff
kind: template
level: not-applicable
research_status: curated
implementation_status: not-applicable
source_validation: not-applicable
input_files: [docs/reviews/supplied/frost-research-handoff.zip]
input_members: [frost-research-handoff/templates/subtopic-handoff.md]
input_sha256: [605767df09f7a631f7c27951c38054216e63de8ece55f9ca4d3f6c7fc65ece0f]
origin_created_at: unknown
imported_at: 2026-11-09
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca
parent_ids: [frost-research-template-prompt]
related_ids: [frost-research-template-landscape, frost-research-template-deep-dive]
supersedes: []
open_questions: []
---

# Focused subtopic handoff template

A handoff moves one question from a parent report to a child researcher. It
must be usable without the conversation that produced the parent. Copy
everything below the rule.

---

```yaml
question_id: REPLACE            # the parent's stable Q-… id; never renumber
parent_report_id: REPLACE
parent_path: REAL_REPOSITORY_RELATIVE_PATH
parent_revision: COMMIT_CONTAINING_PARENT_OR_UNRESOLVED   # never a code baseline where the parent does not exist; resolve before execution
parent_content_sha256: UNRESOLVED_OR_DIGEST   # digest of the parent file as read, when its commit is not yet known
repo_baseline: FULL_COMMIT_HASH_OF_INSPECTED_CODE          # production code baseline, separate from the parent's revision
parent_sections_or_claims: []   # section anchors or claim ids
child_report_id: REPLACE
mode: deep-dive                 # deep-dive | validation
output_path: REAL_REPOSITORY_RELATIVE_PATH
inherited_assumptions: []       # each marked confirmed | working | stale
owning_module_or_crate: REPLACE_OR_UNKNOWN
```

Before execution, the child researcher resolves `parent_revision` to the
commit that actually contains the parent report (or records the parent's
content digest if it is not yet committed) and records the result here. A
newly written parent cannot be pinned to the code baseline it was written
against; `repo_baseline` names only the inspected production code.

## Decision to unlock

One concrete architectural, API, or algorithm decision, and why the parent
overview cannot settle it. If the child is a validation, name the claim id
being tested.

## Known evidence and assumptions to revalidate

Confirmed findings and working assumptions in two separate lists. Link
`SRC-…` sources and claim ids, not only the parent title. Identify version or
implementation claims that may be stale at the parent revision.

## Scope and exclusions

Logical domain, output contract, supported environments and workloads.
Adjacent topics that must not absorb this investigation.

## Competing hypotheses or designs

Include the status quo. For each alternative, state the strongest reason to
choose it and a plausible failure condition.

## Questions to answer

Laws, invariant boundaries, ownership, observable semantics, CPU, memory, and
I/O costs, failure cases, and integration constraints. Avoid a generic request
to research deeply.

## Required evidence and experiments

Primary specifications, papers, and source areas; differential oracles;
counterexamples; reproducible benchmarks. Experimental work is conditional on
actual tool availability; no fictional runs.

## Acceptance criteria

What a maintainer needs in order to choose. A negative result or a clearly
bounded unknown is acceptable; unqualified speculation is not.

## Deliverables and feedback

One standalone report, an evidence ledger, proposed design changes, test or
benchmark plans or actual results, and a concise update to the parent's
claim/decision matrix. Copying the parent's claim does not validate it. A
child report does not approve a design decision or an implementation task.

---
id: frost-research-template-prompt
kind: template
level: not-applicable
research_status: curated
implementation_status: not-applicable
source_validation: not-applicable
input_files: [docs/reviews/supplied/frost-research-handoff.zip]
input_members: [frost-research-handoff/templates/research-prompt.md]
input_sha256: [605767df09f7a631f7c27951c38054216e63de8ece55f9ca4d3f6c7fc65ece0f]
origin_created_at: unknown
imported_at: 2026-11-09
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca
parent_ids: []
related_ids: [frost-research-template-landscape, frost-research-template-deep-dive, frost-research-template-subtopic-handoff, frost-research-template-review-checklist]
supersedes: []
open_questions: []
---

# Frost research prompt (master template)

Use this prompt to launch one research task in one of three modes. It is
self-contained: a researcher needs this file, the repository checkout named in
the parameters, and the corpus index at `docs/research/README.md`. Nothing
here depends on a chat transcript. The `input_sha256` above is the hash of the
handoff archive that supplied the template starter, not of this adapted file;
`input_members` names the member inside that archive.

## Parameters

Fill every value. `UNKNOWN` is an acceptable value; a silently invented value
is not. The template supplies no project facts; the researcher records them
from the checkout.

```yaml
mode: landscape            # landscape | deep-dive | validation
research_id: REPLACE       # stable id, e.g. frost-research-<topic>-<yyyymmdd>
question_id: REPLACE       # Q-… id from the parent synthesis, or NEW
question: REPLACE          # one answerable question
engineering_decision: REPLACE   # the decision this research must unlock
audience: REPLACE          # maintainer, API implementer, engine integrator, …
repository: REPLACE        # owner/name or path
repo_revision: INSPECT_AND_RECORD   # full commit hash of the inspected checkout
toolchain: INSPECT_AND_RECORD       # from rust-toolchain.toml and Cargo rust-version
package_graph: INSPECT_AND_RECORD   # actual packages/features at that revision
inspected_implementation: []        # source, test, and doc paths actually read
parent_report: NONE_OR_REAL_PATH_AND_ID
parent_sections_and_claims: []      # claim ids inherited; each must be revalidated
scope: []
non_goals: []
constraints: []            # semantic contracts that must survive, feature rules, no_std, …
source_cutoff: REPLACE     # latest publication date the researcher may rely on
execution_date: ACTUAL_DATE
candidate_designs: []      # include the status quo
workload_model: REPLACE_OR_UNKNOWN
resource_limits: REPLACE_OR_UNKNOWN   # memory, time, I/O, cancellation
deliverables: []           # report path(s); each must be a real repository-relative path
output_path: REAL_REPOSITORY_RELATIVE_PATH
stop_criteria: []          # acceptance or stop conditions
```

## Role and goal

Act as a Rust systems researcher and algebra/verification engineer. Answer the
question so the maintainer can make the engineering decision. Produce the
report file named in `output_path`, not a plan, a library list, or a chat
summary. Read the repository's current `AGENTS.md`, the implementation contracts it
links (`docs/maintainers/implementation-contracts.md`), `TOOLS.md`, design
documents, relevant code, tests, and the parent research before writing.
State what you could not inspect.

## Project context to verify against the checkout

Do not copy these statements into the report; verify them at `repo_revision`
and record what you found.

- The implemented library is one Cargo package whose Rust library name is
  `frost`, and current project policy keeps one package. Record the actual
  package name, version, features, and toolchain. A workspace split into
  several crates (`crates/frost-*` with a `frost-api` facade) appears in the
  supplied research and its handoff as an unadopted proposal; it is not an
  established maintainer decision and it is not implemented. Treat any such
  layout as a proposal and never infer an implemented API from a research
  sketch.
- Semantic contracts that must survive any proposal: arbitrary lawful element
  and key types without a universal `Clone + Default + Hash + Ord` bound;
  operation-specific bounds; the first-equivalent-key/last-value and
  left-representative rules; fixed mutable/frozen family pairs; structural
  (not deep) immutability; borrowed, decoded, and lending traversal kept
  distinct; features additive.
- Verification uses the root Makefile. Record the commands that exist at the
  revision; do not copy historical scripts, an unpinned nightly, or commands for
  packages that do not exist.

## Evidence and reasoning requirements

Use primary sources: papers, specifications, and versioned official
documentation or source. Record URL or DOI, version or revision, access date,
the section inspected, and what was checked. Register each source in the
corpus source catalog (`docs/research/SOURCES.md`) with a stable
`SRC-…` id, or in a report-local reference list with the same fields.

Classify every important claim with one of: **implemented and inspected**,
**documented upstream**, **derived reasoning**, **proposal**, **experiment
planned**, **experiment executed**, **historical result**, **unverified**.
A derivation states its assumptions and shows enough reasoning to audit. A
measurement states command, source revision, environment, workload, and
retained output. Never manufacture measurements, citations, proofs, or the
contents of files you could not read. Revalidate inherited parent claims
instead of copying them.

Specify logical domains, equivalence relations, operations, laws,
counterexamples, representation invariants, ownership, order, errors,
overflow, callback effects, and snapshot assumptions. Separate membership
equality from observable-result equality. `Fn` bounds do not establish purity.
Laziness does not imply bounded work or memory. Structural freezing does not
imply global consistency, persistence, durability, or freshness.

## Output by mode

**Landscape.** Use `landscape-report.md`. Required: executive decision map,
taxonomy and formal vocabulary, versioned capability matrix, research gaps,
decision dependencies, ranked subtopics, and ready-to-run child briefs made
with `subtopic-handoff.md`.

**Deep dive.** Use `deep-dive-report.md`. Required: formal definitions and
laws; counterexamples; preconditions for each rewrite; API and ownership
contracts with explicit compile status; concrete algorithms and data layouts;
CPU, allocation, memory, and I/O costs with every variable defined; failure
modes; alternatives; verification plan; reproducible experiments; evidence
limits; an implementation-ready recommendation.

**Validation.** Use the deep-dive template's section headings, replacing the
formal-model and physical-plan sections with the following, in order:

1. Falsifiable claim and its preconditions, quoted from the parent.
2. Independent oracle or control, simple enough to be obviously correct.
3. Reproducible setup: revision, toolchain, features, commands, inputs, seeds.
4. Executed versus not executed, as two separate lists.
5. Counterexamples found, with inputs.
6. Actual results, including slower or negative observations.
7. Limits of the evidence and the proposed disposition of the claim.

A validation report may conclude that the recommendation is unsupported. An
unexecuted experiment is a plan, not a result.

Use executive-to-detail layering: decision first, then analysis, then
appendices with tables, proofs, and log pointers. As orientation, a landscape
is often 2,000 to 4,000 words and a deep dive 4,000 to 8,000 words plus
appendices. Stop when the decision is answerable; do not pad or truncate.

## Required conclusion

State the recommendation, the conditions under which it changes, the
unresolved questions, and which implementation or measurement would separate
the remaining alternatives. Produce child handoffs where depth was
insufficient and link each back to its parent question id. Add the new report
to `docs/research/manifest.json` and the index only within your authorized
edit scope. Report commit or publication status only after verifying it.

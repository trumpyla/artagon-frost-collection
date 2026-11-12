# Frost documentation

Start with the [crate README](../README.md) for the rationale, semantic laws,
API examples, and features. This index lists every maintained document by
category. Status labels: **current** describes implemented behavior;
**proposal** describes design with no production API; **research** is curated
input to design, not accepted design; **historical** is dated evidence that
does not validate later changes.

## Current design and contracts

| Document | Purpose | Status |
| --- | --- | --- |
| [Design philosophy](design/philosophy.md) | Durable principles and review criteria | Current |
| [Architecture](design/architecture.md) | Families, capabilities, profiles, and backend boundaries | Current |
| [Frozen API guide](design/frozen-api.md) | Frozen value kinds, plans, recovery, access models, costs, snapshot admission | Current; proposals marked |
| [Design decisions](design/decisions.md) | Why the architecture was selected, and later integration decisions | Current |
| [Set operations](set-operations.md) | Algebra, representatives, complexity, and bitmap universes | Current |
| [Snapshot publication](design/snapshots.md) | Ownership, identity, retries, and linearization | Current |
| [Typed integer profile](design/integer-sets.md) | Canonical key codecs, private storage, policies, and encoded algebra | Current |
| [Crate standard](../CRATES.md) | Package layout, dependency placement, feature/target matrix, API obligations, coverage accounting, test and benchmark metadata | Current standard |
| [Crate layout](design/crate-layout.md) | Current package, the `crates/frost-*` reorganization target, split constraints, and required decisions | Current and proposal |

## Proposals

| Document | Purpose | Status |
| --- | --- | --- |
| [General set algebra](design/set-algebra.md) | Typed views for all set categories, representative semantics, execution, and ownership | Proposal |
| [Collection algebra](design/collection-algebra.md) | Maps, sequences, bags, multimaps, bimaps, and prefix indexes; [naming dispositions](design/collection-algebra.md#naming-dispositions) and corrections | Proposal |
| [Borrowed integer expressions](design/integer-views.md) | IntSetExpr API, bounds, cursors, evaluation, and admission tests | Proposal |
| [Persistent integer storage](design/persistent-integer-storage.md) | Implemented flat shared chunks distinguished from proposed tree directories and a separate family | Current and proposal |
| [Integer roadmap](design/integer-roadmap.md) | Future families and their acceptance criteria | Proposal |
| [Crate split proposal](design/proposals/crate-split.md) | Measured module graph, stage-1 packages, feature map, and proposed changes to epic `frost-wlt` | Proposal |

## Formal models and domain-specific algebra

These are proposed contracts, not new production APIs. Read the abstract model
before specializing it to a collection. Existing names and current standards
remain authoritative for implemented behavior.

| Document | Purpose |
| --- | --- |
| [Formal-algebra index](design/algebra/README.md) | Reading order, authority and scope |
| [Abstract model](design/algebra/formal-model.md) | Denotations, representation invariants, partial operations and observable refinement |
| [Value domains](design/algebra/domains.md) | Admissibility, key equivalence, encodings, conversions and contexts |
| [Collection models](design/algebra/collection-models.md) | Set, sequence/list, map, bag, multimap and BiMap chapters |
| [Frozen refinement](design/algebra/frozen.md) | Structural freezing, materialization, persistence and snapshots |
| [Computed sets](design/algebra/execution-and-computed-sets.md) | Incomplete knowledge, streams, revisions, deltas and fixed points |
| [Rewrite contracts](design/algebra/rewrite-contracts.md) | Preconditions for reassociation, projection and terminal-specific execution |
| [Rust integration](design/algebra/rust-integration.md), [terminology](design/algebra/terminology.md) | Existing capability names, selected proposals and future placement |
| [Proof obligations](design/algebra/proof-obligations.md) | Arguments, finite checks and unexecuted implementation gates |
| [Adversarial review](reviews/value-domain-algebra-review.md) | Same-author review passes, counterexamples and remaining risks |
| [Destination review](reviews/value-domain-algebra-destination-review.md) | Independent review at integration, corrections, gate results and open items |
| [Formal-model research supplement](research/formal-models/README.md) | Input inventory, sources, replayable evidence and follow-up prompt |

## Research corpus

| Document | Purpose | Status |
| --- | --- | --- |
| [Research index](research/README.md) | Corpus layout, reading paths, and artifact status | Research |
| [Research manifest](research/manifest.json) | Machine-readable inventory, hashes, and relationships | Research |
| [Source catalog](research/SOURCES.md) | Primary sources, what each was checked for, and limits | Research evidence |
| [Citation map](research/citation-map.json) | Original citation markers per input and their recovery status | Research evidence |
| [Integration audit](research/integration-audit.md) | Claim and decision matrix, corrections, missing artifacts, checks | Review |
| [Collection algebra survey](research/reports/collection-algebra-survey.md) | Curated survey derived from Frost1 | Research |
| [Collection algebra implementation notes](research/reports/collection-algebra-implementation-notes.md) | Curated implementation notes derived from Frost2, with corrections | Research |
| [Computed sets summary](research/reports/computed-sets-summary.md) | Curated summary of the computed-sets delivery note; the full report is missing | Research |
| [Foundations and engine roadmap](research/synthesis/foundations-and-engine-roadmap.md) | Decision map, capability matrix, and prioritized questions | Research synthesis |

Templates for new research:
[research prompt](research/templates/research-prompt.md),
[landscape report](research/templates/landscape-report.md),
[deep-dive report](research/templates/deep-dive-report.md),
[subtopic handoff](research/templates/subtopic-handoff.md), and
[review checklist](research/templates/review-checklist.md).

Focused prompts:
[algebraic foundations](research/prompts/algebraic-foundations.md),
[computed sets and durable execution](research/prompts/computed-sets-and-durable-execution.md),
[Frozen API contracts](research/prompts/frozen-api-contracts.md), and
[authorization engine integration](research/prompts/authorization-engine-integration.md).

## Reviews and audits

| Document | Purpose | Status |
| --- | --- | --- |
| [Integer adversarial review](reviews/integer-design-audit.md) | Design risks, corrections, and scope decisions | Review |
| [Integer output review](reviews/integer-output-review.md) | Disposition of the supplied follow-up findings and operation contracts | Historical |
| [Revision 2 integration review](reviews/revision2-integration-review.md) | Selective merge, corrected regressions, sharing contracts, and retained main APIs | Review of current merge |
| [Lazy views research review](reviews/lazy-views-adversarial-review.md) | Adversarial findings, corrected source claims, and design decisions | Review |
| [Search-source audit](reviews/lazy-views-source-audit.md) | Versioned Rust/FST/Lucene/Tantivy contracts and checked issue/PR statuses | Review |
| [Views performance audit](reviews/lazy-views-performance-audit.md) | Paper evidence, experiment limits, and future benchmark design | Review |
| [Generic algebra review](reviews/generic-set-algebra-review.md) | Coverage gaps, research iteration, baseline checks, and finite semantic evidence | Review |
| [Integration review](reviews/incoming-assessment.md) | Disposition of the attached patch and review claims | Review |
| [Previous performance review](reviews/performance-review.md) | Earlier kernel improvements and raw evidence | Historical |
| [Supplied inputs](reviews/supplied/README.md) | Unmodified supplied reviews, patches, research reports, summary, and handoff archive | Supplied evidence |

## Validation records

Each record describes checks run on a stated revision. None validates a later
change.

| Document | Purpose | Status |
| --- | --- | --- |
| [Standards implementation evidence](../verification/crate-standards-validation/README.md) | Same-scope coverage before and after the standards change, local gate results, and adoption limits | Dated record |
| [Integration validation](revision2-integration-validation.md) | Commands and results for the merged source before GitHub import, with remaining evidence limits | Dated record |
| [GitHub import and Make checks](../verification/revision2-integration/github-import-ci/README.md) | Later PR checkout and build-tool adaptation, recorded separately from original integration evidence | Dated record |
| [Attachment integration](../verification/attachment-integration-2026-11-09/README.md) | Input digests, conflict resolutions, and checks run on that checkout | Dated record |
| [Expression bounds model](../verification/integer-views/README.md) | Executed finite mathematics checks; no production views implementation | Dated record |
| [Generic algebra model](../verification/set-algebra/README.md) | Reproduced representative/order counterexamples; no production view implementation | Dated record |
| [Prior output validation](integer-output-validation.md) | Output-policy, feature, concurrency, and verification gates | Historical |
| [Prior integer benchmarks](integer-output-benchmarks.md) | Same-harness comparisons and CPU target controls; not merged-source measurements | Historical |
| [Initial integer validation](integer-validation.md) | Evidence before output-policy corrections | Historical |
| [Initial integer benchmarks](integer-benchmarks.md) | Representation tradeoffs and mixed-probe improvement | Historical |
| [Prior integration validation](production-readiness.md) | Checks before the integer profile was added | Historical |

## Maintenance

| Document | Purpose | Status |
| --- | --- | --- |
| [Maintainer guide](maintainers/guide.md) | Code map, invariants, and change-specific checks | Current |
| [Benchmarks](maintainers/benchmarks.md) | Fair measurement and reproduction | Current |
| [Verification](verification.md) | Executable models, proof bounds, and limitations | Current |
| [Testing standard](maintainers/testing.md) | Semantic obligations, coverage denominators, mutation triage, seeds, and gate tiers | Current |
| [Release procedure](maintainers/releasing.md) | Concrete gates before publishing | Current |
| [Crate standard](../CRATES.md) | Layout, dependency, API obligation, coverage, and metadata rules | Current |
| [Verification catalog](../verification/catalog/README.md) | Catalog and benchmark-inventory schemas, structural discovery, and its limits | Current |
| [Coverage policy](../verification/coverage/README.md), [mutation policy](../verification/mutation/README.md), [fuzz harness](../verification/fuzz/README.md) | Denominators and gates, survivor triage, and isolated fuzz targets | Current |
| [Rust verification strategy](research/reports/rust-verification-strategy.md) | Source-backed rationale for the tool choices | Research |
| [GitHub workflow](maintainers/github-workflow.md) | Issue forms and pull requests as intake; tracker records as the work plan | Current |
| [Contributor guide](../CONTRIBUTING.md) | How to make and verify changes | Current |

Planned work is tracked with the Beads issue tracker: the standards epic
`frost-wou` (remaining adoption tasks `frost-wou.6`–`.10`) and the
reorganization epic `frost-wlt` (`frost-wlt.1`–`.6`), which depends on it. Its
state lives under `.beads/`. The configuration and JSONL export are the
portable records; the SQLite database files there are local and ignored by Git.
Open research questions remain in the research synthesis and integration audit.

## Reading paths

- **New contributor:** crate README, Frozen API guide, `examples/`, design
  philosophy, contributor guide, crate standard.
- **Test, benchmark, or verification author:** crate standard, testing
  standard, verification catalog, verification, benchmarks, maintainer guide.
- **Frozen API implementer:** architecture, Frozen API guide, the linked source
  and tests, snapshot publication, verification.
- **Algebra or planner implementer:** set operations, collection algebra,
  general set algebra, generic algebra review and model, research synthesis.
- **Authorization-engine integrator:** the README's engine sections, snapshot
  publication, computed sets summary, research synthesis, authorization prompt.
- **Researcher starting a subtopic:** research index, synthesis, source catalog,
  research prompt template, subtopic handoff template, integration audit.

Historical review and verification documents preserve earlier findings. The
inputs under `reviews/supplied/` are unmodified evidence; their claims are not
automatically current project policy.

## Compact maintenance entry points

[Agent entry point](../AGENTS.md), [implementation contracts](maintainers/implementation-contracts.md),
[repository rules](maintainers/repository-rules.md),
[documentation rules](maintainers/documentation-rules.md), and the
[rules registry](maintainers/rules.json) refer to the existing authoritative
[crate standard](../CRATES.md). They do not replace its layout, test catalog,
coverage accounting, benchmark, or migration requirements.

---
id: frost-research-prompt-computed-sets-and-durable-execution
kind: prompt
level: deep-dive
research_status: curated
implementation_status: not-applicable
source_validation: not-applicable
input_files: []
input_sha256: []
origin_created_at: 2026-11-09
imported_at: 2026-11-09
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca
parent_ids: [frost-research-synthesis-foundations-and-engine-roadmap]
related_ids: [frost-research-template-subtopic-handoff, frost-research-report-computed-sets-summary]
supersedes: []
open_questions: [Q-CSET-1, Q-NARY-1]
---

# Child prompt: computed sets over durable and indexed relations

A ready-to-run handoff produced with the
[subtopic handoff template](../templates/subtopic-handoff.md). New synthesis
dated 2026-11-09. The full computed-set report that the supplied summary
describes was not recovered; this prompt asks for the research that would
replace it, not for a reconstruction of it.

```yaml
question_id: Q-CSET-1
parent_report_id: frost-research-synthesis-foundations-and-engine-roadmap
parent_path: docs/research/synthesis/foundations-and-engine-roadmap.md
parent_revision: UNRESOLVED_AT_HANDOFF   # the parent synthesis is new work and does not exist at the code baseline; resolve the commit (or record the parent's content digest) before research starts
parent_content_sha256: UNRESOLVED_AT_HANDOFF
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca   # inspected production code only
parent_sections_or_claims: [CL-CSET-1, CL-CSET-2, CL-CSET-3, CL-NARY-1, CL-SNAP-1]
child_report_id: frost-research-report-computed-sets
mode: deep-dive
output_path: docs/research/reports/computed-sets.md
inherited_assumptions:
  - "Zanzibar composes usersets with union, intersection, and exclusion (section 2.3.1), evaluates checks at a consistent snapshot no earlier than a client token (sections 2.2 and 2.4.4), cancels subtrees (3.2.3), and Leopard merges a periodic base index with timestamped updates (3.2.4): confirmed by the other lane (Opus) against the PDF, pages 2-8 (SRC-ZANZIBAR-2019); confirm the page numbers in your own read"
  - "fst 0.4.7 n-ary stream operations are heap-based: confirmed by source inspection (SRC-FST-0_4_7-RAW-OPS)"
  - "Lazy terminals avoid result containers but not scans, buffering, deduplication, or recursion: derived (docs/design/set-algebra.md sections 7 and 8; lazy-views source audit)"
  - "Differential dataflow is a suitable later incremental engine: unverified (summary claim; source not audited)"
  - "OpenFGA and SpiceDB confirm computed-set semantics: SpiceDB schema operators confirmed (SRC-SPICEDB-SCHEMA); OpenFGA not audited"
owning_module_or_crate: "docs/design proposals; src/integer (encoded kernels), src/concurrent (snapshot boundary); one package"
```

## Decision to unlock

Whether Frost should specify a computed-set evaluation model (expressions over
sources at a defined revision with terminal-specific execution), and if so,
which obligations it must carry for revision compatibility, overlays, external
memory, budgets, and cancellation. The parent can state the distinctions; it
cannot determine whether the obligations can be met by borrowed typed views
plus the existing snapshot store, or need a separate owned plan arena,
durable cursors, and a revisioned index contract.

## Known evidence and assumptions to revalidate

Confirmed:

- Frost's implemented set algebra is lazy and borrowed for two frozen
  families, with scalar count kernels and bitmap results; it has no
  expression layer, planner, or durable cursor (`SRC-FROST-DOC-SET-OPERATIONS`).
- The snapshot store publishes whole frozen versions; load once and query that
  version for structural consistency (`SRC-FROST-SRC-SNAPSHOT`).
- `fst` 0.4.7 union/intersection/difference/symmetric difference pop and
  refill a `BinaryHeap` of per-stream slots, each with its own key buffer.
  The documented statement is `O(n1 + n2 + …)` over per-stream element
  counts with stream-proportional memory; reading it as omitting the heap,
  key-length, and stream-decoding factors is this corpus's interpretation,
  not upstream wording (`SRC-FST-0_4_7-RAW-OPS`,
  `SRC-FST-0_4_7-OPBUILDER-DOCS`; derivation in the
  [integration audit](../integration-audit.md)).

Working assumptions:

- Membership, existence, bounded enumeration, exact or estimated count, and
  materialization justify different physical plans. Supported by the general
  set-algebra design's terminal table; not measured.
- A base-plus-delta overlay is the practical incremental route before any
  incremental-dataflow engine. From the summary; not derived here.

## Scope and exclusions

In scope: the evaluation model, revision and snapshot compatibility
(index revision, durable snapshot, contextual facts, and cache must agree),
overlays, external-memory plans, resource budgets, cancellation, and the
n-ary cost model (`Q-NARY-1`, see below). Out of scope: policy semantics
(`Q-AUTHZ-1`), new storage engines, and any claim of implemented performance.

### Embedded question Q-NARY-1

Produce the cost model for n-ary sorted operations with all variables: stream
count `k`, per-stream sizes, total elements `N`, key length `L`, output size,
heap or frontier state, per-stream buffers, decoding and I/O per advance, and
cancellation. State separately the conservative heap-merge bound
`O(k + N · (L + 1) · log(k + 1))` (which covers empty streams, a single
stream, and empty keys) plus per-advance decoding and I/O, the
`O(k · (L + 1))` byte merge workspace excluding stream internals, the bound
for a frontier merge, and when a candidate-first probe plan beats both.
Treat the heap derivation as source-based reasoning, never as measured
performance. Do not transfer `fst`'s documented bound to generic Frost
algorithms.

## Competing hypotheses or designs

1. Status quo plus snapshots: applications compose the existing eager or
   lazy family operations over loaded snapshot versions. Strongest reason: no
   new API. Failure: no expression over mixed sources, no durable cursors.
2. Typed borrowed views (general set-algebra proposal) with terminal-specific
   execution and explicit materialization. Strongest reason: consistent with
   current contracts and the finite oracle. Failure: runtime-shaped queries
   and durable sources need a separate arena and cursor contract.
3. Revisioned source contract plus owned plan arena with durable cursors,
   overlays, and budgets. Strongest reason: matches the Zanzibar/Leopard
   requirements. Failure: a large contract surface before any consumer exists;
   risk of promising freshness that only the engine can guarantee.

## Questions to answer

- The formal definition of a computed set: expression, source set, revision,
  and terminal; what "evaluated at revision r" means for each source kind.
- Which terminals may short-circuit, what partial results are legal under
  cancellation, and how budgets are reported (work, memory, I/O).
- Overlay semantics: base snapshot plus ordered delta; tombstones; how a
  delta merged "no later than the query timestamp" is specified.
- External-memory plans: when spill is required and how its cost is exposed.
- Compatibility rule: index revision, durable snapshot, contextual facts, and
  cache must agree; what the API rejects when they do not.
- Representative and explanation obligations under each plan.

## Required evidence and experiments

- Primary sources: the Zanzibar paper sections above; the `fst` 0.4.7 source;
  Rust 1.99.1 `BTreeSet` operation iterators; SpiceDB schema documentation
  with its access date; OpenFGA and Differential Dataflow only if read and
  registered.
- A finite model of overlays and revisions (small domains, explicit
  timestamps) with an independent oracle.
- Work and allocation counters for the candidate plans on synthetic
  workloads; no timing assertions in correctness tests; mark unrun as PLANNED.

## Acceptance criteria

The maintainer can decide between hypotheses 2 and 3 (or a staged path) with
a written compatibility rule, a terminal table with legal short-circuits, and
a cost model whose variables are all defined. Unmet obligations are listed.

## Deliverables and feedback

The report; the `Q-NARY-1` cost model as a section; an update to the parent's
decision matrix rows CL-CSET-1 to CL-CSET-3 and CL-NARY-1; and child briefs
for durable cursor contracts if the report cannot settle them.

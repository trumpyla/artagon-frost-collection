---
id: frost-research-prompt-frozen-api-contracts
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
related_ids: [frost-research-template-subtopic-handoff]
supersedes: []
open_questions: [Q-FROZEN-1]
---

# Child prompt: Frozen API contracts

A ready-to-run handoff produced with the
[subtopic handoff template](../templates/subtopic-handoff.md). New synthesis
dated 2026-11-09. The editorial guide to the *current* contracts is
[docs/design/frozen-api.md](../../../crates/frost-collections-core/docs/frozen-api.md); this prompt asks for
the research that decides what the next contracts should be.

```yaml
question_id: Q-FROZEN-1
parent_report_id: frost-research-synthesis-foundations-and-engine-roadmap
parent_path: docs/research/synthesis/foundations-and-engine-roadmap.md
parent_revision: UNRESOLVED_AT_HANDOFF   # the parent synthesis is new work and does not exist at the code baseline; resolve the commit (or record the parent's content digest) before research starts
parent_content_sha256: UNRESOLVED_AT_HANDOFF
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca   # inspected production code only
parent_sections_or_claims: [CL-FROZEN-1, CL-FROZEN-2, CL-FROZEN-3, CL-SNAP-1, CL-INT-1]
child_report_id: frost-research-report-frozen-api-contracts
mode: deep-dive
output_path: docs/research/reports/frozen-api-contracts.md
inherited_assumptions:
  - "Exact-backend Frozen<B> is sealed by Eligible; there is no universal thaw; into_inner consumes: confirmed (src/frozen.rs)"
  - "Retain preserves the backend without rebuilding; Compact performs the backend's documented conversion: confirmed (src/plan.rs, docs/design/architecture.md)"
  - "Borrowed Entries/Elements, decoded LookupValue/ValueElements, and lending FST streams are distinct access models: confirmed (src/read.rs, src/backends/fst.rs)"
  - "Snapshot publication: sealed SnapshotValue, unconditional publish, identity CAS, retrying update: confirmed (src/concurrent/snapshot.rs)"
  - "A frozen value is structurally immutable only: confirmed (README compile-fail doctests, philosophy)"
  - "Research sketches of FrozenBiMap::try_insert(&mut self) and a universal MapGet<Q> are callable: stale; rejected in the integration audit"
owning_module_or_crate: "src/frozen.rs, src/plan.rs, src/read.rs, src/family, src/concurrent; one package"
```

## Decision to unlock

What contract a new frozen family or exact-backend wrapper must satisfy for
(a) construction and validation, (b) ownership and recovery, (c) borrowed,
by-value, and lending traversal, and (d) snapshot admission, so that bag,
multimap, bimap, map-algebra, or prefix-index proposals can be judged against
one written standard instead of against sketches. The parent can list the
current rules; it cannot decide whether a builder-then-freeze model, a
consume-and-produce model, or both should be required for new families, nor
what a fallible plan must return.

## Known evidence and assumptions to revalidate

Confirmed at the parent revision (verify again at yours):

- `Frozen<B>` exposes no `Deref<Target = B>` and no backend accessor;
  `into_inner` consumes; `Eligible` is sealed (`SRC-FROST-SRC-FROZEN`).
- `FreezeWith<P>`/`TryFreezeWith<P>` with `Retain`, `Compact`, `Sorted`,
  `ExactLen<N>`; `Rejected` returns the original input on validation failure
  for the plans that validate before commitment; `DuplicateKey` returns the
  rejected item, not the whole input (`SRC-FROST-SRC-PLAN`,
  `SRC-FROST-DOC-ARCHITECTURE` section 2 and 3).
- Family pairs: `freeze()` and `thaw()` are fixed named partners; adaptive
  profiles have explicit `thaw_with`/`from_frozen` alternatives.
- `SnapshotValue` is sealed to frozen payloads; `publish` is unconditional;
  `publish_if_current` compares `Arc` allocation identity and returns the
  rejected value plus an observed version that may already be superseded;
  `update` may rerun its callback and returns the displaced version
  (`SRC-FROST-SRC-SNAPSHOT`, `SRC-FROST-DOC-SNAPSHOTS`).
- `FrozenIntSet<K>` stores canonical codes, decodes by value, uses absolute
  Dense coordinates, selects Chunked only explicitly, and shares storage through
  a private `Arc`/`Rc` alias (`SRC-FROST-DOC-INTEGER-SETS`).

Working assumptions from the inputs, to be tested rather than inherited:

- New families should follow "mutable builder, then freeze" rather than
  "frozen value with `&mut self` methods" (input sketch rejected; see the
  [audit](../integration-audit.md) hazard H2).
- Consuming algebra (`into_union`, node moves) can reuse storage for some
  backends. Not demonstrated for any Frost backend.

## Scope and exclusions

In scope: the written contract for frozen construction, validation,
ownership, recovery, traversal modes, error recovery types, allocation cost
disclosure, feature boundaries, auto-trait implications, and snapshot
admission, for existing and proposed families. Out of scope: new algebra
laws (`Q-ALG-1`), computed-set execution (`Q-CSET-1`), and any workspace
split.

## Competing hypotheses or designs

1. Status quo: each family documents its own contract; proposals are judged
   case by case. Strongest reason: nothing to build. Failure: inconsistent
   proposals such as the input's mutable method on a frozen type.
2. Written frozen-family standard: a checklist every new family must satisfy
   (fixed partner, representative rule, validation order, error ownership,
   traversal mode, snapshot eligibility statement, cost table). Strongest
   reason: the integration audit found four hazards a standard would have
   caught. Failure: a standard that over-constrains specialized backends.
3. Trait-encoded standard: express parts of the standard as sealed marker
   traits (like `Eligible` and `SnapshotValue`). Strongest reason: compiler
   enforcement. Failure: GAT and dyn-compatibility limits; feature coupling.

## Questions to answer

- Which plans must be fallible, and what exactly each error type returns
  (rejected item, whole input, or nothing). Not every error contains the
  original input today; decide whether that must become uniform.
- Whether a new family may ever expose `&mut self` structural mutation while
  frozen (expected: never) and how builders relate to plans.
- For each traversal mode (borrowed, decoded by value, lending), which
  iterator promises (`ExactSize`, `DoubleEnded`, `Fused`, `Clone`) a family
  may make and how partial iteration and size hints are specified.
- Snapshot admission rules for any new frozen type, including `PhantomData`
  auto-trait behaviour as in `FrozenIntSet`.
- Allocation and reference-count costs that must be disclosed (empty-value
  allocation, directory copying, hasher cloning on promotion).
- Which rules a `no_std`/`alloc`-only build must keep.

## Required evidence and experiments

- Inspect `src/frozen.rs`, `src/plan.rs`, `src/read.rs`, `src/write.rs`,
  `src/family/`, `src/adaptive/`, `src/integer/`, `src/concurrent/snapshot.rs`
  and the tests named in the Frozen API guide; quote signatures.
- Compile-check any proposed signature on Rust 1.99.1 in a scratch crate;
  record the command. Pseudocode must be labelled.
- Differential tests against standard collections for any proposed validation
  order; drop-count tests for any proposed consuming operation.

## Acceptance criteria

A one-page standard that a reviewer can apply to the bag, multimap, bimap,
and prefix-index proposals, with each rule traced to current source or to a
stated decision. Every rule that is new, not current, is marked as such.

## Deliverables and feedback

The report; a proposed revision to `docs/design/frozen-api.md` (as a diff
proposal, not an edit); an update to the parent's capability matrix; and the
list of proposals it would reject as written.

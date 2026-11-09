---
id: frost-research-prompt-algebraic-foundations
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
related_ids: [frost-research-template-subtopic-handoff, frost-research-report-collection-algebra-survey, frost-research-report-collection-algebra-implementation-notes]
supersedes: []
open_questions: [Q-ALG-1]
---

# Child prompt: algebraic foundations and observationally valid rewrites

This is a ready-to-run handoff produced with the
[subtopic handoff template](../templates/subtopic-handoff.md). It is new
synthesis dated 2026-11-09. It does not recover the missing
algebraic-foundations deep dive described in the
[integration audit](../integration-audit.md); it specifies how to produce one.

```yaml
question_id: Q-ALG-1
parent_report_id: frost-research-synthesis-foundations-and-engine-roadmap
parent_path: docs/research/synthesis/foundations-and-engine-roadmap.md
parent_revision: UNRESOLVED_AT_HANDOFF   # the parent synthesis is new work and does not exist at the code baseline; resolve the commit (or record the parent's content digest) before research starts
parent_content_sha256: UNRESOLVED_AT_HANDOFF
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca   # inspected production code only
parent_sections_or_claims: [CL-ALG-1, CL-ALG-2, CL-ALG-3, CL-REP-1, CL-REP-2]
child_report_id: frost-research-report-algebraic-foundations
mode: deep-dive
output_path: docs/research/reports/algebraic-foundations.md
inherited_assumptions:
  - "Membership-equivalent rewrites do not preserve representatives or order: confirmed (finite oracle, docs/design/set-algebra.md section 3)"
  - "Bag sum and max-union are distinct operations with different laws: working (input reports; literature not yet audited)"
  - "Ordinary bimap union is not closed over partial bijections: confirmed by construction (counterexample in the survey)"
  - "Relational/semiring provenance is relevant to explanations: working (no source audited yet)"
owning_module_or_crate: "docs/design (contracts); src/family and a future src/set or src/map module for any later implementation; one package"
```

## Decision to unlock

Which rewrite rules an eventual Frost planner may apply, per logical domain
and per terminal, without changing an observable result. The parent synthesis
can state that membership, representative, order, multiplicity, failure, and
callback effects are separate observations; it cannot settle which identities
hold for maps with resolvers, bags under each union law, sequences, relations,
and partial bijections, nor which provenance structure (if any) is needed for
explanations. A deep dive must produce the law tables and counterexamples.

## Known evidence and assumptions to revalidate

Confirmed:

- Frost's implemented set contracts: first equivalent key or element, last map
  value, left representative for union and intersection, no mutable keys
  (`SRC-FROST-SRC-READ`, `SRC-FROST-DOC-SET-OPERATIONS`,
  `SRC-FROST-DOC-PHILOSOPHY` in [SOURCES.md](../SOURCES.md)).
- Rust's `HashSet::intersection` may yield the representative from one set or
  the other; Frost's left rule is stronger (`SRC-RUST-HASHSET-INTERSECTION`).
- Rust's `HashMap::get_key_value` returns the stored key even when the lookup
  key is equal but not identical (`SRC-RUST-HASHMAP-GET-KEY-VALUE`).
- The finite semantics oracle demonstrates XOR reassociation and nonlocal
  cancellation changing representatives (`SRC-FROST-VERIFICATION-SET-ALGEBRA`).

Working assumptions:

- The supplied reports' bag operation table (sum, max-union, min-intersection,
  saturating difference, absolute difference) is the right canonical set of
  bag operations. The literature they cite (Albert 1991; Hernich and Kolaitis
  2017) was supplied as a bibliography and has not been read in this
  integration (`SRC-INPUT-ALBERT-1991`, `SRC-INPUT-HERNICH-KOLAITIS-2017`).
- Partial-bijection composition is a more useful bimap operation than forced
  set operators. Not independently derived.

Stale or unverifiable: every opaque export marker in the raw inputs; see the
[citation map](../citation-map.json).

## Scope and exclusions

In scope: sets, lattices of sets under union/intersection, semiring or
provenance annotations where they inform explanations, finite partial maps
with explicit resolvers, bags under both union laws, finite sequences,
relations (set-, bag-, and sequence-valued multimaps), and partial bijections.
Define each domain, its equivalence relation, its operations, its laws, and
the observations an API exposes beyond membership.

Out of scope: physical kernels, benchmark results, a new crate layout, the
computed-set execution model (`Q-CSET-1`), and authorization policy
semantics (`Q-AUTHZ-1`). Do not define a universal collection trait.

## Competing hypotheses or designs

1. Status quo: binary operations with fixed representative rules, no planner
   rewrites beyond what current kernels do. Strongest reason: already tested.
   Failure: n-ary and mixed expressions pay repeated probes.
2. Terminal-split laws: aggressive rewrites for membership and count terminals;
   provenance-preserving plans for reference-producing terminals. Strongest
   reason: matches the oracle evidence. Failure: a rewrite table that is
   subtly incomplete for difference and XOR.
3. Provenance-annotated algebra (semiring style): carry which source
   contributed each member so explanations and representatives follow from
   the annotation. Strongest reason: explanations for authorization engines.
   Failure: annotation cost on hot paths; annotation does not define Rust
   ownership of the stored object.

## Questions to answer

- For each domain: equality relation, representative rule, output order,
  multiplicity, overflow or failure, and callback effects, as one table.
- Which of associativity, commutativity, idempotence, distributivity, and
  absorption hold for membership only, for representatives, and for order.
- For maps: resolver call count and order, whether a resolver may be
  reinvoked under retry, and which join variants retain both values.
- For bags: which identities hold under sum versus max-union; overflow of
  additive sum and the exact error contract; saturation in difference.
- For bimaps: closure properties of intersection, difference, composition;
  the conflict classes a checked merge must report.
- For sequences: which operations are order-preserving and why set vocabulary
  is excluded.
- Whether `Fn` callbacks can be treated as pure for any rewrite (expected: no).

## Required evidence and experiments

- Primary sources: Rust 1.99.1 std collection documentation; the two
  bag-semantics papers if obtainable; Codd's relational model paper if a
  relational claim is made. Register each with a `SRC-…` id.
- An independent finite oracle per domain, extending
  `verification/set-algebra/set_semantics_oracle.py` in approach (not in
  code sharing), covering labelled representatives, resolver call counts, and
  bag multiplicities over small domains.
- Counterexamples for each rejected identity, with inputs.

## Acceptance criteria

A maintainer can read one table per domain and know which rewrites a planner
may apply for which terminal. Each allowed rewrite cites its precondition and
oracle evidence; each forbidden rewrite cites a counterexample. Unknowns are
listed. No API is declared implemented.

## Deliverables and feedback

The report at `output_path`; a claim ledger using the parent's claim ids; a
proposed update to the parent's capability matrix and decision matrix; and a
list of further children, if any, with stable ids. Register the report in
`docs/research/manifest.json`.

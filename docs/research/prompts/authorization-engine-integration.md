---
id: frost-research-prompt-authorization-engine-integration
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
open_questions: [Q-AUTHZ-1]
---

# Child prompt: authorization and symbolic-engine integration boundaries

A ready-to-run handoff produced with the
[subtopic handoff template](../templates/subtopic-handoff.md). New synthesis
dated 2026-11-09. Frost implements no authorization protocol, solver, or
policy language; this prompt asks what an engine built on Frost collections
would need from them and what must remain outside the library.

```yaml
question_id: Q-AUTHZ-1
parent_report_id: frost-research-synthesis-foundations-and-engine-roadmap
parent_path: docs/research/synthesis/foundations-and-engine-roadmap.md
parent_revision: UNRESOLVED_AT_HANDOFF   # the parent synthesis is new work and does not exist at the code baseline; resolve the commit (or record the parent's content digest) before research starts
parent_content_sha256: UNRESOLVED_AT_HANDOFF
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca   # inspected production code only
parent_sections_or_claims: [CL-AUTHZ-1, CL-AUTHZ-2, CL-AUTHZ-3, CL-SYM-1]
child_report_id: frost-research-report-authorization-engine-integration
mode: deep-dive
output_path: docs/research/reports/authorization-engine-integration.md
inherited_assumptions:
  - "Zanzibar separates stored relations from computed usersets and evaluates at consistent snapshots: confirmed by the other lane (Opus) against the PDF, sections 2.2, 2.3.1, 2.4.4, 3.2.3, 3.2.4 (SRC-ZANZIBAR-2019)"
  - "SpiceDB permissions compose with union, intersection, and exclusion: confirmed (SRC-SPICEDB-SCHEMA, mutable docs, access date only)"
  - "GNAP (RFC 9635) delegates authorization to software; DID verification relationships are purpose-specific; a libp2p PeerId is a multihash of an encoded public key: confirmed as boundary facts only (SRC-GNAP-RFC9635, SRC-DID-CORE-1_0, SRC-LIBP2P-PEER-IDS)"
  - "Macaroons, Vanadium, OAuth, UCAN, ZCAP, Kameo: influences only; no protocol detail audited (gap entries in SOURCES.md)"
  - "A deny-overrides set formula is a valid illustration only over complete request tuples with fully evaluated sets: derived"
owning_module_or_crate: "outside the library for policy, proof verification, revocation, and enforcement; src/concurrent and future computed-set work for the data layer"
```

## Decision to unlock

Which collection-level contracts (typed facts, revisioned snapshots,
correlated permission tuples, explanations, explicit unknown/error states) an
authorization or symbolic-execution engine needs from Frost, and which
responsibilities must be documented as outside Frost so that no README,
design document, or API implies an identity server, a solver, or protocol
support. The parent can draw the layer diagram; it cannot specify the data
contracts at the boundary.

## Known evidence and assumptions to revalidate

Confirmed boundary facts (each a narrow check, not a protocol audit):

- Zanzibar: userset rewrites with union, intersection, and exclusion
  (section 2.3.1); checks at a consistent snapshot no earlier than the
  client-provided token (sections 2.2 and 2.4.4); subtree cancellation
  (3.2.3); Leopard's periodic offline index plus online updates merged up to
  the query timestamp (3.2.4); pages 2-8 as read by Opus
  (`SRC-ZANZIBAR-2019`).
- SpiceDB schema: permissions compose with `+`, `&`, `-`
  (`SRC-SPICEDB-SCHEMA`).
- RFC 9635 is Standards Track, October 2024; its abstract describes
  delegating authorization to a piece of software (`SRC-GNAP-RFC9635`).
- DID v1.0 (W3C Recommendation, 19 July 2022) section 5.3: a verification
  relationship expresses the relationship between the DID subject and a
  verification method (`SRC-DID-CORE-1_0`).
- libp2p peer-ids (status Active, revision r2, 2021-04-30): peer ids are
  derived by hashing the encoded public key with multihash; keys serializing
  to more than 42 bytes use sha256, at most 42 bytes use the identity codec
  (`SRC-LIBP2P-PEER-IDS`). A DID and a PeerId are different identifiers with
  different binding rules; do not equate them.

Working assumptions:

- Correlated grants must not be flattened into independent principal, action,
  and resource sets. Derived; the counterexample should appear in the report.
- Unknown, error, conditional, and stale-index outcomes must be preserved
  until an engine-specific combining rule applies. Derived.
- Symbolic execution can use candidate-state sets, symbolic environments,
  dependency indexes, and retained versions; union of environments is not a
  sound state merge. Derived; no symbolic-execution literature audited.

## Scope and exclusions

In scope: the data contracts at the collection boundary: typed fact domains
and dictionary revisions (independent dictionaries are not interchangeable
IDs), request-tuple correlation, revision agreement between index, snapshot,
contextual facts, and cache, explanation payloads, error and unknown states,
and revocation freshness as an engine responsibility. Out of scope: proof
chain verification, cryptographic binding, replay protection, obligation
enforcement, audit policy, solver design, abstract domains, state merging,
and any specific protocol implementation.

## Competing hypotheses or designs

1. Status quo: Frost provides sets, maps, snapshots; engines build everything
   above. Strongest reason: smallest surface. Failure: each engine reinvents
   revision agreement and explanation plumbing inconsistently.
2. Boundary contract document: Frost documents the obligations an engine
   must meet and the guarantees Frost gives (structural versions, left
   representatives, exact counts), with worked counterexamples. Strongest
   reason: cheap and prevents overclaiming. Failure: documentation drifts.
3. Typed boundary types: tuple-correlated permission sets, revisioned source
   handles, and explanation-carrying results as library types. Strongest
   reason: compiler-checked agreement. Failure: Frost absorbs engine
   semantics and feature coupling grows.

## Questions to answer

- The exact request-tuple model and why projection to independent sets
  invents permissions; the counterexample.
- The lattice of outcomes (allow, deny, unknown, error, conditional) and
  which set operations are defined over it.
- What "index revision, durable snapshot, contextual facts, and cache agree"
  means operationally and what to reject otherwise.
- The explanation payload: which representatives and source ids must be
  retained for an explanation to be reproducible at a revision.
- The candidate-state and dependency-index uses in symbolic engines, and the
  precise boundary where solver, path conditions, and sound merging begin.
- Which influences (Macaroons, Vanadium, OAuth, GNAP, DID, PeerId, UCAN,
  ZCAP, Kameo, libp2p) impose a concrete data-shape requirement, if any, with
  a registered primary source for each claim; otherwise record "influence
  only".

## Required evidence and experiments

- The primary sources above plus any protocol specification actually read,
  registered with version and access date. No protocol detail from memory.
- A small finite model of correlated tuples versus flattened sets that
  produces the invented-permission counterexample.
- A worked example of stale index versus fresh snapshot producing different
  answers, with the agreement rule that rejects it.

## Acceptance criteria

A boundary contract that a README section can summarize in one paragraph
without naming a protocol as supported, with each obligation attributed to the
engine or to Frost and each influence either sourced or marked influence-only.

## Deliverables and feedback

The report; proposed wording for the README and design documents (as text,
not edits); updates to the parent's decision matrix rows CL-AUTHZ-1 to
CL-AUTHZ-3 and CL-SYM-1; and the list of influences still unaudited.

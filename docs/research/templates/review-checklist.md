---
id: frost-research-template-review-checklist
kind: template
level: not-applicable
research_status: curated
implementation_status: not-applicable
source_validation: not-applicable
input_files: [docs/reviews/supplied/frost-research-handoff.zip]
input_members: [frost-research-handoff/templates/review-checklist.md]
input_sha256: [605767df09f7a631f7c27951c38054216e63de8ece55f9ca4d3f6c7fc65ece0f]
origin_created_at: unknown
imported_at: 2026-11-09
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca
parent_ids: [frost-research-template-prompt]
related_ids: []
supersedes: []
open_questions: []
---

# Research and documentation review checklist

Use this list when reviewing a new or changed research artifact, a synthesis,
or a README rationale change. Every unchecked item needs a written reason.

## Provenance and completeness

- [ ] Every available original is preserved byte for byte, with its hash in `manifest.json`; raw text is not corrected in place.
- [ ] Summaries are labelled as summaries; missing attachments and failed recovery attempts are visible in the integration audit.
- [ ] The repository revision, toolchain, and inspected source scope are recorded.
- [ ] Every curated citation resolves to a primary source with version, section, and access date, or is explicitly unresolved.
- [ ] Editorial sources that independently support a claim are not presented as the recovered original citation.
- [ ] Opaque export markers are scoped to their originating input in the citation map; equal-looking tokens from different inputs are not merged.
- [ ] Imported, curated, reviewed, proposed, accepted, implemented, and verified states are not conflated.

## Technical validity

- [ ] Logical domain, equality relation, representative rule, order, and multiplicity are explicit for every operation.
- [ ] Map resolver effects and call counts, bimap closure and conflicts, bag overflow, and sequence ordering are addressed.
- [ ] Frozen APIs do not expose structural mutation; persistence, query, snapshot, and materialization are separate axes.
- [ ] Stored, decoded, and lending outputs are not collapsed into fictitious references.
- [ ] Algebraic rewrites preserve the chosen observable contract, errors, and snapshot assumptions; `Fn` is not treated as purity.
- [ ] Computed sets separate membership, existence, bounded enumeration, count, and materialization costs.
- [ ] Streaming claims account for workspace, recursive state, spill, and I/O; a documented bound is not transferred to another algorithm.
- [ ] Every Rust snippet carries a compile status; anything marked compiled was actually compiled on the named toolchain.
- [ ] Current manifest, features, CI, and toolchain are not overwritten by historical research snippets.

## Engine rationale and scope

- [ ] Rationale explains algebra, frozen lifecycle, symbolic and authorization use cases, and non-goals without claiming unimplemented engines.
- [ ] Identity proof, delegated authority, policy evaluation, and enforcement remain distinct responsibilities.
- [ ] Permissions keep their request-tuple correlation; unknown, error, conditional, and stale-index states are not silently absence.
- [ ] Snapshot stability is not advertised as freshness, revocation correctness, or cross-store transactionality.
- [ ] No solver, security protocol integration, or universal speedup is implied without evidence.

## Navigation and execution

- [ ] The root README links to the Frozen API guide, design documents, research corpus, templates, and verification.
- [ ] `docs/research/README.md`, `manifest.json`, and the reading paths include the new artifact with stable ids and resolvable parent/child links.
- [ ] Local links, anchors, front matter, JSON, code fences, and populated template examples were checked; the commands are listed in the audit.
- [ ] Executed checks and unavailable checks are reported separately, with commands.
- [ ] The delivery report distinguishes file edits, commit creation, push, and pull-request status.

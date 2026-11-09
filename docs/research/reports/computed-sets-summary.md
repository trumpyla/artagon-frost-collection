---
id: frost-research-report-computed-sets-summary
kind: summary
level: landscape
research_status: curated
implementation_status: proposed
source_validation: partial
input_files: [docs/reviews/supplied/frost-computed-sets-summary.md]
input_members: []
input_sha256: [6f39fdbcf2d7816a3ea33eb40e371be7115b1b452cf040a7f8fe84a60de8177c]
origin_created_at: unknown
imported_at: 2026-11-09
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca
parent_ids: []
related_ids: [frost-research-synthesis-foundations-and-engine-roadmap, frost-research-prompt-computed-sets-and-durable-execution]
supersedes: []
open_questions: [Q-CSET-1, Q-NARY-1, Q-AUTHZ-1]
---

# Computed sets: curated summary (from Frost3)

**What this is, and what it is not.** `Frost3.md.md` (115 lines, author and
date unknown) is a **summary and delivery note**. It describes a separate
"standalone technical report" and says that report, a research prompt
template, and a ZIP of a `docs/` tree were created. **None of those three
artifacts was supplied, and none was recovered** from the repository, the
handoff archive, or the thread's attachments (recovery attempts are listed in
the [integration audit](../integration-audit.md)). This document curates the
summary only. It must not be read as the full report, and the summary's
statements about what the full report "includes" are inherited as claims
about an unavailable document, not as evidence.

The raw original is preserved byte for byte as
[`frost-computed-sets-summary.md`](../../reviews/supplied/frost-computed-sets-summary.md).

## 1. Executive synthesis

The summary's thesis is sound and consistent with Frost's direction:
**computed sets are executable query plans evaluated against sources at a
defined revision, not resident containers**, with separate semantics for
membership, representative selection, traversal, counting, snapshots, and
explicit materialization. The design space it names (sorted posting streams,
selective probes, bitmap kernels, interning dictionaries, base-plus-delta
overlays, incremental maintenance) is the space the general set-algebra
design and the integer-views proposal already describe for Frost
(`SRC-FROST-DOC-SET-ALGEBRA`, `SRC-FROST-DOC-INTEGER-VIEWS`).

What is established and what is not:

- The Zanzibar facts it relies on were checked against the paper by the other
  documentation lane (Opus) at section and page level (`SRC-ZANZIBAR-2019`);
  the lead's preparatory section note was corrected on that basis. The
  SpiceDB half of its "modern systems confirm" sentence is checked
  (`SRC-SPICEDB-SCHEMA`); the OpenFGA half and the Differential Dataflow
  claims are unverified.
- Its `HashSet` representative point is verified
  (`SRC-RUST-HASHSET-INTERSECTION`); its `BTreeSet` lazy-iterator point is
  secondary-verified (`SRC-RUST-BTREESET-OPS`); its FST lending-stream and
  "memory proportional to the number of streams" points are verified and
  qualified (`SRC-FST-0_4_7-STREAMER`, `SRC-FST-0_4_7-RAW-OPS`).
- Its crate placement names a third set of crate names and is a proposal in
  conflict with the other two inputs; Frost is one package.
- Its API sketches are pseudocode. Nothing in it is implemented.

## 2. Core conclusions, with status

| # | Summary statement | Status |
| --- | --- | --- |
| C1 | Zanzibar separates stored relations from computed usersets; its configuration supports computed usersets, tuple-to-userset, union, intersection, exclusion; `Check` builds boolean expression trees, evaluates leaves concurrently, and cancels subtrees once the answer is determined | Opus-checked: section 2.3.1 (userset rewrites, p. 4) and section 3.2.3 (check evaluation and cancellation, p. 7). Supports terminal-specific execution (`contains`, `is_empty`, bounded enumeration) as a design motivation only |
| C2 | Leopard represents named sets with integer set and element ids, evaluates union/intersection/exclusion over ordered integer lists (skip lists), reduces some group-membership checks to non-empty intersection, and merges a periodic offline index with timestamped online updates at the query snapshot | Opus-checked: section 3.2.4 (pp. 7-8), including ordered integer lists such as a skip list and the non-empty-intersection reduction |
| C3 | Checks evaluate at a snapshot no earlier than a client-provided consistency token | Opus-checked: sections 2.2 (pp. 2-4) and 2.4.4 (p. 5) |
| C4 | `BTreeSet` has lazy ordered set-operation iterators | Secondary-verified: named methods return borrowing iterators; the operators return new sets (`SRC-RUST-BTREESET-OPS`) |
| C5 | `HashSet` intersection can yield the representative from either input | Verified (`SRC-RUST-HASHSET-INTERSECTION`) |
| C6 | Roaring provides compressed integer sets and portable serialized formats | Count kernels verified in 0.12.0 source; serialization format claim unverified |
| C7 | FST supports large compressed byte-string sets/maps with lending `Streamer` semantics because traversal reuses a key buffer; multiple streams combine with memory proportional to the stream count | Verified and qualified: heap-based; the merge's own work is bounded by `O(k + N · (L + 1) · log(k + 1))` plus stream decoding and I/O, with `O(k · (L + 1))` bytes of merge workspace excluding stream internals (`SRC-FST-0_4_7-RAW-OPS`; derivation in the audit) |
| C8 | Differential Dataflow propagates deltas with negative differences and is an optional later engine | Unverified (no source read) |
| C9 | OpenFGA userset schema has direct, computed, tuple-to-userset, union, intersection, difference nodes; SpiceDB permissions are computed subject sets with union, intersection, exclusion, traversal | SpiceDB verified for `+`, `&`, `-` (`SRC-SPICEDB-SCHEMA`); OpenFGA unverified |
| C10 | A live concurrently mutating collection must not silently satisfy the same source contract as a frozen or pinned snapshot | Consistent with Frost's implementation: live backends implement neither borrowed traversal nor `Eligible`; `SnapshotValue` is sealed (`SRC-FROST-SRC-SNAPSHOT`, `SRC-FROST-DOC-ARCHITECTURE`) |

## 3. The proposed model (pseudocode)

```text
let effective = view(&direct_viewers)
    .union(view(&group_viewers))
    .union(view(&inherited_viewers))
    .difference(view(&suspended));

if effective.try_contains(&user_id)? { allow(); }

view(&a).intersection(view(&b)).try_count()?;
let materialized = view(&a).difference(view(&revoked)).materialize(destination)?;
let owned = a.into_union(b)?;
```

This is the same shape as the general set-algebra design's typed views, with
two additions that design leaves open: a fallible `try_contains` (the general
design's `contains` is infallible for in-memory leaves; fallibility belongs
to durable or budgeted sources) and a destination-directed `materialize`. The
capability names it relies on (`Elements`, `SetContains`, `SetGet`,
`ValueElements`) are Frost's actual names (`SRC-FROST-SRC-READ`); the
"future optional sorted/seek capabilities" are not.

For the membership terminal, no result container needs to exist; a planner
can short-circuit probes, use a native bitmap kernel, seek through sorted
sources, or lower compatible integer ids to a specialized representation.
**Qualification:** lazy execution avoids result and intermediate containers;
it does not bound scans, buffering, deduplication, recursive traversal, or
spill, and exact counts over wide unions can still require substantial work
or state (`SRC-FROST-DOC-SET-ALGEBRA`, sections 7-8;
`SRC-FROST-DOC-LAZY-VIEWS-SOURCE-AUDIT`).

The base-snapshot-plus-ordered-delta overlay mirrors Leopard's architecture
and is a plausible incremental route; it is a proposal with no Frost
implementation. Its obligations (index revision, durable snapshot, contextual
facts, and cache must agree; a stale or incomplete index is not proof of
absence) are stated in the [synthesis](../synthesis/foundations-and-engine-roadmap.md)
and are the subject of `Q-CSET-1`.

## 4. Proposed placement (conflict recorded)

The summary recommends `frost-api` as facade, `frost-set-algebra` for
computed sets, `frost-set-kernels` for sorted merge and seek, `frost-integer-sets`
for encoded ids and Roaring, `frost-core` for source capabilities, and
`frost-concurrent` for snapshots, with a dependency diagram. Frost1 proposes
the same spine plus `frost-map-algebra`, `frost-bags`, `frost-multimaps`,
`frost-bimaps`, `frost-sequences`, `frost-integer-maps`, `frost-prefix-index`;
Frost2 uses singular `frost-bag`, `frost-multimap`, `frost-bimap`. All three
are proposals; the repository is one package (`SRC-FROST-CARGO`). The
dependency *direction* (facade depends downward; algebra depends on
capabilities, never on the facade; specialized bridges optional) is a
responsibility map that module boundaries can honour now.

## 5. Correctness and verification direction (mapped)

The summary's major finding, separating set-membership equivalence from
observable representative equivalence and being "more aggressive" for
membership/count terminals and "more conservative" for representative-
producing iteration, is exactly the rewrite rule the general set-algebra
design states and the finite oracle demonstrates
(`SRC-FROST-VERIFICATION-SET-ALGEBRA`).

Its verification plan (finite semantic oracle, property tests, adversarial
equality/representative cases, cross-plan differential testing, benchmark
matrices, Miri for borrowing and cursor safety, Loom for publication, Kani
for bounded cursor/planner invariants) maps onto tools the repository already
runs in isolated packages and Make targets (`SRC-FROST-DOC-VERIFICATION`).
Listing a tool is not a result; no computed-set code exists to run them on.

## 6. Repository status statement (not inherited)

The summary says "the two files were created with the exact
repository-relative paths requested and packaged in the ZIP above" and that
they could not be committed or pushed. This integration confirms only that
the files are absent here. The creation claim describes another session's
sandbox and is **not inherited** (hazard H11). The paths
`docs/research/computed-sets.md` and
`docs/research/templates/research-prompt.md` that the summary names are
reserved as follows: the template path is now occupied by the adapted master
prompt from the handoff, which does not derive from the missing original;
`docs/research/reports/computed-sets.md` is reserved for a future child report
produced from the [computed-sets prompt](../prompts/computed-sets-and-durable-execution.md).

## 7. Claim and evidence ledger

| Claim ID | Claim | Class | Evidence | Disposition |
| --- | --- | --- | --- | --- |
| S3-1 | Computed sets are plans, not containers; terminals differ | derived; consistent with repository design | `SRC-FROST-DOC-SET-ALGEBRA` | Adopted as direction |
| S3-2 | Zanzibar/Leopard facts C1-C3 | documented upstream (Opus-checked sections and pages) | `SRC-ZANZIBAR-2019` | Accepted with section and page citations |
| S3-3 | HashSet representative; BTreeSet lazy iterators | documented upstream | `SRC-RUST-HASHSET-INTERSECTION`, `SRC-RUST-BTREESET-OPS` | Accepted |
| S3-4 | FST lending streams and stream-proportional memory | documented upstream, qualified | `SRC-FST-0_4_7-STREAMER`, `SRC-FST-0_4_7-RAW-OPS` | Qualified (conservative bound in C7) |
| S3-5 | Differential Dataflow and OpenFGA statements | unverified | — | Open |
| S3-6 | Crate placement | proposal, conflicting | `SRC-FROST-CARGO` | Naming policy deferred; one package |
| S3-7 | API sketches | pseudocode | — | Open (`Q-CSET-1`) |
| S3-8 | Full report, template, and ZIP were created | claim about an unavailable artifact | Recovery attempts in audit | Not inherited |
| S3-9 | Live collections must not satisfy the snapshot source contract | implemented and inspected (for Frost's existing boundary) | `SRC-FROST-SRC-SNAPSHOT` | Current behaviour |

## 8. References and lineage

Ids resolve in [SOURCES.md](../SOURCES.md); the 15 marker groups of the raw
file are scoped in [citation-map.json](../citation-map.json). Curated
2026-11-09 from `Frost3.md.md` at baseline `82becb9` with corrections H11 and
H12. Parent: none. Child prompt:
[computed sets and durable execution](../prompts/computed-sets-and-durable-execution.md).

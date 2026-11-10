---
id: frost-research-report-collection-algebra-implementation-notes
kind: report
level: landscape
research_status: curated
implementation_status: mixed
source_validation: partial
input_files: [docs/reviews/supplied/frost-collection-algebra-research-2.md]
input_members: []
input_sha256: [046cdaf91f7f2912ce966835951d00dd42ae230f5a76914b5eeb5b77f8862ad3]
origin_created_at: unknown
imported_at: 2026-11-09
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca
parent_ids: []
related_ids: [frost-research-report-collection-algebra-survey, frost-research-synthesis-foundations-and-engine-roadmap]
supersedes: []
open_questions: [Q-ALG-1, Q-MAP-1, Q-BAG-1, Q-MM-1, Q-BIMAP-1, Q-PREFIX-1, Q-NARY-1]
---

# Collection algebra implementation notes (curated from Frost2)

**What this is.** A curated derivative of the supplied report *Frost
Collections Research: Map, List, Bag, Multimap, and BiMap Algebra in Rust*
(`Frost2.md.md`, 595 lines, author and date unknown), preserved byte for byte
as [`frost-collection-algebra-research-2.md`](../../reviews/supplied/frost-collection-algebra-research-2.md).
It overlaps with the [survey curated from Frost1](collection-algebra-survey.md);
where both say the same thing, this document refers to the survey instead of
repeating it, and concentrates on what Frost2 adds: an ecosystem table with
versions and licenses, implementation and optimization techniques, output
modes, verification commands, and a prioritized bibliography.

The raw file links to a "repository-ready research document" and a
"research template" through session-local `sandbox` paths. Neither file was
supplied; see the [integration audit](../integration-audit.md). Its selected
bibliography uses ordinary URLs and is catalogued as input-supplied,
unverified references in [SOURCES.md](../SOURCES.md).

## 1. Executive synthesis

Frost2 reaches the same architectural conclusion as Frost1: related but
semantically distinct abstractions sharing traversal, lookup, representative
recovery, count, materialization, and planning infrastructure. Its distinct
contributions that survive review:

- The four observable dimensions of a collection algebra: logical membership,
  representative selection, traversal order, and ownership/lifetime. Frost's
  general set-algebra design uses the same separation
  (`SRC-FROST-DOC-SET-ALGEBRA`).
- An explicit **output-mode table** (stored borrowed, lending reconstructed,
  decoded by value, shared owned, independently owned) that matches Frost's
  existing capability boundaries exactly (`SRC-FROST-SRC-READ`,
  `SRC-FROST-SRC-FST-ADAPTER`).
- A `KeyRun` cursor shape for sorted map merges that surfaces collisions to a
  resolver.
- Terminal-specific planning and a table of n-ary strategies by data profile.

Corrections: its target-restriction statement is false for this repository;
its verification commands and CI skeleton describe packages that do not
exist; its `BiMap` trait puts `try_insert(&mut self)` on an unspecified
receiver and is pseudocode; its n-ary linear claim needs the same
qualification as Frost1's; and its ecosystem versions have drifted from the
locked dependencies.

## 2. Algebraic foundations (delta to the survey)

The taxonomy table (set, bag, map, multimap, bimap, list; model, operations,
meaning of duplicates, main hazard) agrees with the survey's section 2. Frost2
adds the bag definitions in mathematical notation (`m_{A⊕B} = m_A + m_B`,
`m_{A⊔B} = max`, `m_{A∩B} = min`, `m_{A−B} = max(m_A − m_B, 0)`) and the
bimap invariant (`(l,r1),(l,r2) ∈ B ⇒ r1 = r2` and symmetrically). Both are
definitions; the literature attributions remain unverified
(`SRC-INPUT-ALBERT-1991`, `SRC-INPUT-HERNICH-KOLAITIS-2017`).

Its `merge_with` example matches the resolver over `(Option<&VL>,
Option<&VR>)` with a `Resolution::{Left, Right}` outcome. It is pseudocode: it
does not define the view type, resolver bounds, or whether the resolver runs
for one-sided keys. Those are the open items in `Q-MAP-1`.

Its representative example (`struct Record { id, label }` with equality by
`id`) is stated in prose rather than as derived code, so it does not contain
the Frost1 derive contradiction. The compiled corrected example in the survey
covers both.

## 3. Ecosystem landscape with versions (status added)

| Crate | Frost2 says | Repository at baseline | Status of other columns |
| --- | --- | --- | --- |
| `std::collections` | MIT OR Apache-2.0; HashMap expected O(1), BTreeMap O(log n) | Rust 1.99.1 | License and complexity claims standard; not separately checked |
| `hashbrown` 0.17.1 | SwissTable, SIMD, `no_std + alloc` | Optional dependency `0.17` (`hashbrown-0_17`) | Unverified |
| `indexmap` 2.14.2 | insertion-ordered; dense indices | Optional dependency `2.14` (`indexmap-2`); 2.14.0 checked in the set-algebra design (S8) | Order semantics secondary-verified |
| `imbl` 7.0.2 | persistent HAMT, B+ tree, RRB | Optional dependency `7.0` (`imbl-7`); 7.0.2 checked (S4, S5) | Secondary-verified for hash/ordered sets |
| `fst` 0.4.7 | lending `Streamer`; n-ary merge streams; `O(Σ inputs)`, `O(k)` memory | Locked 0.4.7 (`fst-0_4`) | Verified and qualified (`SRC-FST-0_4_7-*`) |
| `roaring` 0.11.5 | decoded integers; container kernels | **Locked 0.12.0** (`roaring-0_12`, `int-set`) | Version drift; kernels verified in 0.12.0 source |
| `bimap` 0.6.3, `bidimap` 0.7.0, `multimap` 0.10.1, `multi_bimap` 0.8.0 | various | Not dependencies | Unverified; release dates and "maintenance signal" are dated claims of unknown date |

The license column is reproduced as supplied and was not verified. Treat the
whole table as a snapshot of unknown date, not as current.

Three lessons Frost2 draws are already Frost contracts: ordering is several
capabilities (sorted by key, stable source order, positional), not one;
iteration lifetime depends on storage (borrowed, lending, decoded); and
persistence is a storage strategy, not a wrapper. Each maps to
`SRC-FROST-SRC-READ`, `SRC-FROST-SRC-FST-ADAPTER`, and
`SRC-FROST-DOC-PERSISTENT`.

### Target restrictions (corrected)

Frost2 states that "no explicit target restrictions were given" and assumes
portable Rust with optional `no_std + alloc`, mmap, SIMD, and
platform-specific acceleration (hazard H10). The repository's constraints are
explicit: Rust 1.99.1, edition 2024; the crate is `#![no_std]` with `alloc`
and `std` features; isolated feature checks; embedded compilation on
`thumbv7em-none-eabihf` and `thumbv6m-none-eabi` including the integer
profile's `Arc`/`Rc` branches; no new unsafe outside one module; features
additive (`SRC-FROST-CARGO`, `SRC-FROST-DOC-ARCHITECTURE`,
`SRC-FROST-DOC-VERIFICATION`). Any proposal must satisfy these.

## 4. Architecture and proposed APIs (pseudocode)

The module surface `frost::{set, map, bag, multimap, bimap, seq, integer,
index::prefix, concurrent}` is a naming proposal; `frost::integer` and
`frost::concurrent` exist today, the rest do not.

The small-capability sketch (`Len`, `Contains<Q>`, `Get<Q>`, `Entries`)
corresponds to Frost's `Len`, `SetContains<Q>`, `SetGet<Q>`/`LookupRef<Q>`,
and `Entries` (`SRC-FROST-SRC-READ`). Frost2's point that `Contains` and `Get`
must stay distinct is exactly why `SetContains` promises no reference and
`SetGet` returns the stored representative. The names are not changed.

Typed views (`view(&base).merge_with(view(&overlay), …).difference_keys(view(&deleted))`)
match the typed-views-first recommendation of the set-algebra design. Frost2
spells `difference_keys`; Frost1 spells `difference_by_key`. One spelling
must be chosen when a map-algebra design is accepted (`Q-MAP-1`); neither is
an established public API, so nothing is renamed.

The `BiMap<L, R>` trait with `try_insert(&mut self, …)` (hazard H14) is
pseudocode on an unspecified receiver. The corrected shape (insertion on
`MutableBiMap` or a builder; frozen type structurally immutable) is in the
survey. The `BiInsertError::{LeftOccupied, RightOccupied, BothOccupied}` with
the attempted pair returned is a reasonable conflict model; "transactional"
must mean that a rejected insert leaves both indexes unchanged, which needs a
documented panic and allocation-failure contract.

Borrowed iteration preserving representatives even when the plan probes from
the right ("scan the smaller input" is a performance rule, "return the left
representative" is an API rule) is already how Frost's hash intersection
behaves (`SRC-FROST-DOC-SET-OPERATIONS`).

## 5. Implementation and optimization techniques

**Sorted merge kernels and `KeyRun`.** A sorted two-way map merge can surface
each key as `Left`, `Right`, or `Both { key_left, value_left, key_right,
value_right }` so the resolver chooses the surviving representative and
value. This is a useful cursor shape for a future generic sorted kernel; the
existing ordered set kernel is specialized to sets.

**N-ary strategies (qualified).** Frost2's table (frontier/heap merge for
sorted streams; temporary exact index or chosen build/probe side for hash
sources; container-wise operations for bitmaps; native stream operations for
FST; using the materialization target as the duplicate index; candidate-first
probing for selective intersections) is a sound menu. Its claim that FST
performs n-ary operations "in `O(n₁+n₂+…)` total work and memory proportional
to the number of streams" (hazard H6) carries the same qualification as in
the survey: the implementation is a binary heap, so the merge's own work is
bounded by `O(k + N · (L + 1) · log(k + 1))` with `O(k · (L + 1))` bytes of
merge workspace, plus per-advance decoding and I/O; this is source-based
reasoning, not a measurement (`SRC-FST-0_4_7-RAW-OPS`; derivation in the
audit). A left-nested
lazy union of `k` disjoint hash sources performs `O(n·k²)` probes
(`SRC-FROST-DOC-SET-ALGEBRA`, section 8). These are the inputs to `Q-NARY-1`.

**Hash kernels.** `intersection_count` can scan the smaller side; a borrowed
`intersection_iter` driven from the right needs left representative recovery.
Standard HashMap costs are expected, not worst-case, and traversal visits
capacity (`SRC-RUST-HASHSET-INTERSECTION` for representatives; the capacity
remark is from the set-algebra design, S1).

**Encoded integers.** Frost2's "presence bitmap, rank directory, dense value
array" layout for a future integer map is a proposal; rank/select exist in
the locked Roaring source (`SRC-ROARING-0_12_0-SRC`); the succinct-dictionary
attribution is unverified (`SRC-INPUT-RRR-2002`).

**Prefix indexing.** FST for immutable byte dictionaries (values restricted to
`u64`; keys are byte strings) is consistent with Frost's adapter
(`SRC-FROST-SRC-FST-ADAPTER`). ART for mutable prefix indexes is unverified
(`SRC-INPUT-ART-2013`). The four prefix relations (`get_exact`,
`entries_with_prefix`, `matching_prefixes`, `longest_prefix`) and the rule
that byte, UTF-8, path, and bit prefixes do not share normalization are
accepted as the shape of `Q-PREFIX-1`.

### Output modes

| Output mode | Meaning | Frost today |
| --- | --- | --- |
| stored borrowed | `&T`, `(&K, &V)` from source storage | `Elements`, `Entries`, `LookupRef`, `SetGet` |
| lending reconstructed | borrow valid until the next cursor advance | FST `stream()`; not `Elements` |
| decoded by value | e.g. `u32` from a compressed bitmap | `ValueElements`, `LookupValue`, `FrozenIntSet` iteration |
| shared owned | `Arc<T>` or shared persistent node | `SnapshotStore::load`; integer root sharing |
| independently owned | cloned or consumed value | `cloned()`/`collect()`, `into_inner`, `thaw` |

"Zero-copy" is never a substitute for one of these contracts. This table is
the most directly reusable artifact in Frost2 and is adopted verbatim in the
synthesis.

## 6. Correctness, verification, and benchmarks (mapped)

The four obligations (logical, representative, traversal, ownership
correctness) are adopted. The bimap two-`BTreeMap` inverse-consistency model
is adopted as the reference model for `Q-BIMAP-1`; panic-injection to ensure
no half-updated pair escapes is a required test, not an assumption.

**Verification commands (historical, not runnable here).** The raw file lists
`cargo … --workspace`, `cargo test -p frost-map-algebra`, `cargo +nightly miri
test -p frost-core`, `cargo kani -p frost-bimap`, and a CI skeleton with
`dtolnay/rust-toolchain@stable` and `@nightly` matrix jobs over
`frost-core`, `frost-bag`, and similar packages (hazards H8, H9). None of
those packages exists. The current gates are `make check` (formatting,
links, Clippy, rustdoc, tests, examples), `make verify`, `make embedded`,
`make miri` with the pinned diagnostic nightly, `make bounded`, and the
isolated `make loom` and `make kani` targets; CI pins Rust 1.99.1
(`SRC-FROST-CARGO`, `SRC-FROST-DOC-VERIFICATION`). The historical block is
retained in the raw file as evidence of the proposal's shape only.

**Benchmark dimensions.** Construction cost, allocations, peak memory,
representative recovery, conflict density, duplicate distribution, key length,
value size, cardinality skew, fiber size, shared-prefix depth, selectivity,
terminal type; controls chosen by workload (std/hashbrown for hashing,
IndexMap for insertion order, FST for lexicographic streams, Roaring for
integers). This agrees with `docs/maintainers/benchmarks.md`; no measurement
is inherited.

## 7. Migration path (reconciled)

Frost2's staged table (semantic contracts; simple models; typed views;
sorted/hash kernels; bag/multimap/bimap owners; encoded/indexed profiles;
persistence/concurrency; optimizer) is compatible with the sequence already
recorded in `docs/design/collection-algebra.md` and is adopted with one
change: every stage lives in the existing package; "canonical Frost types and
facade reexports" means modules, not crates, until a separate decision says
otherwise. Frost2's recommendation that the next deep dive be **map algebra
and join planning** is adopted as `Q-MAP-1`'s priority after `Q-ALG-1`.

## 8. Claim and evidence ledger

| Claim ID | Claim | Class | Evidence | Disposition |
| --- | --- | --- | --- | --- |
| S2-1 | Four observable dimensions: membership, representative, order, ownership | derived reasoning | `SRC-FROST-DOC-SET-ALGEBRA` | Adopted |
| S2-2 | Output-mode table | derived; matches implementation | `SRC-FROST-SRC-READ`, `SRC-FROST-SRC-FST-ADAPTER` | Adopted |
| S2-3 | No target restrictions were given | historical assumption, false here | `SRC-FROST-CARGO` | Corrected |
| S2-4 | FST n-ary `O(Σ n_i)` with `O(k)` memory | documented upstream, qualified | `SRC-FST-0_4_7-OPBUILDER-DOCS`, `SRC-FST-0_4_7-RAW-OPS` | Qualified (`Q-NARY-1`) |
| S2-5 | Ecosystem versions and licenses | unverified, dated | Catalog input rows | Snapshot of unknown date; roaring drift noted |
| S2-6 | `BiMap::try_insert(&mut self)` trait | pseudocode | `SRC-FROST-SRC-FROZEN` | Insertion on `MutableBiMap` or a builder; frozen algebra returns a new value |
| S2-7 | Verification commands and CI skeleton | historical proposal | `SRC-FROST-CARGO`, `SRC-FROST-DOC-VERIFICATION` | Replaced by Make targets; not copied |
| S2-8 | Next deep dive: map algebra and join planning | proposal | — | Adopted as `Q-MAP-1` priority |
| S2-9 | Rank/select succinct dictionaries motivate dense integer maps | unverified attribution; rank/select verified in Roaring | `SRC-ROARING-0_12_0-SRC`, `SRC-INPUT-RRR-2002` | Open |
| S2-10 | ART for mutable prefix indexes | unverified | `SRC-INPUT-ART-2013` | Open (`Q-PREFIX-1`) |
| S2-11 | `multimap`, `bimap` crate behaviours | unverified; not dependencies | Catalog gaps | Retained in raw file only |

## 9. References and lineage

Ids resolve in [SOURCES.md](../SOURCES.md); markers are scoped in
[citation-map.json](../citation-map.json). Curated 2026-11-09 from
`Frost2.md.md` at baseline `82becb9` with corrections H4, H6, H8, H9, H10,
H14. Parent: none. Children: `Q-MAP-1`, `Q-NARY-1`, and the domain questions
in the [synthesis](../synthesis/foundations-and-engine-roadmap.md).

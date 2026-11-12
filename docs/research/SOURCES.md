# Source catalog

Every curated research document cites sources by the stable ids below. A
source entry says what claim it supports, where it was checked, by whom, on
what date, and what it does not establish. A bibliography line alone is not
evidence coverage.

Conventions:

- **Access date** is the date the page or file was actually read. All
  external pages in this catalog were read on **2026-11-09 (UTC)** unless a
  row says otherwise. "Lead-checked" means the swarm lead read the source
  during preparation on that date and recorded the observation in the
  assignment context; "Opus-checked" means the other documentation lane read
  the source and reported the observation in its round report; "rechecked"
  means this lane read it again. A lane never claims a recheck it did not do.
- **Editorial support** means the source independently supports a claim in a
  raw input. It is never a recovered mapping for that input's opaque export
  markers; those mappings are unavailable (see
  [citation-map.json](citation-map.json)).
- Repository sources are cited at baseline commit
  `82becb998af5627cc1e58118ae01515d8a3559ca`.
- Each synthesis below stays within a 200-word budget per source.

## Rust standard library, 1.99.1

| ID | Source | Supports | Checked | Limits |
| --- | --- | --- | --- | --- |
| `SRC-RUST-HASHMAP-GET-KEY-VALUE` | [`HashMap::get_key_value`](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.get_key_value), std 1.99.1 | Bounds `K: Borrow<Q>`, `Q: Hash + Eq + ?Sized`; returns the stored key-value pair; documented for "key types where non-identical keys can be considered equal"; its example defines a key whose `PartialEq`/`Hash` use only `id` and ignore `name`, and shows the stored key being returned for an equal lookup key. | Lead-checked; rechecked. | Documents std behaviour, not Frost's. |
| `SRC-RUST-HASHSET-INTERSECTION` | [`HashSet::intersection`](https://doc.rust-lang.org/std/collections/struct.HashSet.html#method.intersection), std 1.99.1 | When both sets hold an equal element, the intersection iterator may return the reference from either one ("one or the other"). Frost's left-representative rule is stronger and must not be delegated to this operation. | Lead-checked; rechecked. | Says nothing about union or about Frost. |
| `SRC-RUST-BTREESET-OPS` | [`BTreeSet`](https://doc.rust-lang.org/std/collections/struct.BTreeSet.html) documentation and [source](https://doc.rust-lang.org/src/alloc/collections/btree/set.rs.html), std 1.99.1 | Named `union`/`intersection` return borrowing ascending iterators; the `&a & &b` operators return new sets; the intersection source chooses among bounds shortcuts, a merge, and a skewed probe strategy. | Verified in the repository's [lazy-views source audit](../reviews/lazy-views-source-audit.md) (R1, R2) on 2026-11-09; not reread for this integration. | Secondary via a repository document. |
| `SRC-RUST-ARC` | [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html), std 1.99.1 | Cloning shares one allocation; `make_mut` clones the inner value when shared; pointer equality is allocation identity; auto traits follow the inner type. | Verified in the repository's [general set-algebra design](../design/set-algebra.md) (S6); not reread. | Secondary via a repository document. |

## `fst` 0.4.7 (locked dependency)

| ID | Source | Supports | Checked | Limits |
| --- | --- | --- | --- | --- |
| `SRC-FST-0_4_7-OPBUILDER-DOCS` | [`fst::map::OpBuilder`](https://docs.rs/fst/0.4.7/fst/map/struct.OpBuilder.html), version-pinned page (the unpinned page also resolved to 0.4.7 on the access date) | Documents that every set operation accepts any number of streams with memory proportional to the stream count and runs in `O(n1 + n2 + n3 + ...)` over the per-stream element counts. Union and intersection yield each key with one indexed value per contributing stream; difference is relative to the first stream; symmetric difference keeps keys present in an odd number of streams. | Lead-checked; rechecked. | The statement names only element counts and stream count. Reading it as hiding fan-in, key-length, and decoding factors is this corpus's interpretation under fixed fan-in and key costs; the upstream text does not say so. See the next row. |
| `SRC-FST-0_4_7-RAW-OPS` | [`fst` 0.4.7 `src/raw/ops.rs`](https://docs.rs/fst/0.4.7/src/fst/raw/ops.rs.html) on docs.rs; the same file in the local Cargo registry (`Cargo.lock` pins 0.4.7) | `StreamHeap` holds boxed streams and a `BinaryHeap<Slot>` (lines 362-365); `new` primes every stream (369-373); `pop` delegates to the heap (376-378); duplicate detection peeks the heap head (380-390); `Union::next` and `Intersection::next` pop and refill (213-231, 247-272); `Difference::next` pops while the head is `<=` the current key (293-316); `SymmetricDifference::next` keeps odd-count keys (332-359); each `Slot` owns a `Vec<u8>` key buffer with initial capacity 64 and is ordered by `(input bytes, output)` reversed (414-459). Basis for the derived n-ary bound in the [integration audit](integration-audit.md). | Local copy inspected by this lane on 2026-11-09; the docs.rs rendering was read by Opus in its round-2 review with matching line numbers; the lead's earlier extraction of lines 210-230, 244 onward, 369, 376-377, and 380 onward also agrees. | Source inspection and derived reasoning, not a measurement; no claim about other versions. |
| `SRC-FST-0_4_7-STREAMER` | [`fst` 0.4.7 `src/stream.rs`](https://docs.rs/fst/0.4.7/src/fst/stream.rs.html) lines 1-14 (also in the local registry copy) and [`fst::Streamer`](https://docs.rs/fst/0.4.7/fst/trait.Streamer.html) | `Streamer` is a streaming iterator whose item borrows from the stream. Its doc comment explains that elements are produced by iterating the transducer and that allocating a fresh value per element would be too expensive, which is why the crate lends keys. Supports the lending-stream distinction in Frost's FST adapter. | Source inspected by this lane; the trait page was lead-checked. | Does not establish any Frost API. |

## `roaring` 0.12.0 (locked dependency)

| ID | Source | Supports | Checked | Limits |
| --- | --- | --- | --- | --- |
| `SRC-ROARING-0_12_0-SRC` | `roaring` 0.12.0 source on docs.rs: [`bitmap/ops.rs`](https://docs.rs/roaring/0.12.0/src/roaring/bitmap/ops.rs.html) lines 29, 56, 77, 98 and [`treemap/ops.rs`](https://docs.rs/roaring/0.12.0/src/roaring/treemap/ops.rs.html) lines 28, 49, 77, 98; `bitmap/inherent.rs` 712, 747 and `treemap/inherent.rs` 520, 549 in the same version (local Cargo registry copy; `Cargo.lock` pins 0.12.0) | `RoaringBitmap` and `RoaringTreemap` expose `intersection_len`, `union_len`, `difference_len`, `symmetric_difference_len` returning `u64`, plus `rank` and `select`. Supports the input claim that count-only kernels and rank/select exist without building a result bitmap. | Local copy inspected by this lane on 2026-11-09; the two docs.rs `ops.rs` pages were fetched the same day and show the same line numbers. | Upstream documentation pages (not source) were not retrieved (two failures recorded by the lead; not retried). Frost2's table names 0.11.5; the locked version is 0.12.0. Ascending-iteration and container-algorithm claims are not verified here. |

## Authorization and identity boundary sources

| ID | Source | Supports | Checked | Limits |
| --- | --- | --- | --- | --- |
| `SRC-ZANZIBAR-2019` | Pang et al., "Zanzibar: Google's Consistent, Global Authorization System", USENIX ATC 2019. [Publication page](https://research.google/pubs/zanzibar-googles-consistent-global-authorization-system/), [PDF](https://storage.googleapis.com/gweb-research2023-media/pubtools/5068.pdf) | Section 2.2 (Consistency Model, pp. 2-4) and section 2.4.4 (Check, p. 5): checks are evaluated at a consistent snapshot no earlier than the client-supplied token (zookie). Section 2.3.1 (Relation Configs and Userset Rewrites, p. 4): rewrite sub-expressions combine by union, intersection, and exclusion. Section 3.2.3 (Check Evaluation, p. 7): leaves evaluate concurrently and a subtree's evaluation is cancelled once the overall result is determined. Section 3.2.4 (Leopard, pp. 7-8): an offline periodic index plus an online layer; updates timestamped no later than the query timestamp are merged during query processing; index tuples are ordered integer lists such as a skip list; some group-membership checks reduce to a non-empty intersection. | Sections and pages read from the PDF by Opus on 2026-11-09 (round-1 report, pp. 2-7; round-2 report, p. 8). The lead's preparatory note had placed userset rewrites in section 2.2 and the snapshot rule in 2.4.3-2.4.4; this catalog adopted Opus's reading on 2026-11-09. This lane rechecked only the publication page (title, authors, venue, PDF link) and did not read the PDF. | No inference about Frost implementation or performance. Page numbers are as reported by Opus; a child report should confirm them against its own read. |
| `SRC-SPICEDB-SCHEMA` | [SpiceDB schema concepts](https://authzed.com/docs/spicedb/concepts/schema) | Permission expressions compose referenced relations or permissions with `+` (union), `&` (intersection of the subject sets), and `-` (exclusion, the difference of two subject sets). | Lead-checked; rechecked. | Mutable documentation without a version or last-updated date; not evidence of Zanzibar internals. |
| `SRC-GNAP-RFC9635` | [RFC 9635, Grant Negotiation and Authorization Protocol (GNAP)](https://www.rfc-editor.org/rfc/rfc9635.html), Standards Track, October 2024 | The abstract describes GNAP as a mechanism for delegating authorization to a piece of software and conveying the results of that delegation to it. | Lead-checked; rechecked (title, status, date, abstract). | Boundary check only; no protocol audit. |
| `SRC-DID-CORE-1_0` | [Decentralized Identifiers (DIDs) v1.0](https://www.w3.org/TR/did/), W3C Recommendation, 19 July 2022 (the `/TR/did-core/` URL redirects here) | Section 5.3, Verification Relationships, defines a verification relationship as the relationship between the DID subject and a verification method; relationships are purpose-specific. | Lead-checked; rechecked (title, status, section). | A DID is not a PeerId; cryptographic binding is a separate concern. |
| `SRC-LIBP2P-PEER-IDS` | [libp2p peer-ids specification](https://github.com/libp2p/specs/blob/master/peer-ids/peer-ids.md), status Active, latest revision r2 (2021-04-30), read from mutable `master` | A peer id is the multihash of the encoded public key: keys that serialize to more than 42 bytes are hashed with sha256, keys of at most 42 bytes use the identity multihash. | Inspected by this lane (the lead had retrieved but not inspected it). | Mutable branch, not a pinned commit. Nothing about DIDs or authorization. |

## Frost repository sources (baseline `82becb9`)

| ID | Source | Supports |
| --- | --- | --- |
| `SRC-FROST-CARGO` | [`Cargo.toml`](../../Cargo.toml), [`Cargo.lock`](../../Cargo.lock), [`rust-toolchain.toml`](../../rust-toolchain.toml), [`Makefile`](../../Makefile), [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml), [`.github/workflows/proofs.yml`](../../.github/workflows/proofs.yml) | One package `frost-collections` 0.4.0, library name `frost`, `rust-version = "1.99.1"`, edition 2024, toolchain channel 1.99.1; features including `fst-0_4`, `roaring-0_12`, `int-set`, `arc-swap-1`, `hashbrown-0_17`, `indexmap-2`, `imbl-7`; locked `fst` 0.4.7, `roaring` 0.12.0, `arc-swap` 1.9.4; `make check` = formatting, links, Clippy, rustdoc, tests, examples; embedded targets `thumbv7em-none-eabihf` and `thumbv6m-none-eabi`. |
| `SRC-FROST-SRC-READ` | [`src/read.rs`](../../src/read.rs) | `Len` (usize) and `Cardinality` (u64) are distinct (lines 56-73); `ValueElements`, `ValueRef`, `LookupValue` yield values or borrow values without inventing stored keys (80-109); `Elements`, `ElementType`, `MapTypes`, `LookupRef` (returns the stored representative key), `Entries` (119-178); `SetContains` promises no reference, `SetGet` returns the stored representative (211-220). |
| `SRC-FROST-SRC-FROZEN` | [`src/frozen.rs`](../../src/frozen.rs) | `Eligible` is sealed and gates `Default`/`FromIterator` (15-26); `Frozen<B>` keeps backend semantics, has no `Deref<Target = B>` or backend accessor, and leaves only through `into_inner` (28-103); compile-fail doctests for mutation. |
| `SRC-FROST-SRC-PLAN` | [`src/plan.rs`](../../src/plan.rs) | `Retain`, `Compact`, `Sorted`, `ExactLen<N>` plans (33-45); `FreezeWith` (52), `TryFreezeWith` (65), `Rejected` (83), closed `FreezeExt` with `seal` and `compact` (153). |
| `SRC-FROST-SRC-SNAPSHOT` | [`src/concurrent/snapshot.rs`](../../src/concurrent/snapshot.rs), [`src/concurrent/publication.rs`](../../src/concurrent/publication.rs) | Sealed `SnapshotValue` and its admitted payloads, including `FrozenIntSet` under `int-set` (62-84); `load` (204), `load_guard` (215), `peek` (242), unconditional `publish` (251), identity-compared `publish_if_current` returning the rejected value and an observed version (285-300), retrying `update` through the shared helper (330-334). |
| `SRC-FROST-SRC-INTEGER` | [`src/integer/key.rs`](../../src/integer/key.rs), [`src/integer/policy.rs`](../../src/integer/policy.rs), [`src/integer/set.rs`](../../src/integer/set.rs) | Safe `IntKey` codec (key.rs 65); `IntSetLimits` take the smaller limit for commutative operations and the left limits for difference (policy.rs 37-48); `IntSetPolicy::auto` with 8 MiB dense and 4,096 partition limits, explicit `sorted`/`dense`/`roaring` (85-139); `FrozenIntSet` (set.rs 44) with `u64` count kernels (299). |
| `SRC-FROST-SRC-FST-ADAPTER` | [`src/backends/fst.rs`](../../src/backends/fst.rs) | Lookups decode values (`LookupValue`), streams lend keys and cannot be exposed as `Entries`/`Elements`; retaining freeze restricted to audited immutable byte storage (1-65). |
| `SRC-FROST-SRC-BITMAP-ADAPTER` | [`src/backends/bitmap.rs`](../../src/backends/bitmap.rs) | Roaring wrappers count in `u64` (135); FixedBitSet wrappers count in `usize` (530). |
| `SRC-FROST-SRC-FAMILY-SETS` | [`src/family/hash_set.rs`](../../src/family/hash_set.rs), [`src/family/hash_set_ops.rs`](../../src/family/hash_set_ops.rs), [`src/family/ordered/set_ops.rs`](../../src/family/ordered/set_ops.rs), [`src/family/ordered/frozen.rs`](../../src/family/ordered/frozen.rs) | Implemented borrowed set algebra and scalar counts for the two frozen families; `try_from_unique` strict builders; `thaw` partners. |
| `SRC-FROST-TESTS` | [`tests/hash_set_ops.rs`](../../tests/hash_set_ops.rs), [`tests/ordered_set_ops.rs`](../../tests/ordered_set_ops.rs), [`tests/mutable_sets.rs`](../../tests/mutable_sets.rs), [`tests/specialized.rs`](../../tests/specialized.rs), [`tests/publication_model.rs`](../../tests/publication_model.rs), [`tests/concurrent/snapshot.rs`](../../tests/concurrent/snapshot.rs), [`tests/integer_sets.rs`](../../tests/integer_sets.rs), [`tests/integer_output.rs`](../../tests/integer_output.rs), [`tests/exhaustive_collections.rs`](../../tests/exhaustive_collections.rs) | Left-representative retention without cloning; FST lending streams; fixed pairs retain representatives and hashers; the production retry loop under a deterministic scheduler; integer policy, sharing, and canonical Hash events; finite exhaustive algebra. |
| `SRC-FROST-DOC-PHILOSOPHY` | [`docs/design/philosophy.md`](../design/philosophy.md) | First-representative/last-value laws; freezing as an ownership transition; capabilities by need; additive features; evidence scope. |
| `SRC-FROST-DOC-ARCHITECTURE` | [`docs/design/architecture.md`](../design/architecture.md) | Family pairs and storage transitions; semantic laws and coherence; exact freezing and plans; adaptive profiles; backend capabilities; concurrency and snapshots; feature boundaries. |
| `SRC-FROST-DOC-DECISIONS` | [`docs/design/decisions.md`](../design/decisions.md) | Why one implementation, fixed pairs, no feature-selected storage, and the revision 2 integer decisions. |
| `SRC-FROST-DOC-SET-OPERATIONS` | [`docs/set-operations.md`](../set-operations.md) | Implemented frozen set algebra: receivers, borrowed iterators, left representatives, cost table, dense universe rules, historical measurements. |
| `SRC-FROST-DOC-SET-ALGEBRA` | [`docs/design/set-algebra.md`](../design/set-algebra.md) | Proposed typed views; membership/representative/order separation; rewrite table; terminal-specific execution; n-ary probe cost; materialization; counts and bounds; persistence; delivery gates. Not implemented. |
| `SRC-FROST-DOC-COLLECTION-ALGEBRA` | [`docs/design/collection-algebra.md`](../design/collection-algebra.md) | Earlier synthesis of Frost1/Frost2: distinct logical contracts, single-library constraint, implementation sequence. |
| `SRC-FROST-DOC-SNAPSHOTS` | [`docs/design/snapshots.md`](../design/snapshots.md) | Snapshot operation table, identity and linearization, callback and drop behaviour, reader lifetime. |
| `SRC-FROST-DOC-INTEGER-SETS` | [`docs/design/integer-sets.md`](../design/integer-sets.md) | Typed integer profile: codecs, sealed words, policies, shared storage, Chunked. |
| `SRC-FROST-DOC-INTEGER-VIEWS` | [`docs/design/integer-views.md`](../design/integer-views.md) | Proposed `IntSetExpr` borrowed integer expressions. Not implemented. |
| `SRC-FROST-DOC-PERSISTENT` | [`docs/design/persistent-integer-storage.md`](../design/persistent-integer-storage.md) | Flat shared chunks implemented; tree directories and a separate persistent family proposed. |
| `SRC-FROST-DOC-VERIFICATION` | [`docs/verification.md`](../verification.md) | Make targets, finite models, snapshot retry model, Miri, embedded, Loom, Kani scope and limits. |
| `SRC-FROST-DOC-LAZY-VIEWS-SOURCE-AUDIT` | [`docs/reviews/lazy-views-source-audit.md`](../reviews/lazy-views-source-audit.md) | Earlier versioned checks of Rust `BTreeSet`, `fst` heap structure (F2), Lucene, Tantivy; the deduction that documented linear complexity is not independence from fan-in. |
| `SRC-FROST-DOC-GENERIC-REVIEW` | [`docs/reviews/generic-set-algebra-review.md`](../reviews/generic-set-algebra-review.md) | Findings on universal bounds, FST references, dyn compatibility, n-ary probes; executed baseline; existing executable contracts by test name. |
| `SRC-FROST-VERIFICATION-SET-ALGEBRA` | [`verification/set-algebra/README.md`](../../verification/set-algebra/README.md) | Independent finite oracle reproducing representative and order counterexamples. |
| `SRC-FROST-ATTACHMENT-RECORD` | [`verification/attachment-integration-2026-11-09/README.md`](../../verification/attachment-integration-2026-11-09/README.md) | Byte-exact preservation of Frost1/Frost2 and the checks executed on that earlier checkout. |

## Bibliography supplied by the inputs (not verified here)

Frost2 (lines 572-591 of the raw file) lists a prioritized source table with
ordinary URLs. These are reproduced as **input-supplied references**. None was
read in this integration; their relevance claims remain the input's. A DOI is
recorded exactly as supplied.

| ID | As supplied by Frost2 | Status |
| --- | --- | --- |
| `SRC-INPUT-RUST-COLLECTIONS-DOCS` | Rust collection documentation, https://doc.rust-lang.org/std/collections/ | Not read here; std 1.99.1 pages above cover the specific claims used. |
| `SRC-INPUT-HASHBROWN-DOCS` | https://docs.rs/crate/hashbrown/latest | Not read. Frost2 names 0.17.1; `Cargo.toml` requires `0.17`. |
| `SRC-INPUT-INDEXMAP-DOCS` | https://docs.rs/crate/indexmap/latest | Not read. Frost2 names 2.14.2; `Cargo.toml` requires `2.14`; the set-algebra design checked 2.14.0 (S8). |
| `SRC-INPUT-IMBL-DOCS` | https://docs.rs/crate/imbl/latest | Not read. Frost2 names 7.0.2; the set-algebra design checked 7.0.2 (S4, S5). |
| `SRC-INPUT-FST-DOCS` | https://docs.rs/fst/latest/fst/ and the `OpBuilder` page | Covered by `SRC-FST-0_4_7-*`. |
| `SRC-INPUT-BIMAP-DOCS` | https://docs.rs/bimap/latest/bimap/ (0.6.3 per Frost2) | Not read; not a Frost dependency. |
| `SRC-INPUT-BIDIMAP-DOCS` | https://docs.rs/crate/bidimap/latest (0.7.0 per Frost2) | Not read; not a Frost dependency. |
| `SRC-INPUT-MULTIMAP-DOCS` | https://docs.rs/multimap/latest/multimap/ (0.10.1 per Frost2) | Not read; not a Frost dependency. |
| `SRC-INPUT-MULTI-BIMAP-DOCS` | https://docs.rs/crate/multi_bimap/latest (0.8.0 per Frost2) | Not read; not a Frost dependency. |
| `SRC-INPUT-ROARING-DOCS` | https://docs.rs/crate/roaring/latest (0.11.5 per Frost2) | Not read; locked source 0.12.0 inspected instead (`SRC-ROARING-0_12_0-SRC`). |
| `SRC-INPUT-ALBERT-1991` | Albert, *Algebraic Properties of Bag Data Types*, VLDB 1991, https://sigmod.org/publications/dblp/db/conf/vldb/Albert91.html | Not read. Cited by both inputs for bag laws. |
| `SRC-INPUT-HERNICH-KOLAITIS-2017` | Hernich and Kolaitis, *Foundations of Information Integration under Bag Semantics*, https://doi.org/10.1109/LICS.2017.8005104 | Not read. |
| `SRC-INPUT-ART-2013` | Leis, Kemper and Neumann, *The Adaptive Radix Tree*, https://doi.org/10.1109/ICDE.2013.6544812 | Not read. |
| `SRC-INPUT-ROARING-PAPER` | Lemire et al., *Roaring Bitmaps: Implementation of an Optimized Software Library*, https://luca.ntop.org/roaring.pdf | Not read; the supplied URL is a third-party host, not the publisher. |
| `SRC-INPUT-RRR-2002` | Raman, Raman and Rao, *Succinct Indexable Dictionaries*, https://doi.org/10.1145/545381.545411 | Not read. |
| `SRC-INPUT-DACIUK-2000` | Daciuk et al., *Incremental construction of minimal acyclic finite state automata*, https://aclanthology.org/J00-1002/ | Not read. |

The license and "maintenance signal" columns in Frost2's ecosystem table are
likewise unverified; they are dated claims of unknown date.

## Prior-art survey sources (2026-11-12)

These rows come from the
[prior-art survey](reports/value-domain-algebra-prior-art.md) of the
value-domain algebra. Three check levels apply, and a row never claims a
level it did not reach:

- **Integration-checked**: the integrator fetched the page on 2026-11-12 (UTC)
  and read the stated passage through a text extraction of the page. Section
  and theorem numbers are as that extraction reported them.
- **Survey-read**: the survey states that it read the page or abstract. The
  integrator's fetch failed or was not attempted, as the row says.
- **Unread**: the survey cites the work from prior knowledge. Nothing may be
  stated from it as fact.

### Integration-checked

| ID | Source | Supports | Limits |
| --- | --- | --- | --- |
| `SRC-PRIOR-LAWSON-PRIMER` | Lawson, "Primer on Inverse Semigroups I", [arXiv:2006.01628](https://arxiv.org/pdf/2006.01628) | Section 2.3: `s ~ t` iff `s⁻¹t` and `st⁻¹` are idempotents; the relation is reflexive and symmetric, and transitive exactly for E-unitary semigroups (Proposition 2.17). Compatible partial bijections have a union that is a partial bijection. `s ⊥ t` iff both products are zero; an orthogonal union in I(X) is a disjoint union. Section 2.1: the idempotents of I(X) are the partial identities `1_A`, with `1_A·1_B = 1_(A∩B)`. Theorem 2.8 (Wagner–Preston): every inverse semigroup embeds in a symmetric inverse monoid. | The converse (a partial-bijection union implies compatibility) is not a standalone statement in the part read; the algebra chapter proves it directly and checks it on three keys. No statement that multiplication distributes over compatible joins was found, so the survey's claim to that effect is not attributed to this source. |
| `SRC-PRIOR-SUCIU-DIFFERENCES` | Suciu, "Different Differences in Semirings", OASIcs, Tannen's Festschrift, article 10, pp. 10:1–10:20, [doi:10.4230/OASIcs.Tannen.10](https://doi.org/10.4230/OASIcs.Tannen.10), [PDF](https://drops.dagstuhl.de/storage/01oasics/oasics-vol119-tannens-festschrift/OASIcs.Tannen.10/OASIcs.Tannen.10.pdf) | Section 4.3: axiom (A5) is `(b ∸ a)·c = b·c ∸ a·c`. It holds in `(N,+,·,0,1,∸)` and in `N[X]` and fails in a three-element max/min m-semiring; the failure is attributed to Amsterdamer, Deutch and Tannen (2011). Where (A5) fails, rewriting `(R − S) ⋈ T` to `R ⋈ T − S ⋈ T` is unsound. Theorem 21: the identities of `(N,+,·,0,1,∸)` are co-r.e. complete, hence not finitely axiomatizable. | Publication year not shown in the text read. **Corrects the survey**, which said that bag difference fails (A5): it holds for natural-number bags. |
| `SRC-PRIOR-GEERTS-POGGI` | Geerts and Poggi, "On Database Query Languages for K-relations", [author copy](https://adrem.uantwerpen.be/bibrem/pubs/GeertsA.pdf) | An m-semiring is a naturally ordered commutative semiring in which `{z : x ≤ y ⊕ z}` always has a least element; `x ⊖ y` is that element. On `N` it is truncated subtraction; on Boolean algebras it behaves like set difference. | Venue and year not shown in the copy read. |
| `SRC-PRIOR-DATALOGO` | Abo Khamis, Ngo, Pichler, Suciu and Wang, "Convergence of Datalog over (Pre-) Semirings", [arXiv:2105.14435](https://arxiv.org/pdf/2105.14435) (version dated December 2022) | Theorem 1.2: every datalog° program converges iff the core semiring is stable, and in a number of steps bounded by the active domain iff it is p-stable. `u` is p-stable when `1 ⊕ u ⊕ … ⊕ uᵖ = 1 ⊕ u ⊕ … ⊕ uᵖ⁺¹`. `N` is naturally ordered and not stable; `x := 1 + 2x` diverges. Semi-naive evaluation generalizes under further restrictions. | The first 100,000 characters were read. The exact class for semi-naive evaluation and the journal venue (the survey gives PODS 2022 and JACM 71(2), 2024) were not checked. |
| `SRC-PRIOR-DBSP` | Budiu, Chajed, McSherry, Ryzhyk and Tannen, "DBSP: Automatic Incremental View Maintenance for Rich Query Languages", PVLDB 16(7), pp. 1601–1614, 2023, [PDF](https://www.vldb.org/pvldb/vol16/p1601-budiu.pdf) | Section 4.1: a Z-set is a finite-support function into the integers; Z-sets form an abelian group. Definition 4.1: sets are Z-sets with all weights one. Definition 4.2: bags are the Z-sets with no negative weight. Definition 4.3: `distinct`. A change is the difference of consecutive snapshots, so deletions carry negative weights. | Says nothing about Frost's representation or about bounded counts. |
| `SRC-PRIOR-GUAVA-MULTIMAP` | Guava 23.0 [`Multimap`](https://guava.dev/releases/23.0/api/docs/com/google/common/collect/Multimap.html) Javadoc | `asMap()` maps each distinct key to a nonempty collection. `get` on an absent key returns an empty view, not null. Removing a key's last value removes the key from every view. Equality is equality of `asMap()` views; a nonempty `SetMultimap` never equals a nonempty `ListMultimap`; any two empty multimaps are equal. | One release of one library; not a Frost dependency. |
| `SRC-PRIOR-GUAVA-BIMAP` | Guava 33.0.0-jre [`BiMap`](https://guava.dev/releases/33.0.0-jre/api/docs//com/google/common/collect/BiMap.html) Javadoc | `put` throws when the value is bound to another key and leaves the bimap unchanged. `forcePut` removes the conflicting entry silently. `putAll` can leave earlier entries inserted when a later one throws, and its outcome can depend on input order. | One release of one library; not a Frost dependency. |
| `SRC-PRIOR-PY-COUNTER` | Python 3.15.2 [`collections.Counter`](https://docs.python.org/3/library/collections.html) | `+` adds counts, `-` subtracts and keeps positive results, `&` is min, `\|` is max, and `^` (added in 3.15) is max minus min. Operator results drop counts of zero or below. `subtract()` keeps zero and negative counts. | A Python type; cited as a design contrast only. |
| `SRC-PRIOR-LEAN-MAPS` | Lean language reference, [Maps and Sets](https://lean-lang.org/doc/reference/latest/Basic-Types/Maps-and-Sets/) (`latest`, no version shown) | Bundled maps carry their well-formedness invariant; `Raw` variants do not, and each raw operation `T.Raw.f` has a lemma `T.Raw.WF.f`. Basic operations are verified against list models; some, such as `HashMap.partition`, are not yet. `EquivBEq` and `LawfulHashable` state the key laws; with a non-reflexive `BEq`, `isEmpty` can be false while nothing can be retrieved. | Mutable `latest` page; the first 100,000 characters were read. |
| `SRC-PRIOR-BIRD-PRG69` | Bird, *Lectures on Constructive Functional Programming*, Technical Monograph PRG-69, Oxford, 1988, [PDF](https://www.cs.ox.ac.uk/files/3390/PRG69.pdf) | Sections 1.2–1.3: lists are the free monoid on singletons; bags add commutativity and sets add idempotence. Section 1.3: the bag size equations fail for sets. Section 1.5: head and last are not defined on bags and sets. Section 2.1: every list homomorphism is a reduction after a map. | The tree level of the hierarchy and the name "Boom hierarchy" (Meertens 1986, per the survey) were not checked here. |
| `SRC-PRIOR-LIBKIN-2016` | Libkin, "SQL's Three-Valued Logic and Certain Answers", ACM TODS, [author copy](https://homepages.inf.ed.ac.uk/libkin/papers/tods16.pdf) | Certain answers are the correctness standard for incomplete data. SQL's three-valued evaluation can be unrelated to them and can return answers that are not certain. A modified evaluation that classifies answers as certainly true, certainly false or unknown returns no false positives; false negatives remain possible. | Volume and year not shown in the copy read (the survey gives 41(1), 2016). |
| `SRC-PRIOR-CEDAR-2024` | Cutler et al., "Cedar: A New Language for Expressive, Fast, Safe, and Analyzable Authorization (Extended Version)", [arXiv:2403.04651](https://arxiv.org/pdf/2403.04651) | Cedar's semantics and validator are formalized in Lean with proved properties. The Rust implementation is compared with the compiled Lean model by differential random testing. The SMT encoding is stated to be sound, complete and decidable, proved on paper; the Lean proofs of the encoder were under way at the time of writing. | The first 100,000 characters were read. The phrase "verification-guided development" is not in this paper; the survey also cites Disselkoen et al., FSE 2024, which was not read. |
| `SRC-PRIOR-SPICEDB-CVE-2026-55866` | [Nixpkgs security tracker issue 563480](https://github.com/NixOS/nixpkgs/issues/563480) (opened 2026-09-15) and the [GitLab advisory database page for SpiceDB](https://advisories.gitlab.com/pkg/golang/github.com/authzed/spicedb/) (generated 2026-11-12) | CVE-2026-55866 (GHSA-4vrg-r928-h5vv, CVSS 3.1 score 3.7): the dispatch cache key for Check requests left out the check hints, so two different checks could share an entry and a check could return full permission where the answer was conditional or negative. It needs an intersection or exclusion over caveated and uncaveated paths, a concurrent lookup with a context, and the dispatch cache. Fixed in 1.54.0. | Secondary pages, not the vendor advisory. The tracker gives the affected range as 1.34.0 through 1.54.0 and the fix as 1.54.0; the survey gives 1.34.0 to 1.53.x. The CWE the survey names is not on either page. |
| `SRC-PRIOR-SPICEDB-ADVISORIES` | [GitLab advisory database page for SpiceDB](https://advisories.gitlab.com/pkg/golang/github.com/authzed/spicedb/) (generated 2026-11-12) | One-line summaries: CVE-2026-46668 (nested-list caveats and improper cache reuse), CVE-2025-65111 (incomplete lookup results for a union naming one relation twice), CVE-2023-35930 (wrong negative decisions from lookup results), CVE-2022-21646 (lookups ignoring wildcards under an exclusion or intersection). | Summaries only; no advisory text was read. Evidence that these operators are where defects were reported, not a measure of frequency. |

`SRC-BIMAP-063` in the [supplement sources](formal-models/SOURCES.md) was also
rechecked on 2026-11-12: `insert` reports `Neither`, `Left`, `Right`, `Both` or
`Pair` and can shrink the map; `insert_no_overwrite` returns the rejected pair
and leaves the map unchanged. The survey's statement that the `bimap` page
calls mutation through `Cell` or `RefCell` a logic error was **not** found on
that page.

### Survey-read, not reread at integration

| ID | Source | The survey reports | Why not reread |
| --- | --- | --- | --- |
| `SRC-PRIOR-ICF-2010` | Lammich and Lochbihler, "The Isabelle Collections Framework", ITP 2010, [author copy](https://www21.in.tum.de/~lammich/pub/itp10.pdf) | Abstract sets and maps refined to red-black trees and hash tables by data refinement; refinement relations rather than functions | The fetch returned a client error. |
| `SRC-PRIOR-SEMIALIGN` | `semialign` 1.3.1, [`Data.Semialign`](https://hackage.haskell.org/package/semialign-1.3.1/docs/Data-Semialign.html) | `align` is commutative up to `swap` and associative up to `assoc`; distribution between `zip` and `align` is listed as not a law | The host disallows automated fetches. |
| `SRC-PRIOR-CONTAINERS-MERGE` | `containers`, [`Data.Map.Merge.Strict`](https://hackage.haskell.org/package/containers/docs/Data-Map-Merge-Strict.html) | `merge` takes two missing-key tactics and one matched-key tactic; union, intersection, difference and symmetric difference are tactic choices; `mergeA` is the effectful form | The host disallows automated fetches. |
| `SRC-PRIOR-VERUS-3176` | [Verus issue 3176](https://github.com/verus-lang/verus/issues/3176) | A key model is specified for the stored key type and not for the borrowed lookup type | Rate-limited twice. Issue state is unknown. |
| `SRC-PRIOR-VERUS-VSTD` | Verus `vstd` [`HashMapWithView`](https://verus-lang.github.io/verus/verusdoc/vstd/hash_map/struct.HashMapWithView.html) | A trusted wrapper whose `View` maps a concrete map to a mathematical map, guarded by `obeys_key_model` | Not attempted. |
| `SRC-PRIOR-AENEAS-2022` | Ho and Protzenko, "Aeneas: Rust verification by functional translation", ICFP 2022, [doi:10.1145/3547647](https://doi.org/10.1145/3547647) | Safe Rust translated to pure functional models | Not attempted. The hash-map case study is unread even by the survey. |
| `SRC-PRIOR-ZWART-MARSDEN` | Zwart and Marsden, "No-Go Theorems for Distributive Laws", [arXiv:2003.12531](https://arxiv.org/pdf/2003.12531) | Which collection monads compose through distributive laws | Not attempted. |
| `SRC-PRIOR-FEGARAS-MAIER-2000` | Fegaras and Maier, "Optimizing object queries using an effective calculus", ACM TODS 25(4), 2000, [doi:10.1145/377674.377676](https://doi.org/10.1145/377674.377676) | A monoid comprehension calculus over sets, bags and lists | Not attempted. |
| `SRC-PRIOR-CHAUDHURI-VARDI-1993` | Chaudhuri and Vardi, "Optimization of real conjunctive queries", PODS 1993, [doi:10.1145/153850.153856](https://doi.org/10.1145/153850.153856) | Bag equivalence of conjunctive queries is isomorphism; a hardness claim for bag containment was later withdrawn and decidability is reported open | Not attempted. The two later papers the survey quotes were not read either. |
| `SRC-PRIOR-HOTTSQL-2017` | Chu, Weitz, Cheung and Suciu, "HoTTSQL", PLDI 2017, [doi:10.1145/3062341.3062348](https://doi.org/10.1145/3062341.3062348) | Machine-checked semantics and rewrite proofs for SQL with bags | Not attempted. |
| `SRC-PRIOR-SPES-2022` | Zhou et al., "SPES", ICDE 2022, [doi:10.1109/ICDE53745.2022.00250](https://doi.org/10.1109/ICDE53745.2022.00250) | Bag-semantics equivalence proving; counts of proved rewrite pairs against two earlier provers | Not attempted. The counts are the survey's and are not repeated in design documents. |
| `SRC-PRIOR-DEMAINE-2000` | Demaine, López-Ortiz and Munro, "Adaptive set intersections, unions, and differences", SODA 2000, [author page](https://erikdemaine.org/papers/SODA2000/) | Adaptive merge algorithms in the comparison model | Not attempted. |

The survey also reports reading `SRC-ZANZIBAR-2019` and `SRC-SPICEDB-SCHEMA`,
which this catalog already holds. Its statement about operator precedence in
SpiceDB schemas is not in the existing row and was not rechecked.

### Unread (cited by the survey from prior knowledge)

No claim in a design document rests on these. The survey marks each as
inferred, or as bibliography confirmed with content from memory.

| ID | Work as the survey cites it |
| --- | --- |
| `SRC-PRIOR-HOARE-1972` | Hoare, "Proof of correctness of data representations", Acta Informatica 1(4), 1972 |
| `SRC-PRIOR-HE-HOARE-SANDERS-1986` | He, Hoare and Sanders, "Data refinement refined", ESOP 1986 |
| `SRC-PRIOR-LISKOV-GUTTAG-1986` | Liskov and Guttag, *Abstraction and Specification in Program Development*, 1986 |
| `SRC-PRIOR-GREEN-2007` | Green, Karvounarakis and Tannen, "Provenance semirings", PODS 2007, doi:10.1145/1265530.1265535 (bibliography confirmed by the survey; content from memory) |
| `SRC-PRIOR-IMIELINSKI-LIPSKI-1984` | Imieliński and Lipski, "Incomplete information in relational databases", JACM 31(4), 1984, doi:10.1145/1634.1886 (bibliography confirmed; content from memory) |
| `SRC-PRIOR-GRUMBACH-MILO-1996` | Grumbach and Milo, "Towards tractable algebras for bags", JCSS 52(3), 1996, doi:10.1006/jcss.1996.0042 (bibliography confirmed; content from memory) |
| `SRC-PRIOR-USEMIRING-2018` | Chu, Murphy, Roesch, Cheung and Suciu, PVLDB 11(11), 2018, doi:10.14778/3236187.3236200 (bibliography confirmed; content partly from memory) |
| `SRC-PRIOR-COSETTE-2017` | Chu, Wang, Weitz and Cheung, "Cosette", CIDR 2017 (bibliography confirmed) |
| `SRC-PRIOR-UNREAD-OTHER` | Meertens 1986; Buneman, Naqvi, Tannen and Wong 1995; Okasaki's Edison; Coq MSets; Why3, Dafny, VeriFast, Creusot, Prusti and Kani; Java `Comparator` documentation; Haskell `containers` left bias; `hashbrown` and `equivalent`; `ordered-float`; King's "Parse, don't validate"; Liquid Haskell and F*; `nutype`, `garde` and `validator`; Unicode UAX #15; Davey and Priestley; Binnig, Hildenbrand and Färber 2009; Guava `Multiset`, Eclipse Collections, `hashbag`; the SQL standard's bag operators; differential dataflow papers; Apt, Blair and Walker 1988; Apache Commons `BidiMap`; Haskell `bimap`; Foster et al. 2007; cats `Align` and `Ior`; itertools `EitherOrBoth`; Coutts, Leshchinskiy and Stewart 2007; Ellis and Gibbs 1989; Clojure transients; .NET frozen collections; kotlinx immutable collections; `im`, `rpds`, `imbl`; `Object.freeze`; Kleene and Belnap logics; Pawlak rough sets; OpenFGA and Ory Keto; O'Neil et al. 1996; EQUITAS; egg and egglog; Baeza-Yates 2004; Lemire, Boytsov and Kurz 2016; QuickCheck and its law-suite libraries; `proptest` |

`SRC-INPUT-ALBERT-1991` and `SRC-INPUT-ROARING-PAPER` above stay unread. The
survey confirms Albert's bibliography only, and reports reading the two
Roaring papers (doi:10.1002/spe.2325 and doi:10.1002/spe.2560) through a
subagent; neither was reread.

## Unaudited influences and gaps

The following were named by the task or the inputs but have **no primary
source registered here**. Curated documents label claims about them
"influence only" or "unverified". Nothing about their protocols, data
shapes, or guarantees may be stated from memory.

| Gap | Named by | Status |
| --- | --- | --- |
| Macaroons, Vanadium blessings and caveats, OAuth, UCAN, ZCAP, Kameo actor system, libp2p beyond peer ids | Task handoff | Influence only; no source read. |
| OpenFGA userset schema | Frost3 | Unverified; SpiceDB half of the same sentence verified (`SRC-SPICEDB-SCHEMA`). |
| Differential Dataflow (incremental maintenance, negative differences) | Frost3 | Unverified. Signed weights in DBSP are covered by `SRC-PRIOR-DBSP`; that is a different paper. |
| Guava `Multiset`/`Multimap`/`BiMap`, Java `Map.merge`/`ConcurrentHashMap`, C++ `set_union`/`merge`, Haskell `unionWith`, OCaml `Map.merge`, Clojure `merge-with`, Scala `updatedWith`, PostgreSQL `UNION ALL` | Frost1 | Unverified cross-language claims; plausible but unchecked. Guava `Multimap` and `BiMap` Javadoc were read on 2026-11-12 (`SRC-PRIOR-GUAVA-MULTIMAP`, `SRC-PRIOR-GUAVA-BIMAP`); the rest remain unread. |
| Codd's relational model, Bagwell's HAMT, `proptest`/QuickCheck, Kani, Loom documentation claims | Frost1 | Unverified as cited; Frost's own Loom and Kani harnesses are documented in `SRC-FROST-DOC-VERIFICATION`. |
| `hashbrown` raw-entry warnings, `indexmap` order semantics, `imbl` persistence claims | Frost1, Frost2 | Partially covered by the set-algebra design's S4, S5, S8 checks; raw-entry warning unverified. |
| Leopard algorithm details beyond the sections and pages listed | Frost3 | Section- and page-level citations recorded from Opus's read; deeper algorithmic detail unaudited. |
| The complete computed-set report, research template, and ZIP that Frost3 links to; a standalone algebraic-foundations report | Frost3, task | Not supplied; recovery attempts recorded in the [integration audit](integration-audit.md). |

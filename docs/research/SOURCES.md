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
| `SRC-FROST-SRC-READ` | [`src/read.rs`](../../crates/frost-collections-core/src/read.rs) | `Len` (usize) and `Cardinality` (u64) are distinct (lines 56-73); `ValueElements`, `ValueRef`, `LookupValue` yield values or borrow values without inventing stored keys (80-109); `Elements`, `ElementType`, `MapTypes`, `LookupRef` (returns the stored representative key), `Entries` (119-178); `SetContains` promises no reference, `SetGet` returns the stored representative (211-220). |
| `SRC-FROST-SRC-FROZEN` | [`src/frozen.rs`](../../crates/frost-collections-core/src/frozen.rs) | `Eligible` is sealed and gates `Default`/`FromIterator` (15-26); `Frozen<B>` keeps backend semantics, has no `Deref<Target = B>` or backend accessor, and leaves only through `into_inner` (28-103); compile-fail doctests for mutation. |
| `SRC-FROST-SRC-PLAN` | [`src/plan.rs`](../../crates/frost-collections-core/src/plan.rs) | `Retain`, `Compact`, `Sorted`, `ExactLen<N>` plans (33-45); `FreezeWith` (52), `TryFreezeWith` (65), `Rejected` (83), closed `FreezeExt` with `seal` and `compact` (153). |
| `SRC-FROST-SRC-SNAPSHOT` | [`src/concurrent/snapshot.rs`](../../crates/frost-collections/src/concurrent/snapshot.rs), [`src/concurrent/publication.rs`](../../crates/frost-collections/src/concurrent/publication.rs) | Sealed `SnapshotValue` and its admitted payloads, including `FrozenIntSet` under `int-set` (62-84); `load` (204), `load_guard` (215), `peek` (242), unconditional `publish` (251), identity-compared `publish_if_current` returning the rejected value and an observed version (285-300), retrying `update` through the shared helper (330-334). |
| `SRC-FROST-SRC-INTEGER` | [`src/integer/key.rs`](../../crates/frost-collections-integer/src/integer/key.rs), [`src/integer/policy.rs`](../../crates/frost-collections-integer/src/integer/policy.rs), [`src/integer/set.rs`](../../crates/frost-collections-integer/src/integer/set.rs) | Safe `IntKey` codec (key.rs 65); `IntSetLimits` take the smaller limit for commutative operations and the left limits for difference (policy.rs 37-48); `IntSetPolicy::auto` with 8 MiB dense and 4,096 partition limits, explicit `sorted`/`dense`/`roaring` (85-139); `FrozenIntSet` (set.rs 44) with `u64` count kernels (299). |
| `SRC-FROST-SRC-FST-ADAPTER` | [`src/backends/fst.rs`](../../crates/frost-collections-core/src/backends/fst.rs) | Lookups decode values (`LookupValue`), streams lend keys and cannot be exposed as `Entries`/`Elements`; retaining freeze restricted to audited immutable byte storage (1-65). |
| `SRC-FROST-SRC-BITMAP-ADAPTER` | [`src/backends/bitmap.rs`](../../crates/frost-collections-core/src/backends/bitmap.rs) | Roaring wrappers count in `u64` (135); FixedBitSet wrappers count in `usize` (530). |
| `SRC-FROST-SRC-FAMILY-SETS` | [`src/family/hash_set.rs`](../../crates/frost-collections-core/src/family/hash_set.rs), [`src/family/hash_set_ops.rs`](../../crates/frost-collections-core/src/family/hash_set_ops.rs), [`src/family/ordered/set_ops.rs`](../../crates/frost-collections-core/src/family/ordered/set_ops.rs), [`src/family/ordered/frozen.rs`](../../crates/frost-collections-core/src/family/ordered/frozen.rs) | Implemented borrowed set algebra and scalar counts for the two frozen families; `try_from_unique` strict builders; `thaw` partners. |
| `SRC-FROST-TESTS` | [`tests/hash_set_ops.rs`](../../crates/frost-collections/tests/hash_set_ops.rs), [`tests/ordered_set_ops.rs`](../../crates/frost-collections/tests/ordered_set_ops.rs), [`tests/mutable_sets.rs`](../../crates/frost-collections/tests/mutable_sets.rs), [`tests/specialized.rs`](../../crates/frost-collections/tests/specialized.rs), [`tests/publication_model.rs`](../../crates/frost-collections/tests/publication_model.rs), [`tests/concurrent/snapshot.rs`](../../crates/frost-collections/tests/concurrent/snapshot.rs), [`tests/integer_sets.rs`](../../crates/frost-collections/tests/integer_sets.rs), [`tests/integer_output.rs`](../../crates/frost-collections/tests/integer_output.rs), [`tests/exhaustive_collections.rs`](../../crates/frost-collections/tests/exhaustive_collections.rs) | Left-representative retention without cloning; FST lending streams; fixed pairs retain representatives and hashers; the production retry loop under a deterministic scheduler; integer policy, sharing, and canonical Hash events; finite exhaustive algebra. |
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

## Unaudited influences and gaps

The following were named by the task or the inputs but have **no primary
source registered here**. Curated documents label claims about them
"influence only" or "unverified". Nothing about their protocols, data
shapes, or guarantees may be stated from memory.

| Gap | Named by | Status |
| --- | --- | --- |
| Macaroons, Vanadium blessings and caveats, OAuth, UCAN, ZCAP, Kameo actor system, libp2p beyond peer ids | Task handoff | Influence only; no source read. |
| OpenFGA userset schema | Frost3 | Unverified; SpiceDB half of the same sentence verified (`SRC-SPICEDB-SCHEMA`). |
| Differential Dataflow (incremental maintenance, negative differences) | Frost3 | Unverified. |
| Guava `Multiset`/`Multimap`/`BiMap`, Java `Map.merge`/`ConcurrentHashMap`, C++ `set_union`/`merge`, Haskell `unionWith`, OCaml `Map.merge`, Clojure `merge-with`, Scala `updatedWith`, PostgreSQL `UNION ALL` | Frost1 | Unverified cross-language claims; plausible but unchecked. |
| Codd's relational model, Bagwell's HAMT, `proptest`/QuickCheck, Kani, Loom documentation claims | Frost1 | Unverified as cited; Frost's own Loom and Kani harnesses are documented in `SRC-FROST-DOC-VERIFICATION`. |
| `hashbrown` raw-entry warnings, `indexmap` order semantics, `imbl` persistence claims | Frost1, Frost2 | Partially covered by the set-algebra design's S4, S5, S8 checks; raw-entry warning unverified. |
| Leopard algorithm details beyond the sections and pages listed | Frost3 | Section- and page-level citations recorded from Opus's read; deeper algorithmic detail unaudited. |
| The complete computed-set report, research template, and ZIP that Frost3 links to; a standalone algebraic-foundations report | Frost3, task | Not supplied; recovery attempts recorded in the [integration audit](integration-audit.md). |

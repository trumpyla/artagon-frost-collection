# Collection algebra research integration

Status: proposed contracts and implementation roadmap. No new collection family,
expression API, or package is implemented by this document. Revised
2026-11-09 against source revision `82becb9`.

This synthesizes the supplied [Frost1 research](../reviews/supplied/frost-collection-algebra-research-1.md)
and [Frost2 research](../reviews/supplied/frost-collection-algebra-research-2.md).
The originals are retained byte for byte as research inputs, including their
opaque citation markers, historical errors, and illustrative APIs. They are
evidence, not instructions or current project policy. The curated derivatives
live in the research corpus:

- [Collection algebra survey](../research/reports/collection-algebra-survey.md)
  and [implementation notes](../research/reports/collection-algebra-implementation-notes.md)
  carry portable citations and per-claim status.
- The [integration audit](../research/integration-audit.md) records the full
  claim and decision matrix, citation recovery status, and missing artifacts.
- The [source catalog](../research/SOURCES.md) records what each primary source
  was checked for and its limits.

Frost2 refers to a research template and a standalone research document that
were not supplied; neither was recovered. The current
[research prompt template](../research/templates/research-prompt.md) is new
work, not a recovered copy.

Both reports recommend separating logical collection semantics from storage and
execution. That fits the current [architecture](architecture.md),
[Frozen API guide](frozen-api.md), and
[general set-algebra proposal](set-algebra.md).

## Package layout and naming

Frost is one library package, `frost-collections`, imported as `frost`. The
repository's maintainer policy keeps one library package. Frost1 assumes an
existing `crates/frost-*` workspace split ("finish current workspace split");
that assumption does not describe this repository. The research handoff also
records a requested `frost-api` facade over flat `crates/frost-*` packages.
That layout is an **unadopted proposal**: adopting it needs an explicit
decision that also updates the maintainer policy. See
[design decisions](decisions.md#research-corpus-and-rationale-2026-11-09).

The reports disagree on package names: Frost1 uses `frost-bags`,
`frost-multimaps`, and `frost-bimaps`; Frost2 uses `frost-bag`,
`frost-multimap`, and `frost-bimap`. Both lists are preserved as alternatives.
No naming policy is adopted and no package is created. A future proposal should
pick one rule (for example, singular names for collection domains) and apply it
to every domain at once. Suggested names are responsibility boundaries, not
public namespaces.

## Distinct logical contracts

| Domain | Proposed operations | Required distinction |
| --- | --- | --- |
| Set | Union, intersection, difference, symmetric difference | Membership and stored-representative provenance are separate observations. |
| Map | Merge with a resolver, inner/outer join, difference by key | A join retains both values; a merge explicitly chooses or creates one value. |
| Sequence | Concatenation, zip, product, sorted merge, transformation | Preserve occurrence order and duplicates; projection to a set or bag is explicit. |
| Bag | Additive sum, max-union, min-intersection, truncated subtraction, absolute difference | Count multiplicity separately from membership; distinguish sum from max-union. |
| Multimap | Pair algebra, grouping, joins, projections | Sequence-, set-, and bag-valued fibers have different order and duplicate laws. |
| Bimap | Pair intersection/difference, checked merge, composition | Both projections must stay unique; ordinary pair union is not closed over bimaps. |
| Prefix index | Exact lookup, prefix scan, matching prefixes, longest prefix | Prefix matching is a specialized query capability, not ordinary map union. |

For a bag with multiplicities `a(x)` and `b(x)`, the candidate definitions are
`sum = a + b`, `max_union = max(a, b)`, `intersection = min(a, b)`,
`difference = a.saturating_sub(b)`, and `symmetric_difference = abs_diff(a, b)`.
An additive count overflow must be reported explicitly. Saturation is part of
the subtraction definition, not an implicit overflow policy for addition.
`max_union`, `intersection`, truncated difference, and absolute difference
cannot exceed the larger input count. Counts, distinct-element counts, and
address-sized lengths are separate queries. A compressed bag stores one
representative and a count; it cannot reproduce the identities of multiple
equal occurrences supplied at construction.

Map overlap should preserve the left stored key. An inner join conceptually
yields `(&K, &VL, &VR)`. A merge resolver explicitly determines the output value;
right-only entries retain right representatives. Difference by key keeps only
left entries whose key is absent on the right. Resolver invocation count,
evaluation order, owned output type, and panic behavior require an API design
before implementation. The reports differ here: Frost1 invokes its resolver
only on collisions, while Frost2's examples pass optional values for every key
and one example unwraps a missing value, which would panic. The research
snippets are not compile-tested APIs.

Bimap merge must validate conflicts on both sides. For example, combining
`{alice -> 1}` with `{alice -> 2}` or `{bob -> 1}` cannot yield an ordinary
bimap union. A checked merge should report the conflict and preserve the input
versions on an ordinary returned error. Destructive replacement, a resolver,
and relation-valued output are distinct operations. Panic and allocation-failure
guarantees need explicit design rather than an implied transactional promise.
Composition of two partial bijections whose middle domains align is again a
partial bijection.

## Capabilities and observable behavior

Lookup, traversal, sorted traversal, multiplicity, and reconstruction should be
independent capabilities. A generic lookup may need the stored key as well as
the value. Stable insertion order does not authorize a sorted merge. Borrowed
lookup must remain available without imposing `Clone` or `Default` on elements.

The reports invent capability names. Proposals must map them to the existing
traits in [`read`](../../src/read.rs) rather than silently renaming public APIs:

| Report name | Existing Frost capability | Note |
| --- | --- | --- |
| `MapGet` (Frost1), `Get` (Frost2) | `LookupRef<Q>` with `MapTypes`, or `SetGet<Q>` for sets | `LookupRef` already returns the stored key and value |
| `Contains` (Frost2) | `SetContains<Q>` | Promises no stored reference |
| `MapEntries`, `Entries` | `Entries` | Already shares `MapTypes` with lookup and mutation |
| `SortedMapEntries` | `Entries` plus `OrderedEntries` | Ascending order is a separate promise |
| `Len` | `Len`; `Cardinality` for `u64` counts | Counts are never truncated to `usize` |
| `BagCount`, `PrefixIndex` | None | New capabilities; need their own proposal |
| `difference_by_key` (Frost1), `difference_keys` (Frost2) | None | Pick one name in a map-algebra proposal |

Ordinary families continue to keep the first equivalent key or element and the
last map value. Borrowed union and intersection keep left representatives.
Native wrappers keep their documented native behavior. New bag, multimap, and
bimap contracts must state representative selection independently of equality
and order. Tests should use keys with observable metadata ignored by equality.

Borrowed entries, decoded integer values, and lending byte-key streams remain
separate access models. The implemented [integer profile](integer-sets.md)
reconstructs values; it cannot fabricate stored references. Its shared storage
and explicit Chunked policy do not implement lazy collection expressions. Typed
integer domains stay separate, and identifiers from independently versioned
dictionaries are not interchangeable.

No optimizer may infer that membership-equivalent rewrites preserve the full
result. Representative provenance, traversal order, resolver calls, and panic
behavior are observable. A callback's `Fn` bound does not prove that it is
pure, and a semantic law does not prove that an arbitrary callback obeys it.
The existing [set-algebra counterexamples](../reviews/generic-set-algebra-review.md)
remain constraints on reassociation and cancellation. Multi-source joins also
need an explicit compatibility contract for key equivalence and ordering.

## Corrections to the supplied sketches

The raw reports keep these errors. Curated documents and future proposals must
not copy them.

| Supplied sketch | Problem | Disposition |
| --- | --- | --- |
| Frost1 `FrozenBiMap::try_insert(&mut self, ...)` | Mutation on a frozen type contradicts frozen semantics | Put insertion on a mutable bimap or builder; frozen algebra consumes or borrows and returns a new value. See the [Frozen API guide](frozen-api.md#11-proposals-not-current-api). |
| Frost1 `#[derive(Hash, Eq, PartialEq)]` key whose equality "depends only on `logical_id`" | Derived traits compare and hash every field, including `display` | Write `PartialEq`, `Eq`, and `Hash` by hand over the same fields. The README's [representative example](../../README.md#contracts-that-matter) is a compiled doctest. |
| Lookup traits over unconstrained `Q` (`MapGet<Q>`, `get_by_left<Q>`, `get_left<Q: ?Sized>`) | No relation ties `Q` to the stored key, so an implementation cannot look anything up correctly | Keep the relation on each implementation, as `LookupRef` does. Do not add a blanket `Borrow` bound that excludes `Equivalent`-style backends. |
| Frost1 `model_inner_join(a: &BTreeMap, b: &BTreeMap) -> Vec<(&K, &A, &B)>` | Two reference inputs and no `self`: lifetime elision fails, so the sketch does not compile | Name the lifetimes explicitly. Treat every report snippet as a sketch until compiled. |
| N-ary sorted operations described as total-input-linear | The upstream bound hides stream count and key length | See [n-ary merge cost](#n-ary-sorted-merge-cost). |
| Frost1 facade feature table (`serde`, `roaring`, `integer-sets`) | Conflicts with versioned feature names such as `serde-1`, `roaring-0_12`, and `int-set` | Historical proposal only; never copy into `Cargo.toml`. |
| Frost2 commands and CI skeleton (`--workspace`, `-p frost-*`, `dtolnay/rust-toolchain@stable`, unpinned nightly) | Assumes packages that do not exist and an unpinned toolchain | Use the root Makefile and the pinned Rust 1.99.1 toolchain; nightly is for Miri and diagnostics only. |
| Frost2 "no explicit target restrictions were given" | The repository pins Rust 1.99.1, edition 2024, and a `no_std` core with `alloc` and `std` features | Restore the actual constraints in every derived document. |
| Frost2 crate version, license, and release-date table | Unverified and mutable | Recheck against current upstream releases before relying on it. |
| Report statements that a document, template, or ZIP "is available" | Those linked files were not supplied or recovered | Recorded as missing in the integration audit. |

## N-ary sorted merge cost

Both reports cite `fst`'s statement that its map and set operations run in
`O(n1 + n2 + ...)` with memory proportional to the number of streams. That is
**documented upstream** for `fst` 0.4.7. The implementation in
[`src/raw/ops.rs`](https://docs.rs/fst/0.4.7/src/fst/raw/ops.rs.html) (read
2026-11-09) merges streams through a `BinaryHeap` of slots, each holding the
stream's current key as owned bytes. Union and intersection pop the least key,
pop every slot with an equal key, and refill each popped stream.

**Derived reasoning** (this guide's interpretation, not upstream wording). Let
`k ≥ 0` be the number of streams, `N ≥ 0` the total number of input keys, and
`L ≥ 0` the maximum key length in bytes. Assume each stream is already sorted.

- Initialization primes every stream once: `O(k)` work and `k` stream
  advances, even when every stream is empty.
- A binary heap of at most `k` slots costs `O(log(k + 1))` comparisons per push
  or pop. The `+ 1` keeps the factor positive for a single stream (`k = 1`);
  with `k = 0` there are no keys to merge.
- Each input key is pushed and popped once. A byte-slice comparison costs
  `O(L + 1)`, and copying a key into its slot costs `O(L + 1)`; the `+ 1`
  covers empty keys, which still cost constant work.

Total merge work is therefore `O(k + N · (L + 1) · log(k + 1))`, **plus** the
cost of advancing and decoding each input stream and any I/O, which this bound
does not include. The merge's own workspace is `O(k · (L + 1))` bytes for the
slots and their key buffers, separate from whatever state each input stream
keeps internally. Treating `k` and `L` as bounded constants reduces the work to
`O(N)`; that fixed-fan-in reading is ours. The upstream documentation states
only `O(n1 + n2 + ...)` and memory proportional to the number of streams.

Frost must not transfer the upstream bound to its own algorithms. A Frost n-ary
plan states its own variables: source count, comparison cost, workspace,
decoding, and I/O. Binary sorted merges over in-memory families remain
`O(n + m)` comparisons; the existing ordered kernel's seeking strategy is
documented in [set operations](../set-operations.md).

## Execution and ownership boundaries

Start with a simple semantic implementation before specialized kernels. A later
planner may choose sorted merge, smaller-side probing, or a native bitmap
operation when capabilities justify it. Count and predicate terminals should
use scalar kernels where available; collecting members is a separate operation.
Prepared indexes, temporary allocations, and caching must have explicit owners
and costs. A reusable expression does not imply cached results, and laziness
does not imply bounded work or memory.

Consuming operations may reuse owned storage when a backend permits it, with
documented representative, drop, and panic behavior. Persistence and concurrency
are separate choices: immutable sharing must preserve every retained version;
live per-key access does not establish a coherent whole-collection snapshot.
The existing [snapshot publication contract](snapshots.md) continues to govern
whole-value publication and retry callbacks.

Encoded integer maps, rank/select, FST streams, and radix/prefix indexes remain
specialized proposals. They need their own codec, cardinality, ownership, and
query contracts. No feature may silently change an ordinary family's type,
representation, hasher, public bounds, or frozen partner.

## Implementation sequence and acceptance gates

1. Specify map lookup/traversal and join/merge contracts first. Settle stored-key
   provenance, resolver types and call counts, ordering, and materialization.
2. Build small reference models and exhaustive finite cases before typed views.
   Use standard maps for joins and bag counts, pair sets for set-valued multimaps,
   sequences for ordered fibers, and two maps for bimap inverse consistency.
3. Add one collection domain at a time within the current package. Require
   non-Clone/non-Default examples, borrowed queries, partial-iterator checks,
   overflow and conflict tests, and documented returned-error/panic behavior.
4. Add sorted/hash/native execution only with differential equivalence to the
   reference model. Record construction, staging, allocation, destruction, and
   output timing boundaries; retain slower benchmark observations.
5. Add persistent or concurrent variants only with retained-version ownership
   tests and relevant Miri or scheduler-model checks. State bounds and the exact
   production code each model executes.

The reports' proposed names and signatures remain open design questions. The
[research synthesis](../research/synthesis/foundations-and-engine-roadmap.md)
orders the follow-up questions. This integration preserves the reports' full
content and records a compatible roadmap; it does not advertise those APIs as
available or claim that research sketches passed the library's verification
suite.

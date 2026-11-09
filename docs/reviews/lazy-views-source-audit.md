# Source audit: lazy views and search cursors

Access date: **2026-11-09 (UTC)**. This reviews the search and iterator claims in
[the supplied research](supplied/lazy-views-research.md). It is a source ledger,
not an implementation claim or a replacement for a Frost API design. Published
versions, historical source, open proposals, and deductions are distinguished
below. No upstream benchmark was rerun for this audit.

## Findings that affect the design

| Research claim | Audit result | Consequence for Frost |
| --- | --- | --- |
| Named `BTreeSet` algebra methods return borrowing iterators. | Verified. Its borrowed operators instead return new sets. [R1] | State which API is lazy; laziness is not a property of every algebra spelling. |
| The surveyed cursor interfaces do not expose arbitrary random-access membership. | Supported for the particular interfaces inspected. The broader claim about every mature system is not established by that sample. [R1, F1, L1, T1] | A short-circuiting view `contains` remains a valid design option. The supplied research itself proposes it. |
| Search results never materialize. | Too strong. Lucene exposes `intoBitSet`; Tantivy's buffered union accumulates results in a bounded buffer. [L1, T4] | Distinguish whole-result materialization, bounded working buffers, and lazy traversal. They can coexist. |
| Cost and size hints provide cardinality bounds. | Not generally. Lucene permits inaccurate cost estimates; Tantivy describes best-effort hints. [L1, T1] | Keep correctness bounds separate from planning estimates and exact counts. |
| Flat n-ary operations have constant setup and metadata cost. | Not established. `fst` keeps per-stream state and primes its streams; the inspected Lucene conjunction collects and sorts children. [F2, L3] | Account separately for plan construction, repeated metadata reads, and evaluation workspace. |
| Tantivy's 3.125% threshold is an established universal selection rule. | Incorrect. The cited PR is open and contains additional clause-count and query-shape conditions. [T7] | Treat it as an experiment to compare, not a Frost default. |
| A borrowed view prevents unbounded expression chains. | Lifetime choice alone cannot establish that property. | A plan needs explicit size/depth/resource rules if such bounds are promised. |

### Rust and `fst`: supported facts and limits

The Rust 1.99.1 documentation gives `BTreeSet::union` and `intersection`
ascending borrowing iterators. `&a & &b`, in contrast, returns a new `BTreeSet`
and requires cloning elements. [R1] The current intersection source chooses
among bounds shortcuts, a merge, and a skewed strategy that advances the
smaller set and calls `contains` on the larger one. [R2] This is concrete
precedent for keeping membership probes in an adaptive intersection planner;
it does not prove that they win for every representation or size ratio.

`fst` 0.4.7 supplies n-ary streams for union, intersection, difference, and
symmetric difference, with sorted output and workspace proportional to the
number of input streams. Its n-ary difference removes every later stream from
the first; its n-ary symmetric difference uses odd parity. These are different
contracts, and a future Frost API must define zero- and one-input cases rather
than infer them from a binary operator name. [F1]

The inspected `fst` implementation stores boxed streams and a `BinaryHeap`.
Construction calls into every stream to populate that heap. [F2] Its documented
linear total-input complexity should therefore not be interpreted as
independence from variable fan-in: the actual implementation has heap
maintenance, byte-key comparisons, per-stream storage, and input priming. This
is a source-based complexity qualification, not a measured regression in
`fst`.

### Seek semantics are not interchangeable

| Contract point | Lucene 10.3.1 `DocIdSetIterator` [L1] | Tantivy 0.26.2 `DocSet` [T1, T2] |
| --- | --- | --- |
| Initial position | Unpositioned, `docID() == -1`. | Already on the first document, or terminated. |
| Seek at the current document | `advance(target)` requires a target beyond the current position. | `seek(target)` allows equality with the current document. |
| Exhausted advancement | Calls after exhaustion are outside the documented contract. | `advance` remains terminated; `seek` explicitly permits termination. |
| Exhaustion value | `Integer.MAX_VALUE`, excluded from valid IDs. | `i32::MAX as u32`, excluded from valid IDs. |
| Planning quantity | `cost()` is an estimate. | `size_hint()` and `cost()` are best-effort hints. |

Lucene's use of “undefined” for invalid calls describes an API precondition; it
is not a statement about Rust memory undefined behavior. Its `slowAdvance`
also demonstrates why a seek signature alone gives no sublinear complexity
guarantee. [L1]

Tantivy's `seek_danger` has a further state distinction: a lower-bound response
can leave a cursor unsuitable for normal access until later danger seeks
produce a found position. Such a lower bound is not a matching document. Its
published source also makes the terminal sentinel explicit. [T2] A generic
Frost cursor must preserve every legal `u32`/`u64` code, including the maximum;
an out-of-band exhausted state is required if adopting this style of cursor.

Lucene's `TwoPhaseIterator` separates a candidate superset from confirmation.
Reading a candidate as a verified set member would bypass its `matches()`
contract. [L2] This is the same distinction Frost must preserve if a future
planner uses filters or approximate summaries.

The directly inspected Lucene `ConjunctionDISI` implementation is historical:
the intervals module at commit
`7ada4032180b516548fc0263f42da6a7a917f92b`. It flattens its own exact conjunction
class, sorts children by cost, and selects two leading cursors. [L3] This is
evidence for that design technique, not verification that every current
Lucene conjunction follows identical source. Attempts to retrieve the current
class did not succeed during this audit.

## Tantivy issue and pull-request ledger

Statuses below describe the pages retrieved on the access date. The existence
of a historical bug is not evidence of a defect in the current release. In
particular, **#3086 is an issue, not a pull request**. The API discussion above
is pinned to 0.26.2; the crate index lists a newer 0.27.0, published
2026-10-17. [T8]

| Item | Verified type and status | Evidence and bounded conclusion |
| --- | --- | --- |
| [#2538](https://github.com/quickwit-oss/tantivy/pull/2538) | PR, merged 2025-12-30. | Introduced the danger-seek approach; the 0.26 changelog includes it. Neither its name nor its merge proves a universal speed advantage. [T3, T9] |
| [#2810](https://github.com/quickwit-oss/tantivy/pull/2810) | PR, merged 2026-01-23. | Adjusted termination/contract handling and intersection implementation. The 0.26 changelog includes it; it is not an outstanding patch. [T5, T9] |
| [#2863](https://github.com/quickwit-oss/tantivy/pull/2863) | PR, merged 2026-03-27. | Combines buffered non-score union changes with a `TinySet` improvement. The author's timings are specific to that experiment. [T4, T9] |
| [#3086](https://github.com/quickwit-oss/tantivy/issues/3086) | Issue, opened and closed 2026-09-08. | Reports false positives in 0.26.1 from consuming an invalid child cursor. Closed through #3088; a maintainer identifies the released fix as 0.26.2. [T6] |
| [#3088](https://github.com/quickwit-oss/tantivy/pull/3088) | PR, merged 2026-09-08. | Removes the buffered-union danger-seek override in favor of the default seek-based implementation. Adds regression and contract coverage. This is a conservative correctness fix, not proof that all nested-cursor cases are verified. [T10] |
| [#3103](https://github.com/quickwit-oss/tantivy/issues/3103) | Issue, opened 2026-09-14, closed as completed 2026-09-15. | Reports missing matches for a nested union intersected with a term filter in 0.26.0. The maintainer identifies #3086 as the fix. This is not independently established as a separate unresolved defect. Retrieved through the GitHub connector after the web reader failed. [T11, T12] |
| [#3224](https://github.com/quickwit-oss/tantivy/pull/3224) | PR, **open** when retrieved. | Proposes a density-sensitive choice with an eight-clause cap, an ordinary 128/4096 threshold, and a stricter 16/4096 count-only rule under additional conditions. This is not evidence that those rules shipped in 0.26.2 or 0.27.0. [T7] |

The published 0.26.2 changelog independently records removal of the
buffered-union cursor override that could consume invalid state. [T9] This is a
logical state error, not a claim that the Rust implementation used an `unsafe`
block. The repeatable lesson is to distinguish a lower bound, a valid
match, and exhaustion, and to test nested composition and progress. The audit
did not run the upstream reproducers and cannot certify their current source.

For #3103, the reporter specifies `tantivy-py` 0.26.0 with bundled core 0.26.0
on Apple M4. Their 250,000-document reproducer reports 7,447 missing matches,
no invented matches, and an exact flat-query control. The author explicitly
could not test 0.26.2. [T11] The maintainer's reply points to #3086 as the fix.
[T12] Combined with the published changelog, that links the report to the
already documented 0.26.2 correction; it is not our independent execution of
the reproducer or evidence that the current release still has this bug.

## Deductions to retain as design constraints

The following are reasoning from set semantics and the proposed representation,
not claims attributed to an upstream performance measurement.

**Constant-time metadata needs a construction contract.** For two known exact
counts `a` and `b`, union bounds `[max(a,b), a+b]` and intersection bounds
`[0, min(a,b)]` require constant work. Folding bounds for an arbitrary `k`-input
plan takes work proportional to its inputs unless already cached. A cached
`cardinality_bounds()` can be constant time while constructing that cache is
not. Bounds may also be loose: two equally large sets can be disjoint or
identical. They are not sufficient by themselves to promise the best plan or
an affordable output allocation. Sums and full-width universe sizes need
checked/wider arithmetic; the `u64` universe has `2^64` possible values.

**Flattening is local to a compatible operator.** Reassociating AND with AND or
OR with OR preserves membership. Rewriting a mixed expression into one list
does not: `(A union B) intersection C` is not the intersection of all three.
Distributing mixed operators may expand the expression substantially.
Difference is not associative. Any future n-ary XOR must specify parity, and
the empty intersection needs a universe or an explicit API rule. Flattening
also does not prove that seek, probes, and block kernels have equal costs.

**Sharing and laziness have independent costs.** A view can borrow operands
and still accumulate an arbitrarily large plan. An owned plan can be bounded.
An `Arc` root makes root cloning cheap; it does not make a shared flat directory
cheap to update. Copying a directory with `c` chunk references costs at least
proportional to `c`, even if just one container changes. This differs from
path copying in a persistent tree. The supplied chunk proposal should be
evaluated with both shapes, including scattered `u64` prefixes.

Rust documents that `Arc` cloning shares an allocation, `make_mut` clones the
inner value when required, and pointer atomics are a target requirement. It
does not add thread safety to an otherwise non-`Sync` inner type. [R3] These
ownership guarantees do not verify container bounds, codec laws, cached
cardinality, result-policy limits, or canonical hashing. Replacing `Arc` with
`Rc` also changes auto traits; it is not an invisible feature-level fallback
for an existing Frost family.

**Automatic materialization after two traversals is a proposal, not a measured
rule.** Partial iteration is not equivalent to consuming an expression twice.
Cache allocation, synchronization, retained operand memory, limits, and error
behavior need explicit semantics before such an optimization is adopted.

**Old implementation descriptions need version labels.** The supplied research
says mixed algebra currently falls back globally to Sorted. That describes
the earlier implementation reviewed in
[the integer output audit](integer-output-review.md), not every path in the
revised crate. Similarly, its reported `advance_to` timings do not establish
that a representation can never profit from seeking under other distributions.

## Evidence boundary

This ledger verifies the listed Rust, FST, Lucene, and Tantivy claims. It does
not certify the supplied CRoaring/Java issue statuses, PostgreSQL cost-model
history, succinct-crate features, paper performance ratios, or the entire
sharing proposal. Those require their own versioned sources and experiments.
The supplied file remains historical research input, including claims marked
unverified there. No production code or dependency was changed by this audit.

## Primary sources

All sources below were accessed **2026-11-09**. Moving URLs are identified by
the version or commit actually inspected above.

- **R1:** [Rust 1.99.1 `BTreeSet` documentation](https://doc.rust-lang.org/std/collections/struct.BTreeSet.html).
- **R2:** [Rust 1.99.1 `BTreeSet` source](https://doc.rust-lang.org/src/alloc/collections/btree/set.rs.html), especially `intersection` and `Intersection::next`.
- **R3:** [Rust 1.99.1 `alloc::sync::Arc`](https://doc.rust-lang.org/alloc/sync/struct.Arc.html).
- **F1:** [`fst` `OpBuilder`](https://docs.rs/fst/latest/fst/set/struct.OpBuilder.html), resolved to 0.4.7.
- **F2:** [`fst` raw operations source](https://docs.rs/fst/latest/src/fst/raw/ops.rs.html), resolved to 0.4.7 and cross-checked against the cached 0.4.7 crate source.
- **L1:** [Lucene 10.3.1 `DocIdSetIterator`](https://lucene.apache.org/core/10_3_1/core/org/apache/lucene/search/DocIdSetIterator.html).
- **L2:** [Lucene 10.3.1 `TwoPhaseIterator`](https://lucene.apache.org/core/10_3_1/core/org/apache/lucene/search/TwoPhaseIterator.html).
- **L3:** [Historical Lucene intervals `ConjunctionDISI`, pinned commit](https://apache.googlesource.com/lucene-solr/+/7ada4032180b516548fc0263f42da6a7a917f92b/lucene/queries/src/java/org/apache/lucene/queries/intervals/ConjunctionDISI.java).
- **T1:** [Tantivy 0.26.2 `DocSet`](https://docs.rs/tantivy/0.26.2/tantivy/trait.DocSet.html).
- **T2:** [Tantivy 0.26.2 `docset.rs`](https://docs.rs/tantivy/0.26.2/src/tantivy/docset.rs.html).
- **T3:** [Tantivy PR #2538](https://github.com/quickwit-oss/tantivy/pull/2538).
- **T4:** [Tantivy PR #2863](https://github.com/quickwit-oss/tantivy/pull/2863).
- **T5:** [Tantivy PR #2810](https://github.com/quickwit-oss/tantivy/pull/2810).
- **T6:** [Tantivy issue #3086](https://github.com/quickwit-oss/tantivy/issues/3086).
- **T7:** [Tantivy PR #3224](https://github.com/quickwit-oss/tantivy/pull/3224).
- **T8:** [Tantivy published-version index](https://docs.rs/crate/tantivy/latest), listing 0.27.0 and 0.26.2 publication dates.
- **T9:** [Tantivy published 0.26.2 changelog](https://docs.rs/crate/tantivy/0.26.2/source/CHANGELOG.md).
- **T10:** [Tantivy PR #3088](https://github.com/quickwit-oss/tantivy/pull/3088).
- **T11:** [Tantivy issue #3103](https://github.com/quickwit-oss/tantivy/issues/3103), retrieved through the GitHub connector: issue body, creation/closure dates, and completed status.
- **T12:** [Maintainer reply identifying #3086 as the fix for #3103](https://github.com/quickwit-oss/tantivy/issues/3103#issuecomment-5676719997), retrieved through the GitHub connector.

# Borrowed set views: performance and research audit

Status: design review and proposed experiment plan, 2026-11-09. This document
does not announce an implemented view, sharing backend, or new performance
result. The supplied
[research review](supplied/lazy-views-research.md) remains historical input.
The implementation baseline is the current integer profile, after the
[output-policy review](integer-output-review.md).

## Recommendation

Keep the existing eager integer algebra. Evaluate an explicit borrowed query
view against the corrected implementation, with separate execution paths for
membership, early termination, counting, and materialization. Prototype the
query plan before committing to a public seek trait or a new storage backend.
Structural sharing and Elias–Fano address different costs and deserve separate
experiments; neither is a prerequisite for measuring a useful view.

The supplied review identifies valuable techniques, particularly bounded
progress, native bulk kernels, and deferred container repair. Several conclusions
are stronger than their evidence: an old two-input iterator measurement does
not prohibit seek algorithms, select does not establish constant-time
successor, and a flat expression must preserve difference and negation
boundaries. Replacing eager results or silently caching on a second traversal
would also change costs that Frost currently makes explicit.

## 1. What the primary papers establish

### Cardinality maintenance: a platform-specific result

Chambi et al., *Better bitmap performance with Roaring bitmaps*, arXiv
1402.6407v10, §4, estimate 700 million 64-bit OR-and-write words per second on
their Java platform, falling to 500 million with cardinality maintenance.
The reported approximately 30% is a throughput reduction; those rounded rates
imply approximately 40% more elapsed time for fixed work. Their platform uses
Oracle JVM 1.7 and an AMD FX-8150. Algorithm 4 already groups equal-key
containers and computes cardinality once after the updates. These are
materialized container operations, not a borrowed result stream.
[Primary paper, §4–5](https://arxiv.org/html/1402.6407v10).

The supplied optimized-C parenthetical is absent from the inspected v10 HTML
and PDF. A secondary historical quotation suggests it appeared in an older
revision, but primary v7 retrieval failed during this review. Treat that clause
as revision-unverified, not disproven. Neither wording establishes the
cardinality cost of the current Rust kernels, compiler flags, or machine.

### Lazy Roaring union means deferred repair

Lemire, Ssi-Yan-Kai and Kaser, *Consistently faster and smaller compressed
bitmaps with Roaring*, arXiv 1603.06549v4, §5.1, describe materializing union
payloads while marking some cardinalities unknown, then repairing metadata and
container forms at the end. They compare repeated folds with priority queues:
the growing-output model can favor the heap, whereas equal-range uncompressed
bitmaps give both O(BN) work and the fold uses less working storage. Their
experiments find no consistent winner across datasets. This supports testing
aggregation order and repair frequency, not prescribing one global algorithm.
[Primary paper, §5.1 and §6](https://arxiv.org/html/1603.06549v4).

Four independent choices should therefore have separate names:

| Choice | Work it postpones or avoids |
| --- | --- |
| Borrowed expression | Construction of an owned result until requested |
| Incremental value iteration | Work after the last value the caller consumes |
| Deferred repair | Repeated metadata and container-normalization work during aggregation |
| Global k-way execution | Intermediate results or unproductive nested traversal |

An evaluator can combine these choices. An eager k-way materializer can defer
repair; a borrowed view can still choose a native eager kernel for a terminal
operation.

### Bound propagation: more than an ordinary seek method

Arntzenius, *Fair intersection of seekable iterators* (miniKanren 2025), exposes
either a found key or a lower bound, with inclusive, strict, and exhausted
states. Seeking includes the current position and is idempotent. This lets an
inner intersection communicate progress before finding its next member.
The paper reports approximately 1.2 seconds versus under 1 millisecond for its
Haskell array experiment, but its prose and code disagree: the timing paragraph
repeats `evens` as the third operand, while §3 and executable Figure 1 use the
sparse `ends = {0, n}`, with `n = 30_000_000`. Admission benchmarks should
follow the figure's evens/odds/ends workload; the sparse third operand supplies
the useful endpoint bound. Do not attribute that gain to flattening a
repeated-evens expression. The source also warns that its linear-list example
has no asymptotic speedup. We did not rerun the upstream benchmark or verify a
separate Rust trait/blog implementation.
[Primary paper, §3–4 and Figure 1](https://arxiv.org/html/2510.26016v1).

For Frost, merely adding `seek_ge(x) -> Option<Word>` to a binary iterator can
still hide arbitrarily much finite work inside a child. A private flat
intersection scheduler is one candidate. A bounded-progress protocol is
another. Compare them before exposing a trait whose semantics become a lasting
compatibility commitment.

### SIMD and skew: a measured dispatch problem

Lemire, Boytsov and Kurz, *SIMD Compression and the Intersection of Sorted
Integers*, arXiv 1401.6399v13, distinguish decompression from intersection:
0.7 cycles per integer is a 32-bit decompression result. Their SIMD intersection
algorithms and conjunctive-query experiments provide separate improvements,
often around twofold against the selected controls. Their implementation
switches among V1, V3 and SIMD galloping using input-size ratios, and explicitly
says different widths need different heuristics. Literature comparisons vary
with synthetic versus real data. This supports multiple kernels and matched
controls, not a universal choice of probing or seeking.
[Primary paper, §2, §6–7](https://arxiv.org/pdf/1401.6399).

### Elias–Fano: indexed access is not successor

Pibiri and Venturini's author-hosted survey explains constant-time indexed
access using select, but its successor construction takes
O(1 + log(U/n)): select locates a high-bit bucket and a search finishes inside
it. The familiar bound of at most `n * ceil(log2(U/n)) + 2n` bits describes the
encoding; auxiliary indexes and implementation overhead still need accounting.
An expected constant bound requires additional assumptions about the
distribution or a specific implementation. It does not follow from select
alone.
[Primary survey, §3.4](https://pages.di.unipi.it/pibiri/papers/ii_survey.pdf).

The inspected `sux` documentation makes the capability distinction concrete:
`EfSeq`, `EfDict`, and `EfSeqDict` support different combinations of indexed
access and predecessor/successor according to their high-bit indexes.
[sux 0.14.0 documentation](https://docs.rs/sux/0.14.0/sux/dict/elias_fano/index.html).

Do not generalize the supplied three-crate survey into “Rust Elias–Fano requires
std.” The retrieved `vers-vecs` 1.8.2 and `sux` 0.14.0 crate roots have no
`no_std` declaration; the inspected sux EF source also uses `std`. This pass
did not establish `sucds`'s target/feature build contract. Separately,
`sbits`' retrieved 0.2.3 root explicitly declares `no_std`, imports `alloc`,
and gates its `std` import. That is a candidate to investigate, not an audit
or a successful Frost bare-metal build. Pin exact versions and feature sets,
inspect their safety requirements, and compile the required targets before any
dependency decision.
[vers-vecs source](https://docs.rs/vers-vecs/latest/src/vers_vecs/lib.rs.html),
[sux source](https://docs.rs/sux/latest/src/sux/lib.rs.html),
[sux EF source](https://docs.rs/sux/latest/src/sux/dict/elias_fano.rs.html),
[sbits source](https://docs.rs/sbits/latest/src/sbits/lib.rs.html).
The `latest` URLs above returned the stated revisions during this audit;
their future contents may differ.

## 2. Use the corrected Frost baseline

The supplied claim that mixed algebra always returns Sorted describes the prior
implementation. The current common harness and direct payload diagnostic show
the following. Times are pooled medians for the baseline x86-64 profile on the
recorded machine; these rows are context for future experiments, not view
benchmarks.

| Workload | Prior source | Current source | Current output |
| --- | ---: | ---: | --- |
| Million alternating Dense members union Roaring singleton | 1.025 ms | 4.805 µs | Dense; 250,000 buffer bytes |
| 32 singleton updates, including initial clone | 19.084 ms | 154.668 µs | Dense; 250,000 buffer bytes |
| Two distant runs, each with a 524,288-byte Dense payload | 35.586 ms | 348.385 µs | Roaring |
| Dense/Roaring materialized intersection | 7.340 ms | 119.468 µs | Native compressed result |

Sources:
[comparison CSV](../../verification/integer-output/comparison.csv),
[measurement metadata](../../verification/integer-output/bench-metadata.json),
[exact payload diagnostic](../../verification/integer-output/payload-check.csv),
and [common harness](../../crates/frost-collections-integer/benches/integer_output.rs).
The current Dense+singleton time is close to its prepared native Dense
clone-and-update control. A new sharing design must therefore beat a compact
eager copy, rather than only the obsolete Sorted expansion.

`contiguous_payload_bytes` excludes allocator headers, wrapper storage, and
temporary allocations. Its `None` for Roaring means no contiguous-payload
measurement is exposed, not zero resident memory. The distant union's 1,824
serialized bytes are a wire-size diagnostic, not a resident-size claim.

The supplied 5–8 ns/element and 3.8× `advance_to` claims were not measured by
our common harness. Keep them labeled as supplied workload observations.
They justify including probe and seek controls, not banning an API forever.
A useful comparison must specify width, container distribution, seek distance,
overlap, operand ratio, conversion cost, and terminal operation.

The current evidence also retains slower cases. For example, balanced
Sorted-minus-Roaring takes about 9.475 ms versus 7.946 ms previously while
producing a 250,000-byte Dense buffer instead of a 4,000,000-byte Sorted buffer.
A future view may avoid that materialization when only a prefix is needed;
that remains a hypothesis to test. Do not present every avoided allocation as
a latency improvement.

## 3. Cost model for the borrowed view

Use the following accounting model to choose experiments, not as a predictive
equation with universal constants:

```text
time =
    plan construction and cursor initialization
  + sum(member advances * advance cost)
  + sum(seeks * seek cost)
  + sum(membership probes * probe cost)
  + sum(words or containers processed * bulk cost)
  + decoded outputs * decode cost
  + result allocation, normalization, and destruction
```

Each coefficient depends on representation, CPU support, data locality, and
the evolving cursor position. Global cardinality alone does not describe these
costs. A million dense members can be cheaper to count word-wise than to visit
one by one; a two-member seed can be cheaper to probe against a large sorted
set than to scan that set.

Choose execution by terminal operation:

| Terminal | Candidate strategy | Cost that must remain visible |
| --- | --- | --- |
| `contains(key)` | Encode once; evaluate Boolean membership with short-circuiting | Up to O(k) operand probes; repeated calls repeat this work |
| First member or small prefix | Small seed plus probes, or forward cursors | Startup and skipped work before the first result |
| Exact count | Dense word kernels, native Roaring counts, or seed filtering | Count can require all relevant words/members even without a result allocation |
| Full iteration | Ordered merge/seek or blocked kernels that drain set bits | Output extraction and decoding remain necessary |
| Materialization | Native bulk operation plus current result policy | Conversion, allocation, limits, and normalization |
| Repeated queries | Explicitly materialize once when useful | Construction amortizes only over the actual future reuse |

For the same repeated terminal, a useful break-even comparison is
`materialize_cost + q * frozen_query_cost < q * view_query_cost`. The values
depend on the terminal and data. “Cache after two traversals” has no general
economic basis. It can unexpectedly allocate, retain an entire result after a
short traversal, introduce synchronization requirements, and complicate panic
behavior. Prefer an explicit materialization boundary until measured demand
justifies a separately specified cache.

A borrowed view avoids copying member storage at creation; it does not
automatically avoid all allocations. A dynamically sized flat operand list,
heap, batch buffer, or normalized expression arena needs working space. Measure
that space separately from the final result. Document whether a simple binary
view has an inline path and whether deep expressions have a construction or
evaluation limit.

Native kernels must remain available beneath the view. Dense operands with
different base offsets require aligned coordinates. Roaring operations may
benefit from native aggregation without exposing its containers. Sorted inputs
need merge, galloping, or probing alternatives. A per-window adapter that
converts every input on every traversal can cost more than the temporary result
it removes.

## 4. Semantic boundaries that constrain planning

### Flatten only a proven associative group

Flatten adjacent intersections or adjacent unions. XOR is associative too, but
duplicate operands cancel by parity instead of being idempotent. Do not
flatten an entire mixed expression just because all leaves are integer sets.

Difference is not associative: for `A = B = C = {1}`,
`(A - B) - C` is empty while `A - (B - C)` is `{1}`.
A deliberate rewrite of `(A - B) - C` as `A - (B union C)` is valid, but
reordering all three leaves is not. Generate candidates from the positive
side, and reorder negative membership filters only within the equivalent
predicate. General distributive rewrites can increase expression size
exponentially; they need a bounded policy.

Complement needs an explicit valid universe, especially for restricted
`IntKey` implementations. Use `universe - expression`, not numeric bit
complement that manufactures invalid codes. Preserve the same logical key type
throughout a view.

The integer profile has canonical codes. Do not transfer reordering rules
unchanged to ordinary frozen families: their union/intersection preserve the
left stored representative.
[Project contracts](../../AGENTS.md),
[design philosophy](../design/philosophy.md).

### Separate estimates, exact counts, and iterator bounds

Planning estimates may be inaccurate without changing the answer. Rust
`Iterator::size_hint` has a stricter protocol: the returned lower and upper
bounds must enclose the remaining output length. It is still a safe trait, so
consumers may not turn a bad hint into unchecked memory access.
[Rust Iterator contract](https://doc.rust-lang.org/core/iter/trait.Iterator.html#method.size_hint).

For an untouched expression, sound bounds follow from set identities:
intersection's upper bound is the smallest operand cardinality; union's upper
bound is the checked sum and its lower bound is the largest operand
cardinality; difference's upper bound is its positive operand cardinality.
These are not predictions of overlap. Use sufficiently wide arithmetic and
checked conversion to `usize`; return an unknown iterator upper bound if it
cannot be represented.

A cursor's partial-consumption bounds require a separate proof, accounting for
pending values and duplicate suppression. The original expression bounds
cannot simply be reused unchanged as lower bounds after yielding members.
Zero is a useful conservative lower bound. Do not implement
`ExactSizeIterator` from an estimate, or imply that an exact expression count
is O(1). An O(1) cached bound also requires accounting for its O(k) construction.

The independent [cardinality-bounds model](../../verification/integer-views/README.md)
exhaustively checks the proposed binary bound formulas over universes of size
0 through 6 and selected arithmetic boundaries for universes of size `2^32`
and `2^64`. It checks 1,003,621 subset/enclosing-interval combinations and
endpoint attainability, plus 63,404 interval refinements for disjoint support
ranges. The latter retain the universe cap even when upper bounds are loose.
This is finite mathematical evidence, not a symbolic proof or a test of a
production view implementation. Its script, recorded output, and source hash
are preserved outside the release gate.

### Seek and snapshot contracts must be explicit

A future seek protocol must define whether the current value remains eligible,
what a backwards target means, how strict bounds work, when exhaustion is
permanent, and what constitutes progress. Represent “strictly greater than
`u64::MAX`” as exhaustion or an excluded bound; do not compute an overflowing
successor. A pending lower bound is not a found member.

A borrowed view should keep its operands alive for its lifetime. When those
operands come from a snapshot, load the intended version once before deriving
the view. Separate store loads do not establish a transaction across versions.
The view does not become an owned publishable snapshot merely because its
leaves are frozen. Preserve the existing explicit publication boundary.
[Snapshot contracts](../../AGENTS.md).

## 5. Structural sharing is a separate cost decision

For a proposed flat index of `c` shared containers, an update that retains the
old snapshot generally copies O(c) index entries and clones the changed
container payload. Wrapping that index in an `Arc` makes a root clone cheap,
but a shared-root update still copies the index. A persistent tree changes the
index path-copy cost and introduces its own nodes and traversal overhead.
These are layout consequences, not measurements of an implemented Frost
backend.

Atomic reference increments, per-container allocations, pointer chasing, and
retained historical versions all belong in the comparison. Sparse full-width
u64 inputs can have almost one container per member. A 2^16 window is therefore
a candidate granularity, not an automatic optimum for the entire profile.

`Arc::make_mut` provides clone-on-write ownership and `Arc::ptr_eq` can
recognize shared allocations. They do not prove a custom container's ordering,
cardinality, normalization, or mixed-kernel correctness. Pointer-width atomic
support is also a target requirement for `Arc`; replacing it by `Rc` changes
auto traits and should not silently redefine an existing family.
[Rust Arc documentation](https://doc.rust-lang.org/alloc/sync/struct.Arc.html).

Measure sharing on retained snapshots, clustered single-container updates,
scattered updates, unrelated inputs, and full rebuilds. Keep a borrowed-view
experiment independent so a failure of one storage proposal does not obscure
whether early termination is useful.

## 6. Proposed benchmark and verification plan

No new view benchmarks were executed for this review. The following is a
concrete plan for an isolated prototype.

### Competing algorithms

Use the corrected eager Frost implementation as the production control.
Compare pairwise folds in documented orders, smallest-seed probing, sorted
merge/galloping, flat k-way seek, and bounded-progress nested seek where
implemented. Include prepared native Roaring aggregation and direct Dense
word kernels. Label conversions separately from prepared native controls;
an already-converted bitmap is not a fair control for an operation that must
convert a Sorted operand inside timing.

Test k = 2, 3, 4, 8, 32, and 128. Separate view construction, first use, and
reuse. Include equivalent flat and deeply associated expressions, plus mixed
trees whose differences cannot be reassociated.

### Workloads and terminals

| Corpus | Risk it exposes |
| --- | --- |
| Tiny selective input among large inputs, with selective input in every position | Blind scan of the first/large operand |
| Large pairwise overlaps with empty three-way intersection | Expensive intermediate output and hidden progress |
| Even/odd inputs plus an early terminating outer constraint | Nested-cursor scheduling |
| Long runs, alternating bits, clustered sparse values, and random sparse values | Bulk work versus member iteration |
| Same and different Dense bases; 64-bit-word and container crossings | Coordinate alignment and skip arithmetic |
| u64 values spread over many upper prefixes | Directory and seek cost hidden by cardinality |
| Signed extrema, u64::MAX, and restricted key gaps | Encoding, strict bounds, and invalid universe assumptions |
| Identical roots, related snapshots, and unrelated equal values | Pointer shortcuts versus logical equality |
| Difference under union/intersection, repeated leaves, and XOR duplicates | Incorrect flattening and ordering |

For each relevant corpus measure membership hits and misses; first result;
prefixes of 1, 8, 64, and 1,024 members; emptiness; exact count; full iteration;
and materialization under explicit policies. Repeat selected terminals 2, 8,
and 64 times to expose reuse cost. Include no-result cases and cases whose
first result occurs late.

Validate all answers against a `BTreeSet` oracle before timing. For cursors,
compare every partial output and resume state, size hints after advancement,
repeated/backwards/strict seeks under the chosen contract, and exhaustion.
Exercise contradictory estimates without changing results. Exhaustive small
domains and expression trees give useful bounded semantic evidence; they are
not a proof of unbounded evaluator behavior or dependency internals.

### Measurements and acceptance criteria

Record allocation count, allocated bytes, peak live working bytes, final
payload, and elapsed time. Add diagnostic counts for values decoded, advances,
seeks, probes, words/containers visited, and expression-node visits. Keep
instrumented work counts separate from uninstrumented timing. Hardware
counter results are optional and must identify the available event and method.

Run competing binaries serially with the same compiler, dependency versions,
flags, affinity, and input ownership. Keep baseline x86-64 and x86-64-v3 as
separate profiles. Preserve raw per-run samples, source fingerprints, slower
cases, and the location of construction/allocation/drop relative to timing.
Do not call an experiment “cold cache” without controlling that condition.

Acceptance requires correctness for all terminals and supported targets,
bounded expression/cursor state, preserved native binary count paths, and
measured benefits for the claimed early-termination or k-way workloads.
Report full-traversal regressions and the memory/time tradeoff explicitly.
A benchmark win does not authorize estimates as exact lengths, implicit
materialization, a new dependency in core-only builds, or weakened result
limits.

## 7. Scope and remaining evidence

This audit verifies the papers, selected crate sources, current local benchmark
context, and the performance consequences of the proposed view contract.
It does not revalidate every database, pull-request, persistent-library, or
compression ratio in the supplied survey. Recent upstream claims need their
exact version and source before becoming project policy.

The next reviewable artifact should be an isolated evaluator prototype and
the comparison above. Public naming, a sharing backend, a general seek trait,
automatic caching, and a succinct storage integration should follow their own
demonstrated requirements.

# Adversarial review: integer views and structural sharing

Review date: **2026-11-09**. Scope: the supplied
[lazy-views research](supplied/lazy-views-research.md), the corrected Frost
integer implementation, primary-source verification, and a proposed API.
This review does not implement a lazy engine or persistent integer family.

## Decision

Keep eager integer-set algebra. Add an explicit borrowed `IntSetExpr<'a, K>`
for queries that can benefit from avoiding intermediate result collections.
Investigate `PersistentIntSet<K>` separately for repeated derivation of related
snapshots. The supplied research identifies these distinct needs, but its
recommendation to replace the storage layer first is not established by its
evidence, and several proposed API contracts need correction.

The detailed proposals are [borrowed expressions](../../crates/frost-collections-integer/docs/integer-views.md)
and [persistent storage](../../crates/frost-collections-integer/docs/persistent-integer-storage.md). Both are
marked unimplemented. The current `FrozenIntSet<K>` retains its fixed Sorted,
Dense, and Roaring candidates and its existing eager API.

## 1. The starting implementation matters

The research's statement that mixed algebra always falls back globally to
Sorted describes the earlier source, not the corrected revision in this
archive. The current implementation retains compact Dense results for small
updates, promotes appropriate distant or mixed results to Roaring, preserves
result limits, and uses native operations and validated sorted construction.
These changes do not introduce structural sharing.

In the final paired harness on the baseline x86-64 compiler target, the
million-member alternating Dense set unioned with a Roaring singleton changes
from **1,025.254 microseconds to 4.805 microseconds**. The result's exact
contiguous payload remains **250,000 bytes**, including after the tested
32-update sequence. This is this project's measured workload, not a rerun of
the supplied review's 3.3 ms figure. See the
[complete benchmark report](../../crates/frost-collections-integer/docs/integer-output-benchmarks.md) and
[payload evidence](../../verification/integer-output/payload-check.csv).

Clone and version retention still warrant investigation. Today a deep copy
can remain proportional to payload size, while a shared version could copy
less. That opportunity is distinct from choosing a compact output. The
comparison baseline for new work must be the corrected implementation, with
conversion and retention costs charged, rather than the superseded fallback.

The final measured implementation also has slower cases, including some
fragmented Hash workloads and balanced Sorted-minus-Roaring difference. These
remain in the report and raw samples. The evidence supports specific wins;
it does not establish that every operation is now optimal or that a new
persistent engine would improve every workload.

## 2. Design findings and dispositions

| Finding | Why it matters | Decision |
| --- | --- | --- |
| Eager values, deferred queries, and shared ownership solve different problems. | A lazy query can avoid intermediates without sharing materialized chunks; a shared set can remain fully eager. | Keep three distinct concepts and names. |
| A flat Arc-shared directory still has O(C) shared-update cost. | O(1) root clone does not remove copying C chunk handles on the next changed version. | Benchmark a flat prototype honestly; require a persistent directory before claiming path-copy complexity. |
| Safe Arc does not verify set invariants. | Counts, prefix coordinates, normalization, result limits, and wire validation can be wrong in safe Rust. | Retained-version and metadata oracles remain mandatory. |
| Borrowing does not bound expression growth. | A borrowed plan can contain arbitrarily many nodes or borrow static data. | Omit sealed SnapshotValue support; use explicit plan/resource contracts. |
| General expressions cannot all become one n-ary node. | Mixed AND/OR, difference, and XOR have different laws. | Flatten homogeneous groups only; retain a checked mixed plan. |
| “Materialize after two calls” has no demonstrated threshold. | Prefix iteration, exact count, and repeated full scans have different costs and memory effects. | Explicit materialization; no hidden result or count cache initially. |
| Query estimates are not guaranteed bounds. | Inaccurate estimates can invalidate iterator hints or skip real members if treated as facts. | Separate mathematical bounds, exact counts, and heuristic costs. |
| u64 is insufficient for every unmaterialized result. | Two legal operands can cover all 2^64 codes. | u128 bounds and `count_wide`; checked u64 adapter and materialization. |
| “No output collection” does not mean “no allocation.” | Plan nodes, cursor state, heaps, conversions, and general membership scratch have costs. | State allocation scope per operation and measure it. |
| Fixed promotion and density thresholds are workload-specific. | 1,024 and 3.125% do not encode universal speed or memory crossovers. | Retain as experimental settings, not adopted Frost constants. |
| Search-specific sentinels and approximate states cannot be copied literally. | Every primitive code is legal in Frost; a lower bound is not a found key. | Out-of-band exhaustion; total seek contract; distinguish Found from Bound. |
| A lossy database bitmap requires external rechecking. | Frost's exact-set API has no external row predicate to repair false positives. | Exact fallback or explicit error under any future strict budget. |

These are contract and complexity deductions. They do not depend on any
unverified upstream timing ratio.

## 3. Source corrections that change the conclusion

### Search engines do not imply a universal lazy-set API

Rust's named BTreeSet algebra methods are borrowing iterators, whereas its
borrowed operators can return newly allocated sets. Its current intersection
source includes smaller-side iteration with `contains` probing. Lucene's
cursor API exposes `intoBitSet`; Tantivy's buffered union constructs bounded
working results. Therefore streaming, buffering, and materialization can
coexist. The sampled APIs do not establish that every mature system forbids
membership on an unmaterialized expression. The supplied recommendation itself
proposes `contains`, so its survey-wide wording should not be read as a ban.
Primary links and versioned contracts are in the
[search-source audit](lazy-views-source-audit.md).

Lucene explicitly permits inaccurate `cost()` heuristics. Tantivy's
`DocSet::size_hint` is a best-effort estimate, unlike the guaranteed interval
required by Rust's standard `Iterator::size_hint`. These identically suggestive
names do not make their contracts interchangeable. Planning can tolerate a
bad estimate; correctness cannot.

### Recent reports need status, release, and attribution

Tantivy PR [#3224](https://github.com/quickwit-oss/tantivy/pull/3224) was **open**
when checked. Its proposal includes an eight-clause cap, an ordinary
128/4096 threshold, and a different 16/4096 count-only threshold under further
conditions. The supplied single 3.125% rule omits that context and is not
evidence of shipped behavior. API discussion in the source audit is pinned
to 0.26.2; it does not call that the newest release.

Issue [#3086](https://github.com/quickwit-oss/tantivy/issues/3086) was addressed
by merged PR [#3088](https://github.com/quickwit-oss/tantivy/pull/3088), with the
change recorded in 0.26.2. The nested-union false-negative report
[#3103](https://github.com/quickwit-oss/tantivy/issues/3103) was also retrieved:
the reporter used 0.26.0, and a maintainer
[closed it as covered by #3086](https://github.com/quickwit-oss/tantivy/issues/3103#issuecomment-5676719997).
This review did not rerun that reproducer or establish a separate remaining
defect. The lesson is to test nested cursor state, not to report an already
closed history as a current library failure.

The CRoaring serialization and ownership fixes cited in the research were
verified, including merged #928 and #946. Java's shared-container fix #874
was merged, but its proposed persistent class in
[#885](https://github.com/RoaringBitmap/RoaringBitmap/pull/885) was **open and
unmerged**. Its own benchmark includes slower conversion and operation cases.
The [persistent-storage source ledger](../../crates/frost-collections-integer/docs/persistent-integer-storage.md)
records the exact statuses and relevant invariants. These are reports about
other implementations, not defects established in Frost's Rust dependency.

### The seek example has a material prose/code discrepancy

Arntzenius's [2025 paper](https://arxiv.org/html/2510.26016v1) distinguishes a
found value from a bound on future values and reports a large improvement in
a small Haskell array experiment. However, its timing paragraph repeats
`evens` as the third operand, while section 3 and Figure 1 use
`ends = {0, n}`. The executable figure's sparse endpoint operand provides
the useful bound propagation. The research copied the inconsistent prose.

Use **evens/odds/ends** as the admission workload, preserve all associations,
and separately test repeated identical operands. Do not promise that merely
flattening a repeated-evens expression makes an O(n) disjointness scan take
under a millisecond. The paper supports a useful interface distinction; its
reported number is not a Frost result or a universal flattening effect.

### Performance numbers retain their experimental scope

The inspected Roaring paper reports a Java throughput reduction from roughly
700 million to 500 million 64-bit OR-and-write words per second when computing
cardinality. Its hardware, JVM, and algorithm matter. The supplied claim that
the penalty disappears in optimized C is absent from the inspected v10 HTML
and PDF; it remains revision-unverified here, rather than proven false.
The subsequent paper supports deferred repair and does not establish a single
k-way union strategy as a universal winner. See the primary sources in the
[performance audit](lazy-views-performance-audit.md).

Likewise, 0.7 cycles per decoded integer describes decoding in a particular
codec experiment, not total set intersection. The supplied 5–8 ns
`advance_to` and 3.8x comparisons were not reproduced by the common Frost
harness. They justify retaining probing as a control, not a permanent ban on
Roaring seeks or a decision to own every container algorithm.

### Succinct representations do not have the advertised blanket guarantees

The author-hosted Pibiri–Venturini survey distinguishes O(1) Elias–Fano access
from a successor bound of O(1 + log(U/n)). Constant-time select alone is not a
universal constant-time successor result. Distribution assumptions and
additional indexes must be named.

The limited survey of three Rust crates cannot prove that Rust EF requires
std or must be written in-house. The inspected `sbits` 0.2.3 root explicitly
uses no_std plus alloc; it is an **unaudited candidate**, not a recommendation
or a tested embedded dependency. Other representation-table ratios and crate
statuses not used for the proposal remain supplied research, not independently
certified Frost measurements. Ordered Eytzinger traversal also need not
duplicate the sorted array as a matter of correctness; layout affects the
cost and complexity of traversal, which should be measured.

## 4. Concrete expression design

`a.view().and(&b).and(c.view().or(&d))` describes a borrowed plan. Membership
short-circuits encoded probes; iteration emits ordered unique K values;
`count_wide()` returns an exact u128 count; `cardinality_bounds()` returns a
cached proven interval; and `materialize(policy)` produces an ordinary eager
FrozenIntSet with explicit policy validation. This is a proposed spelling,
not newly callable code.

Use a private arena with flattened compatible groups and preserved mixed
structure. Keep independent execution state in each iterator, with no hidden
shared scratch or automatic cache. Fast flat membership can avoid allocation;
general mixed membership may use explicit scratch. The whole-plan bounds
read is O(1) because construction folds the metadata, not because arbitrary
query analysis is free.

A small initial engine can derive a **candidate cover** from subset laws:
choose one child's cover for intersection, the left cover for difference,
and union child covers for union/XOR. Enumerate that cover, then evaluate the
original expression's encoded membership. This gives a correct mixed-query
baseline over current storage, without nested approximate cursor semantics
or a new bitmap implementation. Specialized counts, smallest-side probes,
and later window kernels can be checked against it.

Exact cursors use BeforeFirst/At/Exhausted state. Seeking at or below the
current value leaves it in place; advancing moves strictly forward. MAX is
data, so neither exhaustion nor unknown state uses a legal code as a sentinel.
A later bounded-progress engine must keep Found and Bound distinct and prove
that repeated unresolved steps make internal progress. The public API need
not expose that machinery.

The view is intentionally ineligible for SnapshotStore through the sealed
SnapshotValue boundary. Borrowing alone cannot enforce that restriction.
Callers explicitly materialize when they want an independently owned result
or repeated use with cached exact metadata. This keeps allocation and result
limits visible at the point where ownership changes.

## 5. Structural sharing remains a separate experiment

The proposed `PersistentIntSet<K>` uses immutable shared chunks and a directory
whose update complexity is stated honestly. A flat directory can be a useful
baseline. A persistent tree or radix directory is required if small edits are
to copy a path rather than every chunk handle. Wrapping an ordinary BTreeMap
in Arc does not itself make BTreeMap path-copying.

The chunk model needs a u32 cardinality for 65,536-member full windows, a
checked u64 representation of upper-48-bit prefixes, canonical run endpoints,
and complete result metadata before publication. Arc identity shortcuts are
valid only in matching encoded coordinates and after required policy checks.
Safe ownership does not prove these logical invariants.

Benchmark retained versions, sparse full-width keys, clone/drop, actual
changed and unchanged edits, failed snapshot retries, and final-owner release.
Chunk sharing can reduce copied payload while adding allocations, atomic
reference traffic, directory overhead, and retained-memory costs. Neither
constant-time cloning nor one small changed chunk establishes bounded latency
or a global memory limit. The new profile must earn its place against the
corrected eager implementation.

## 6. Evidence and next implementation gates

The review adds an executable
[cardinality-bounds model](../../verification/integer-views/README.md). The
copied script passed over **5,461 subset pairs**, **1,003,621 enclosing-interval
cases**, **402 disjoint-support pairs with 63,404 enclosing-interval cases**,
and **1,568 wide-boundary interval pairs**. It checks soundness and finite
endpoint attainability of the proposed
mathematics. Python integers do not test Rust overflow behavior; the design
explicitly uses u128 and saturating subtraction. This is not a production-view
test or symbolic proof.

Review caught one bound-composition trap: disjoint support does not make loose
upper bounds additive without a cap. The design now specifies
`[la + lb, min(U, ua + ub)]` for disjoint union/XOR, and the 63,404 additional
interval cases check this condition against actual sets.

Before shipping views, require arbitrary-tree differential tests, candidate
cover inclusion, partial iteration and every size hint, full-width code and
count boundaries, exact-versus-bound cursor cases, plan-depth and allocation
checks, concurrent independent iterators, and compile-fail lifetime/domain
tests. Performance gates must distinguish count, prefix, full traversal,
materialization, and reuse under the same datasets and CPU profile.

Before shipping persistence, additionally require version-retention oracles,
prefix/container transitions, pointer sharing checks, immutable old versions,
policy failures, snapshot identity and retry behavior, and lifecycle memory
measurements. The existing concurrency model assumes the publication primitive;
it does not prove a new tree, ArcSwap internals, or unbounded progress.

The archive's [executed implementation validation](../../crates/frost-collections-integer/docs/integer-output-validation.md)
and [benchmark evidence](../../crates/frost-collections-integer/docs/integer-output-benchmarks.md) remain separate from
these proposed gates. No existing test total is presented as validation of
the unimplemented views or persistent profile.

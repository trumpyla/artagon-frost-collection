# Borrowed integer expressions

Status: **proposed API and execution design; not implemented**. Reviewed on
2026-11-09 against the corrected integer profile. The existing eager methods
and the [integer-set contract](integer-sets.md) remain unchanged. The
[adversarial research review](../reviews/lazy-views-adversarial-review.md),
[source audit](../reviews/lazy-views-source-audit.md), and
[performance audit](../reviews/lazy-views-performance-audit.md) explain the
evidence behind this proposal. Structural sharing has its own
[persistent-storage design](persistent-integer-storage.md).

## 1. Decision and names

Add `IntSetExpr<'a, K>` as an explicit, borrowed query over integer sets. Keep
`FrozenIntSet<K>::union`, `intersection`, `difference`, and
`symmetric_difference` eager and value-returning. An expression describes a
calculation; a frozen set owns a fully valid result. A proposed
`PersistentIntSet<K>` would own a fully valid, structurally shared result.
Neither owned type is an expression history.

The companion [general set-algebra proposal](set-algebra.md) extends query
composition to ordinary stored objects and other set categories. This integer
profile remains specialized: generic borrowed objects need representative
provenance, and their XOR cannot inherit this profile's unrestricted parity
flattening. The general proposal recommends typed views first for borrowed
objects, while this runtime integer arena retains its separate scope.

Use `IntSetExpr` rather than the broad `SetExpr`: its canonical codec, word
width, by-value traversal, and wide cardinality are integer-profile contracts.
Use `view()` as the short constructor, without adding a second public leaf
wrapper. Reserve the roadmap's `FrozenIntSetView<'a, K>` spelling for a future
validated view of encoded backing storage, if that work proceeds. A storage
view and an algebra expression have different validation and ownership rules.

The first implementation can use today's Sorted, Dense, and Roaring storage.
It does not require an in-house Roaring replacement, a persistent directory,
Elias–Fano, runtime code generation, unsafe intrinsics, or a public cursor trait.
The first benefit to demonstrate is a terminal query that avoids constructing
intermediate result sets. Introducing a new storage engine is a separate cost.

## 2. Proposed public surface

The following signatures are a design sketch, **not APIs available in this
archive**. The lifetimes distinguish the expression object from its operands.

```text
impl<K: IntKey> FrozenIntSet<K> {
    fn view(&self) -> IntSetExpr<'_, K>;
}

impl<'a, K: IntKey> From<&'a FrozenIntSet<K>> for IntSetExpr<'a, K>;

impl<'a, K: IntKey> IntSetExpr<'a, K> {
    fn and(self, rhs: impl Into<Self>) -> Self;
    fn or(self, rhs: impl Into<Self>) -> Self;
    fn and_not(self, rhs: impl Into<Self>) -> Self;
    fn xor(self, rhs: impl Into<Self>) -> Self;

    fn contains(&self, key: &K) -> bool;
    fn is_empty(&self) -> bool;
    fn iter(&self) -> IntSetExprIter<'_, 'a, K>;
    fn cardinality_bounds(&self) -> IntSetCardinalityBounds;
    fn count_wide(&self) -> u128;
    fn try_cardinality(&self) -> Result<u64, IntSetError>;
    fn materialize(&self, policy: IntSetPolicy)
        -> Result<FrozenIntSet<K>, IntSetError>;
}
```

For example, the intended spelling is:

```text
let expr = a.view().and(&b).and(c.view().or(&d));
let possible = expr.cardinality_bounds();
let exact = expr.count_wide();
let first_twenty = expr.iter().take(20);
let result = expr.materialize(IntSetPolicy::auto())?;
```

`and_not` means left difference. `xor` means symmetric difference, including
odd-parity semantics in a flattened XOR group. There is no unrestricted unary
complement. `universe.view().and_not(&set)` supplies an explicit valid universe.
All operands have the same K, not merely the same `K::Word`. Restricted codecs
remain valid because evaluation only selects existing input codes.

There is no zero-input conjunction constructor in the initial surface: `view`
starts from an actual set. A later bulk constructor should require a first
operand for intersection, or an explicit universe. Empty union and XOR can
legitimately mean the empty set; do not accidentally give all four operators
the same empty-input rule.

`iter()` yields K by value in strictly increasing logical order, without
duplicates. It implements `Iterator` and `FusedIterator`; it initially makes
no `DoubleEndedIterator`, `ExactSizeIterator`, or cheap-clone promise. Each
call creates independent execution state. No method borrows an `&K` from
compressed or computed storage.

`is_empty()` returns as soon as it finds one verified member; it does not
compute the entire count. Proven empty/nonempty bounds may answer it earlier.

The expression can implement `ElementType`, `SetContains<K>`, and
`ValueElements`. It does not implement `Elements`, `SetGet`, the existing
reference-valued `OrderedElements`, `Len`, or the u64-valued `Cardinality`.
It also does not implement `Eq`/`Hash`: plan identity and logical set equality
are different, and an implicitly expensive logical comparison is unnecessary
for this first API. Materialized values retain their existing traits.

## 3. Ownership, allocation, and repeated use

A leaf view should be inline and allocation-free. Combining leaves may
allocate plan metadata; creating an iterator may allocate cursor and merge
workspace. Avoid describing an arbitrary k-input expression as allocation-free.
The promise is that streaming and count queries need not allocate a collection
containing every result member. Workspace, input conversion, and materialized
output must be reported separately in measurements.

The expression owns immutable plan metadata and borrows its operands. A private
arena stores nodes and child indices, with no recursively owned node chain.
Dropping a deeply composed expression therefore does not recursively drop a
tree of boxes. Node construction, evaluation, and plan compaction also use
explicit traversal state rather than unbounded call-stack recursion.

Appending a leaf to a compatible root group should be amortized constant work
plus its small metadata update. Combining two already built plans may require
moving and rebasing O(nodes) metadata; document and measure this cost. A later
explicit planner can amortize more expensive analysis. Rebuilding the entire
plan on every `.and(&leaf)` would make a common construction pattern quadratic
and fails the admission criteria.

Simple leaf and flat conjunction/disjunction membership can use allocation-free
loops. A general mixed Boolean plan may need O(plan depth) evaluation scratch;
the initial contract permits that allocation. `contains` never implicitly
materializes the resulting set. Encode the query once and evaluate encoded
membership, short-circuiting where the operator allows it. Repeated membership
could later use an explicit prepared evaluator with reusable scratch, if that
workspace is a measured cost. Do not introduce hidden mutable scratch or a
complicated decision-diagram compiler just to claim zero allocations.

Borrowing does not bound the number or depth of plan nodes, and a borrow can
have a `'static` lifetime. Keep `SnapshotValue` sealed and do **not** implement
it for expressions, regardless of lifetime. A snapshot reader loads an owned
version handle, then constructs a view borrowing that handle's value for the
query. Retaining the handle retains its old storage until the query ends.

Repeated queries repeat evaluation. There is **no automatic materialization
after two calls**, no hidden exact-count cache, and no implicit `OnceCell`.
A partial iterator, `take(1)`, exact count, and complete iteration have different
costs and cannot be treated as equal reuse events. Explicit materialization
lets the caller select the result policy, handle its errors, release operand
borrows, and decide whether retained storage is worthwhile.

The immutable expression and independent iterators use natural Rust auto traits.
Retain K's restrictions and the borrowed operands' restrictions. No manual
`Send`/`Sync` implementations are needed. The initial expression profile uses
`alloc` and does not need Arc, pointer-width atomics, or std. Its availability
must not change according to an unrelated persistent-storage feature.

Resource control is explicit about its scope. Ordinary builders follow normal
Rust allocation behavior; they do not claim recoverable allocator failure or
a hard plan-memory budget. A parser accepting untrusted query descriptions
must limit node count before constructing a plan. If Frost later provides a
bounded builder, its fallible API must enforce node, edge, and workspace limits
before allocation; borrowing alone is not that mechanism.

## 4. Plan semantics and safe normalization

Private nodes represent Empty, Leaf, All, Any, Difference, and Parity. Flatten only
adjacent groups with compatible operators. For example:

- `(A AND B) AND C` can become one All group.
- `(A OR B) AND C` retains an Any child under All.
- `(A - B) - C` can deliberately become `A - (B OR C)`, but ordinary
  reassociation into `A - (B - C)` is incorrect.
- XOR duplicates cancel in pairs. Removing every repeated XOR operand would
  change the answer; ordinary AND/OR identity deduplication rules do not apply.

An All group always has at least one child, including after normalization.
Zero-child Any or Parity groups normalize to Empty; a zero-child All group is
not an implicit full encoded universe. Difference retains exactly two ordered
children unless a proven identity replaces the node.

Avoid distributing expressions into conjunctive or disjunctive normal form:
the expanded plan can be exponentially larger than the user's expression.
The internal arena is acyclic and its indices are checked; an initial tree of
occurrences is sufficient. Common-subexpression sharing is not a prerequisite.

Empty and same-operand identities may simplify value evaluation. Pointer
identity can prove that two borrowed operands are the same set; unequal
pointers cannot prove unequal contents. Validate any result policy even when
an identity returns a clone. Do not use semantic simplification to drop an
explicit dense or partition limit.

## 5. Guaranteed bounds and exact counts

`IntSetCardinalityBounds` has private u128 lower and upper fields, with
`lower()`, `upper()`, and `exact() -> Option<u128>`. Its invariant is
`0 <= lower <= actual_count <= upper <= U`, where the safe encoded universe
cap U is `2^32` or `2^64`. For a restricted K this can be a loose cap: it does
not assert that every code is valid or permit creating missing codes.

Each node caches sound bounds during plan construction. Reading the root's
bounds is O(1); constructing or recomputing all node metadata is O(nodes).
Bounds are not a selectivity estimate. A separate private estimate may guide
execution order or a checked, capped capacity reservation. It must not bypass
allocation limits, prove that a fixed buffer is sufficient, suppress matches,
or become an Iterator correctness promise.

Given operand intervals `[la, ua]` and `[lb, ub]` in the same encoded universe,
these binary rules are safe:

| Operation | Lower | Upper |
| --- | --- | --- |
| Union | `max(la, lb)` | `min(U, ua + ub)` |
| Intersection | `(la + lb).saturating_sub(U)` | `min(ua, ub)` |
| Difference | `la.saturating_sub(ub)` | `min(ua, U - lb)` |
| XOR | `max(la.saturating_sub(ub), lb.saturating_sub(ua))` | `min(U, ua + ub, 2*U - la - lb)` |

Every binary intermediate is at most `2U`, so u128 is sufficient. Clamp/fold
per operation instead of collecting an unchecked sum across arbitrary fan-in.
Disjoint encoded support envelopes give stronger results: intersection is
empty, difference keeps the left interval, and union/XOR have bounds
`[la + lb, min(U, ua + ub)]`. Retain the universe cap even for disjoint support:
loose upper bounds need not have been tightened to each envelope's capacity.
Overlapping envelopes alone do not establish any matching member. Compute
inclusive widths and endpoint arithmetic in u128.

The independent [bounds oracle](../../verification/integer-views/README.md)
checks these mathematical rules over finite universes and wide boundary
cases. It is not a test of a production expression implementation, which does
not exist in this revision.

Two representable u64 sets can have a union with **2^64 members**. Therefore
the view's exact terminal is `count_wide() -> u128`. The optional convenience
`try_cardinality()` uses `IntSetError::CardinalityOverflow` when the result
does not fit u64. Neither method promises O(1) work; exact cached bounds or a
specialized kernel can avoid traversal, otherwise evaluation is required.
Do not silently saturate, wrap, or repurpose a valid count as an unknown marker.

Iterator `size_hint()` describes **remaining** output, not original leaf counts.
It starts with a proven interval and subtracts consumed result members safely,
or returns a more conservative interval. Consumption includes matching members
skipped by `nth` or `advance_by` if those methods are overridden; rejected
candidates and duplicate cover entries are not consumed result members.
A u128 lower bound larger than usize
may be capped at `usize::MAX`; an unrepresentable upper bound becomes `None`.
Pending candidates and duplicate suppression must be accounted for. Returning
`(0, None)` is preferable to an invalid estimate. Exhaustion returns `(0,
Some(0))`. The ordinary `Iterator::count()` still returns usize; callers who
need a wide exact count use the expression terminal. See the
[Rust Iterator contract](https://doc.rust-lang.org/core/iter/trait.Iterator.html).

## 6. A correct initial execution engine

Begin with a small, independently checkable engine. For a general expression,
derive a **candidate cover**: a set of borrowed leaves whose union contains
every possible output member.

| Node | Valid candidate cover |
| --- | --- |
| Empty | No leaves; no candidates. |
| Leaf A | A |
| All(children) | Any one child's cover; choose a promising small cover. |
| Any(children) or Parity(children) | Union of the child covers. |
| Difference(left, right) | The left cover. |

This rule follows directly from subset relations, not heuristic accuracy.
For `(A OR B) AND C`, either C or the union of A and B is a valid cover.
For `A - (B OR C)`, enumerate A. Even a badly estimated choice remains exact.
Selecting a child of All selects that child's **whole cover**, not an arbitrary
small leaf inside it. Derive the root's selected leaf frontier once; do not
cache a complete descendant-leaf vector at every node, which can make a deep
plan quadratic in space. Per-node planning summaries can remain fixed-size.

Enumerate that cover in encoded ascending order, deduplicate candidates, and
evaluate the original Boolean expression's encoded membership before yielding.
A single-leaf cover needs one cursor; several leaves can use a linear frontier
for small fan-in and a heap for larger fan-in. The original expression remains
the membership oracle, so a cover containing extra candidates cannot create
false positives. No candidate is decoded until it is yielded as K.
Each iterator/count execution allocates any general membership frame buffer
once and reuses it for all candidates. It must not invoke a separately
allocating public `contains` implementation for each candidate. This internal
workspace reuse does not require a new public prepared-query type.

This supplies a correct mixed-expression baseline without forcing a nested
union cursor to expose a speculative match. Its worst case can scan a broad
cover and issue many probes, so it is a baseline, not a universal performance
claim. The invariant `result is a subset of cover` needs its own property tests
before optimizing plan selection.

Add direct terminal dispatch where existing operations already win:

- A flat intersection can iterate its smallest exact leaf and probe the others.
  Predicate ordering may use estimated rejection benefit and probe cost; length
  alone does not model a Dense probe versus a Roaring tree lookup.
- A two-leaf count can reuse encoded native kernels. Compute union/XOR counts
  from u128 operand counts and a safe intersection count, avoiding the existing
  u64 overflow path for a full-domain view.
- Matching Dense windows can fuse word operations and count ones without
  emitting members. Differently based windows require absolute coordinate
  alignment and padding masks.
- A pure union can merge/group inputs directly. A count-only terminal may
  prefer a different plan from an iterator that will stop after one result.

The existing mixed count path can allocate conversion workspace. Reusing it
does not make view count queries allocation-free. Measure that workspace and
keep the current limits; an output cap is not a total-memory cap.

## 7. Cursor contract and later window execution

Keep cursors private initially. A leaf cursor has an out-of-band state:
BeforeFirst, At(word), or Exhausted. `seek_ge(target)` is total: from BeforeFirst
it finds the least member at least target; from At(x), a target <= x retains x;
otherwise it advances to the least remaining member at least target. Repeated
seeks are idempotent. `advance()` moves strictly beyond the current member, or
exhausts, and can position a BeforeFirst cursor. Exhaustion is absorbing.
Every u32/u64 value, including MAX, remains available as data.

Do not implement advancement as an unchecked `current + 1`. An explicit
exclusive bound or cursor advancement avoids overflow at MAX. Dense cursors
skip empty words; Sorted cursors may gallop then binary-search; a Roaring
adapter can use its available public operations. A uniform method name does
not promise uniform cost, and the supplied `advance_to` measurement does not
justify banning future seek strategies.

An advanced mixed engine may later expose private progress states:

```text
Found(word)
Bound(Inclusive(word) | Exclusive(word))
Exhausted
```

`Bound` is a statement about remaining possible output, **not a member**.
Only Found may be yielded or counted. Exclusive(MAX) means exhaustion without
integer wraparound. Repeating an unchanged bound must either advance an
internal child state or exhaust; a driver must not spin forever because the
visible bound stayed unchanged. Specify a finite progress measure and bounded
work per progress step before replacing the baseline with this protocol.

Arntzenius's [seekable-intersection paper](https://arxiv.org/html/2510.26016v1)
motivates this separation; its sparse-endpoint example and the source's
prose/code discrepancy are discussed in the performance audit. It is not proof
that a Frost implementation is correct or faster. Flattening homogeneous
conjunctions is useful independently of adopting the more advanced protocol.

Window execution should use encoded absolute coordinates, initially testing
2^16 windows as a kernel unit. It does not require converting every operand
into persistent chunks. Existing sorted slices can delimit a window; Dense
storage supplies aligned words; Roaring remains behind a public adapter.
Iterate occupied windows instead of scanning the entire u64 span. Prefix
increment and the final window use checked/wider arithmetic.

Dispatch once per window or block. Array/bitmap intersection can probe array
members; bitmap pairs use word operations; a run descriptor is useful only
where available without reconstructing the whole input. Avoid a fresh 8 KiB
temporary for every sparse overlap or one full bitmap per operand. Reuse
bounded workspace and charge conversion costs in benchmarks.

For k-way union, deferring cardinality inside a private writable accumulator
can save repeated work. Repair and validate before exposing a frozen result.
Choose promotion from measured overlap, output shape, and peak workspace;
1,024 is not a universal threshold. A 16-bit array's payload reaches 8 KiB at
4,096 elements, while duplicate overlap, run encoding, and headers change the
decision. Never publish an unknown-cardinality sentinel as a valid set.

## 8. Materialization and limits

`materialize(policy)` explicitly chooses the output policy for the existing
three-candidate FrozenIntSet profile. It is equivalent in membership to
collecting the expression, but should feed canonical encoded results directly
to a private validated result builder rather than decoding and re-encoding
every K. The builder retains exact metadata and never fabricates a key.

Inherited limits of ordinary eager binary methods remain unchanged. This
separate explicit policy is intentional: a query plan has no unambiguous
default policy across arbitrary unions, differences, and nested operands.
Forced-representation errors remain observable. Identity or empty shortcuts
must still check the requested policy. A result larger than u64 cardinality
returns `CardinalityOverflow`; an upper bound above u64::MAX alone is not
enough to reject a smaller actual answer.

`IntSetLimits` currently bounds Dense payload and Roaring partition count. It
does not bound plan memory, Sorted fallback, conversion buffers, allocator
overhead, peak build memory, or storage kept alive by input borrows. Any future
strict workspace limit needs a separate named contract and an exact fallback
or an error. A lossy page bitmap that needs an external database recheck is
not a valid result for Frost's exact membership API.

## 9. Admission sequence and verification

First implement the arena, bounds, candidate-cover oracle, four Boolean
operators, and independent iterators over existing storage. Keep optimized
paths private and compare each against this baseline. Admit specialized
flat-intersection and binary-count dispatch next, then measured window
kernels. Advanced lower-bound propagation and explicit prepared workspace
are later steps with their own evidence. Persistence can proceed separately.

The release gate for the **future implementation** must cover:

1. Differential arbitrary expression trees against BTreeSet, including every
   leaf representation, both word widths, signed boundaries, repeated leaves,
   restricted codecs, and unlike domains rejected at compile time.
2. Exhaustive reduced-universe membership, exact count, bounds, materialized
   outputs, candidate-cover inclusion, and valid/invalid rewrite rules.
3. Cursor first/repeated/backward-target seeks, strictly advancing output,
   exhaustion, MAX, partial batches, and nested Found/Bound separation.
4. Iterator prefixes, all remaining-size-hint checks, independent iteration,
   drop without completion, and u128 counts with synthetic full-domain models.
5. Concurrent readers of one immutable expression, each with private iterator
   state; snapshot handle lifetime and compile-fail SnapshotValue eligibility.
6. No-std alloc and 32-bit checks, deep plan construction/drop without stack
   overflow, allocation/work counters, and Miri on executed ownership paths.

Benchmark complete terminals: construction plus one query, first match,
prefix iteration, exact count, full traversal, materialization, and repeated
use. Include skew, comparable inputs, sparse u64 prefixes, runs, alternating
values, sparse endpoints, mismatched Dense bases, and every mixed Boolean
shape. Retain losing cases. Compare against the current eager profile and
native controls under identical CPU flags. Do not use timing assertions as
correctness tests or import a published threshold without Frost measurements.

Existing [validation](../integer-output-validation.md) verifies the current
integer implementation only. None of its test counts or timings is evidence
that these proposed expression types have been implemented or validated.

# Set algebra across all Frost set families

Status: **design proposal, not an implemented API**. Review date: **2026-11-08,
America/New_York**. Target remains **Rust 1.99.1, edition 2024**.

This iteration broadens the scope from integer expressions to all supported set
categories. It is grounded in the uploaded repository, the
[supplied research](../reviews/supplied/lazy-views-research.md), and its existing
[adversarial review](../reviews/lazy-views-adversarial-review.md). The companion
[review ledger](../reviews/generic-set-algebra-review.md) records findings,
executed checks, and the distinction between current behavior and this proposal.

## Formal-domain companion

This detailed expression proposal is complemented by the
[abstract refinement model](algebra/formal-model.md),
[value and key domains](algebra/domains.md), and [set laws](algebra/sets.md).
For runtime-bound sources, compatible dictionaries and revision contexts are
additional preconditions, not consequences of a common Rust element type.
Left-representative semantics are retained. The companion implements none of
these views and adds no domain parameter; whether a refactor should is open
under the [API compatibility decision](decisions.md#api-compatibility-is-not-a-constraint-2026-11-12).
Its [rewrite rules](algebra/rewrite-contracts.md) separate membership from
representatives, order, effects, error behavior and completeness.

## 1. Recommended decision

Make algebra a capability of sets and set expressions, independent of whether
the members are integers and whether the input is mutable or frozen. Preserve
the existing semantic families and specialized kernels.

The first implementation should add typed borrowed views for ordinary stored
elements. Keep encoded integer evaluation and lending byte-stream evaluation
as specialized profiles under the same operation vocabulary. Materialization
explicitly selects an output collection and ownership policy. Persistence is a
separate storage decision.

The architecture has four boundaries:

| Boundary | Responsibility | Must not silently decide |
| --- | --- | --- |
| Set semantics | Membership, equivalence, representative selection, and promised traversal order | Storage, hasher, or whether elements can be cloned |
| Expression | Which operations the user requested; operand lifetimes | Automatic caching or a history of published set updates |
| Execution | Merge, probes, native bitmap operations, streams, and terminal-specific planning | Different representatives or a different equality relation |
| Materialization | Result family, ownership transfer, hasher, limits, and final validation | Unrequested element copies or a change in logical domain |

Do not introduce a universal `Set<T>` enum containing every tree, hash table,
bitmap, and FST. Do not impose `T: Clone + Hash + Ord + IntKey` on all sets.
Generic algebra means capability-directed operations, not identical physical
storage or identical iterator item types.

### What changes relative to the integer proposal

The existing [integer expression design](integer-views.md) remains a valid
specialized proposal. Its integer codecs, u128 universe arithmetic, encoded
candidate covers, and word kernels cannot simply be generalized by replacing
`K: IntKey` with `T`.

This design adds three requirements that the integer-only plan largely avoids:

1. Preserve the actual stored representative, including metadata ignored by
   equality or ordering.
2. Preserve borrowed lookup and the individual backend's query relation without
   requiring every set to support every query type.
3. Distinguish stable references, reconstructed values, and lending streams.

The research's recommendation to keep integer algebra eager does not change the
existing ordinary families: their named binary methods already return lazy
borrowed iterators. That behavior stays intact.

## 2. Current repository coverage

| Category | Implemented today | Proposed extension |
| --- | --- | --- |
| `FrozenOrderedSet<T>` | Four borrowed algebra operations, predicates, scalar counts, ascending results | Compose expressions and combine with other compatible sets |
| `FrozenHashSet<T, S>` | Four borrowed operations, predicates, scalar counts, different operand hashers, left representatives | Compose expressions and combine with other compatible sets |
| `MutableOrderedSet<T>` and `MutableHashSet<T, S>` | Membership, traversal, mutation, fixed freeze partners | Borrow directly for algebra without a freeze/rebuild |
| Native standard sets and `Frozen<B>` wrappers | Native APIs on native types; Frost read capabilities on both | A common explicit Frost view, including mixed backends |
| hashbrown and indexmap sets | Membership and borrowed traversal adapters | Generic views respecting native query relations |
| imbl hash and ordered sets | Membership and borrowed traversal adapters | Generic views; optional representative retrieval only where supported |
| PHF sets | Membership and borrowed traversal, including core-only support | Core-compatible binary views; declaration order is not sorted order |
| `FrozenIntSet<K>` | Specialized eager algebra, encoded kernels, policies, wide counts | The existing proposed `IntSetExpr<K>` query path |
| Roaring and FixedBitSet | Native/exact-wrapper bitmap operations and value traversal | Specialized value algebra and explicit compatible bridges |
| FST byte-string sets | Membership and lending streams | Stream algebra, explicit owned materialization, directional membership filters |
| Live concurrent sets | Guard-scoped/live access | Capture a coherent snapshot or keep a separately documented live-filter API |

Source entry points are [read capabilities](../../crates/frost-collections-core/src/read.rs),
[ordinary set families](../../crates/frost-collections-core/src/family/sets.rs),
[hash algebra](../../crates/frost-collections-core/src/family/hash_set_ops.rs),
[ordered algebra](../../crates/frost-collections-core/src/family/ordered/set_ops.rs),
[backend adapters](../../crates/frost-collections-core/src/backends/mod.rs), and
[current operation contracts](../set-operations.md).

No current wrong-answer bug in the ordinary frozen kernels was established by
this review. The problem is incomplete coverage and a missing general expression
contract, not absence of generic `T` support.

## 3. Membership, representatives, and order are separate contracts

Suppose a record contains an ID and a label, while equality, hashing, and ordering
all use only the ID. These are equal members but observably different objects:

```text
A: Record { id: 7, label: "from A" }
B: Record { id: 7, label: "from B" }
```

Frost's ordinary family contract retains the left stored representative for
union and intersection. Extend that contract recursively to expressions:

| Expression | Membership | Representative if present |
| --- | --- | --- |
| `A union B` | Present in either child | A's representative if present there; otherwise B's |
| `A intersection B` | Present in both children | A's representative |
| `A difference B` | Present in A and absent from B | A's representative |
| `A symmetric_difference B` | Present in exactly one child | That child's representative |

Here A and B can themselves be expressions. A representative is selected by
the original expression, not by whichever physical iterator discovers a key
first. It is also not necessarily the earliest matching leaf in a flattened
list: in `(A difference B) union C`, an A member rejected by B must not displace
the surviving C representative.

Rust's native HashSet intersection permits either stored representative. Frost
has a stronger contract, so directly delegating to a native operation requires
checking that operation's representative semantics. [S1]

### Rewrites require terminal-specific justification

Let A, B, and C each contain ID 7 with their own label:

```text
(A symmetric_difference B) symmetric_difference C  -> C's object
A symmetric_difference (B symmetric_difference C)  -> A's object
```

The member is present in both answers. Flattening the generic expression into
one parity node loses the representative rule. Even cancellation of the same
leaf can be invalid:

```text
(A symmetric_difference B) symmetric_difference A  -> A's object
simplifying the whole expression to B               -> B's object
```

Consequently a count/membership plan may apply parity identities while a
reference-producing plan retains original provenance. A direct binary
`A symmetric_difference A` can still simplify to empty.

| Rewrite | Membership | Representatives | Additional order condition |
| --- | --- | --- | --- |
| Reassociate union without changing operand sequence | Preserved | Preserved | Preserve any specified source priority |
| Reassociate intersection without changing operand sequence | Preserved | Preserved | Preserve the first source's traversal contract |
| Swap union or intersection operands | Preserved | May change | May also change order |
| Reassociate XOR or cancel nonadjacent repeated XOR leaves | Preserved | May change | Cannot generally preserve full observable output |
| Reassociate or commute difference | Not generally preserved | Not generally preserved | Forbidden without a proven identity |
| Rewrite `(A intersection B) union C` as `(A union C) intersection (B union C)` | Preserved | May change | C can be replaced by A |

Even a representative-preserving rewrite can change traversal order. For
`A=[x,y]`, `B=[y]`, and `C=[x]`, left-first enumeration of
`A intersection (B union C)` gives `[x,y]`; distributing it into
`(A intersection B) union (A intersection C)` gives `[y,x]`.

The [finite semantics oracle](../../verification/set-algebra/README.md) checks
these distinctions independently of the production implementation. The examples
are design counterexamples, not claims that current binary kernels are wrong.

## 4. Capability design

Retain the existing small capabilities:

| Existing capability | What it promises | What it does not promise |
| --- | --- | --- |
| `Elements` | Borrowed iteration of stored members | Uniqueness, sorted order, or fusion by itself |
| `SetContains<Q>` | Membership for a query type | A stored reference, a snapshot, or a particular lookup cost |
| `SetGet<Q>` | Retrieval of the stored representative | Cheap arbitrary expression traversal |
| `ValueElements` | Values reconstructed or copied during traversal | Stable addresses for those values |
| `Len` / `Cardinality` | Existing usize/u64 member-count contracts | Universal expression count width or constant counting cost |

Add a safe semantic opt-in, provisionally `SetSource`, representing a finite
set. Any supplied traversal yields every logical member exactly once before
its first `None`; it is complete as well as duplicate-free. Each new traversal
has independent state and observes the same stable membership, without a
required iteration order. Queries and traversal use one coherent logical
identity. Logical traversal ends at the first `None`: the algebra never resumes
it, even if an underlying iterator type lacks a `FusedIterator` promise.
Late elements cannot repair a traversal that omitted members before exhaustion.
Membership-only sources remain finite even when they expose no enumeration.
Keep this semantic opt-in independent of `Elements`,
`SetGet`, and a blanket element-trait bundle. Apply operation-specific bounds
where they are actually needed. Implement it for audited set adapters; do not
blanket-implement it for arbitrary `Elements + SetContains` combinations.

This is a logical contract, not an unsafe trait. Incorrect downstream
implementations may produce incorrect algebra; they must never justify
unchecked memory access. Structural stability does not prohibit interior
mutation of metadata that leaves equality and membership unchanged. Ordinary
Rust key laws still apply.

Basic same-element borrowed operations can require both sources to describe the
same T and a coherent membership relation. Later directional filters can admit
membership-only right operands, such as an FST queried by a stored String's
bytes. Such a filter does not automatically enable union or right-side output.

### Minimum bounds depend on the operation

| Borrowed baseline | Needed traversal | Needed membership | Representative lookup |
| --- | --- | --- | --- |
| Intersection | Left | Right can query the left member type | None |
| Difference | Left | Right can query the left member type | None |
| Union | Both, yielding the same T | Left can query right members | None |
| XOR | Both, yielding the same T | Each can query the other | None |
| Intersection driven from right | Right | Matching left lookup | Left `SetGet<T>` or an equivalent explicit capability |

A complete expression's `contains<Q>` requires the relevant leaves to support
`SetContains<Q>`. This bound is separate from the bounds required merely to
iterate that expression. Do not force a traversal-only operand to have a usable
hasher solely because another terminal could require one.

There is no global `Clone`, `Default`, `Debug`, `Send`, `Sync`, or `'static`
requirement. Ordered backends bring their ordering bounds; hash backends bring
their hashing bounds. Cross ordered/hash operations naturally need whatever
both concrete operands need. A type used only in ordered sets need not gain
`Hash`.

### Representative retrieval is an optional optimization

The imbl hash adapter intentionally has no `SetGet`: its supported native API
has membership but no stored-value lookup. Baseline left traversal covers this
case without a hidden linear representative search. imbl's ordered set exposes
retrieval. [S4, S5]

Do not implement `SetGet` for every expression by quietly scanning the whole
expression. Membership and representative lookup have different costs. A
planner can use a cheap resolver where it exists, retain left-driven traversal
where it does not, or explicitly pay for an index when the caller chooses that
execution policy.

### Equivalence and borrowed queries

Keep each backend's supported relation. Standard hash sets use lawful
`Eq`/`Hash` with borrowed queries; hashbrown and indexmap can support their
`Equivalent` relation. [S1, S8] A wrapper must not silently narrow those queries
to `Borrow` alone.

The first general algebra combines one logical member domain. Different
hashers are allowed; different definitions of identity are not silently
reconciled. Case-sensitive and case-insensitive strings need an explicit common
key wrapper or normalization step. Projection onto a record ID is a separate
keyed/join-like operation with a documented representative policy.

## 5. Proposed API: typed views first

Use a new module, provisionally `frost::set`, with a free `view` constructor.
Do not rely on a blanket extension trait named `union` to override or fall back
from the existing inherent methods.

The following is **proposed syntax**, not code available in this repository:

```text
use frost::set::view;

// Different set representations; each stores String.
let query = view(&requested_permissions)
    .intersection(view(&allowed_permissions))
    .difference(view(&revoked_permissions))
    .union(view(&administrator_permissions));

let can_read = query.contains("read"); // str lookup where every leaf supports it
let first_match = query.iter().next(); // borrowed &String
let exact_count = query.try_count()?;  // checked, wide exact count

let refs: FrozenOrderedSet<&String> = query.iter().collect();
let owned: FrozenOrderedSet<String> = query.iter().cloned().collect();
```

The concrete expression structure remains visible to the compiler:

```text
SetView<Leaf<'a, S>>
SetView<Union<L, R>>
SetView<Intersection<L, R>>
SetView<Difference<L, R>>
SetView<SymmetricDifference<L, R>>
```

Leaf views borrow inputs. Operator nodes own child views and preserve their
semantic left/right order. Applications normally use type inference. The public
wrapper and node names can be crate-owned with private fields; backend types
appear only in inferred query temporaries, not in ordinary family identities.

This approach permits a method generic over Q to inherit the leaves' precise
lookup capabilities. `Elements` uses a generic associated iterator type and is
not dyn-compatible; a universal `dyn Elements<Item=T>` leaf is not viable under
the current Rust trait-object rules. [S2]

### Availability and lifetime contracts

The core borrowed binary machinery should require only `core`. It can therefore
serve PHF and downstream core-only sets without adding an allocation dependency.
Native containers still require their normal features. Dynamic plan metadata,
reference indexes, owned output, and persistent storage have separate `alloc`
or `std` requirements.

View construction and a binary fallback need no output collection and no
element cloning. A static expression has state proportional to its structure;
do not describe an arbitrarily large expression as constant-space. Its typed
shape is fixed at compilation, so an arbitrary runtime loop cannot keep
reassigning increasingly nested expressions to one variable. Runtime query
construction needs a separate planner described below.

Initially tie an iterator's borrow conservatively to the view and its operands.
Do not promise that returned references can outlive a temporary expression
without checking the actual public signatures. Type aliases and GAT bounds
must be compile-tested with nonstatic borrowed members. An iterator may be
fused by wrapping source iterators in `Fuse`; `Elements` alone does not provide
that guarantee. Avoid unnecessary higher-ranked GAT constraints that can
accidentally force `'static`.

Repeated `.iter()` calls create independent execution state. `size_hint` reports
remaining output using proven bounds, including after partial consumption; a
conservative hint is preferable to a false exact length. No universal
`ExactSizeIterator`, `DoubleEndedIterator`, or cheap `Clone` promise is made.

### Native and existing family methods retain their contracts

`frozen_ordered.union(&other)` continues to use the existing ascending kernel.
`frozen_hash.intersection(&other)` keeps its current small-side behavior and
left representative lookup. `FrozenIntSet::union` continues to return an eager
owned set with policy validation.

The new wrapper is an explicit query API. Its mathematical vocabulary is shared,
but it does not change the result type of an existing method.

## 6. Traversal order

Choose **unspecified traversal order** for the initial general `SetView::iter`.
Always preserve membership, uniqueness, and representative provenance.

The correct baseline happens to emit left members before right-only members.
That encounter order is not a public promise of the general view. This leaves
room for a smaller right input to drive an intersection when representative
recovery is available. Existing ordered and native insertion-order APIs keep
their existing guarantees.

When ascending traversal is required, use the existing ordered family methods,
materialize an ordered result explicitly, or add a separately admitted
`ordered_view` profile. The latter needs an actual sorted-unique traversal
capability. `T: Ord`, `OrderedElements` extrema, an `IndexSet`, and a PHF
`OrderedSet` do not establish that traversal is sorted.

For such an ordered profile, union/XOR require sorted streams on both sides for
a merge; intersection/difference can filter a sorted left stream using an
unordered right membership source. A planner may not drive from an unordered
right source and claim ascending output without restoring order and accounting
for its cost.

Defer a general stable/ordered/unordered mode matrix. Insertion-ordered
materialization from an unordered view has the view's chosen encounter order,
which is not reproducible by contract. A caller needing stable encounter order
must use a documented stable/native path; the destination container alone
cannot create determinism. IndexSet's order is explicitly distinct from hash
order and can be affected by its mutation APIs. [S8]

## 7. Correct baseline and terminal-specific execution

### Borrowed enumeration

Define an independent baseline recursively:

| Node | Enumerate |
| --- | --- |
| Leaf | Its unique stored members |
| Intersection | Left output whose members satisfy right membership |
| Difference | Left output whose members do not satisfy right membership |
| Union | All left output, then right output absent from left |
| XOR | Left output absent from right, then right output absent from left |

Filters preserve uniqueness. Concatenated parts are disjoint. Selecting the
surviving child's original value preserves the representative law. No new
deduplication table, element clone, hash requirement, or sort is needed by this
baseline beyond the existing operands' capabilities.

This algorithm is an oracle and a fallback, not a claim of optimal execution.
A membership query on a nested expression recursively visits leaf predicates;
lazy composition does not automatically fuse all that work.

### Candidates and representatives

For `A intersection B`, the smaller B can supply a complete candidate cover.
If B produces ID 7, a successful membership check proves that ID 7 belongs in
the answer. It does not authorize returning B's record. The engine must recover
A's representative or choose a traversal that already carries it.

Keep physical candidate choice and semantic representative resolution separate.
For mixed expressions, resolve against the original provenance structure. A
count-only terminal can often omit this work completely.

### Select execution for the requested terminal

| Terminal | Primary objective | Semantic restriction |
| --- | --- | --- |
| `contains<Q>` | Short-circuit supported leaf probes | No output allocation or hidden materialization |
| `is_empty` | Stop after a verified match, or a proven bound | Do not compute full cardinality just to answer existence |
| `try_count` | Exact count using native counts, probes, or suitable workspace | Checked arithmetic; representative recovery unnecessary |
| `iter` | Produce requested members incrementally | Preserve representatives and any selected order contract |
| Owned materialization | Construct the requested destination efficiently | Explicit element ownership, hasher, validation, and limits |

The count and iteration plans may be different. Boolean evaluation order is
not a representative-selection rule. Pure, coherent key comparison and lookup
are normal collection assumptions; do not promise a user-visible order or
fixed number of Hash/Eq/Ord callback invocations across optimized algorithms.

Existing set predicates remain expressible without additional storage:
`A subset B` is `(A difference B).is_empty()`, `A superset B` reverses that
difference, and `A disjoint B` is `(A intersection B).is_empty()`. The initial
view can expose these compositions; named convenience methods can later reuse
the same semantics with specialized early-exit kernels.

### Kernel selection without unstable specialization

Retain existing family kernels as the direct fast paths. The generic fallback
must not replace them globally. On stable Rust, an unconstrained blanket
implementation and an overlapping specialized implementation are not an
automatic dispatch mechanism. Use nonoverlapping adapter types, explicit
ordered/value profiles, or closed private dispatch for known supported pairs.
Keep extension points honest about which path they select. [S9]

The first generic implementation can use the fallback consistently. Admit each
faster pair/terminal only with semantic differential checks and measured benefit.
No public kernel trait or N-by-N ecosystem dispatch matrix is required at the
first release.

## 8. Performance model

Let `scan(A)` be traversal work, `nA` its cardinality, and `probe(B)` the cost of
one membership query of the relevant key. The generic intersection baseline
costs approximately `scan(A) + nA * probe(B)`; union adds both traversals and
right-to-left probes; XOR probes in both directions. Early termination can
reduce work.

These are workload models, not universal constant-time promises. Standard
HashSet traversal currently visits capacity, including empty buckets. Hashing a
long string and comparing long common prefixes can dominate the set machinery.
Using fewer members as the driver can be worse when that hash table is heavily
overallocated. [S1]

| Operand shape | Candidate execution | Cost that must be counted |
| --- | --- | --- |
| Two sorted contiguous sets | Existing merge or skew-sensitive seek | Comparisons, compared bytes, emitted references |
| Two hash sets | Scan/probe, with optional smaller-side drive | Bucket capacity, hashing bytes, collisions, representative lookup |
| Ordered plus hash | Select a source according to terminal and representative capability | Tree comparisons versus hash probes; output-order constraints |
| Insertion-ordered sets | Generic view or native ordered traversal | Stable order may constrain which side drives |
| Persistent sets | Probes or native subtree operations when semantics match | Node visits, sharing, clone bounds, and source representative rules |
| Integer/bitmap sets | Encoded or word/container kernels | Relevant spans/containers, conversion scratch, result policies |
| FST sets | Sorted lending streams | Byte comparisons, per-stream state, and owned key copies if requested |

Never reuse a hash calculated by one operand in a second table solely because
the hasher types match. Builder state and seeds can differ. A cross-table hash
reuse path needs an explicit compatibility contract.

### N-ary operations need their own algorithms

A left-associated lazy union of k disjoint sources with n members each can
perform `n * k * (k - 1) / 2` primitive membership probes: values from each
later source are checked against earlier sources. Avoid claiming linear union
because no intermediate set was allocated.

For a flat intersection, pick a useful candidate source and order remaining
predicates by rejection benefit and actual probe cost, while retaining the
semantic representative source. Sorted k-way unions can use merge cursors and
O(k) frontier state. Unordered k-way union may use a temporary exact index of
references or the materialization target itself as the membership index; that
adds memory proportional to distinct values. The memory/work tradeoff must be
explicit. No zero-allocation linear-time promise is made for arbitrary
unordered union.

For repeated queries, let callers explicitly prepare workspace or materialize.
Do not automatically cache after two calls. A prefix read, failed membership
query, exact count, and complete traversal have different costs. The supplied
research's threshold is not a measured general policy.

Benchmark complete terminals: construction plus one use, first match, short
prefix, full traversal, exact count, materialization, repeated reuse, and final
release. Use strings with differing lengths/common prefixes, custom records,
collision-heavy hashes, capacity-heavy tables, balanced/skewed operands, empty
results, and several expression shapes. Preserve slow cases and native controls.
No new runtime performance measurements are claimed in this design iteration.

## 9. Materialization and ownership

There is no universal zero-copy materialization of a borrowed expression into
an independently owned set of arbitrary T.

| Requested output | Ownership consequence |
| --- | --- |
| `Iterator<Item=&T>` | Borrows source storage; no T clone |
| `FrozenOrderedSet<&T>` or a hash set of references | Allocates result indexes/storage, retains source lifetimes |
| `FrozenOrderedSet<T>` from `.cloned()` | Explicitly clones surviving representatives, then builds ordered storage |
| `FrozenHashSet<T, S>` with an explicit hasher | Explicit cloning/moving plus hashing using that chosen builder |
| Consuming algebra | May move T when the backend exposes owned traversal and the algorithm permits it |
| Persistent output | Shares storage according to a separate persistent-family contract |

Keep `collect` as the initial destination interface. Add destination-aware bulk
builders only when measured costs justify them. Different operand hashers do
not determine one inevitable output hasher; accept a chosen builder or clearly
use the target's documented default. Querying does not require either input
hasher to be cloned or default-constructed.

A consuming API is a separate addition, for example `into_union` and
`into_intersection` with a specified output policy. Its design must cover
duplicate disposal, drop counts, partial iteration, and panic behavior. XOR may
need destructive matching or an auxiliary membership index; moving all left
values first can destroy information needed to identify right-only values.
Do not promise every consuming operation is buffer-free.

Shared persistent nodes cannot always yield owned T without cloning while old
versions remain alive. A backend-imposed `Clone` bound belongs to that backend
or operation, not to every borrowed query. Likewise, collecting references
does not make a result independently own the referenced records.

Output limits and execution-workspace limits are different. A result-size bound
does not cap a temporary index, input conversion, recursive state, or old storage
kept alive by borrowed references. A future fallible builder must state which
policy failures it recovers from; a `try_` name does not promise recovery from
every allocator termination or arbitrary user callback panic.

## 10. Counts, bounds, and complements

Retain current count types on existing collections and methods. For the new
general expression API, use an exact checked terminal provisionally named
`try_count() -> Result<u128, CountOverflow>`, with a separately checked usize
conversion if needed. Standard iterator `.count()` still has its usual usize
return type; it is not the wide terminal.

Arbitrary T has no declared finite universe. Do not copy integer lower-bound
formulas that subtract `2^32` or `2^64`. Use count bounds with a u128 lower bound
and an optional upper bound; `None` means no representable known upper bound,
not an infinite result or an overflowed exact answer.

For exact input counts a and b with unknown overlap:

| Operation | Safe lower | Safe upper |
| --- | --- | --- |
| Union | `max(a,b)` | checked `a+b`, otherwise unknown |
| Intersection | `0` | `min(a,b)` |
| Difference | `a.saturating_sub(b)` | `a` |
| XOR | `abs_diff(a,b)` | checked `a+b`, otherwise unknown |

For interval inputs `[la, ua]` and `[lb, ub]`, upper endpoints can be unknown.
Define `gap(l, u)` as `l.saturating_sub(u)` when u is known, and zero otherwise:

| Operation | Lower | Upper |
| --- | --- | --- |
| Union | `max(la, lb)` | Checked sum of known uppers; unknown if either is unknown or addition overflows |
| Intersection | `0` | Minimum of the known uppers; unknown only if neither is known |
| Difference | `gap(la, ub)` | `ua` |
| XOR | `max(gap(la, ub), gap(lb, ua))` | Checked sum of known uppers; otherwise unknown |

An unknown/overflowed upper bound does not establish overflow of the actual
result. Each bound must satisfy `lower <= actual`, and `actual <= upper` when
an upper is known. A known finite universe is an optional domain capability,
not a fact inferred from element size. Subtracting emitted elements from a
bound must track remaining output, not rejected or duplicated candidates.
When projecting u128 bounds to an iterator's usize `size_hint`, cap an oversized
lower at `usize::MAX` and report an oversized upper as `None`; never truncate.
An exhausted iterator reports `(0, Some(0))`.

`view(&set)` should not perform an expensive cardinality scan. Exact cheap
metadata may be used where available; FixedBitSet's addressable extent is only
an upper bound on membership, and arbitrary `Cardinality` providers may have
nonconstant counting cost. Typed `bounds()` may traverse the expression; an
O(1) bound read requires an explicit construction/preparation-time cache and
its associated cost. Avoid hidden caches and gratuitous size-hint evaluation.

There is no unrestricted unary complement. Express it as a difference from an
explicit universe with the same logical identity. Do not generate all numeric
codes for a restricted enum or infer every possible String. A zero-input
intersection requires an explicit universe or is rejected; empty union/XOR can
be empty.

## 11. How specialized sets participate

### Encoded integers and bitmaps

Keep `FrozenIntSet<K>` and the proposed `IntSetExpr<K>` separate from ordinary
reference-valued views. Preserve canonical codecs, logical K separation,
encoded evaluation, wide counts, and current Dense/Roaring/Sorted result
selection. The [integer proposal](integer-views.md) already specifies a useful
candidate-cover and terminal-dispatch approach for this domain.

A source that reconstructs values cannot implement ordinary borrowed
`Elements`. Provide a value profile or explicit adapters when combining a
Roaring u32 set with an ordinary u32 set. An ordinary iterator can be copied
into a value path when its item supports that operation; it is not silently
converted into a bitmap. Do not narrow a usize or u64 domain to u32 without a
checked conversion.

Yielding by value alone does not make representative provenance irrelevant:
an arbitrary Copy struct can still contain ignored metadata. Only the encoded
profile's stronger canonical-value laws justify its more aggressive rewrite
rules.

### FST byte strings

Use a lending stream or callback interface, not `Iterator<Item=&'set T>`.
FST already exposes multi-input stream algebra with lexicographic output.
Its stream item is tied to advancing the stream. A retained key needs an
explicit copy or owned conversion. [S3, S10]

For same-profile algebra, adapt the native stream machinery before designing
another byte-string engine. For a hash set of String filtered against an FST,
the stored String can be the output representative and its bytes a membership
query. Producing a union with FST-only strings requires a byte-value/lending
profile or owned conversion. UTF-8 validation is explicit when converting
arbitrary byte keys to String. A batch cannot retain references into one reused
buffer; it must copy, own, or callback-consume each key before advancing.

### Arbitrary keys projected to bitmaps

Interning can be useful for repeatedly queried string or record universes, but
it is an optional indexed profile. Require a collision-free mapping for the
chosen identity, stable IDs, and compatible dictionary/domain versions. Two
independently numbered dictionaries cannot be combined by bitwise OR merely
because both IDs are u32.

Hashing arbitrary values into u64 is not an exact encoding. A canonical
dictionary also selects its own representatives; it cannot silently preserve
every source record's ignored metadata. Name the canonicalization or retain a
separate provenance mapping. This indexed profile is not a prerequisite for
ordinary generic set algebra.

### Live concurrent inputs

Borrow ordinary mutable sets immutably for the query; Rust then prevents
structural mutation through that owner while the borrow is active. Interior
concurrent sets require a different contract. Independent membership probes
can observe different moments and cannot promise one coherent set result.

Use a retained snapshot handle or an explicitly coordinated extraction when a
consistent result is required. Cloning a live collection must itself have a
documented snapshot mechanism before being described as atomic. Leave the
current `SnapshotValue` boundary sealed; expression types remain ineligible
even if their borrows are `'static`. Materialize before publication.

## 12. Runtime expressions are a separate representation

Typed views are well suited to queries whose structure is known in Rust source.
They cannot represent an arbitrary runtime-length plan with one ever-changing
nested type. If runtime composition is needed, add a separate owned arena of
node indices borrowing its leaf data.

A proposed `PreparedSetExpr<'a, T, Q>` must fix its query domain and specify how
T is projected to Q for candidate checks. It cannot silently retain an arbitrary
generic `contains<Q2>` method through one erased vtable. Borrowed lookup with a
fixed Q, such as str for String, remains possible with an explicit adapter.

Leaf adapters can erase membership and create boxed cursors or use closed
backend cursor enums. Each choice has measurable dispatch, allocation, and
extensibility costs. Do not use `Any` downcasts that impose `'static` on borrowed
keys, or transmute GAT iterators to manufacture an object-safe interface.

Keep original provenance nodes available when the physical membership plan
flattens/reorders compatible operators. Traverse and drop runtime metadata
iteratively, cap untrusted plan size before construction, and reuse per-query
scratch instead of allocating a frame buffer for every candidate. Record
workspace separately from output storage. Plans and iterators follow actual
auto traits; no manual Send/Sync claims are required.

The integer arena can share operation definitions, validation ideas, and
semantic tests with this design. It need not share one trait object or one
cursor item type with borrowed objects and FST keys. Avoid prematurely exposing
a generic optimizer framework as the public collection API.

## 13. Persistence for generic sets

The research correctly separates lazy queries from sharing materialized data.
The [persistent integer proposal](persistent-integer-storage.md) targets numeric
chunks. Arbitrary records generally need ordered tree nodes or hash-trie nodes,
with their own representative and element-ownership contracts. imbl's hash and
ordered structures provide existing controls for those experiments. [S4, S5]

`Arc<Set<T>>` can share the whole set and make handle cloning cheap.
`Arc::make_mut` may clone its inner value when another version retains it;
wrapping an ordinary flat table or tree in Arc does not create path copying.
Pointer equality establishes shared allocation identity, not logical equality.
[S6]

Generic persistent storage has an additional choice:

| Node ownership | Benefit | Cost/constraint |
| --- | --- | --- |
| Shared nodes containing owned T | Compact elements inside nodes | Copying a changed shared node can require `T: Clone` |
| Shared nodes containing `Arc<T>` | Share individual non-Clone values | Per-value allocation/indirection/reference-count costs |
| Immutable backing storage plus shared indexes | Stable element handles | Retention, reclamation, and extraction become separate concerns |

Do not promise a universal non-Clone persistent family without choosing and
measuring one ownership model. Keep any new persistent ordered/hash families
separate from ordinary frozen families. Test updates with retained versions,
equal members with different metadata, conversions, failed publication retries,
and final-owner release. Neither root cloning nor a high pointer-sharing rate
alone demonstrates useful end-to-end performance.

## 14. Delivery sequence and acceptance gates

### Phase A: establish the generic contract

Add the explicit set-source laws and typed borrowed binary/composite views.
Cover ordinary mutable/frozen families and supported native/wrapped stored-value
sets. Preserve current specialized methods. Include noninteger examples and
explicit cloned/reference materialization. This is the priority for the user's
request; it does not depend on a new bitmap engine.

Compile-check the public API with Eq+Hash-only types, Ord-only types, non-Clone
members, local borrowed strings, custom hashers, wrapper/native mixtures, and
imbl hash sources without `SetGet`. Verify method bounds separately from the
bounds on constructing the expression. Required modules can live under
`src/set/`, with public reexports added deliberately; this design adds no module
to the implementation yet.

### Phase B: admit useful physical kernels

Add smaller-side count/existence paths, representative-preserving right-driven
intersection where legal, and an explicit sorted profile if demanded. Compare
each optimized terminal with an independent baseline. Evaluate n-ary union
memory/work policies before claiming fused general execution.

Do not add unstable specialization, new unsafe code, a global feature-selected
backend, or a mandatory allocation dependency to the core binary layer.

### Phase C: finish representation-specific coverage

Provide value and lending-stream algebra through their appropriate adapters,
with checked cross-domain conversion and clear ownership. Runtime-generated
queries can introduce a fixed-domain arena after real workloads justify it.
Completion of ordinary borrowed views alone must not be advertised as a
shipped universal FST/bitmap expression engine.

### Phase D: evaluate persistence independently

Use retained-version benchmarks to decide whether additional generic or integer
persistent families earn their maintenance cost. Do not delay general borrowed
set support behind a custom Roaring implementation.

### Required verification before implementation release

1. Differential membership against independent standard sets for all four
   operations, mixed expressions, and representative-labelled objects.
2. Dedicated provenance tests for XOR grouping, nonlocal XOR cancellation,
   distribution, operand swapping, and smaller-side candidate enumeration.
3. The complete supported mutable/frozen/native/wrapper matrix, including
   hashbrown, indexmap, PHF, imbl, and explicit value/lending boundaries.
4. Non-Clone/non-Default elements, nonstatic borrows, custom non-Default hashers,
   collision-heavy keys, and distinct lawful Eq/Ord/Hash trait availability.
5. Partial traversal, every remaining size hint, repeated exhaustion,
   independent iterators, empty/self/disjoint cases, and dropping early. Include
   an underlying non-fused iterator whose complete membership precedes its
   first `None`, and verify that later resumptions cannot escape `Fuse`.
6. Query-domain and lifetime compile-fail cases, including unsafe-to-express
   FST reference retention and publishing expressions as snapshots.
7. Core-only, alloc-only, isolated feature, all-feature, and applicable 32-bit
   checks; ownership diagnostics where new ownership code is introduced.
8. Work/allocation counters and complete-terminal benchmarks with native
   controls; no timing assertions in correctness tests.
9. Exact counts versus independent enumeration, emptiness versus exact count,
   sound nested bounds, and checked usize conversion. Synthetic count-only
   models must cover actual overflow and unknown uppers without enormous
   allocations; a loose upper must not cause rejection of a small actual result.

The current repository passed its baseline checks, and the independent finite
model supplies evidence for these semantic rules. Neither is verification of
the proposed Rust views, runtime planner, or persistent profiles.

## Sources and evidence

External pages below were inspected during this review. Versioned local source
and Cargo.lock were also inspected for the adapter-specific findings. Moving
documentation resolved to the versions noted; no upstream benchmarks or recent
bug reports were rerun in this pass.

- **S1:** [Rust HashSet documentation](https://doc.rust-lang.org/std/collections/struct.HashSet.html), Rust 1.99.1: equality/hash laws, intersection representatives, and capacity-sensitive iteration.
- **S2:** [Rust Reference: dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility): GAT and generic-method constraints on trait objects.
- **S3:** [fst Streamer](https://docs.rs/fst/latest/fst/trait.Streamer.html), 0.4.7: lending item lifetimes.
- **S4:** [imbl GenericHashSet](https://docs.rs/imbl/latest/imbl/hashset/struct.GenericHashSet.html), 7.0.2; corroborated against the cached 7.0.2 source and Frost's persistent adapter.
- **S5:** [imbl GenericOrdSet](https://docs.rs/imbl/latest/imbl/ordset/struct.GenericOrdSet.html), 7.0.2: ordered persistent structure and stored-member lookup.
- **S6:** [Rust Arc documentation](https://doc.rust-lang.org/std/sync/struct.Arc.html), Rust 1.99.1: sharing, clone-on-write, pointer identity, and auto-trait conditions.
- **S7:** [Existing operation contract](../set-operations.md), [capabilities](../../crates/frost-collections-core/src/read.rs), and [generic review ledger](../reviews/generic-set-algebra-review.md): repository facts and executed checks.
- **S8:** [IndexSet documentation](https://docs.rs/indexmap/2.14.0/indexmap/set/struct.IndexSet.html), 2.14.0, and the [Frost indexmap adapter](../../crates/frost-collections-core/src/backends/indexmap.rs): order and query contracts. The [hashbrown adapter](../../crates/frost-collections-core/src/backends/hashbrown.rs) records its separate Equivalent relation.
- **S9:** [Rust Reference: implementation coherence](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence): overlapping implementations are rejected.
- **S10:** [fst set OpBuilder](https://docs.rs/fst/latest/fst/set/struct.OpBuilder.html), 0.4.7: multi-input lexicographic stream algebra.
- **S11:** [Finite representative model](../../verification/set-algebra/README.md): reproduced semantic counterexamples, scope, and raw results.

The supplied report and previous audits retain their historical findings. This
design adopts their separation of queries, materialization, and sharing, while
correcting the assumption that canonical integer semantics can serve unchanged
as the contract for arbitrary stored objects.

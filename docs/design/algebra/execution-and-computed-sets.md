---
id: FROST-ALG-COMPUTED
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Computed sets, durable sources and incomplete knowledge

A computed set denotes an expression S(θ), not necessarily a resident container.
Frost's current local membership traits remain infallible; a durable/contextual
adapter needs a separate result protocol and must not convert I/O failure into
false. This is a proposed execution contract, not an implemented Zanzibar engine.

## 1. Logical source and context

A source binds semantic identity, dictionary/encoding, order and a snapshot
provider. θ additionally binds relevant policy/model revision, request facts,
caveat inputs, tenant and completeness requirements. A static Rust tag does not
establish runtime dictionary or tenant identity.

An engine selects an admissible common view before a query needs coherent
cross-source answers. A minimum freshness token and an exact retained revision
are different requirements. Permission revocation, cache invalidation, and
check-to-use enforcement belong to the surrounding engine. A sorted frozen
index by itself proves none of them.

[CTX-KEY] A memoized, cached, prepared or resumed result may answer a request
only if the request's context agrees with the producing context on every
component the result depends on: semantic binding, dictionary, snapshot,
policy or model revision, request facts and caveat inputs, completeness
requirement, and terminal. Argument: by [FM-REFINE](formal-model.md) the
result is a denotation at θ together with its completion status, so a key that
omits a component identifies two contexts whose results can differ
(`CX-CTX-CACHE-KEY`). Reuse never upgrades a status: an incomplete or
conditional result stays incomplete or conditional.

A component may be left out of a key only with a proof that the result does
not depend on it. That proof is a rewrite and is recorded under
[rewrite contracts](rewrite-contracts.md). One production authorization engine
reported the unproved case: a dispatch cache key omitted per-request hints,
and a check returned full permission where the answer was conditional, for a
permission that combined caveated and uncaveated paths by intersection or
exclusion (`SRC-PRIOR-SPICEDB-CVE-2026-55866`). The continuation tokens of
section 3 are one instance of this rule.

## 2. Lower and upper bounds on knowledge

For a fixed compatible universe/context, describe incomplete knowledge as
[L,U] with L⊆S⊆U. L contains established members; U bounds possible members.
U may be implicit or infinite. This is an epistemic specification, not a
requirement to materialize two sets.

[KNOW-BOUNDS] Independent sound transfer rules are:

| Operation | Definite members | Possible members |
| --- | --- | --- |
| A∪B | L_A∪L_B | U_A∪U_B |
| A∩B | L_A∩L_B | U_A∩U_B |
| A\B | L_A\U_B | U_A\L_B |
| A XOR B | (L_A\U_B)∪(L_B\U_A) | (U_A\L_B)∪(U_B\L_A) |

Proof: substitute L_X⊆X⊆U_X; union/intersection are monotone in each argument,
and difference is monotone in its left argument and antitone in its right.
XOR follows by its two directional differences. These rules are sound but may
be imprecise when sources are correlated. For the exact same pinned source A,
A\A is known empty even when the independent interval rule is looser. That
simplification still requires a contract that permits suppressing evaluation
errors/effects and validates the source/context identity.

Read per key, [L,U] gives three states: certain member, possible only, and
certain non-member. The four rules are the strong three-valued connectives on
those states: or, and, and-not, and exclusive or. In the incomplete-database
literature, answers that hold however the unknowns are resolved are certain
answers. L plays that role, and the requirement that an evaluation return
only certain answers is called "no false positives"
(`SRC-PRIOR-LIBKIN-2016`). The matching term for U, possible answers, comes
from a source that was not read. These are aliases; the names here stay L
and U.

A candidate k is definitely present if k∈L; definitely absent if k∉U.
Existence is settled true when L is nonempty and false when U is empty.
Exact count needs a certified finite result; |L| and |U| are bounds, not estimates
mistaken for exact counts. A failed source read is diagnostic state, not merely
a convenient change to an empty lower bound.

Difference is security-sensitive: a missing deny posting is not proof that the
candidate is not denied. A completed relevant negative probe or a sound upper
bound excluding the candidate is required before emitting it as established.

## 3. Streaming and pagination

An iterator/cursor protocol distinguishes item, successful exhaustion,
incomplete/budget stop, cancellation and failure. Absence of another immediately
available item is not exhaustion. An async stream's Pending state is not empty.
Every emitted prefix obeys the [formal prefix rule](formal-model.md).

Sorted union/difference/intersection can stream using frontier cursors without
materializing all inputs. Hash probing may avoid enumerating a large operand
but requires sound membership and original representative recovery. Bounded
listing can return fewer than the requested number while incomplete; an exact
negative result is a different outcome.

Continuation state binds expression identity, semantic domain, dictionary,
chosen snapshot, policy/context, ordering and a cursor/last-key boundary. Tokens
need expiry and authentication at an untrusted engine boundary. A token's hash
of context is not by itself proof that the source revision still exists.
Seek-after pagination requires stable key order and duplicate rules; do not
restart at current main/current policy and call it the same query continuation.

## 4. Base plus ordered delta

For immutable base S_b and complete ordered events through r, compute
S_r=fold(apply_event,S_b,events_(b,r]). Each event is an insertion/deletion with
unambiguous order; later events for the same key win in the simple set profile.
[DELTA-REPLAY] Compaction at c preserves answers at r when base_c is exact and
the remaining suffix is complete and ordered, with compatible domains/codecs.

The expression (base ∪ all inserts) \ all deletes is not general replay:
delete(k); insert(k) must retain k. Missing events, duplicate non-idempotent
bag deltas, out-of-order application, dictionary reuse and stale policy indexes
require explicit handling. Complete replay and complete index catch-up are
contractual predicates, not inferred from a watermark number alone.

## 5. Fixed points and negative recursion

For positive rules F over a product of finite Boolean relation lattices,
monotonicity and iteration from bottom give an ascending sequence and a least
fixed point. If the universe has N possible facts, at most N strict fact additions
occur; this is not a bound on total CPU/I/O work per iteration. Semi-naive
execution propagates newly derived facts when its delta equations are valid.

[CLOSURE-STABLE] For weights in a semiring write u^(p) = 1+u+…+u^p. An
element u is p-stable when u^(p) = u^(p+1), and the semiring is stable when
every element is p-stable for some p. Every recursive program over the weights
converges iff the semiring is stable (`SRC-PRIOR-DATALOGO`, Theorem 1.2; cited
here, not re-proved). The source states this for Datalog programs over a
partially ordered pre-semiring, as a condition on its core semiring. The
Boolean semiring is 0-stable. N is not stable:
1^(p) = p+1 for every p, so one unit-weight cycle already diverges
(`CX-CLOSURE-N-DIVERGES`).

A recursive weighted composition, such as a closure of a BagMultiMap, is
therefore admitted only in one of three cases:

1. The terminal observes support only, and the plan computes over Booleans.
2. The recursion's dependency graph is acyclic, so the iteration over N is
   finite.
3. Counts are checked, and `Overflow` or a budget stop is a declared outcome.
   Neither is a negative answer.

A count capped at M, min(M,·), is a different and stable algebra. Capping
commutes with sum and product, so each round of a capped iteration is the cap
of the same round over N. Its result means "at least M" at a capped entry. It
is a separately named operation, as saturating addition is in
[bags](bags.md).

Bags over N, unbounded key generation, and unconstrained symbolic terms do not
inherit the finite-height argument. Even one key can accumulate unbounded
multiplicity around a cycle. Sequences and left-biased map merge are not generic
join-semilattice instances. Recursion through negation requires an explicit
stratification or other semantics; a cycle detector alone does not define it.

## 6. Physical cost and memory obligations

For n,m sorted input items, binary merge uses O(n+m) comparisons with a constant
number of frontier items, not necessarily constant bytes: count key buffers,
page pins, decoders, ownership and outputs. A heap merge over k sources with N
items takes O(k+N log(k+1)) heap comparison work plus comparator/key copy and
stream-advance/I/O costs. Comparator cost can depend on shared-prefix length.

Scan/probe charges table capacity traversal, key hashing/comparison, misses,
decoding and I/O. Bitmaps charge touched words/containers, not just logical
cardinality. Recursion retains visited/worklist state; joins can have quadratic
output; hash dedup can require output-sized memory; spill trades memory for I/O.
Memory mapping does not prove bounded resident memory. A budget covers all
relevant operator state and distinguishes failure from a complete negative.

The abstraction permits no-full-materialization execution; it does not promise
that every plan is bounded-memory, every count is cheap, or any query terminates.

## 7. Engine boundary

Keep authorization facts correlated as complete tuples, not independent sets of
principals/actions/resources that invent a Cartesian product of grants. Typed
collection domains help retain these distinctions, but credential verification,
trust roots, caveats, deny/permit/error rules and enforcement remain external.
Symbolic engines also need their own language semantics, satisfiability and
sound abstract-state joins. The formal collection model is a substrate, not
an authorization policy or abstract interpreter.

The [computed-set input](../../research/reports/computed-sets-summary.md) is a
summary; missing linked full reports remain missing. This chapter is new synthesis.

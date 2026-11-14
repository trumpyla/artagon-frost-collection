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

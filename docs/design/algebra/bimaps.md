---
id: FROST-ALG-BIMAPS
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# BiMap algebra: finite partial bijections

BiMap(K,L) consists of finite relations R⊆K×L satisfying:

    (k,l1),(k,l2)∈R ⇒ l1=l2
    (k1,l),(k2,l)∈R ⇒ k1=k2.

Both sides are key domains. Values valid individually do not establish these
collection-level constraints. The current repository selects MutableBiMap or a
builder for insertion and FrozenBiMap for structurally immutable results; both
remain proposals.

## 1. Closed operations

| Operation | Output / precondition |
| --- | --- |
| inverse(R) | BiMap(L,K), endpoint pairs swapped |
| intersection_pairs(R,S) | BiMap(K,L), equal logical pairs only |
| difference_pairs(R,S) | BiMap(K,L), remove equal logical pairs only |
| restrict_left(R,A) | Retain pairs whose left endpoint is in A |
| restrict_right(R,B) | Retain pairs whose right endpoint is in B |
| compose(R,S) | S∘R in BiMap(K,M), compatible middle key domains |

[BIMAP-CLOSED] Pair subsets preserve both uniqueness implications. Composition
preserves them too: uniqueness of a right endpoint for k follows successively
from functionality of R and S; reverse uniqueness follows from their converse
functionality. Inverse is involutive.

[BIMAP-IDENTITY] R⁻¹∘R is identity on dom(R), and R∘R⁻¹ is identity on ran(R).
These are *partial* identities, not identities over every possible key. A finite
universe U supports a global identity relation; unrestricted finite partial
bijections on an infinite key domain do not contain the infinite identity.
Composition is associative; inverse(S∘R)=inverse(R)∘inverse(S).

“Difference by left key” is not pair difference. With R={(a,1)} and S={(a,2)},
pair difference keeps (a,1), while deleting all S left keys removes it. Keep
these names distinct even if their storage algorithms share primitives.

## 2. Checked union and conflicts

Define compatible(R,S) iff the pair union is functional both ways. Then

    try_union(R,S) = R∪S when compatible, otherwise Conflict.

Reject left conflict, right conflict, or simultaneous two-sided conflicts.
Compatible union retains the left pair representative on an equal logical pair.
A result with arbitrary pairs is a relation, not a secretly relaxed BiMap.

[BIMAP-PARTIAL] Checked union is associative as a *partial operation's domain
and successful logical result*: either grouping succeeds iff R∪S∪T is a partial
bijection. Proof: adding pairs cannot repair an existing uniqueness conflict;
every intermediate union is a subset of the final union. Its chosen first
conflict diagnostic or callback order need not be associative.

Checked XOR is different. Let R={(a,1)}, S=T={(a,2)}. (R XOR S) is invalid, so
left-associated checked evaluation fails; S XOR T is empty, so right-associated
evaluation succeeds with R. Therefore a final relation-level XOR cancellation
does not preserve the definedness of binary checked BiMap evaluation.

Replacement/upsert that evicts conflicting pairs is not union: it removes data
and can retain different endpoint objects. The external bimap crate's native
insertion policy must not silently define Frost's proposed algebra policy.

## 3. Representatives, ownership and construction

Inverse swaps the actual selected endpoint observations. A simple composition
selects the outer left representative from R and outer right representative
from S at the unique compatible middle class; it need not preserve a middle
object or allocate an output pair in advance. Borrowed pair fields can have
different source lifetimes.

A borrowed checked union can validate and build a new owner while leaving input
structures untouched on an ordinary error. A streaming union is not a globally
validated BiMap before all constraints are checked: a later right-endpoint
conflict can invalidate what looked valid in a prefix. Either stage/validate
first or expose a provisional relation stream with explicit completion status.
Do not publish a frozen BiMap from an unvalidated prefix.

Consuming insertion/construction has separately specified error recovery. If
staging permits input recovery, state that; otherwise state which accepted and
rejected values were moved or dropped. A returned conflict guarantee does not
cover callback side effects, panics or allocation aborts.

## 4. Conversion and tests

Changing either endpoint through a many-to-one conversion can introduce a
conflict. Key-only injectivity on occupied endpoint classes suffices for logical
uniqueness, but the conversion must also preserve the requested representatives
and runtime domain compatibility.

Tests: enumerate finite partial bijections; validate both projections against
a relation oracle; check inverse, local identities and composition; distinguish
pair/key restrictions; verify partial union definedness; reproduce checked-XOR
failure; cover both conflicting endpoints and duplicate equivalent pairs;
exercise non-Clone fields, returned errors, partial consumption and drops in the
future Rust implementation.

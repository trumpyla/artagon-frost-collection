---
id: FROST-ALG-BAGS
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Bag and count algebra

Bag(K) is the finite-support function b:K→N, with missing entries denoting zero.
Canonical proposed storage omits zero-count entries and retains one selected
representative for each positive key. Its support and total multiplicity differ:

    support(b)={k | b(k)>0}; distinct_count=|support(b)|; total=Σ_k b(k).

## Count operations

| Proposed name | Pointwise count |
| --- | --- |
| sum(a,b) | a+b |
| max_union(a,b) | max(a,b) |
| intersection(a,b) | min(a,b) |
| difference(a,b) | max(a-b,0), also called monus |
| absolute_difference(a,b) | abs(a-b) |

[BAG-LAWS] Sum is a commutative monoid and not idempotent. Max and min are a
distributive lattice under pointwise order, with bottom zero but, for nonempty K, no finite-count
top in unrestricted Bag(K). Difference is not an additive inverse. Absolute
difference is commutative, zero-self and nonassociative:
abs(abs(1-1)-2)=2, but abs(1-abs(1-2))=0.

The historical name symmetric_difference denotes absolute_difference here; it
must not import Boolean XOR's associativity. Prefer the explicit longer name in
new bag APIs. Signed incremental weights belong to a different carrier Z^(K),
not ordinary bags. A retracted materialized count must be checked for negativity.

The established name for Z^(K) is the Z-set: finite-support integer weights,
which form an abelian group. Bags are its members with no negative weight, and
a change between two bags is a Z-set (`SRC-PRIOR-DBSP`, section 4.1). Python's
`Counter` shows what one type for both carriers costs: its operators drop
counts of zero or below, while `subtract()` keeps them (`SRC-PRIOR-PY-COUNTER`).
Frost keeps the two carriers apart.

## Arithmetic and representation

A bounded count C_M={0,...,M} with checked addition is a partial refinement of N.
[COUNT-PARTIAL] Both parenthesizations of a+b+c succeed iff the nonnegative total
fits M; successful values agree. The first reported overflowing key, partial
prefix, or callback trace can still depend on evaluation order. Saturating
addition changes the algebra and must be a separately named operation.

Max/min/monus/absolute difference do not exceed max(a,b) per key. A total across
distinct keys can still exceed an aggregate counter even if every multiplicity
fits. For min and monus the result total is at most the left operand's total, so
this is new only for max and absolute difference, whose totals are bounded by
the sum of both input totals. Compressed bags must not expand occurrences merely to count
or combine counts. A lazy sum can remain unevaluated, but its terminal must
report overflow instead of claiming a representable materialized result.

## Difference is monus; its identities are admitted one at a time

Bag difference is the monus of the naturally ordered semiring N: a∸b is the
least z with a ≤ b+z (`SRC-PRIOR-GEERTS-POGGI`). "Monus" is an alias;
`difference` stays the proposed spelling.

[BAG-MONUS] For all natural counts a, b, c, with ∨ for max and ∧ for min:

| Identity | Note |
| --- | --- |
| (a+b)∸b = a | Sum then difference cancels |
| a∸(b+c) = (a∸b)∸c | Iterated difference |
| (a∸b)+b = a∨b and (a∸b)+(a∧b) = a | Difference recombines with max and min |
| a∸(a∸b) = a∧b | Double difference is min |
| (a∨b)∸c = (a∸c)∨(b∸c) and (a∧b)∸c = (a∸c)∧(b∸c) | Left argument: monotone |
| a∸(b∨c) = (a∸b)∧(a∸c) and a∸(b∧c) = (a∸b)∨(a∸c) | Right argument: antitone |
| (b∸a)·c = b·c∸a·c | Product distributes over difference in N |

Each follows by cases on the order of the arguments, since a∸b is a-b when
a ≥ b and 0 otherwise. Pointwise, they hold for bags.

Four identities of set difference **fail** when union is read as bag sum and
intersection as min:

| Set identity | Bag reading | Counterexample |
| --- | --- | --- |
| (A∪B)\B = A\B | (a+b)∸b against a∸b | a=b=1: 1 against 0 (`CX-BAG-MONUS-SUM-CANCEL`) |
| (A∪B)\C = (A\C)∪(B\C) | (a+b)∸c against (a∸c)+(b∸c) | a=b=c=1: 1 against 0 (`CX-BAG-MONUS-SUM-DIST`) |
| A∩(B\C) = (A∩B)\C | a∧(b∸c) against (a∧b)∸c | a=1, b=2, c=1: 1 against 0 (`CX-BAG-MONUS-MIN`) |
| A\(B\C) = (A\B)∪(A∩C) | a∸(b∸c) against (a∸b)∨(a∧c) | a=1, b=2, c=1: 0 against 1 (`CX-BAG-MONUS-NESTED`) |

The first two hold again when union is read as max_union. A set identity is
therefore not evidence for its bag reading; the reading must name sum or max.

No finite list of identities is complete. The identities of
(N,+,·,0,1,∸) are not finitely axiomatizable
(`SRC-PRIOR-SUCIU-DIFFERENCES`, Theorem 21), so a planner cannot derive bag
rewrites from a base set of axioms. It admits a rewrite only from a recorded
list; see [rewrite contracts](rewrite-contracts.md).

The product identity is a fact about N. In the chain 0 < 1 < 2 with max as
sum and min as product it fails (`CX-MSEMIRING-A5`), as the same source
reports for m-semirings in general (section 4.3). If counts are ever
generalized to another weight algebra, every law in this chapter must name
the algebra it holds in.

An identity with a sum on one side changes definedness under
[COUNT-PARTIAL]: (a+b)∸b can overflow where a cannot. Rewriting toward the
sum can add an `Overflow`, and rewriting away from it can remove one. The
admitted direction follows the terminal's error policy.

## Representatives

This refinement chooses a uniform **left-present policy**: when the result
count is positive and the left input has a positive count for that key, return
the left representative; otherwise use the right representative. This also
applies when max/absolute difference gets a larger numeric contribution from
the right. Counts and representative provenance are independent. An alternative
“surplus-side” policy would be a different contract, not an optimization.

The rule is associative for sum/max/min representative output under their
logical laws, but is not commutative when metadata differs. One compressed
representative plus count cannot recover the identities of equal occurrences
previously inserted. Expanded traversal repeats the retained representative;
a sequence is required to preserve original occurrence identities.

## Projections and conversions

[BAG-SUPPORT] support preserves sum and max as set union and min as intersection.
It does not preserve monus: a(x)=2 and b(x)=1 leave positive bag support, while
subtracting equal singleton supports yields empty. It also does not preserve
absolute difference as set XOR in general.

For a possibly key-collapsing f, bag pushforward adds counts for all keys mapped
to one destination. It preserves sum, but collisions invalidate max/min and
monus preservation. Aggregate overflow must be checked during coalescing.

## Current Frost and acceptance

There is no implemented Frost bag family in the inspected baseline. MutableBag,
FrozenBag and BagView remain candidate names from earlier design work, not a
completed naming/API decision. Do not make map values Eq/Ord solely because
maps of counts are convenient reference models.

Tests: all count equations, zero normalization, support laws and counterexamples,
checked-width boundaries, representative provenance, partial expansion, totals
versus support size, conversion coalescing, and recursive multiplicity growth.
A finite key universe does not bound multiplicities or guarantee termination of
bag fixed points. Keep that distinction explicit in dataflow integrations.

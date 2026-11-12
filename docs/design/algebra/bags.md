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

## Arithmetic and representation

A bounded count C_M={0,...,M} with checked addition is a partial refinement of N.
[COUNT-PARTIAL] Both parenthesizations of a+b+c succeed iff the nonnegative total
fits M; successful values agree. The first reported overflowing key, partial
prefix, or callback trace can still depend on evaluation order. Saturating
addition changes the algebra and must be a separately named operation.

Max/min/monus/absolute difference do not exceed max(a,b) per key. Their total
across distinct keys can still exceed the aggregate counter even if every
multiplicity fits. Compressed bags must not expand occurrences merely to count
or combine counts. A lazy sum can remain unevaluated, but its terminal must
report overflow instead of claiming a representable materialized result.

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

---
id: FROST-ALG-SETS
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Set algebra

Let K be a compatible key domain. The finite set carrier is P_f(K).
A stored-value source is enriched with a representative section ρ:S→V_D such
that q_D(ρ(k))=k, plus an independently stated traversal order. An encoded source
may have only decoded values, not ρ as a borrowed reference.

## Operations

| Operation | Denotation | Default representative when returned |
| --- | --- | --- |
| union(A,B) | {k | k∈A or k∈B} | Left when present, otherwise right |
| intersection(A,B) | {k | k∈A and k∈B} | Left |
| difference(A,B) | {k | k∈A and k∉B} | Left |
| symmetric_difference(A,B) | (A\B) ∪ (B\A) | Sole surviving child |
| is_subset(A,B) | A⊆B | No representative |
| is_disjoint(A,B) | A∩B=∅ | No representative |
| complement_in(A,U) | U\A, with A⊆U for Boolean-complement laws | Universe representative if borrowed |

The last operation is a proposal, not an implemented universal complement API.
For infinite K, finite sets form a generalized Boolean algebra under difference,
not a Boolean algebra with a global finite top. P(U) for finite U has top U and
relative complement. A membership predicate over an infinite domain does not
thereby support finite enumeration or exact count.

## Laws and their observation level

[SET-BOOLEAN] At membership level, union and intersection are associative,
commutative, idempotent, and distributive. Empty is union's identity and
intersection's annihilator; intersection's identity is U only when a universe
is fixed. Difference is neither commutative nor associative. XOR is associative,
commutative, has empty identity, and A XOR A=empty at membership level.

[SET-REP] Left-representative union and intersection are associative at the
representative-value level under stable compatible keys: the first participating
source supplies the representative (union) or the original left source supplies
it (intersection). They are not commutative when equal keys have different
metadata. These arguments do not fix unspecified iteration order or callback
traces.

Absorption and distribution hold for representatives only in the orientations
that keep the left operand first: A∪(A∩B)=A, A∩(B∪C)=(A∩B)∪(A∩C) and
A∪(B∩C)=(A∪B)∩(A∪C). Their mirror images can select a different object. With
one key held by A (label a) and B (label b) and absent from C, (B∩A)∪A returns
b where A returns a, and (B∩C)∪A returns a where (B∪A)∩(C∪A) returns b.

XOR cannot be reassociated for representative-producing terminals. If A,B,C
all contain the same key with labels a,b,c, then (A XOR B) XOR C returns c,
while A XOR (B XOR C) returns a. Nor may (A XOR B) XOR A simplify to B at that
observation. Retain the original expression for representative recovery even
when membership evaluation uses a canonical Boolean expression.

## Filtering and conversion

A pure key predicate p supports filter_p(A union B) = filter_p(A) union
filter_p(B), and analogous membership laws, when p is constant on key classes.
A predicate that reads a representative's label is not constant on those classes;
retain representative-sensitive semantics. Key projection through a noninjective
conversion preserves union but can invalidate intersection, difference and XOR
rewrites. See [domains](domains.md).

## Execution contracts

The unordered generic fallback enumerates left and probes right for intersection
and difference; union enumerates left then only right keys absent from left;
XOR enumerates the two exclusive parts. This yields one result per logical key
without a universal Eq+Hash+Ord+Clone requirement.

Scanning the smaller right operand for intersection still needs retrieving the
left representative. A count-only terminal does not. An unordered output may
use this optimization; an ascending output must preserve order. Stable insertion
order is not ascending key order. Native HashSet representative choices cannot
silently replace Frost's documented left rule.

SetContains<Q> is an implemented infallible membership capability. Durable I/O
or incomplete evidence requires a separate fallible/contextual protocol, not an
adapter that turns read failure into false. Elements and SetGet are optional
capabilities, not requirements for every computed/encoded set.

For finite a=|A|, b=|B|, i=|A∩B|:

    |A∪B| = a + (b-i)
    |A\B| = a-i
    |A XOR B| = (a-i)+(b-i).

Use a sufficiently wide checked domain and subtraction-before-addition where
appropriate; an overflowing intermediate a+b is not proof the result overflows.
Distinguish exact counts from valid bounds and unknown upper bounds.

## Current Frost and acceptance

Current family names remain MutableOrderedSet/FrozenOrderedSet and
MutableHashSet/FrozenHashSet. FrozenIntSet is a separate encoded owned profile;
its algebra is eager. SetView and IntSetExpr remain proposals in their existing
specifications. Existing ordered/hash binary methods keep their return types,
ordering and representative rules.

Before optimization: check all finite membership pairs/triples, metadata keys,
non-Clone/borrowed keys, hash collisions, domain mismatch including empty inputs,
partial/fused traversal, count overflow, and equivalence of specialized kernels.
The independent finite models test laws, not Rust lifetimes or source algorithms.

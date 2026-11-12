---
id: FROST-ALG-MULTIMAPS
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Multimap fibers and relation algebra

Main selects the proposed names **SetMultiMap**, **BagMultiMap**, and
**SeqMultiMap**. Retain them. No family is implemented yet, and this specification
does not silently replace them with older MutableSeqMultiMap/ListMultiMap names.
The logical carrier is MultiMap(K,F): finite-support maps into a fiber F with
empty element 0_F. The choice of F is part of the type's meaning.

## 1. Empty-fiber convention and counts

[MM-SUPPORT] For the canonical proposed semantic family, absent iff empty:
lookup(k)=absent exactly when the fiber is 0_F. Removing its last member removes
the group. This is a proposed normalization rule, not a claim about every native
multimap or Map<K,Vec<V>>. A native source that distinguishes absent from an
explicit empty fiber retains that distinction until an explicit conversion.

The same convention is an established library contract: a key is present only
with at least one value, and removing the last value removes the key
(`SRC-PRIOR-GUAVA-MULTIMAP`). That library also treats any two empty multimaps
as equal across fiber kinds. Frost does not adopt that rule: the fiber is part
of the type's meaning, and the selected families are distinct types.

Count independently: number of nonempty groups; number of distinct pairs;
number of occurrences (bag/sequence); and size of each fiber. A method named
len cannot silently switch between them. Per-fiber counts fitting a machine
word does not prove a global total fits.

## 2. Set-valued fibers

SetMultiMap(K,L) with normalized empty fibers is isomorphic, at membership level,
to Rel(K,L)=P_f(K×L): (k,l)∈R iff l∈M(k). Pointwise fiber union, intersection,
difference and XOR are the corresponding pair-set operations. Removing whole
keys is different from subtracting matching pairs; name the operation.

[REL-COMP] For R⊆K×L and S⊆L×M,

    S ∘ R = {(k,m) | exists l: (k,l)∈R and (l,m)∈S}.

Composition is associative and distributes over pair union. Multiple middle
witnesses collapse to one Boolean output pair. This does not preserve the set,
count, order or identity of derivation witnesses unless they are an explicit
additional payload. Reverse relation swaps endpoints; it is not generally a
BiMap. Projection to keys/values deduplicates in the set profile.

## 3. Bag-valued fibers

BagMultiMap(K,L) is a finite-support multiplicity function R:K×L→N. Pointwise
sum/max/min/monus have the bag definitions. Weighted composition is

    (S ∘ R)(k,m) = Σ_l R(k,l) × S(l,m).

[BAGREL-COMP] Associativity follows by rearranging the finite double sum and
using distributivity/associativity of natural arithmetic. Addition represents
alternative derivations and multiplication combines witnesses along a path.
Boolean support forgets derivation multiplicity; it must not be used when
explanations, accounting or duplicate-sensitive queries observe counts.

A BagMultiMap is a relation with natural-number weights, and Z^(K×L) is its
signed counterpart for changes (see [bags](bags.md)). Iterating weighted
composition to a fixed point is a separate operation with its own admission
rule, [CLOSURE-STABLE] in [computed sets](execution-and-computed-sets.md).

A bounded implementation checks every product, sum and aggregate. Error traces
and resource use do not inherit the natural-number equalities automatically.
Negative differential weights require a different carrier and maintenance
contract; they are not valid stored bag counts.

## 4. Sequence-valued fibers

SeqMultiMap(K,V) uses Seq(V) fibers. Pointwise concatenation is associative and
not commutative. Grouped sequences preserve per-key order, not the original
global interleaving of entries for different keys. Flattening therefore needs
a declared group order; grouping then flattening is not an inverse to an arbitrary
input sequence.

A sequence-valued composition must define witness order explicitly. For a
nested-loop variant, enumerate each left fiber occurrence in order, then the
matching right fiber in its order. Preserve duplicates. Do not reorder this
into hash-join encounter order or infer Boolean relation laws for its traces.
Restrictions and projections state whether they retain duplicates and order.

## 5. Group and pair representatives

For a binary grouped operation, choose the left group-key representative if the
left contains that group and the output group survives; otherwise choose the
right group key. Values inside the group follow the selected fiber's policy.

This may pair the left group's key object with a right-only value object. It is
a logically valid pair but not necessarily an original stored tuple object.
Return independently borrowed fields or an owned pair, never promise a reference
to a tuple that was not stored. A flat pair-source contract can instead retain
left/right original pair representatives, but that is a different observation
profile and conversion must disclose the change.

For relational composition with multiple witnesses, a representative-producing
terminal needs a declared witness-selection policy. A membership-only plan may
discard witnesses. It cannot later reconstruct an audit explanation merely from
the membership bit. Input/domain order alone need not order witness objects.

## 6. Constraints and validation

Joint invariants involving a group key's metadata and its payload need closure
checks beyond per-value validity. Multimap key and member domains are independent;
a “principal → roles” map cannot be composed with “resources → permissions” just
because both middle encodings are u32.

Tests: normalized empty fibers versus native present-empty; pair-versus-key
subtraction; duplicate witnesses; bag products/sums and overflow; per-key/global
order; group/value representatives from different operands; composition of
compatible and incompatible middle domains; and witness-preserving versus
membership-only terminals.

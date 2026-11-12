---
id: FROST-ALG-DOMAINS
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Value domains, key domains and conversions

Read [the formal model](formal-model.md) first. These are specification
abstractions; existing Frost collections do not acquire a mandatory domain
generic parameter in this change.

## 1. Admission is separate from identity

For D=(T,P), V_D={x | P(x)} is the admitted carrier. The predicate can encode
range constraints, normalized syntax, schema well-formedness or another explicit
restriction. A key domain K=(D,~) adds a lawful equivalence relation on V_D.
An implementation may expose a classifier κ:V_D→Keys satisfying
x~y iff κ(x)=κ(y), but no allocation of quotient objects is required.

[DOM-EQ] ~ is reflexive, symmetric and transitive and stable for the indexed
lifetime. Floating-point NaN under ordinary IEEE equality is a counterexample
to reflexivity; use a deliberately chosen key wrapper/domain instead of silently
assuming every Rust value is a lawful key. A sequence of floats needs no such
key contract until an operation deduplicates or indexes them.

Distinct records may have the same logical identity. Keep their metadata and
stored representative separate from key-class identity. Do not require an
application payload predicate P to be invariant on *all raw T* merely because
keys collapse metadata. However, any operation claimed to act only on the
quotient must be invariant under replacing admissible representatives.

[DOM-CONGRUENCE] A membership filter p on K must satisfy
x~y ⇒ p(x)=p(y). Filtering actual representatives by display metadata is allowed
as a different, representative-sensitive operation; it is not a Boolean-algebra
homomorphism on key classes. Likewise, a resolver that inspects representative
metadata needs that input in its denotation, not just a key-only equation.

## 2. Validation is not normalization

A validator's soundness is [DOM-VALID]: validate(x)=Ok ⇒ x∈V_D.
Completeness and termination are separate claims. A bounded schema checker can
be sound but return Unsupported or BudgetExceeded for an otherwise valid value.
Such a result is not proof of invalidity. A constructor with private fields can
preserve validity after admission only if exposed APIs cannot invalidate it.

[DOM-NORMALIZE] Normalization n:T⇀V_D chooses a representation and may change meaning relative
to the raw domain. State its failure and collision policies. If x is outside
V_D, the output equivalence ~ on V_D cannot even compare n(x) with x. Introduce
raw meaning μ_raw on the accepted raw inputs and valid-value meaning μ_D, then
require μ_D(n(x))=μ_raw(x) for a meaning-preserving normalization. Alternatively,
state an explicit raw equivalence on a carrier containing both values. Only for
already-valid x may the simpler n(x)~x equation use the key-domain equivalence.

If valid outputs can be fed back as raw inputs (possibly through an explicit
embedding), canonicalization also requires n(n(x))=n(x) at the named observation.
With different raw/output types, write the embedding instead of an ill-typed
composition. Idempotence alone is insufficient: a constant function is idempotent
but can collapse every distinct identity. Unicode/path/case normalization needs
an explicit version and matching query semantics, not a convenient string helper.

## 3. Four different compatibility checks

| Layer | Examples | Required claim |
| --- | --- | --- |
| Semantic identity | principal versus resource, tenant, case-sensitive versus case-folded keys | Operands denote the same key classes, or an explicit translation exists |
| Physical encoding | integer dictionary, codec version, byte ordering | Equal codes mean equal logical keys before direct encoded operations |
| Evaluation snapshot | data/model revision, transaction, authorization context | Source combination satisfies the engine's consistency contract |
| Universe/completeness | scoped principals, wildcard expansion, upper bound | Complement and exact negatives have the required universe/evidence |

Equal Rust types are useful but not sufficient for runtime tenant/dictionary
identity. Different physical encodings may describe the same semantic domain,
but then direct bitwise algebra is not justified. Equal numeric revision labels
from different stores are not proof of one snapshot.

A runtime descriptor is attached once to a source/plan where possible, not once
per element. Descriptor equality is an assertion under a trusted adapter
contract, not a cryptographic proof of truthful remote data. Use stable structured
identity or provider-issued opaque handles; a truncated hash is not exact domain
identity. Static phantom/newtype tags complement, not replace, runtime checks.

## 4. Encodings and dictionary translation

For a key-only code c:K→W, [DOM-CODE] requires injectivity on K and a partial
inverse on its image. Order-preserving merge additionally requires
k1<k2 iff c(k1)<c(k2) in the admitted order. A membership bitmap needs neither
stored references nor arbitrary-T decoding.

Frost's implemented IntKey contract is stronger than a key-class-only codec:
decode(encode(x)) reconstructs the complete original value, and accepted codes
are canonical. Preserve that contract; do not reinterpret it as permission to
discard metadata ignored by Eq. Its sealed word widths remain u32/u64. Domain
validity may leave holes in those words, so complement over all bit patterns is
not generally complement over valid K.

Runtime dictionaries must not be hidden inside the implemented IntKey codec,
whose mapping is stable across calls, collections and time. Use raw typed codes
plus a source-level dictionary binding/translation adapter for that profile.

Runtime dictionaries must not be hidden inside the implemented IntKey codec,
whose mapping is stable across calls, collections and time. Use raw typed codes
plus a source-level dictionary binding/translation adapter for that profile.

Runtime dictionaries must not be hidden inside the implemented IntKey codec,
whose mapping is stable across calls, collections and time. Use raw typed codes
plus a source-level dictionary binding/translation adapter for that profile.

Independently assigned dictionary IDs require translation even if both tables
use u32. A lossless semantic reindexing between dictionaries may still cost
lookups, sorting, allocation, and collision checks; it is not a free cast.

## 5. Conversion laws

Let f:K→L be total on the relevant keys and f₍*₎(A)={f(a)|a∈A}.

[DOM-IMAGE] Direct image always preserves empty and union. It preserves
intersection, difference and XOR for all finite A,B iff f is injective on the
relevant domain. Proof of necessity: choose distinct a,b with f(a)=f(b), and
A={a}, B={b}. Image(intersection) is empty but intersection(images) is not.
The same example disproves difference/XOR preservation. For sufficiency, use
injectivity to show a destination member has one source class.

Direct-image complement requires an explicit universe and target f(U); even
injectivity does not justify complement over a larger target universe.
Preimage f⁻¹ preserves Boolean operations, including relative complement over
compatible universes, without injectivity. But a preimage can be infinite or
impossible to enumerate efficiently. Logical preservation supplies no iterator.

For bags, pushforward sums counts along a fiber:

    (f₍*₎b)(l) = Σ_{k:f(k)=l} b(k).

It preserves additive sum for total f, not generally max, min, or truncated
subtraction under collisions. For maps, changing keys through noninjective f
can violate key uniqueness and requires a checked collision resolver. For
BiMaps, both endpoint translations require injectivity on the occupied endpoint
sets, or output must be validated as a new partial bijection.

Sequence map applies f to occurrences without collapsing their positions.
The map from sequences to bags forgets order but preserves concatenation as
bag sum; bag support preserves sum/max as set union and min as intersection,
but not truncated subtraction.

These are denotational laws. Fallible/partial conversions, representatives,
callback effects, and overflow add obligations before an execution rewrite.
For example, converting a whole input before filtering may fail on a value
that filter-then-convert would never visit.

## 6. Rust realization

Prefer ordinary newtypes, private fields, and TryFrom/checked constructors.
Native Eq/Ord/Hash and borrowed lookup remain collection-specific contracts.
Hash requires equivalent keys to hash equally under the same hashing scheme;
Ord needs a consistent key order. Do not reuse a cached hash across independently
seeded tables merely because they share a hasher type.

Optional interfaces, shown only as uncompiled design sketches:

```text
ValueDomain { associated Value: possibly unsized }
ValidateValue: ValueDomain { validate(&self, &Value) -> Result<(), Error> }
DomainBoundSource { semantic_binding(); representation_binding(); }
```

ValueDomain alone exposes no equality, iterator, serializer or solver. Do not
claim ValidateValue permanently proves immutability. Borrow<Q> may be used only
when borrowed and stored forms agree on identity/hash/order; a case-insensitive
key must not pretend a case-sensitive str is the same key query. Existing
LookupRef/SetContains implementations decide lawful borrowed-query bounds.

## 7. Do not confuse abstract-analysis domains

An abstract interpretation domain additionally supplies a concretization,
approximation order, sound transfer/join laws, and convergence assumptions.
That is not required of ordinary collection values. Structural equality of
symbolic syntax is not solver equivalence under an evolving theory. A solver
query that returns Unknown cannot implement a total equivalence relation by
silently mapping Unknown to false. Collection union of symbolic-state IDs does
not establish a sound program-state join.

See [Rust integration](rust-integration.md), [sources](../../research/formal-models/SOURCES.md),
and [the review](../../reviews/value-domain-algebra-review.md).

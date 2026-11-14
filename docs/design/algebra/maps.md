---
id: FROST-ALG-MAPS
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Map algebra

Map(K,V) is a finite partial function, with absence written ⊥. A stored map adds
a key representative ρ(k), payload access, and traversal guarantees. Payloads
are values in V, not automatically key classes and not required to implement Eq.
If V itself contains Option, ⊥ and Some(None) are different states.

## 1. Merge, alignment and joins are different operations

| Proposed name | Domain and equation |
| --- | --- |
| merge_left(A,B) | Same V; A(k) if present, otherwise B(k) |
| merge_right(A,B) | Same V; B(k) if present, otherwise A(k) |
| merge_with(A,B,r) | Same V; call r only at collisions, carry unique-side values unchanged |
| align_with(A,B,g) | g on tagged Left(v), Right(w), Both(v,w), once per union key; output W may differ |
| inner_join(A,B) | k↦(A(k),B(k)) for common keys; Map(K,V×W) |
| left_join(A,B) | k↦(A(k), optional-presence(B(k))) for keys of A |
| full_join(A,B) | Map(K, Left(V) + Right(W) + Both(V×W)) |
| intersection_by_key(A,B) | Restrict A to dom(A)∩dom(B), ignoring B's payload |
| difference_by_key(A,B) | Restrict A to dom(A)\dom(B), ignoring B's payload |

The last two spellings are recommendations of this formal refinement, not new
implemented APIs; historical difference_keys maps to difference_by_key.
A collision-only resolver cannot define heterogeneous unique-side conversions;
use align_with or specify those converters explicitly. An align callback never
receives BothAbsent. Its optional removal result is separate from a stored
Option payload.

Established names, recorded as aliases only: the three-way tag is called
These, Ior or EitherOrBoth in other libraries, and per-side behaviours are
called merge tactics (`SRC-PRIOR-CONTAINERS-MERGE`, `SRC-PRIOR-SEMIALIGN`).
Both rows are survey-read and were not reread at integration, so no law here
depends on them.

## 2. Closure and partial resolver algebra

If r:V×V→V is total and closed, extend it to V⊥ with ⊥ as identity:

    r̂(⊥,⊥)=⊥; r̂(⊥,v)=v; r̂(v,⊥)=v; r̂(v,w)=r(v,w), for v,w in V.

[MAP-LIFT] Pointwise map merge is associative iff the relevant lifted operation
is associative; a total associative r without deletion has an associative lift.
The forward implication uses maps containing a single key. The reverse follows
by checking each key independently. Empty map is the identity.

For r:V×V⇀V⊥ that can delete or fail, analyze r̂ on the full lifted domain,
including definedness, not just r on successful collisions. Deletion can make a
previously absent side reappear in another grouping. For r(v,w)=⊥ when v=w,
otherwise v, values a,a,b (a≠b) produce b under left association and ⊥ under
right association. This disproves unrestricted merge reassociation.

Left/right override are associative and idempotent on values, but generally
not commutative. A subtraction resolver fails associativity even without
removal: (10-3)-2=5, while 10-(3-2)=9. Effectful resolvers need their own trace
contract even when their value-level arithmetic is associative.

If there is a dependent payload rule V(k), the resolver must be closed in
V(k) for each key. If a stronger invariant relates the *stored representative's
metadata* to its value, retaining a left key with a right payload can violate it.
Such a constrained map requires a compatible resolver/representative policy or
a checked error; logical key equality alone is insufficient.

## 3. Representatives and join isomorphisms

Default overlap selects the left stored key. A right-biased value winner does
not imply a right key representative. Right-only bindings retain the right key.
An inner join returns the left key and both payloads, conceptually
(&K, &VL, &VR) with independent borrowing lifetimes.

Join commutativity needs a payload swap and may change the stored key. Join
associativity requires an explicit isomorphism ((a,b),c)↔(a,(b,c)); these are
not identical Rust types. A count-only key match may ignore representatives
but still cannot ignore domain mismatch, errors or resolver effects.

Restricting by a pure key predicate is idempotent. Map key projection forgets
payloads. Value projection can have duplicates: it is naturally a sequence or
bag under a declared traversal, not automatically a set.

## 4. Lazy execution and materialization

A pure borrowed join may stream matching references without cloning payloads.
Sorted sources use merge/seek; hash sources use probe plans. Scanning the right
source does not authorize returning its key when the contract selects the left.
User hashing/comparison and payload interior mutation are not automatically pure.

A lazy resolver expression may invoke callbacks again on repeated traversal;
“once per key” means once per traversal of the consumed logical position, not
once for the lifetime of the expression. Prepared/cached results are separate
objects. State callback order (ascending for a sorted profile; otherwise a named
or unspecified traversal), error precedence, cancellation and prefix behavior.

For fallible all-or-nothing materialization, stage results before publishing a
new owner. Returned-error rollback does not promise rollback of side effects,
panics, allocation aborts or user-owned interior state. Ordinary borrowed inputs
remain structurally unchanged; consuming variants state recoverable ownership.

## 5. Existing capabilities and acceptance

Use MapTypes, LookupRef<Q>, ValueRef<Q>, Entries and OrderedEntries. Do not
replace them with the reports' MapGet/Get/SortedMapEntries names. Ordinary and
adaptive storage families keep current freeze partners. Proposed MapView owns
expression composition, not the existing inherent collection methods.

Tests: presence versus present-None; first-key/last-construction-value versus
merge resolution; resolver call counts and effect order; key-dependent payload
validity; non-Clone values; joins up to explicit tuple maps; domain-conversion
collisions; returned-error versus panic; and differential sorted/hash plans.

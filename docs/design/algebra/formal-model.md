---
id: FROST-ALG-FORMAL
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Abstract model and refinement

This document defines the mathematical specification used by every domain
chapter. Equations are project definitions and derived propositions, not claims
that Frost already implements a generic algebra API.

## 1. Values, keys, carriers and contexts

A value domain is D = (T, P), with admissible values V_D = {x in T | P(x)}.
P is a mathematical predicate; its decision procedure is a separate capability.
There is no universal requirement for Eq, Ord, Hash, Clone, Send, or enumeration.
A key domain additionally chooses an equivalence relation ~ on V_D, with
quotient K_D = V_D / ~ and quotient map q_D. Representatives remain observable
Rust objects, even when their images under q_D agree.

For payloads and sequences, do not silently replace V_D with K_D. A map can
hold non-Eq payloads, and a sequence keeps two equal occurrences at different
positions. Relevant value observations are part of its contract.

| Carrier | Definition |
| --- | --- |
| Set(K) | Finite subsets of key classes K |
| Map(K,V) | Finite partial functions K ⇀ V |
| Bag(K) | Finite-support functions K → N |
| Seq(V) | Finite words V* |
| Rel(K,L) | Finite subsets of K × L |
| MultiMap(K,F) | Finite-support functions K → F, for a fiber F with named empty element |
| BiMap(K,L) | Finite relations functional in both directions |

Any additional application restriction J, such as nonemptiness, a maximum
length, or a key-dependent payload restriction, defines a subcarrier. Closure
of an operation on the unrestricted carrier does NOT imply closure on that
subcarrier. For example, intersecting two nonempty sets can produce empty.

An evaluation context θ binds semantic domains, representation contexts and
source snapshots. Its compatibility conditions are operation-specific. There
is no universal rule that equal timestamps from unrelated stores identify one
snapshot. A context may be local and implicit for an ordinary borrowed set;
it must be explicit for runtime-scoped or durable sources.

## 2. Physical representation and observational structure

For a representation R define:

- I_R(r,θ): its invariant, including stable keys, complete indexes, valid codes,
  counts, length/uniqueness, inverse consistency, and any source assumptions.
- α_R(r,θ): its denotation in a named carrier, defined only when I_R holds.
- ρ_R(r,k): its stored representative, if that access model really retains one.
- τ_R(r): its traversal contract, including occurrences, order and lifetimes.
- C_R: allowed ownership, panic, returned-error, callback and cost observations.

Established vocabulary: I_R is a representation invariant or well-formedness
predicate, and α_R is an abstraction function or view. A verified map library
pairs each raw operation with one lemma that the operation preserves the
predicate, and verifies operations against a list model (`SRC-PRIOR-LEAN-MAPS`).
[FM-REFINE] below is that obligation together with the denotation equation.
The names here stay I_R and α_R.

α is not necessarily injective. Different Sorted/Dense/Roaring/Chunked storage
can denote the same set. Neither α-equivalence nor freezing proves that their
allocation behavior, representation diagnostics or iterator costs agree.
Encoded keys use a value-returning decoder, not a fictitious stored reference.

A representation may expose interior-mutable payloads. α at a particular
observation can then depend on external state. For a computation that needs a
stable denotation, explicitly assume that state is stable, snapshot it, or
restrict the admitted payload/source profile. Structural freezing alone is
insufficient.

## 3. Operations and partiality

Specify an operation as a typed partial mathematical function:

    f : C1 × ... × Cn × Parameters ⇀ Cout

with an explicit validity predicate Pre_f. Cout need not equal any input
carrier: joins produce product/tagged payloads; BiMap union may produce a
relation; counts and membership are scalar terminals.

Successful execution must satisfy [FM-REFINE]:

    all I_Ri(ri,θ) and Pre_f(α_R1(r1,θ), ..., p)
    and execute_f(r1,...,p,θ) = Complete(rout)
      imply I_Rout(rout,θ)
      and α_Rout(rout,θ) = f(α_R1(r1,θ),...,p).

Each precondition must be classified as statically enforced, checked at an
import/operation boundary, or an explicit caller/adapter semantic obligation.
Rust cannot generally decide whether arbitrary Eq, Hash, Ord or callbacks obey
all their laws. A checked-admission API must not report successful admission
after its required validity or compatibility check fails; an unchecked semantic
obligation still cannot justify unsafe memory access.

This is a safety implication, not a totality theorem. A separate progress claim
states when a valid input is guaranteed to complete. Budget exhaustion,
unsupported operations, failed I/O, and diverging callbacks can prevent
completion even when the mathematical result exists.

Distinguish InvalidInput, IncompatibleContext, Overflow, SourceFailure,
Incomplete/BudgetExceeded, and Complete outcomes when the API needs them.
Panics and allocation aborts are not ordinary returned errors. A checked error
must state whether inputs are returned, retained, or partly consumed. A
collection is not transactional simply because the function returns Result.

## 4. Equivalence and refinement for nondeterministic contracts

Let Beh_C(p,θ) be the set of allowed observable traces for plan p under contract
C: result values, selected representatives, documented ordering, effects,
failures, completion, and context. Costs enter the trace only when the contract
promises them; unspecified hash iteration does not acquire a fixed sequence.

[FM-OBS] A replacement implementation is a refinement if every produced trace
is allowed by the contract. A bidirectional optimizer equality p ≈_C q needs
equal sets of allowed traces, or a documented weaker observational quotient.
This handles unspecified iteration order without demanding arbitrary byte-for-
byte trace equality. It also avoids calling every faster traversal a breaking
change merely because it makes fewer undocumented comparisons.

Every rewrite must name its domain, operation, terminal, allowed effects,
representative policy, error policy, and context. Eq/Ord/Hash user code can
observe calls or panic. Fn is not a proof of purity. An optimizer may rely only
on laws explicitly admitted by the chosen profile; it must not introduce
memory unsafety when a safe semantic trait lies.

## 5. Streams and terminal observations

A streaming plan emits a prefix and then Complete, Incomplete, or Failed.
[FM-PREFIX] Each emitted member/entry is justified by the result at θ, duplicate
and order rules hold, and any iterator-lent reference lives only as promised.
Complete additionally certifies that the stream omits no required result.
A correct prefix does not certify absence of everything it has not emitted.

Membership, exists, exact count, bounds, bounded listing and full materialization
are different terminals. A positive witness may settle exists before enumeration
completes. A negative membership/existence answer requires sufficient complete
negative evidence, not necessarily exhaustion of every irrelevant source.
Complete logical annihilators can supply such evidence when error semantics
permit it. See [computed sets](execution-and-computed-sets.md).

Construction of an expression does not certify that every output will fit a
machine count or that every callback will succeed. An exact empty result still
has a semantic domain and context; it cannot erase incompatible operands before
validating a contract that requires upfront compatibility.

## 6. Algebraic laws versus machine bounds

The default mathematical integers in this specification are unbounded. A u64
or usize implementation is a partial refinement with checked conversions and
arithmetic. State count width, distinct count, total multiplicity, key span,
length, workspace and output size separately.

For nonnegative checked addition, either parenthesization succeeds exactly when
the total fits, and both successful totals agree. That does not equate their
first overflow diagnostic, callback trace or partially emitted prefix. Later
subtraction cannot legitimize silently overflowing an earlier addition.

The current integer profile's representation policy is history-aware. In
particular, Auto construction and Auto algebra with eligible identity sharing
are different contracts. Do not add a mathematical axiom that every empty
Auto result is Sorted. Mathematical empty identity permits multiple physical
representations; output policy and allocation-sharing observations remain
separate. This specification does not alter existing policy APIs.

## 7. Composition obligations

A plan composition is sound when each child refines its stated denotation,
interfaces agree on domains and observations, and the parent only uses the
capabilities actually supplied. Prove or check these obligations:

1. Domain validity and key identity stability.
2. Closure, or a correctly typed output domain / returned error.
3. Compatibility of dictionaries, sorting relations and revisions.
4. Representative, ordering, multiplicity and callback rules.
5. Prefix soundness and honest completion/negative evidence.
6. Rust ownership/aliasing separately from the mathematical equation.

The domain chapters instantiate this model. The
[proof ledger](proof-obligations.md) distinguishes proof arguments, finite
checks, counterexamples and unexecuted implementation gates.

---
id: FROST-ALG-PROOFS
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Proof and implementation obligations

The specification has 27 named obligations. A proof argument here is
ordinary mathematical reasoning, not a machine-checked theorem. The finite
models enumerate their recorded domains and reproduce counterexamples. They
execute no production Rust, perform no unsafe-code analysis, and establish no
performance advantage. A large assertion count is not a coverage percentage.

| ID | Contract | Argument / obligation | Evidence and limit |
| --- | --- | --- | --- |
| `FM-REFINE` | [Representation refinement](formal-model.md) | Safety implication with typed preconditions; separate progress and error obligations | definition; no generic executable implementation |
| `FM-OBS` | [Observable refinement](formal-model.md) | Allowed-trace sets model unspecified order; equality is stronger than refinement | definition; effect/representation counterexamples |
| `FM-PREFIX` | [Stream prefix soundness](formal-model.md) | Every emitted item justified; completeness must be separately established | definition; knowledge-bound finite model |
| `DOM-EQ` | [Stable key equivalence](domains.md) | Equivalence classes require reflexivity, symmetry, transitivity and stability | finite classifier-generated equivalences; Rust-law obligation remains |
| `DOM-CONGRUENCE` | [Quotient congruence](domains.md) | Key-only operations constant on each equivalence class | finite functions/filters and representative counterexample |
| `DOM-VALID` | [Validation soundness](domains.md) | Successful admission implies predicate; completeness/termination not inferred | finite percentage boundaries; general obligation |
| `DOM-NORMALIZE` | [Typed normalization](domains.md) | Separate raw meaning from valid-output equivalence; explicit embedding for idempotence | proof obligation; raw-to-integer example and constant-normalizer counterexample |
| `DOM-CODE` | [Code injectivity](domains.md) | Key-code inverse on image; existing IntKey has stronger full-value laws | proof obligation; current source mapping, no codec implementation executed |
| `DOM-IMAGE` | [Conversion homomorphisms](domains.md) | Total direct image preserves union; injection necessary/sufficient for intersection/difference/XOR | proof argument plus all functions on three keys |
| `SET-BOOLEAN` | [Finite set laws](sets.md) | Membership laws, explicit universe and count formulas | bounded exhaustive equations and independent sorted merge |
| `SET-REP` | [Representative refinement](sets.md) | Left union/intersection associate; commutation and XOR reassociation can fail | pointwise argument and metadata enumeration |
| `MAP-LIFT` | [Map merge lifting](maps.md) | Lift collision resolver through absence and check totality/definedness | pointwise proof argument; finite overrides/sum/deletion counterexample |
| `BAG-LAWS` | [Count algebras](bags.md) | Natural sum versus max/min/monus/absolute difference | pointwise argument, exhaustive small counts and counterexamples |
| `COUNT-PARTIAL` | [Bounded count addition](bags.md) | Either association succeeds iff total fits; errors/traces separate | nonnegative arithmetic argument and width-three model |
| `BAG-SUPPORT` | [Support projection](bags.md) | Preserves sum/max as union and min as intersection; not monus/XOR | pointwise proof and finite model |
| `SEQ-MONOID` | [Free monoid](sequences.md) | Concatenation identity/association, occurrence preservation | word argument and finite words |
| `SEQ-EFFECT` | [Execution effects](sequences.md) | Pure equations do not establish callback/error/drop equivalence | design obligation; future Rust effect/drop tests required |
| `MM-SUPPORT` | [Empty-fiber normalization](multimaps.md) | Canonical proposed multimap absent iff fiber empty; native maps may differ | definition and present-empty counterexample |
| `REL-COMP` | [Boolean relation composition](multimaps.md) | Existential witnesses; associative and union-distributive | witness proof argument and finite Boolean matrices |
| `BAGREL-COMP` | [Weighted composition](multimaps.md) | Sum of products counts derivations; no general cycle convergence | finite double-sum argument and small natural matrices |
| `BIMAP-CLOSED` | [Partial-bijection closure](bimaps.md) | Subsets/inverse/composition preserve both uniqueness projections | proof argument and all small partial bijections |
| `BIMAP-IDENTITY` | [Local inverse identities](bimaps.md) | Identities on domain/range; global finite identity requires finite universe | proof argument and finite model |
| `BIMAP-PARTIAL` | [Checked operations](bimaps.md) | Union partial definedness associates; checked XOR does not | subset proof argument, enumeration and explicit conflict counterexample |
| `FROZEN-DENOTE` | [Phase transition](frozen.md) | Successful freeze of lawful family preserves denotation, not all representation details | typed contract; actual Rust transitions not executed |
| `FROZEN-STABLE` | [Structural versus deep stability](frozen.md) | Fixed structure is not fixed heap payload or source revision | abstract heap counterexample; no Rust aliasing proof |
| `KNOW-BOUNDS` | [Incomplete set knowledge](execution-and-computed-sets.md) | Sound lower/upper transfer; not correlation-complete or error absorption | inclusion proof and all interval realizations on three keys |
| `DELTA-REPLAY` | [Revisioned overlays](execution-and-computed-sets.md) | Exact base plus ordered complete suffix; ordered compaction preserves result | fold decomposition argument and exhaustive short histories |

## Proof sketches beyond the tables

**Map lift.** Every key independently holds either absence or a payload.
Associativity of the lifted scalar operation gives associativity at every key,
therefore of finite maps. Conversely, singleton maps recover the scalar case.
Deleting/failing resolvers require the same argument over presence and definedness;
checking arithmetic only on successful payloads is insufficient.

**Inner join.** For each common key, both groupings contain exactly the three
input payloads. Their types are `((V,W),X)` and `(V,(W,X))`, so equality requires
an explicit associator. Swapping inputs similarly requires swapping payloads.
Neither isomorphism guarantees equal key representatives or callback traces.

**Relation and weighted composition.** Boolean composition expands to an
existential over two intermediate keys in either grouping. Weighted composition
expands to the same finite sum of products, using natural associativity and
distributivity. Proof/provenance witnesses remain additional observations.

**Positive Boolean recursion.** A monotone transformer on finitely many Boolean
facts iterated from bottom forms an ascending chain. Every strict change adds
a fact, hence the chain stabilizes. This is not a runtime bound on a whole query.
For bags, `x -> 1+x` already has no finite natural fixed point on one key; a
finite key set alone supplies no termination theorem.

**Observational caveat.** A logical equation does not prove equality of all
possible implementation traces. Representation diagnostics, emitted prefixes,
first error, suppressed read, callback order, retained allocations and borrowed
lifetimes require their own contract. Existing safe semantic trait failures must
not cause memory unsafety, regardless of the mathematical assumption.

## Current executable evidence

The [result file](../../research/formal-models/evidence/model-results.json)
records **258,624 assertions**, **99,118 counted finite cases**,
14 groups and 33 explicit counterexamples.
Counters count assertions/loop cases, not distinct Rust tests or exhaustive
production states. The final group detects six deliberately wrong mathematical
implementations; it is not cargo-mutants execution or a mutation score.

## Before implementing any public API

Map each obligation to current CRATES test/catalog requirements. Compile borrowed
and non-Clone examples, test returned errors and panics separately, retain
representative/order/callback expectations, check feature isolation and embedded
profiles, and run required ownership/concurrency tools on actual code. A
compile-fail example must fail for the intended semantic restriction, not a
missing proposed import. Current catalog IDs and benchmark workloads are not
changed by this proposal.

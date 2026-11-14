---
id: FROST-ALG-INDEX
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Value domains and collection algebra

This is a formal-first refinement of the research corpus and prior review
artifacts, reconciled with main at the revision in this document's metadata.
The [research supplement](../../research/formal-models/README.md) lists its
inputs. It specifies future contracts; it does not add production types or
approve a workspace migration.

These chapters were written under an API-compatibility constraint that no longer
applies: the [decision of 2026-11-12](../decisions.md#api-compatibility-is-not-a-constraint-2026-11-12)
permits names, signatures, bounds and type parameters to change. Statements
here that an existing API is kept describe the text's baseline, not a
requirement on a refactor. Semantic contracts, including documented
native-wrapper semantics, are unaffected. [CRATES.md](../../../CRATES.md)
remains the standard for layout, dependency isolation, API obligations, tests,
and benchmarks.

## Read in this order

| Layer | Document | Question answered |
| --- | --- | --- |
| Abstract specification | [Formal model](formal-model.md) | What does implementation correctness mean? |
| Values and identity | [Domains](domains.md) | Which values are admitted, what identifies them, and which conversions preserve laws? |
| Collection carriers | [Sets](sets.md), [sequences / lists](sequences.md), [maps](maps.md), [bags](bags.md) | What is the denotation and operation-specific contract? |
| Relations and uniqueness | [Multimaps](multimaps.md), [BiMaps](bimaps.md) | What changes with fibers, witnesses, and two-sided uniqueness? |
| Ownership | [Frozen values](frozen.md) | What does freezing preserve, and what does it not guarantee? |
| Execution | [Computed sets](execution-and-computed-sets.md), [rewrite contracts](rewrite-contracts.md) | What may stream, short-circuit, be reordered, or require a snapshot? |
| Rust integration | [Capabilities and placement](rust-integration.md), [terminology](terminology.md) | How does this fit current Frost without a universal trait or new mandatory parameters? |
| Assurance | [Proof obligations](proof-obligations.md), [adversarial review](../../reviews/value-domain-algebra-review.md) | Which propositions have arguments, bounded tests, or unresolved implementation gates? |

The [research supplement](../../research/formal-models/README.md) contains the
input inventory, primary references, executable finite models, results, a
focused research prompt, and the source-to-contract map. The
[contract catalog](catalog.json) registers stable names and law IDs.

## The core separation

```text
Value domain: admissible values
Key domain: logical identity of selected values
Collection carrier: arrangement, multiplicity, order and uniqueness
Representation invariant: validity of one physical encoding
Operation contract: preconditions, denotation and observable behavior
Evaluation context: compatible domains, dictionaries, revisions and request data
Execution plan: an implementation of a terminal under that contract
Frozen lifecycle: access/ownership restrictions, not another carrier
```

No step implies the next. A valid value need not be a key. A key type need not
be enumerable. A finite universe does not make bag recursion terminating.
A frozen index need not be fresh. A valid BiMap does not admit arbitrary union.

## Scope and authority

For implemented behavior, the source, rustdoc, and actual tests at the pinned
revision remain authoritative. Within this proposed refinement, this index and
catalog name the domain-specific contracts. The existing
[collection integration guide](../collection-algebra.md) retains its research
history and selected names; [general set algebra](../../../crates/frost-collections-core/docs/set-algebra.md) retains its
more detailed expression proposal. Conflicts are recorded, not silently solved
by renaming an implemented type.

An earlier proposed `MutableSeqMultiMap` spelling is not imposed on
main: main already selects `SeqMultiMap`, `SetMultiMap`, and `BagMultiMap`.
Lifecycle partners beyond those settled names remain design questions. Research
originals, archived patches, and historical logs are evidence, not instructions.

A successful finite model is not a Lean/Kani proof, a Rust compilation result,
a Miri ownership result, or a benchmark. This revision has no measured engine
speedup and introduces no authorization, symbolic-execution, or storage engine.

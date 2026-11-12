---
id: FROST-ALG-CARRIERS
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Collection model directory

| Model | Carrier | Key distinction |
| --- | --- | --- |
| [Value domains](domains.md) | Predicate-refined values, optionally quotiented by key identity | Validity, identity, universe, encoding and revision are independent |
| [Sets](sets.md) | Finite subsets of key classes | Membership and representatives differ |
| [Sequences / lists](sequences.md) | Finite ordered occurrences | No identity bound for concatenation |
| [Maps](maps.md) | Finite partial functions | Absence, payload validity and resolver semantics |
| [Bags](bags.md) | Finite-support N-valued functions | Sum is not max; absolute difference is not XOR |
| [Multimaps](multimaps.md) | Key-to-fiber / pair relations | Fiber multiplicity/order and witness provenance |
| [BiMaps](bimaps.md) | Finite partial bijections | Both endpoint projections unique |
| [Frozen values](frozen.md) | Lifecycle refinement of a representation | Not an independent collection carrier |

Begin with [the abstract refinement model](formal-model.md), then read each
carrier's closed/partial operations, laws, counterexamples and implementation
mapping. Exact-complete and incomplete-computed source semantics are separated
in [execution](execution-and-computed-sets.md).

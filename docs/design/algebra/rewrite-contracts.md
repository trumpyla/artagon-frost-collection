---
id: FROST-ALG-REWRITES
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Rewrite admission

The proof unit is **domain + operator + terminal + observation profile +
context + preconditions**, not an unqualified algebraic identity.

| Candidate | Denotational status | Extra restrictions / counterexample |
| --- | --- | --- |
| Set union/intersection commute | Valid membership | Left representatives/order/effects can change |
| Set XOR reassociate | Valid membership | Fails representative observation with three equal differently labelled keys |
| Set absorption and distribution | Valid membership | Representatives survive only in the left-first orientations; (B∩A)∪A→A and (B∩C)∪A→(B∪A)∩(C∪A) change the selected object |
| Noninjective key conversion through union | Valid membership | Representative and order rules of the conversion still apply |
| Noninjective key conversion through intersection/difference/XOR | Invalid in general | Two distinct keys can collapse. Converting first can only add to an intersection and only remove from a difference or XOR |
| Preimage through Boolean operations | Valid with compatible universes | May not be finite/enumerable; callback failures remain visible |
| Bag sum reassociate | Valid in N; checked successful totals agree | Error order and emitted prefixes can differ |
| Bag absolute difference reassociate | Invalid | Counts 1,1,2 |
| Set-difference identity read on bags | Conditional, one identity at a time | [BAG-MONUS](bags.md) lists the readings that hold; four fail, for example (A sum B) minus B is A, not A minus B. The reading must name sum or max |
| Product through bag difference, (b∸a)·c to b·c∸a·c | Valid for natural counts | Fails in another weight algebra (`CX-MSEMIRING-A5`); a sum or product side can add or remove an overflow |
| Map merge reassociate | Conditional | Need associative lift including absence/deletion/definedness |
| Inner join commute/associate | Only up to output isomorphism | Tuple shape, key representative and effects matter |
| Sequence concat reassociate | Valid word | Preserve stream effects and cleanup rules |
| Sequence concat commute or zip distribute over mismatched concat | Invalid | Changes occurrences/alignment |
| Relation compose reassociate | Valid membership / natural weights | Witness selection, multiplicity, overflow and traversal differ |
| Checked BiMap union reassociate | Same success domain and logical result | Conflict choice/order may differ |
| Checked BiMap XOR reassociate | Invalid even for success/failure | An intermediate conflict can disappear only after cancellation |
| N-ary checked BiMap union validated against one common operand | Invalid | Compatibility is not transitive; every pair of operands must be compatible ([BIMAP-COMPAT](bimaps.md)) |
| Composition through a compatible BiMap union | Valid logical pairs | Conflict choice and the representative of an equal pair follow the union's rule |
| Weighted composition iterated to a fixed point | Conditional | Stable weights, an acyclic recursion, or a checked bound with `Overflow` as a declared outcome ([CLOSURE-STABLE](execution-and-computed-sets.md)) |
| Freeze twice is idempotent | Usually ill-typed | Family phase transition is not a universal endofunction |
| Empty/identity shortcut before domain checks | Conditional or invalid | Empty sources still carry domains; compatibility errors must not disappear |
| Difference against incomplete index | Not justified as exact result | Need complete negative evidence / sound upper exclusion |
| Same pinned source A\A→empty | Membership valid | Validate identity/context and honor error/effect policy |
| Reuse of a cached or prepared result under another context | Invalid unless independence is proved | The key holds every context component the result depends on ([CTX-KEY](execution-and-computed-sets.md)) |

## Congruence obligation

An operation on quotient keys must be well-defined: replacing an argument with
an equivalent key representative cannot change its key-only denotation. If a
resolver/filter intentionally observes metadata, include that metadata in the
enriched semantics and prohibit membership-only rewrites that lose it. A trait
implementation named Associative is evidence only under its documented law
contract; it is not a compiler proof or an unsafe memory-access precondition.

## Effects and errors

Do not claim every comparison/hash call count is stable unless the API promises
it. Conversely, callback order/count and which errors are suppressed cannot be
ignored when the terminal specifies them. A fast membership profile can permit
short-circuiting and unspecified internal traversal; an audit/explanation profile
can require complete contributing evidence. Those are different contracts.

A planner may maintain an optimized membership plan and an original provenance
plan. Its terminal selects the appropriate observation, or preserves enough
provenance through optimization. Cached/prepared values bind the domain/context
and callback assumptions under which they were produced.

## Admission is a list, not a decision procedure

A rule is admitted because its record below exists, not because it follows
from a base set of axioms. For bag arithmetic no such base set can exist: the
identities of natural numbers with sum, product and truncated subtraction are
not finitely axiomatizable (`SRC-PRIOR-SUCIU-DIFFERENCES`, Theorem 21). A set
identity is not evidence for its bag reading, and a law of one weight algebra
is not evidence for another.

## Evidence record

Each admitted rule records stable ID, equation, carrier/output type, preconditions,
allowed observations, implementation scope, proof argument or checked bounds,
known counterexample, and fallback. Cost improvement is evaluated only after
correctness. No benchmark threshold is inferred from a mathematical identity.
See [proof obligations](proof-obligations.md) and the executable research model.

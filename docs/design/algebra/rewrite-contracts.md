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
| Noninjective key conversion through intersection/difference | Invalid in general | Two distinct keys can collapse |
| Preimage through Boolean operations | Valid with compatible universes | May not be finite/enumerable; callback failures remain visible |
| Bag sum reassociate | Valid in N; checked successful totals agree | Error order and emitted prefixes can differ |
| Bag absolute difference reassociate | Invalid | Counts 1,1,2 |
| Map merge reassociate | Conditional | Need associative lift including absence/deletion/definedness |
| Inner join commute/associate | Only up to output isomorphism | Tuple shape, key representative and effects matter |
| Sequence concat reassociate | Valid word | Preserve stream effects and cleanup rules |
| Sequence concat commute or zip distribute over mismatched concat | Invalid | Changes occurrences/alignment |
| Relation compose reassociate | Valid membership / natural weights | Witness selection, multiplicity, overflow and traversal differ |
| Checked BiMap union reassociate | Same success domain and logical result | Conflict choice/order may differ |
| Checked BiMap XOR reassociate | Invalid even for success/failure | An intermediate conflict can disappear only after cancellation |
| Freeze twice is idempotent | Usually ill-typed | Family phase transition is not a universal endofunction |
| Empty/identity shortcut before domain checks | Conditional or invalid | Empty sources still carry domains; compatibility errors must not disappear |
| Difference against incomplete index | Not justified as exact result | Need complete negative evidence / sound upper exclusion |
| Same pinned source A\A→empty | Membership valid | Validate identity/context and honor error/effect policy |

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

## Evidence record

Each admitted rule records stable ID, equation, carrier/output type, preconditions,
allowed observations, implementation scope, proof argument or checked bounds,
known counterexample, and fallback. Cost improvement is evaluated only after
correctness. No benchmark threshold is inferred from a mathematical identity.
See [proof obligations](proof-obligations.md) and the executable research model.

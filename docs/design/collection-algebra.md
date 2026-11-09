# Collection algebra research integration

Status: proposed contracts and implementation roadmap. No new collection family,
expression API, or package is implemented by this document.

This synthesizes the supplied [Frost1 research](../reviews/supplied/frost-collection-algebra-research-1.md)
and [Frost2 research](../reviews/supplied/frost-collection-algebra-research-2.md).
The originals are retained byte for byte as research inputs. Their citation
tokens, ecosystem claims, and sketches have not been independently verified in
this integration. Frost2 refers to a research template that was not supplied.

Both reports recommend separating logical collection semantics from storage and
execution. That fits the current [architecture](architecture.md) and
[general set-algebra proposal](set-algebra.md). Frost1 assumes an existing
`crates/frost-*` workspace split; that assumption does not describe this repository.
Frost remains one library package, `frost-collections`, exposed as `frost`.
Suggested crate names are possible responsibility boundaries, not approved
package changes or existing public namespaces.

## Distinct logical contracts

| Domain | Proposed operations | Required distinction |
| --- | --- | --- |
| Set | Union, intersection, difference, symmetric difference | Membership and stored-representative provenance are separate observations. |
| Map | Merge with a resolver, inner/outer join, difference by key | A join retains both values; a merge explicitly chooses or creates one value. |
| Sequence | Concatenation, zip, product, sorted merge, transformation | Preserve occurrence order and duplicates; projection to a set or bag is explicit. |
| Bag | Additive sum, max-union, min-intersection, truncated subtraction, absolute difference | Count multiplicity separately from membership; distinguish sum from max-union. |
| Multimap | Pair algebra, grouping, joins, projections | Sequence-, set-, and bag-valued fibers have different order and duplicate laws. |
| Bimap | Pair intersection/difference, checked merge, composition | Both projections must stay unique; ordinary pair union is not closed over bimaps. |
| Prefix index | Exact lookup, prefix scan, matching prefixes, longest prefix | Prefix matching is a specialized query capability, not ordinary map union. |

For a bag with multiplicities `a(x)` and `b(x)`, the candidate definitions are
`sum = a + b`, `max_union = max(a, b)`, `intersection = min(a, b)`,
`difference = a.saturating_sub(b)`, and `symmetric_difference = abs_diff(a, b)`.
An additive count overflow must be reported explicitly. Saturation is part of
the subtraction definition, not an implicit overflow policy for addition.
Counts, distinct-element counts, and address-sized lengths are separate queries.
A compressed bag stores one representative and a count; it cannot reproduce
the identities of multiple equal occurrences supplied at construction.

Map overlap should preserve the left stored key. An inner join conceptually
yields `(&K, &VL, &VR)`. A merge resolver explicitly determines the output value;
right-only entries retain right representatives. Difference by key keeps only
left entries whose key is absent on the right. Resolver invocation count,
evaluation order, owned output type, and panic behavior require an API design
before implementation. The research snippets are not compile-tested APIs.

Bimap merge must validate conflicts on both sides. For example, combining
`{alice -> 1}` with `{alice -> 2}` or `{bob -> 1}` cannot yield an ordinary
bimap union. A checked merge should report the conflict and preserve the input
versions on an ordinary returned error. Destructive replacement, a resolver,
and relation-valued output are distinct operations. Panic and allocation-failure
guarantees need explicit design rather than an implied transactional promise.

## Capabilities and observable behavior

Lookup, traversal, sorted traversal, multiplicity, and reconstruction should be
independent capabilities. A generic lookup may need the stored key as well as
the value. Stable insertion order does not authorize a sorted merge. Borrowed
lookup must remain available without imposing `Clone` or `Default` on elements.

Ordinary families continue to keep the first equivalent key or element and the
last map value. Borrowed union and intersection keep left representatives.
Native wrappers keep their documented native behavior. New bag, multimap, and
bimap contracts must state representative selection independently of equality
and order. Tests should use keys with observable metadata ignored by equality.

Borrowed entries, decoded integer values, and lending byte-key streams remain
separate access models. The implemented [integer profile](integer-sets.md)
reconstructs values; it cannot fabricate stored references. Its shared storage
and explicit Chunked policy do not implement lazy collection expressions.

No optimizer may infer that membership-equivalent rewrites preserve the full
result. Representative provenance, traversal order, resolver calls, and panic
behavior are observable. The existing [set-algebra counterexamples](../reviews/generic-set-algebra-review.md)
remain constraints on reassociation and cancellation. Multi-source joins also
need an explicit compatibility contract for key equivalence and ordering.

## Execution and ownership boundaries

Start with a simple semantic implementation before specialized kernels. A later
planner may choose sorted merge, smaller-side probing, or a native bitmap
operation when capabilities justify it. Count and predicate terminals should
use scalar kernels where available; collecting members is a separate operation.
Prepared indexes, temporary allocations, and caching must have explicit owners
and costs. A reusable expression does not imply cached results.

Consuming operations may reuse owned storage when a backend permits it, with
documented representative, drop, and panic behavior. Persistence and concurrency
are separate choices: immutable sharing must preserve every retained version;
live per-key access does not establish a coherent whole-collection snapshot.
The existing [snapshot publication contract](snapshots.md) continues to govern
whole-value publication and retry callbacks.

Encoded integer maps, rank/select, FST streams, and radix/prefix indexes remain
specialized proposals. They need their own codec, cardinality, ownership, and
query contracts. No feature may silently change an ordinary family's type,
representation, hasher, public bounds, or frozen partner.

## Implementation sequence and acceptance gates

1. Specify map lookup/traversal and join/merge contracts first. Settle stored-key
   provenance, resolver types and call counts, ordering, and materialization.
2. Build small reference models and exhaustive finite cases before typed views.
   Use standard maps for joins and bag counts, pair sets for set-valued multimaps,
   sequences for ordered fibers, and two maps for bimap inverse consistency.
3. Add one collection domain at a time within the current package. Require
   non-Clone/non-Default examples, borrowed queries, partial-iterator checks,
   overflow and conflict tests, and documented returned-error/panic behavior.
4. Add sorted/hash/native execution only with differential equivalence to the
   reference model. Record construction, staging, allocation, destruction, and
   output timing boundaries; retain slower benchmark observations.
5. Add persistent or concurrent variants only with retained-version ownership
   tests and relevant Miri or scheduler-model checks. State bounds and the exact
   production code each model executes.

The reports' proposed names and signatures remain open design questions. This
integration preserves their full content and records a compatible roadmap; it
does not advertise those APIs as available or claim research sketches passed
the library's verification suite.

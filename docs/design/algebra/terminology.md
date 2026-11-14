---
id: FROST-ALG-TERMS
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Canonical concepts and naming scope

Main's [naming dispositions](../collection-algebra.md#naming-dispositions) are
preserved. [catalog.json](catalog.json) records this refinement's terminology
and law IDs. It is metadata, not generated Rust declarations.

| Concept | Current name / chosen interpretation | Status |
| --- | --- | --- |
| Valid values | Value domain D=(T,P) | Formal concept; ValueDomain trait optional |
| Indexed identity | Key domain (D,~), key class K | Formal concept, not a universal Rust bound |
| Scope identity | Semantic binding | Proposed source descriptor, separate from representation/revision |
| Encoded meaning | Codec/dictionary binding | Required for direct encoded cross-source work |
| Consistent observation | Evaluation context θ | Engine/adaptor contract, not Eq implementation |
| Set families | MutableOrderedSet/FrozenOrderedSet; MutableHashSet/FrozenHashSet | Implemented |
| Generic set computation | SetView | Existing proposal; no current universal view API |
| Integer owned set | FrozenIntSet | Implemented; eager algebra |
| Integer query / storage view / persistent family | IntSetExpr / FrozenIntSetView / PersistentIntSet | Three distinct proposals |
| Map families | MutableOrderedMap/FrozenOrderedMap; MutableHashMap/FrozenHashMap | Implemented |
| Map composition | MapView | Proposed expression; no container rename |
| Sequence / mathematical list | Seq(V); MutableVec/FrozenVec and inline profiles | Logical concept; existing container names retained |
| Sequence-valued multimap | SeqMultiMap | Name selected on main; family not implemented |
| Set-/bag-valued multimap | SetMultiMap / BagMultiMap | Names selected on main; families not implemented |
| Bag | Bag(K); candidate MutableBag/FrozenBag/BagView | Formal model; concrete names remain candidates |
| Bidirectional unique relation | BiMap(K,L); MutableBiMap / FrozenBiMap | Logical model and selected proposed lifecycle names |
| Frozen wrapper | Frozen<B> | Implemented exact-backend type; no universal FrozenValue |

Lists are not a promise of linked-list storage. Do not invent FrozenList.
Older MutableSeqMultiMap/FrozenSeqMultiMap spellings in earlier unmerged patches are not
substituted for the current main decision. Whether selected multimap names
receive separate frozen partners is a future API design decision, not resolved
by inserting prefixes in this corpus.

## Operation spellings in this refinement

- Sets retain union, intersection, difference, symmetric_difference.
- Maps distinguish collision-only merge_with, all-key align_with, joins, and
  the recommended difference_by_key/intersection_by_key names. These are
  proposed names; difference_keys/intersection_keys are historical alternatives.
- Bags distinguish sum, max_union, intersection, difference, absolute_difference.
  Historical bag symmetric_difference means absolute difference, not XOR laws.
- BiMaps distinguish pair intersection/difference from left/right restrictions,
  and checked try_union from eviction/replacement.
- Sequence concat is not set union; fiber concat is not relation pair union.

No universal Algebra<Collection> interface is implied. A public method needs a
signature and observation contract, not merely the same English verb elsewhere.

## Documentation authority

Maintained implementations, admitted naming decisions and new proposals have
different authority. Preserve raw inputs and historical evidence verbatim, and
label incompatible names through provenance rather than global search/replace.
Update current summaries and the registry together; a later proposal cannot
silently claim to change a published type or the CRATES standard. This review
provides targeted reconciliation, not a certification of every historical claim.

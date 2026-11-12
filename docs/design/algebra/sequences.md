---
id: FROST-ALG-SEQUENCES
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Sequence algebra (logical lists)

The carrier Seq(V)=V* contains finite ordered occurrences, including duplicates.
“List” is a mathematical synonym here, not a promise of linked-list storage.
No Eq/Ord/Hash/Clone bound is required merely to concatenate or borrow a sequence.

## Operations and laws

[SEQ-MONOID] Concatenation ++ with empty word ε is a free monoid:
(s++t)++u=s++(t++u), ε++s=s=s++ε. It is generally neither commutative nor
idempotent. Position and occurrence order are part of the denotation.

| Operation | Denotation / output |
| --- | --- |
| concat(s,t) | All s occurrences followed by all t occurrences |
| map(f,s) | f(s[0]), ..., f(s[n-1]); output domain may differ |
| filter(p,s) | Original occurrences satisfying p, in original order |
| flat_map(f,s) | f(s[0]) ++ ... ++ f(s[n-1]) |
| zip(s,t) | Pair positions 0..min(len(s),len(t)); no synthetic padding |
| zip_exact(s,t) | Proposed checked variant, error when lengths differ |
| slice(s,i,j) | Positions i through j-1 under stated valid bounds |
| reverse(s) | Occurrences in reverse positional order |
| stable_merge(s,t) | Merge already key-sorted streams; left first on ties |

For total pure f and p, map and filter preserve concatenation. map(g,map(f,s))
=map(g∘f,s) denotationally. reverse(s++t)=reverse(t)++reverse(s).
Stable merge is not concatenation or deduplication: all equal occurrences remain.
Tie order, comparator stability and sorted-input preconditions are explicit.

zip is not concatenation-homomorphic without alignment conditions. For
s=[a], t=[b], u=[], v=[c], zip(s++t,u++v)=[(a,c)], whereas
zip(s,u)++zip(t,v)=[(b,c)]. Reassociating nested zip needs explicit tuple
reassociation, and its length is the minimum of all participating lengths.

## Ambiguous operations are not silently standardized

“List intersection” could mean retaining every left occurrence whose key occurs
on the right, multiplicity-minimum bag intersection ordered by left occurrence,
a longest common subsequence, or a set projection. These differ. For [a,a]
and [a], a membership filter retains two a occurrences while bag intersection
has multiplicity one. Name the chosen operation and its output carrier.

Similarly, list difference can mean occurrence subtraction, key filtering,
position deletion, or an edit script. An edit script is an ordered program over
a particular base version; concatenating scripts is not set union, and edits
may conflict or use rebased positions. Rope/persistent-vector layouts are
physical choices, not new denotational algebras.

## Pure equations versus execution effects

[SEQ-EFFECT] Lazy short-circuit traversal specifies callback order and frequency
for actually visited occurrences. Mapping then filtering can call f on elements
that a pushed-down filter would skip. A rewrite that preserves the final word
but suppresses errors, reorders effects, repeats callbacks, or changes exposed
borrowing must satisfy the chosen observational profile.

An owned materializer can move, clone, decode, or receive newly produced values;
a borrowed materializer can retain references. Neither concatenation nor freezing
magically creates owned non-Clone elements from shared references. Partial
consumption must release any owned unvisited suffix exactly as documented.

## Conversions

The occurrence-count projection count:Seq(K)→Bag(K) preserves ++ as additive
sum. support∘count maps ++ to set union, forgetting order and occurrence identity.
Neither conversion is invertible. Mapping sequence keys through a noninjective
function preserves positions but subsequent deduplication changes multiplicity.

## Current Frost and acceptance

Keep MutableVec/FrozenVec and InlineVec/InlineFrozenVec. Existing native
VecDeque/LinkedList backends retain their native capability contracts. Do not
introduce FrozenList or rename SeqMultiMap to ListMultiMap. SequenceView is a
prospective expression name, not an implemented container.

Tests: finite concat/map/filter/reverse laws; zip misalignment counterexamples;
duplicate occurrences; empty and length-boundary cases; early exit; non-Clone
payloads; drops under panic; and source-order versus sorted-merge distinctions.
A finite model of occurrences does not establish Rust pointer or drop safety.

---
id: FROST-ALG-ADVERSARIAL
status: reviewed-proposal
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Adversarial review: value domains and collection algebras

This is the delivery's own review. The
[destination review](value-domain-algebra-destination-review.md) records an
independent review at integration; where the two differ, that one is current.

Reviewed against `27b14399b0245df2a018fd5eb6f24f47ce7c2dbe` on 2026-11-12.
**Review method: four sequential passes by the same author/model, with executable
finite counterexample searches and deliberately incorrect replacements. No
independent model, external reviewer, Lean solver, Kani verifier, or Rust suite
was run.** Independence is not inferred from giving the passes different names.

## Passes and dispositions

1. **Mathematical model:** attack quotient congruence, closure, conversion laws,
   partiality, inverse identities, fixed points and machine bounds.
2. **Observable semantics:** attack representatives, multiplicity, occurrence
   order, deletion callbacks, staged failure, borrowing and incomplete prefixes.
3. **Rust and repository compatibility:** compare current capabilities, IntKey,
   frozen lifecycle, naming decisions, CRATES ownership and old patch assumptions.
4. **Assurance and packaging:** run bounded models and mutant sensitivity; verify
   exact touched-file baseline, links/metadata/names, patch application and no
   production/build changes. Actual packaging results are in the validation record.

The findings below identify traps in the design space and conflicts in session
artifacts/current prose; they are **not 29 demonstrated production Rust bugs**.
Each listed issue is addressed in the proposed specification or explicitly kept
outside this documentation patch. Remaining implementation assurance is open.

| ID | Severity | Attack / conflict | Disposition | Evidence |
| --- | --- | --- | --- | --- |
| R01 | high | Key identity was being treated as whole-value equality | Define quotient keys separately from stored representatives; current IntKey still reconstructs the full value. | CX-DERIVED-IDENTITY |
| R02 | high | A validator or idempotent normalizer could be mistaken for a permanent invariant | Separate admission soundness, normalization identity preservation and lifetime stability. | CX-CANONICAL-IDEMPOTENCE |
| R03 | high | Projection through intersection/difference could invent or erase membership | Require key injectivity for these direct-image rewrites; give necessity/sufficiency argument. | CX-COLLAPSED-INTERSECTION; CX-COLLAPSED-DIFFERENCE |
| R04 | high | Metadata-sensitive filters were being treated as quotient functions | Require congruence or explicitly enriched representative-sensitive semantics. | CX-REP-FILTER |
| R05 | high | Valid inputs were assumed to imply collection closure | Add explicit subcarrier restrictions and operation preconditions. | CX-NONEMPTY-CLOSURE; CX-CARDINALITY-CAP |
| R06 | high | Set XOR membership association could discard object provenance | Preserve representative plan or reject the rewrite for reference-producing terminals. | CX-SET-XOR-REP |
| R07 | high | A closed map resolver could be assumed associative | Check lifted operation including absence, deletion and definedness; separate align from collision merge. | CX-MAP-DELETE-ASSOC; CX-MAP-SUBTRACT-ASSOC |
| R08 | medium | Join association could be presented as literal type equality | Require explicit product/tagged output isomorphisms and retained key policy. | CX-JOIN-TUPLE-TYPE |
| R09 | high | Stored None could be conflated with absent binding | Use independent presence/tagged alignment. | CX-MAP-PRESENT-NONE |
| R10 | high | Left key plus right value could break a dependent invariant | Require representative/payload-compatible resolver or checked failure. | CX-MAP-REP-PAYLOAD |
| R11 | high | Bag absolute difference could inherit XOR rules | Name absolute_difference and document nonassociativity. | CX-BAG-ABS-ASSOC |
| R12 | medium | Support or key coalescing could preserve too much bag algebra | Limit homomorphisms; counts combine by sum under pushforward. | CX-BAG-SUPPORT-MONUS; CX-BAG-PUSHFORWARD-MAX; CX-BAG-PUSHFORWARD-MIN |
| R13 | high | Finite keys could be claimed sufficient for recursive convergence | Finite-height proof only for positive Boolean facts; unbounded natural multiplicity needs another rule. | arithmetic x != 1+x; bounded recursion group |
| R14 | medium | List intersection and zip/concat could have implicit set semantics | Specify occurrence meaning; show differing list interpretations and zip misalignment. | CX-SEQ-INTERSECTION; CX-ZIP-CONCAT |
| R15 | high | Set/bag/sequence multimap fibers could be conflated | Separate carriers, counts, witness multiplicity, empty-fiber normalization and ordering. | CX-REL-WITNESSES; CX-MM-PRESENT-EMPTY; CX-MM-FLATTEN |
| R16 | medium | Grouped output could promise an original tuple reference that never existed | Allow independently borrowed key/value fields; distinguish flat pair representatives. | CX-MM-GROUP-REP |
| R17 | high | BiMap union/XOR could be treated as closed relation operations | Check both uniqueness projections, or return a relation. | CX-BIMAP-UNION-CLOSURE; CX-BIMAP-RIGHT-UNIQUENESS |
| R18 | high | Checked XOR could be reassociated when its final pair set is lawful | Preserve intermediate definedness; checked union and checked XOR have different partial laws. | CX-BIMAP-XOR-DEFINEDNESS |
| R19 | medium | Pair difference could be confused with deleting matching keys | Keep pair and endpoint-restriction operations distinct. | CX-BIMAP-PAIR-KEY-DIFF |
| R20 | high | Missing deny data could be interpreted as an exact negative | Use lower/upper knowledge bounds and explicit completeness/error status. | CX-INCOMPLETE-DENY |
| R21 | high | Base/delta algebra could ignore event order | Replay ordered events or exact compaction; union-inserts-minus-deletes is not general replay. | CX-DELTA-ORDER |
| R22 | high | An empty shortcut could remove mandatory compatibility errors | Validate required domain/context compatibility before such simplification. | CX-EMPTY-DOMAIN-SHORTCUT |
| R23 | high | Frozen could imply deep or revision stability | Model structural phase separately from external heap state, freshness and engine consistency. | CX-FROZEN-DEEP-STABILITY |
| R24 | medium | Materialized could imply lifetime-independent payload ownership | Correct root README; distinguish borrowed, shared and independently owned elements. | CX-OWNED-VS-BORROWED |
| R25 | high | Old archives could overwrite names and rules selected on newer main | Retain SeqMultiMap/SetMultiMap/BagMultiMap and CRATES; preserve current AGENTS clauses behind the requested symlink. | exact-main byte checks; terminology catalog |
| R26 | medium | An expression crate could be rejected because inherent methods cannot move | Existing methods stay with their types; a new view owner is a separate responsibility. | Rust coherence source; rust-integration.md |
| R27 | high | Auto construction could be mistaken for Auto algebra normalization | Document current eligible empty Chunked sharing without editing the stale fuzz oracle or production code. | Confirmed at destination: `src/integer/storage.rs` can return an empty Chunked operand under an Auto output policy, while `verification/fuzz/src/lib.rs` asserts that empty non-forced outputs are Sorted |
| R29 | high | Normalization preservation used output equivalence on inadmissible raw inputs | Define raw meaning and output meaning separately; type-check any normalization reapplication. | raw/output normalization example; domain proof obligation |
| R28 | medium | Successful finite models could be presented as production proof or independent review | Record bounds, same-author passes, no Rust imports, unavailable gates and no measurements. | execution metadata; model limitations |

## Important post-draft corrections

The review added explicit categories for preconditions: statically enforced,
validated at an import/operation boundary, or trusted semantic obligations.
A checked-admission function cannot return success after its required check
fails, but no safe trait requires magically deciding arbitrary callback laws.
The BiMap union proof was corrected to say that **adding** pairs cannot repair
an existing uniqueness conflict. The no-top bag statement now excludes the
trivial empty-key domain. Runtime dictionaries were explicitly excluded from
current IntKey implementations; they belong in source-level bindings/adapters.

## Executed evidence and limits

As delivered, the finite models (a Node script, since replaced by Rust tests)
executed 258,624 assertions over 99,118 counted cases and recorded 33
counterexamples. The destination review found that statement overstated: the
"six deliberate incorrect variants" compared constants, nine counterexamples
were pairs of literals, and the relation-matrix cross-check named here did not
exist. The revised model and its numbers are in the
[validation record](../research/formal-models/validation.md#destination-results-2026-11-12).
All model code is same-author work and is not an independently developed oracle.

Missing Rust compilation, trait-coherence prototypes, Miri/ownership, Loom,
Kani/solver proof, and production benchmarks remain required future gates.
Review of Eq/Ord/Hash or source consistency is only as strong as the declared
adapter assumptions. This task does not fix the separately observed CI failures.

## Residual design decisions

- Choose concrete API signatures and ownership recovery for map/bag/BiMap families.
- Specify frozen partners for selected multimap names without renaming main's choices.
- Decide public domain-binding and fallible cursor protocols from a working consumer.
- Prove/measure optimized rewrites under explicit observation profiles; do not
  infer soundness from a trait named Associative or from a passing small model.
- Run independent maintainer or model review before treating the proposal as accepted.
- Apply to a complete checkout of the pinned base and run the actual required gates.

[Formal index](../design/algebra/README.md) ·
[Proof ledger](../design/algebra/proof-obligations.md) ·
[Validation](../research/formal-models/validation.md)

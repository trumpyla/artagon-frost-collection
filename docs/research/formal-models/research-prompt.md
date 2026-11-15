---
id: FROST-RESEARCH-FORMAL-PROMPT
status: research-template
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Focused formal-model research prompt

Use this with the existing [master research prompt](../templates/research-prompt.md)
and [deep-dive report template](../templates/deep-dive-report.md). For a broader
survey start with [landscape](../templates/landscape-report.md); hand off one
question using [subtopic](../templates/subtopic-handoff.md). Do not assume this
conversation or unmounted artifacts are available to the receiving model.

## Copyable brief

```text
Repository: trumpyla/artagon-frost-collection
Baseline: resolve current main; do not assume the recorded baseline is current.
Parent: docs/design/algebra/README.md and docs/research/formal-models/README.md
Question: choose one precisely stated child question below.

Inspect AGENTS.md, CRATES.md, relevant code, and the naming dispositions first.
Build on the formal-model/domain chapters. Public API compatibility is not
required (docs/design/decisions.md, 2026-11-12), but semantic and safety
contracts are, and no new package is created without a separate accepted
decision. Argue any new domain parameter on its design merits. Do not implement code unless the task explicitly asks for it.

For the selected question, define carrier, validity, equivalence, representation
invariant, input/output domains, preconditions and closure/partiality. Separate
logical results from representatives, order, multiplicity, effects, errors,
ownership, context and completeness. Classify each precondition by enforcement.
State typed conversion/normalization laws, not untyped idempotence slogans.

Give a proof argument and independently understandable reference model. Search
for minimal counterexamples before admitting rewrites, including partial-operation
definedness. Run bounded cases and deliberate incorrect variants when feasible.
State bounds and exactly what production code, if any, the verifier executes.
Use primary sources for implementation/language claims and preserve unresolved
citations. Distinguish independent reviewers from same-author passes.

Provide a technical report, compatibility map, open decisions, acceptance gates
and a patch against the inspected revision if requested. Update indexes and
metadata. Report only commands actually run; no invented tests, push, citations,
performance or report recovery. Preserve raw inputs and unrelated work.
```

## Seeded child questions

| ID | Refines corpus question | Question | Acceptance criterion |
| --- | --- | --- | --- |
| Q-VDOM-1 | `Q-CSET-1`, `Q-AUTHZ-1` | Minimal source-binding API for semantic domain, codec/dictionary, revision and universe | Compile nonstatic borrowed examples; no runtime dictionary hidden in IntKey; context mismatch and empty-source tests |
| Q-MAPLIFT-1 | `Q-MAP-1` | Resolver, alignment and error/recovery signatures for generic map views | Absence and present-None distinguished; lifted laws and deletion counterexample; real callback/drop tests |
| Q-RELPROV-1 | `Q-MM-1` | Group/pair representatives and derivation witnesses across multimap composition | Preserve selected SeqMultiMap name; witness-count versus Boolean semantics and grouped-reference lifetimes explicit |
| Q-PARTIALBI-1 | `Q-BIMAP-1` | Checked BiMap operations with streaming and transactional returned-error policy | Both endpoint conflicts, intermediate definedness, minimal checked-XOR counterexample and actual ownership tests |
| Q-CSETKNOW-1 | `Q-CSET-1` | Fallible cursor protocol with certified negative evidence and budgets | Separate Pending, End, Incomplete and Failed; interval/epoch model; revisioned pagination and fault injection |
| Q-OPTVALID-1 | `Q-ALG-1` | Law certificates and provenance-preserving plan rewrites | No unsafe reliance on semantic traits; observer-specific rules, property tests and cost evidence |
| Q-PRIORART-1 | `Q-ALG-1` | Reread the sources that the prior-art survey cites from prior knowledge or that could not be fetched | Every `SRC-PRIOR-…` row is integration-checked with section and date, or stays out of the chapters; catalog updated |
| Q-BAGID-1 | `Q-BAG-1` | The list of bag-difference rewrites a planner may use, beyond `BAG-MONUS` | Proof and counterexample per rule; overflow behaviour under `COUNT-PARTIAL`; checks added to the finite model |
| Q-STABLE-1 | `Q-MM-1`, `Q-CSET-1` | Weight algebras admitted for recursive composition and the result protocol of a checked closure | Table of algebra, stability and closure outcome; decision among Boolean, capped and checked natural weights |
| Q-CTXKEY-1 | `Q-CSET-1`, `Q-AUTHZ-1` | Components of the evaluation context that a memo, plan or cursor key must hold | Dependency footprint per terminal; fault model covering the cited cache defects; a test that a missing component is detected |
| Q-MECH-1 | `Q-ALG-1` | Verification route for the first implemented carrier | Comparison of a proved model with differential testing, functional translation and a Rust verifier on one carrier, with tool versions |

The last five rows were seeded by the
[prior-art report](../reports/value-domain-algebra-prior-art.md), which gives
their dependencies and reasons.

These IDs are local to this supplement. They are not registered in the corpus
[manifest](../manifest.json), whose questions in the
[research synthesis](../synthesis/foundations-and-engine-roadmap.md#8-prioritized-research-questions)
remain the dependency-ordered list. Each seeded question narrows the corpus
question named beside it and does not replace it.

A child report must return its findings to the parent source map and law ledger.
A report is not acceptance, an accepted contract is not implementation, and a
passing finite model is not a production proof or benchmark.

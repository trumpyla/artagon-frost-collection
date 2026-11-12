---
id: frost-research-report-value-domain-algebra-prior-art
kind: report
level: landscape
research_status: curated
implementation_status: proposed
source_validation: partial
input_files: [docs/reviews/supplied/value-domain-algebra-prior-art-survey.md, docs/reviews/supplied/value-domain-algebra-prior-art-survey-sources.txt]
input_members: []
input_sha256: [e96cc829088ddb57e3ad1b00c2d8d4ff270a6d9482fffcf3bcf32132c6a17d59, 02ce8d7e5ce80b13752d661ded9c15befcb6006f24dc48254fcb19c0640ca45c]
origin_created_at: 2026-11-12
imported_at: 2026-11-12
repo_baseline: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
parent_ids: [frost-research-synthesis-foundations-and-engine-roadmap]
related_ids: [frost-research-report-collection-algebra-survey, frost-research-report-computed-sets-summary, frost-research-source-catalog]
supersedes: []
open_questions: [Q-ALG-1, Q-BAG-1, Q-MM-1, Q-BIMAP-1, Q-CSET-1, Q-AUTHZ-1]
---

# Prior art for the value-domain and collection algebra (curated survey)

**What this is.** A curated derivative of a prior-art survey that a research
tool produced on 2026-11-12 for the proposed
[value-domain and collection algebra](../../design/algebra/README.md). The
survey's text is kept unchanged as
[`value-domain-algebra-prior-art-survey.md`](../../reviews/supplied/value-domain-algebra-prior-art-survey.md),
with the list of pages it consulted beside it. This document follows the
[landscape template](../templates/landscape-report.md), replaces the survey's
inline links with catalog ids from [SOURCES.md](../SOURCES.md), records what
was reread at integration, and corrects the statements that the rereading did
not support.

**What is implemented today.** Nothing in this report is implemented. The
algebra chapters are proposed contracts. Frost has no bag, multimap, BiMap,
map-merge, computed-set or weighted-relation type. The implemented algebra is
the set algebra of the ordered, hash, bitmap and integer families described in
[set operations](../../set-operations.md).

**Check levels.** A claim is *integration-checked* when the integrator read
the cited passage on 2026-11-12, *survey-read* when only the survey read it,
and *unread* when the survey cites it from prior knowledge. The catalog
section [Prior-art survey sources](../SOURCES.md#prior-art-survey-sources-2026-11-12)
defines the levels and holds every `SRC-PRIOR-…` id used below.

## Executive synthesis

**Decision.** How should the algebra chapters use established theory: which
names to align with, which laws to tighten, and which citations to add?

**Recommendation.** Keep every selected Frost name. Record the established
term beside it as an alias. Add four laws that the literature states more
sharply than the chapters did, each with its own finite check:

1. `BAG-MONUS`: which set-difference identities survive for bag difference,
   which fail, and why admission stays one identity at a time.
2. `BIMAP-COMPAT`: checked BiMap union is the compatible join of partial
   bijections; n-ary union needs pairwise compatibility.
3. `CLOSURE-STABLE`: recursive weighted composition is admitted only for a
   stable weight algebra, an acyclic input, or a checked bound.
4. `CTX-KEY`: a cached or resumed result is keyed by every part of the
   evaluation context it depends on.

**Strongest evidence.** Fifteen pages were reread at integration. Four
sources carry the four laws: Lawson's primer for compatibility
(`SRC-PRIOR-LAWSON-PRIMER`), Suciu for monus identities
(`SRC-PRIOR-SUCIU-DIFFERENCES`), Abo Khamis et al. for stability
(`SRC-PRIOR-DATALOGO`), and one SpiceDB advisory for context in cache keys
(`SRC-PRIOR-SPICEDB-CVE-2026-55866`). DBSP (`SRC-PRIOR-DBSP`) supports the
separate signed-weight carrier. The
[supplementary checks](../formal-models/evidence/prior-art-checks.mjs) test
each law on small domains.

**Result against the chapters.** The rereading found **no stated law of the
chapters to be false**. The survey's summary said that three laws were false
or needed stronger preconditions. Two of those were already qualified in the
chapters (bag difference is not an inverse; bag fixed points have no
termination argument), and the third rests on a misreading that
[the corrections](#survey-statements-corrected-at-integration) record.

**Key uncertainty.** About half of the survey's rows are unread. They cover
the origin of the refinement vocabulary, the free-algebra view of collections
beyond one monograph, validated-value libraries, frozen lifecycles in other
ecosystems, and law-testing practice. No design statement rests on them.

**Next step.** `Q-PRIORART-1` rereads the unread rows that a chapter would
want to cite. `Q-BAGID-1` and `Q-STABLE-1` turn two of the new laws into
complete rule tables.

## Context and current implementation

Repository revision: branch `docs/value-domain-algebra` (named
`claude/frost-domain-algebra-w598vp` until 2026-11-12) at
`89a80b69269b93b96bb6cf6d2f30956074ab50bf`, based on main at
`27b14399b0245df2a018fd5eb6f24f47ce7c2dbe`. Inspected: every file under
`docs/design/algebra`, the
[formal-model supplement](../formal-models/README.md), both reviews of the
delivery, and the research rules. No Rust source was read for this report, and
none is changed.

The requirement was to find similar implementations and research papers for
the constructs the chapters define, then to bring the result into the
repository in the research format and update the design documents.

Unavailable evidence: the pages listed as survey-read or unread in the
catalog; the vendor's own text of the SpiceDB advisory; the journal versions
of two papers read as preprints.

| Implemented mechanism | Target architecture named by the chapters |
| --- | --- |
| Left-representative set algebra on ordered and hash families; eager `FrozenIntSet` algebra | Typed views (`SetView`, `MapView`) and computed sets with an evaluation context |
| `Frozen<B>` and named frozen partners | Frozen as a lifecycle refinement of every carrier |
| `IntKey` full-value codec | Source-level dictionary and semantic bindings |
| No bag, multimap, BiMap, map merge or relation composition | The carriers and laws in the algebra chapters |
| One finite JavaScript model with a mutant run | Rust law suites and a reference model under `CRATES.md` |

## Scope and non-goals

Audience: the maintainer deciding which contracts to accept, and an
implementer who needs the standard name of a construct to search the
literature.

In scope: the sixteen constructs of the survey, the terms the chapters coin,
and the laws for which a published result is sharper than the chapter.

Out of scope: accepting any contract; renaming a selected type or operation;
choosing a proof assistant; benchmarks; any claim about how another system is
implemented beyond its cited documentation. The report does not survey prefix
indexes or n-ary cost models, which `Q-PREFIX-1` and `Q-NARY-1` own.

## Taxonomy and formal vocabulary

The chapters define each domain before its operations; see the
[formal model](../../design/algebra/formal-model.md). This section maps the
chapters' terms to the established ones. The Frost term stays the name in
this repository.

| Frost term | Established term | Where it is used | Check level |
| --- | --- | --- | --- |
| Representation invariant `I_R` | Well-formedness predicate (`Raw.WF`) | Lean standard library maps | integration-checked (`SRC-PRIOR-LEAN-MAPS`) |
| Denotation `α_R` | Abstraction function; view; refinement relation | Data refinement; Verus; Isabelle collections | survey-read or unread |
| Key domain `(D,~)` | Lawful equality and hashing instances (`EquivBEq`, `LawfulHashable`) | Lean | integration-checked |
| Left representative | Left-biased union | Haskell `containers` | unread |
| Seq, Bag, Set carriers | Free monoid, free commutative monoid, and its idempotent quotient | Bird's lectures | integration-checked (`SRC-PRIOR-BIRD-PRG69`) |
| Bag `difference` | Monus, truncated subtraction, in an m-semiring | Geerts and Poggi; Suciu | integration-checked |
| Signed-weight carrier `Z^(K)` | Z-set | DBSP | integration-checked (`SRC-PRIOR-DBSP`) |
| BagMultiMap, weighted composition | N-relation; K-relation over a semiring | Provenance semirings | unread (`SRC-PRIOR-GREEN-2007`) |
| Terminating weighted closure | Stable, p-stable semiring | Datalog over semirings | integration-checked (`SRC-PRIOR-DATALOGO`) |
| `compatible(R,S)`, `try_union` | Compatibility relation; compatible join | Inverse semigroups | integration-checked (`SRC-PRIOR-LAWSON-PRIMER`) |
| Union with disjoint endpoints | Orthogonal join | Inverse semigroups | integration-checked |
| `R⁻¹∘R`, local identity | Partial identity `1_A`; idempotent | Inverse semigroups | integration-checked |
| `align_with` tags | `These`, `Ior`, `EitherOrBoth` | Haskell, Scala, Rust libraries | survey-read or unread |
| Per-side merge behaviour | Merge tactics | Haskell `containers` | survey-read (`SRC-PRIOR-CONTAINERS-MERGE`) |
| Knowledge bounds `[L,U]` | Certain answers for `L`; possible answers for `U` | Incomplete databases | integration-checked for `L` and its criterion (`SRC-PRIOR-LIBKIN-2016`); the term for `U` is from an unread source |
| Per-key state under `[L,U]` | Strong three-valued (Kleene) connectives | Logic | derived and checked here; no source read |
| Snapshot in `θ` | Zookie, consistency token | Zanzibar | existing catalog row `SRC-ZANZIBAR-2019` |
| Computed set expression | Userset rewrite | Zanzibar | existing catalog row |

Observable semantics of the four added laws:

| Law | Equality relation | Representative rule | Order | Multiplicity | Failure or overflow | Callback effects |
| --- | --- | --- | --- | --- | --- | --- |
| `BAG-MONUS` | Key classes | Unchanged left-present policy; the law is about counts only | Not observed | Natural counts | Sum sides are subject to `COUNT-PARTIAL`; a rewrite can add or remove an overflow | Not covered; each rewrite still needs its effect profile |
| `BIMAP-COMPAT` | Key classes on both endpoints | Left pair on an equal logical pair, as in `BIMAP-PARTIAL` | Not observed | One pair per endpoint | Conflict when some pair of operands is incompatible; which conflict is reported is not fixed | Not covered |
| `CLOSURE-STABLE` | Key classes of relation endpoints | Witness selection stays a separate policy | Not observed | Weights in the named algebra | Divergence over `N` on a cycle; `Overflow` under a checked bound | Not covered |
| `CTX-KEY` | Equality of context components | Not applicable | Not applicable | Not applicable | A reused result carries its completion status and must not upgrade it | A cached result suppresses re-evaluation, so the profile must permit that |

## Evidence and ecosystem map

One row per construct of the survey. "Disposition" says what this integration
did. Capability claims about a library are limited to the cited page and
version.

| # | Construct | Closest prior art and check level | What matches | What differs | Disposition |
| --- | --- | --- | --- | --- | --- |
| 1 | Invariant, denotation, observational refinement | Lean maps (`SRC-PRIOR-LEAN-MAPS`, integration-checked); Isabelle Collections Framework (`SRC-PRIOR-ICF-2010`, survey-read); Verus `vstd` (survey-read); Hoare 1972 and data refinement (unread) | A raw structure, a well-formedness predicate, and one preservation lemma per operation; verification against a list model | Lean proves the lemmas; Frost's ledger holds arguments and finite checks. No source read here models allowed-trace sets | Alias recorded in the formal model. The lemma-per-operation shape is recommended for a future law suite |
| 2 | Collections as free algebras | Bird's lectures (`SRC-PRIOR-BIRD-PRG69`, integration-checked); monoid comprehensions (survey-read); the tree level and other libraries (unread) | Lists, bags and sets differ by commutativity and idempotence of one operator; an operation that is not a homomorphism is not defined on the quotient | No keys with representatives, no maps, no partial operations | Cited in the sequence and bag chapters as the reason some sequence operations have no bag or set form |
| 3 | Key domains and representatives | Lean key laws (integration-checked); Verus borrowed-key issue (`SRC-PRIOR-VERUS-3176`, survey-read); Java, Haskell, Rust library conventions (unread) | Equality and hashing laws are stated obligations; a non-reflexive equality breaks retrieval | None of the read sources states laws about *which* equal element an operation returns | The domain chapter names the Lean failure beside the NaN example. The survey found no prior statement of the representative-level counterexamples; that is an absence in one survey, not a novelty claim |
| 4 | Validated value domains | All unread | — | — | No change. Candidates for `Q-PRIORART-1` |
| 5 | Image and preimage of key conversions | Standard set theory; no source read | The adjunction `f(A) ⊆ B` iff `A ⊆ f⁻¹(B)` restates `DOM-IMAGE` | — | The adjunction is stated in the domain chapter as a derived fact, with no citation |
| 6 | Bag algebra | Geerts and Poggi; Suciu (both integration-checked); Python `Counter` (`SRC-PRIOR-PY-COUNTER`, integration-checked); Albert 1991 and bag query languages (unread) | Bag difference is monus in the m-semiring `N`. `Counter` implements sum, monus, min, max and, since 3.15, absolute difference | `Counter` keeps negative counts through `subtract()` in the same type; Frost separates `Bag(K)` from `Z^(K)` | New law `BAG-MONUS`. `Counter` cited as the contrast for the separate signed carrier |
| 7 | Relations, weighted composition, recursion | Datalog over semirings (`SRC-PRIOR-DATALOGO`); DBSP (`SRC-PRIOR-DBSP`), both integration-checked; provenance semirings, differential dataflow, stratification (unread) | Composition as a semiring matrix product; convergence of recursion tied to stability; signed weights as a group that contains bags | The read sources fix one weight algebra per program; Frost's chapters name only Boolean and natural weights | New law `CLOSURE-STABLE`. Z-set recorded as the established name of `Z^(K)` |
| 8 | Multimaps | Guava `Multimap` (`SRC-PRIOR-GUAVA-MULTIMAP`, integration-checked) | A key is present only with at least one value; removing the last value removes the key | Guava compares multimaps of different fiber kinds and treats all empty ones as equal; Frost makes the fiber part of the type's meaning | Cited under `MM-SUPPORT`. The cross-kind equality rule is recorded as not adopted |
| 9 | BiMaps as partial bijections | Lawson's primer (integration-checked); Guava `BiMap` (`SRC-PRIOR-GUAVA-BIMAP`) and `bimap` 0.6.3 (`SRC-BIMAP-063`), both integration-checked | The compatibility relation, partial identities, orthogonal joins, and the embedding theorem. Libraries expose reject, evict, and partially applied bulk insertion as different operations | The primer defines compatibility by idempotent products; the chapter defines it by a functional union. They agree on partial bijections | New law `BIMAP-COMPAT`. Library behaviours cited as the contrast for checked union |
| 10 | Map merge and alignment | Haskell `containers` merge tactics and `semialign` (both survey-read); other libraries (unread) | Per-side tactics; alignment that is commutative and associative up to an explicit isomorphism | — | Names recorded as aliases in the map chapter with their check level. No law changed |
| 11 | Sequences | Bird's lectures (integration-checked); `semialign` non-law (survey-read); stream fusion and effect order (unread) | The free monoid; `zip` does not distribute | — | Citation added to `SEQ-MONOID`. No law changed |
| 12 | Frozen lifecycle | All unread | — | — | No change. Candidates for `Q-PRIORART-1` |
| 13 | Computed sets with incomplete knowledge | Libkin (`SRC-PRIOR-LIBKIN-2016`); SpiceDB advisories (`SRC-PRIOR-SPICEDB-CVE-2026-55866`, `SRC-PRIOR-SPICEDB-ADVISORIES`); Cedar (`SRC-PRIOR-CEDAR-2024`), all integration-checked; Zanzibar and SpiceDB schema (existing rows); incomplete-database origins (unread) | A lower bound that never contains a non-member is the "no false positives" criterion. Reported defects sit at intersection, exclusion, caveats and cache reuse | The read sources give no interval transfer rules; those stay Frost derivations | New law `CTX-KEY`. `KNOW-BOUNDS` gains the three-valued reading, checked here |
| 14 | Rewrite admission | Suciu, Theorem 21 (integration-checked); bag-query equivalence and provers (survey-read) | No finite axiom set yields every identity of natural-number monus arithmetic | The survey-read results concern relational queries, not collection operators | The rewrite chapter states that admission is a list of proved rules, citing only the integration-checked theorem |
| 15 | Integer set representations | Roaring papers and adaptive intersection (survey-read; existing unread rows) | Container choice is representation-only; adaptive merge is an execution choice | — | No change; the integer profile's own documents already cover the locked `roaring` source |
| 16 | Law-testing practice | Cedar's differential testing (integration-checked); law-suite libraries (unread) | A proved model used as an oracle for the Rust implementation | Cedar's model is in Lean; Frost's is a finite JavaScript model that executes no Rust | Recorded as an option under `Q-MECH-1` |

### Survey statements corrected at integration

| ID | Survey statement | What the reread source says | Effect here |
| --- | --- | --- | --- |
| C1 | Bag difference fails axiom (A5) | (A5) holds in `(N,+,·,0,1,∸)` and in `N[X]`. It fails in other m-semirings (`SRC-PRIOR-SUCIU-DIFFERENCES`, section 4.3) | `BAG-MONUS` states that (A5) holds for natural-number bags. `CX-MSEMIRING-A5` computes a failure in a three-element chain, so a future weight parameter must name its algebra |
| C2 | Three chapter laws are false or under-qualified | No chapter law is contradicted. The chapters already deny that difference is an inverse and that bag fixed points terminate | The laws are sharpened, not reversed |
| C3 | The primer states "compatible iff the union is a partial bijection" | The forward direction is stated; the converse is not a standalone statement in the part read | The chapter proves both directions; the check covers all 34 partial bijections on three keys |
| C4 | Multiplication distributes over compatible joins (attributed to inverse-semigroup theory) | Not found in the primer | Distribution of composition over a compatible union is derived from `REL-COMP` and `BIMAP-CLOSED` and checked; no citation |
| C5 | The `bimap` documentation calls hash mutation through `Cell` or `RefCell` a logic error | Not on the page (`SRC-BIMAP-063`) | Not cited. `FROZEN-STABLE` keeps its own argument |
| C6 | The SpiceDB defect affects 1.34.0 to 1.53.x and is classified CWE-863 | The tracker page gives 1.34.0 through 1.54.0 with the fix in 1.54.0; no CWE on either page read | Version range reported as the pages give it; no CWE stated |
| C7 | "Verification-guided development" attributed to the Cedar paper | The phrase is not in the paper read | The practice is described without the phrase |
| C8 | DBSP: bags are Z-sets "where all weights are positive" | Definition 4.2 names the property positive and defines it as no negative weight | Stated as nonnegative |
| C9 | Bird's PRG-69 covers trees, lists, bags and sets | The part read covers lists, bags and sets | The tree level is left to the unread Meertens citation |

Survey recommendations **not adopted**, with reasons:

| Recommendation | Reason |
| --- | --- |
| Rename checked union to "compatible join" and `[L,U]` to "certain/possible answers" | A research import never renames. The terms are recorded as aliases in [terminology](../../design/algebra/terminology.md) |
| State that empty set-fibered and list-fibered multimaps are equal | The fiber is part of a Frost multimap's meaning, and the families are distinct types. Guava's rule is a property of one Java interface |
| Make checked union all-or-nothing by law | The BiMap chapter already requires staging before publication and a stated recovery policy. Whether every checked operation is transactional is `Q-PARTIALBI-1` |
| Restrict weighted closure to Boolean weights | `CLOSURE-STABLE` names three admissible cases instead; the choice among them is `Q-STABLE-1` |

## Candidate architectures and tradeoffs

The decision is how far the chapters lean on established theory.

| Option | Description | For | Against |
| --- | --- | --- | --- |
| Status quo | Keep the chapters self-contained; cite only Rust documentation | No dependence on sources that may be misread | A reader cannot find the literature; three precise criteria stay informal |
| Conservative extension (**recommended, applied**) | Keep names; add aliases, citations with check levels, and the four laws with finite checks | Each added statement is either proved in the chapter or tied to a reread theorem; nothing is renamed | The ledger grows; one more evidence script to maintain |
| Specialization | Parameterize bags and relations by a weight semiring (K-relations) and state laws per algebra | One statement covers sets, bags, tropical costs and provenance | A new type parameter on families that have no second weight algebra today; `CX-MSEMIRING-A5` shows laws do not transfer between algebras. Needs `Q-STABLE-1` first |
| Mechanize now | Port the model to Lean, or verify a reference implementation with a Rust verifier | Proofs replace bounded checks | No carrier is implemented yet; the choice of tool is `Q-MECH-1` |
| Rename to established terms | Use "compatible join", "monus", "certain answers" as primary names | Shorter path from the literature | Reverses recorded naming dispositions without a decision record; rejected for this integration |

Decision criteria: a statement enters a chapter only if the chapter proves it
or an integration-checked source states it; a name changes only through a
decision record; a new parameter needs an executable consumer.

## Cross-cutting contracts

- **Representatives.** None of the read sources constrains which equal
  element an operation returns. `SET-REP`, the bag left-present policy and
  the BiMap pair rule remain Frost definitions. The new laws are stated at
  the logical level and do not relax them.
- **Ordering.** Not affected. Stability and compatibility are order-free.
- **Ownership.** Not affected. A cached result under `CTX-KEY` is a prepared
  value in the sense of the rewrite chapter and keeps its ownership contract.
- **Errors and overflow.** `BAG-MONUS` identities with a sum on one side can
  change whether a checked count overflows. `CLOSURE-STABLE` makes `Overflow`
  the expected outcome of a checked natural-weight closure on a cycle; that
  is an error, not a negative answer.
- **Mutable, frozen, persistent, snapshot, materialized.** Not affected. A
  Z-set is a change carrier and is not a frozen bag.
- **Concurrency.** The cited cache defect needs concurrent requests.
  `CTX-KEY` is a keying rule; it says nothing about locking or invalidation.
- **Features and coherence.** No trait, feature or dependency is proposed.
- **Security boundary.** The SpiceDB advisories are evidence about one
  engine's reported defects. They do not describe Zanzibar, and Frost
  implements no authorization engine.

## Research gaps and decision dependencies

1. The unread rows (`SRC-PRIOR-HOARE-1972` through `SRC-PRIOR-UNREAD-OTHER`)
   cannot support a chapter statement. Rereading comes first because later
   questions may want to cite them.
2. `BAG-MONUS` lists identities; it is not a complete rule set, and by
   `SRC-PRIOR-SUCIU-DIFFERENCES` no finite complete set exists. The useful
   set for a planner is undecided. Depends on 1 only for citations.
3. `CLOSURE-STABLE` names three admissible cases without choosing a default,
   and does not say what a checked closure returns besides `Overflow`.
   Depends on 2, because a capped count is a different algebra with its own
   monus identities.
4. The survey-read claim that bag containment of conjunctive queries is not
   known to be decidable was not reread. The rewrite chapter does not rely on
   it.
5. No source read here gives interval transfer rules for computed sets under
   incomplete knowledge. `KNOW-BOUNDS` stays a Frost derivation.
6. Which verification route fits a first implemented carrier is open and
   depends on which carrier is implemented first.

## Prioritized research questions

The ids are local to the formal-model supplement, like its other seeded
questions. Each refines a corpus question and is registered in the
supplement's [research prompt](../formal-models/research-prompt.md), which is
the child brief until a question is launched.

| Question ID | Decision to unlock | Why existing evidence is insufficient | Depends on | Child brief | Acceptance criteria |
| --- | --- | --- | --- | --- | --- |
| `Q-PRIORART-1` | Which further citations a chapter may carry | The survey cites about forty works from prior knowledge | — | [seeded row](../formal-models/research-prompt.md#seeded-child-questions) | Every row is integration-checked with section and date, or stays out of the chapters; the catalog is updated |
| `Q-BAGID-1` (refines `Q-BAG-1`) | The bag rewrite list a planner may use | `BAG-MONUS` checks ten identities and four failures on counts 0..6 only | `Q-PRIORART-1` for citations | seeded row | A named list with proof and counterexample per rule, including overflow behaviour under `COUNT-PARTIAL`; checks added to the finite model |
| `Q-STABLE-1` (refines `Q-MM-1`, `Q-CSET-1`) | The weight algebras admitted for recursive composition, and the result protocol of a checked closure | Only instances of the stability theorem are checked; the capped algebra is checked for two homomorphism laws | `Q-BAGID-1` | seeded row | A table of weight algebra, stability, and closure outcome; a decision on Boolean-only, capped, or checked natural weights |
| `Q-CTXKEY-1` (refines `Q-CSET-1`, `Q-AUTHZ-1`) | The components of `θ` that a memo, plan or cursor key must hold | `CTX-KEY` is checked on a three-outcome toy model | `Q-VDOM-1` | seeded row | A dependency footprint per terminal; a fault model that covers the cited cache defects; tests that a missing component is detected |
| `Q-MECH-1` (refines `Q-ALG-1`) | The verification route for the first implemented carrier | Only one route's paper was reread (`SRC-PRIOR-CEDAR-2024`) | First carrier chosen | seeded row | A comparison of a proved model with differential testing, functional translation, and a Rust verifier on one carrier, with tool versions and what each proves |

## Recommendation and roadmap

Near-term acceptance gates, in order:

1. The maintainer accepts or rejects the four added laws as proposed
   contracts. They are additive; rejecting one removes its catalog entry and
   chapter paragraph only.
2. `Q-PRIORART-1` runs before any chapter cites an unread row.
3. `Q-BAGID-1` and `Q-STABLE-1` run before a bag or weighted-relation family
   is designed.

Deferred: a weight-semiring parameter; mechanization; any rename.

Owners: not assigned.

Conditions that would reverse the recommendation: a reread shows that a cited
theorem was misreported; a counterexample to one of the four laws on a larger
domain; a maintainer decision to adopt established terms as primary names.

This report creates no crate, API or implementation status.

## Claim and evidence ledger

| Claim ID | Claim | Class | Source or derivation | Scope and confidence | Open validation |
| --- | --- | --- | --- | --- | --- |
| PA-01 | Checked BiMap union succeeds exactly when the operands are compatible in the inverse-semigroup sense | derived | Proof in the [BiMap chapter](../../design/algebra/bimaps.md); definition from `SRC-PRIOR-LAWSON-PRIMER` | Exhaustive on three keys; proof is general | None for the logical claim |
| PA-02 | Compatibility is not transitive, so n-ary union needs every pair compatible | derived | Computed counterexample `CX-BIMAP-COMPAT-TRANSITIVE`; the primer states non-transitivity in general | Three keys, all triples | None |
| PA-03 | (A5) holds for natural-number bag counts and fails in another m-semiring | documented-upstream | `SRC-PRIOR-SUCIU-DIFFERENCES`, section 4.3; checked on counts 0..6 and a three-element chain | High for `N`; the chain is this report's construction, not the paper's example | Compare with the paper's own example |
| PA-04 | No finite set of identities yields every identity of natural-number monus arithmetic | documented-upstream | `SRC-PRIOR-SUCIU-DIFFERENCES`, Theorem 21 | As stated by the source; not re-proved | None planned |
| PA-05 | Four set-difference identities fail for bag difference with sum and min | experiment-executed | [Supplementary checks](../formal-models/evidence/prior-art-checks.mjs), group `BAG-MONUS` | Scalar counterexamples; each lifts to one-key bags | Extension in `Q-BAGID-1` |
| PA-06 | Recursive weighted composition converges for every program iff the weight semiring is stable; `N` is not stable | documented-upstream | `SRC-PRIOR-DATALOGO`, Theorem 1.2 | Theorem cited; instances checked on small bounds | The exact semi-naive conditions were not read |
| PA-07 | Capping counts at `M` commutes with sum and product | experiment-executed | Supplementary checks, group `CLOSURE-STABLE` | Values 0..20, caps 1..5 | General proof belongs to `Q-STABLE-1` |
| PA-08 | The `[L,U]` transfer rules are the strong three-valued connectives applied per key | experiment-executed | Supplementary checks, group `KNOW-BOUNDS as three-valued membership` | All intervals on two keys | No source read; attribution of the connectives is standard but uncited |
| PA-09 | A result cached under a key that omits a context component can be returned for a different request | experiment-executed | Supplementary checks, group `CTX-KEY`; incident `SRC-PRIOR-SPICEDB-CVE-2026-55866` | Toy model; the incident is one engine | Vendor advisory text unread |
| PA-10 | Bags are the Z-sets with no negative weight, and changes are Z-sets | documented-upstream | `SRC-PRIOR-DBSP`, section 4.1 | High | None |
| PA-11 | A key present only with a nonempty fiber is an existing library convention | documented-upstream | `SRC-PRIOR-GUAVA-MULTIMAP` | One library release | None |
| PA-12 | Reject, evict and partially applied bulk insertion are distinct BiMap behaviours in existing libraries | documented-upstream | `SRC-PRIOR-GUAVA-BIMAP`, `SRC-BIMAP-063` | Two libraries | None |
| PA-13 | A verified map library states one well-formedness lemma per operation and requires lawful key instances | documented-upstream | `SRC-PRIOR-LEAN-MAPS` | Mutable page | Pin a Lean release |
| PA-14 | Lists, bags and sets are the free monoid and its commutative and idempotent quotients | documented-upstream | `SRC-PRIOR-BIRD-PRG69`, sections 1.2–1.5 | High | Tree level unread |
| PA-15 | The origin of abstraction functions and representation invariants is Hoare 1972 | unverified | `SRC-PRIOR-HOARE-1972` | Survey from prior knowledge | `Q-PRIORART-1` |
| PA-16 | Alignment is commutative and associative up to isomorphism, and zip does not distribute over it | unverified | `SRC-PRIOR-SEMIALIGN`, survey-read | The chapters prove their own versions (`CX-ZIP-CONCAT`, inner-join associator) | Reread when the host allows |
| PA-17 | Decidability of bag containment for conjunctive queries is open | unverified | `SRC-PRIOR-CHAUDHURI-VARDI-1993`, survey-read | Not used by any chapter | `Q-PRIORART-1` |
| PA-18 | The survey found no prior statement of representative-level set laws | historical | The survey's own statement | An absence in one survey | A targeted search would be needed to claim more |

## References

All references are catalog rows in [SOURCES.md](../SOURCES.md): the
`SRC-PRIOR-…` ids under
[Prior-art survey sources](../SOURCES.md#prior-art-survey-sources-2026-11-12),
plus `SRC-ZANZIBAR-2019`, `SRC-SPICEDB-SCHEMA`, `SRC-INPUT-ALBERT-1991` and
`SRC-INPUT-ROARING-PAPER` there, and `SRC-BIMAP-063` in the
[supplement sources](../formal-models/SOURCES.md). Each row gives the version
or identifier, the passage, the access date and the check level.

## Change and lineage record

- **Input.** The survey text and its consulted-page list under
  `docs/reviews/supplied/`, written into the repository on 2026-11-12 from the
  research tool's output. The survey has no opaque citation markers and no
  session-local links.
- **Parent.** The corpus synthesis, whose questions `Q-ALG-1` and `Q-BAG-1`
  note that the literature was unread. This report narrows that gap and does
  not close either question. It surveys the formal-model supplement
  (`FROST-RESEARCH-FORMAL-SUPPLEMENT`), which keeps a separate manifest.
- **Children.** None yet. Five seeded questions are listed above.
- **Corrections.** C1 to C9 above. The survey text is unchanged.
- **Design changes made with this report.** Laws `BAG-MONUS`, `BIMAP-COMPAT`,
  `CLOSURE-STABLE` and `CTX-KEY`; prior-art notes in the chapters; the alias
  table in terminology; recorded in
  [design decisions](../../design/decisions.md#prior-art-survey-integration-2026-11-12).
- **Reviewers.** One AI model wrote the survey, reread the sources, wrote the
  checks and wrote this report, in one session. No person and no second
  reviewer has checked it.
- **Next review trigger.** A reread under `Q-PRIORART-1`; a new release of a
  cited mutable page; a change to any of the four laws.

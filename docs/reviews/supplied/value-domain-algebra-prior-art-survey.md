# Prior-Art Survey for the Frost Value-Domain and Collection Algebra Specification

Almost every construct in the Frost spec re-derives established theory, and three sources match it closely enough to cite as foundations right away. The first is the Isabelle Collections Framework / Refinement Framework (refinement relations between abstract sets/maps and concrete structures). The second is Lawson's inverse-semigroup theory, where "partial bijections f and g are compatible if and only if f ∪ g is a partial bijection" is exactly Frost's checked BiMap union. The third is the K-relation / semiring line (Green–Karvounarakis–Tannen 2007 → Geerts–Poggi monus → Abo Khamis et al. Datalog° → DBSP Z-sets), which already separates bags (ℕ), signed weights (ℤ), and the convergence conditions for recursive composition. The spec should adopt their names ("compatible join", "monus", "stable semiring", "Z-set", "certain answers", "well-formedness/WF", "view") rather than coin new ones. It should also tighten three laws that prior results show are either false or need stronger preconditions.

## TL;DR

- **Closest prior art per pillar.** Refinement and representation invariants: Hoare 1972, the Isabelle Collections Framework (Lammich & Lochbihler, ITP 2010), Lean 4 `Std.HashMap.Raw.WF`, and Verus `vstd` views. Collection algebra: the Boom hierarchy (Meertens 1986, Bird) and monoid comprehensions (Fegaras & Maier, TODS 2000). Bags and relations: provenance semirings (Green et al., PODS 2007), m-semirings (Geerts & Poggi), Datalog° (PODS 2022 / JACM 2024), and DBSP (VLDB 2023). BiMaps: Lawson's inverse semigroups. Computed sets: Zanzibar, SpiceDB, Cedar, and certain-answer semantics (Imieliński & Lipski 1984; Libkin 2015/2016).
- **Laws to tighten.**
  - Monus-based bag difference fails axiom (A5), which query optimizers rely on (Tannen's result, as recounted in Suciu's "Different Differences in Semirings"). Suciu adds that keeping every bag identity means keeping all identities of (ℕ,+,·,0,1,∸), a set that is "co-r.e. complete, and, thus, it is not finitely axiomatizable."
  - Least fixpoints over ℕ need not converge; termination is tied to *stability* of the semiring (Abo Khamis et al.).
  - Bag-semantics equivalence of conjunctive queries is isomorphism, and whether bag containment is even decidable is still open (Chaudhuri & Vardi 1993 and later work). So bag-rewrite admission should be a curated whitelist, not a decision procedure.
- **Laws that prior art supports.**
  - Lawson's compatibility theorem supports the BiMap checked-union law.
  - The `semialign` package explicitly marks zip/align distributivity as a *non*-law, which supports the zip non-homomorphism.
  - Guava's "asMap is a map to nonempty collections" supports the empty-fiber-means-absent invariant.
  - DBSP's "Bags are Z-sets where all weights are positive" supports keeping Z^(K) as a separate carrier.
  - SpiceDB's caveat/exclusion CVE (cache keys omitting check hints) is direct evidence that the evaluation context θ must be part of every memo/plan key.

## Key Findings

1. **Frost's I_R/α_R is standard Hoare-style data refinement.** Modern verified-collection practice uses the same split under different names: Lean `Raw.WF` ("well-formedness"), Verus `View`/`@` ("view"), Isabelle "refinement relation". Lean 4 `Std` is the strongest *verified implementation* of a hash map found (operations proved to preserve `WF`). Verus's std `HashMap` support is a *trusted* specification wrapper, not a verified implementation.
2. **Frost's checked BiMap union is the inverse-semigroup "compatible join" restricted to the symmetric inverse monoid I(X).** Lawson gives the theorem, the name, partial identities 1_A (with 1_A·1_B = 1_{A∩B}), and the stronger "orthogonal join" (disjoint domains *and* ranges).
3. **Bag difference should be named "monus" (truncated subtraction).** The spec should not claim the full set-difference law suite for it. Suciu, "Different Differences in Semirings" (§4.3), reports that "Tannen proved that m-semirings fail to satisfy an important axiom (called (A5)… that is needed in query optimization". Suciu then shows that keeping every bag identity would require the full equational theory of (ℕ,+,·,0,1,∸), which is not finitely axiomatizable.
4. **Recursive weighted composition over ℕ needs a termination precondition.** Abo Khamis et al. (arXiv 2105.14435, JACM version) show that "termination is guaranteed iff the POPS enjoys a certain algebraic property called stability", stated in Theorem 1.2 as a condition on the core semiring P⊕⊥. ℕ is not stable, so an unbounded closure over N-relations may diverge.
5. **The [L,U] computed-set design matches certain/possible-answer semantics and Zanzibar-family systems.** SpiceDB's production bugs cluster exactly at intersection/exclusion under partial knowledge (caveats, wildcards). That supports Frost's explicit transfer rules and its requirement that θ be part of evaluation identity.

## Details by Construct

Status legend: **V** = verified (source page or abstract read in this survey); **I** = inferred (from prior knowledge, not read in this survey; check before citing).

### 1. Representation invariant, abstraction function, observational refinement

| Prior art | Type | Link | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|---|
| Hoare 1972, "Proof of correctness of data representations", Acta Informatica 1(4) | Paper | (Springer, Acta Informatica) | Origin of abstraction function + representation invariant | Deterministic operations; no trace sets | Cite as the source of I_R/α_R | I |
| He, Hoare & Sanders 1986, "Data refinement refined" (ESOP) | Paper | — | Forward/backward simulation for nondeterministic data refinement | Relational, not functional, α | Use "forward simulation" for plans with nondeterministic iteration order | I |
| Liskov & Guttag, *Abstraction and Specification in Program Development* (1986) | Book | — | "rep invariant", "abstraction function" naming | Informal | Adopt the names verbatim | I |
| Isabelle Collections Framework (Lammich & Lochbihler, ITP 2010) + Refinement Framework / Autoref (Lammich, ITP 2013) | Library + papers | https://www21.in.tum.de/~lammich/pub/itp10.pdf ; https://isa-afp.org/browser_info/current/AFP/Collections/document.pdf | Refines abstract sets/maps to red-black trees and hash tables via data refinement; nondeterministic specification monad | Refinement *relations* (not functions); targets code generation to SML/Haskell/OCaml | Frame α_R as a relation when several concrete reps share one abstract value, and use refinement of nondeterministic results (≤ ordering) for unspecified iteration order | V |
| Lean 4 `Std.HashMap` / `DHashMap` / `TreeMap` | Library (verified) | https://lean-lang.org/doc/reference/latest/Basic-Types/Maps-and-Sets/ | `Raw` type + `Raw.WF` well-formedness predicate; every operation has a `WF.op` preservation lemma; lemmas proved by reducing to lists | Invariant can be bundled or unbundled; no trace semantics | Adopt the naming convention "WF.op lemma per operation" for I_R preservation obligations | V |
| Verus `vstd` (`HashMapWithView`, `std_specs::hash`, `std_specs::btree`) | Library (specs) | https://verus-lang.github.io/verus/verusdoc/vstd/hash_map/struct.HashMapWithView.html | `View` maps a concrete map to the mathematical `Map`; lookup guarded by `obeys_key_model`/`obeys_cmp` (key-domain coherence obligations) | `HashMapWithView` is a *trusted* wrapper around std; open issues show borrowed-key (`Q`) types are unconstrained and a `Hash` impl may carry `requires false` | Adopt "view" vocabulary; make "key type obeys key model" an explicit precondition, *including for the borrowed lookup type* (the gap Verus issue #3176 documents) | V |
| Aeneas (Ho & Protzenko, ICFP 2022) | Tool + paper | https://dl.acm.org/doi/10.1145/3547647 | Translates safe Rust to pure functional models (Lean etc.); includes a verified hash-map case study | Functional translation, not refinement of a spec type | Possible route to mechanize Frost's α_R for a reference implementation | V (paper); hash-map case study I |
| Coq MSets/FSets; Why3, Dafny, VeriFast collection libraries; Creusot, Prusti, Kani | Libraries/tools | — | Coq MSets = ordered/weak sets with specs functor (`WSetsOn`, `eq` setoid) | Mostly sets/maps only; Kani is bounded model checking, not refinement | MSets' "weak sets over a decidable setoid" is the closest formal analogue of Frost's key domain | I |

Which verify what: hash maps are verified in Lean 4 Std (**V**) and in an Aeneas case study (**I**); Verus is a trusted spec (**V**). The ICF verifies red-black trees and hash tables (**V**, from its abstract). B-tree verification exists in Verus-based storage work (**I**). This survey found no mechanized verification of Roaring bitmaps.

### 2. Collection types as free algebras

| Prior art | Type | Link | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|---|
| Boom hierarchy (Meertens 1986; Bird, *Lectures on Constructive Functional Programming*, PRG-69) | Papers | https://www.cs.ox.ac.uk/files/3390/PRG69.pdf | Trees → lists → bags → sets by adding associativity, commutativity, idempotence to one binary constructor; homomorphisms = folds; `head`/`last` undefined on bags/sets because ⊕ is commutative | No keys/equivalence; no maps | State Seq/Bag/Set as free monoid / free commutative monoid / free semilattice, and justify "operation X is not defined on Bag" by non-homomorphism | V |
| Zwart & Marsden, "No-go theorems for distributive laws" (arXiv 2003.12531) | Paper | https://arxiv.org/pdf/2003.12531 | Which Boom-hierarchy monads compose via distributive laws | Category-theoretic | Cite when Frost nests collections (e.g., MultiMap fibers) to warn that not all nestings compose lawfully | V |
| Fegaras & Maier, "Optimizing object queries using an effective calculus", TODS 25(4) 2000 | Paper | doi:10.1145/377674.377676 | Monoid comprehension calculus treating sets, bags, lists, aggregates uniformly; general unnesting | Query language, not a library | Adopt "monoid homomorphism" as the admission criterion for comprehension rewrites | V (via subagent) |
| Buneman, Naqvi, Tannen, Wong, "Principles of programming with complex objects and collection types" (TCS 1995) | Paper | — | Structural recursion on collections; well-definedness requires the combining operation to respect the algebra's laws | — | Cite for "fold over Set requires an ACI combiner" | I |
| Okasaki Edison; Scala 2.13, Guava, Eclipse Collections, Apache Commons, Kotlin, Swift Collections, .NET | Libraries | — | Edison has explicit class laws; others mostly informal contracts | Informal or unchecked laws | Edison is the only one with stated laws worth citing | I |

### 3. Key domains, quotients, representatives

| Prior art | Type | Link | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|---|
| Setoids / quotient types (Coq setoids, Lean `Quotient`) | Theory | — | Equivalence ~ plus quotient | Proof-relevant, no representative selection | Name the classifier "canonical projection" | I |
| Java `Comparator` "consistent with equals" (TreeSet/TreeMap docs) | Library doc | — | Warns that sorted sets break the `Set` contract if compare ≠ equals | Treated as a footnote, not a parameter | Frost's explicit key domain is a principled version; cite it as the motivating failure | I |
| Haskell `Data.Set.union` / `Data.Map.union` left-bias | Library | — | Left-biased: retains the left representative/value on collision | Bias is a documented behavior, not a law family | Adopt the term "left-biased" | I |
| Rust `hashbrown`/`equivalent::Equivalent`, `Borrow` | Library | — | Heterogeneous lookup; Borrow requires Eq/Hash to agree | Verus shows the borrowed type `Q` is a specification gap | Require lookup-key types to satisfy the same key-model obligation | V (Verus issue); rest I |
| IEEE-754 NaN, `f64::total_cmp`, `ordered-float` | Library | — | Non-reflexive `==` excluded from Eq; total order wrappers | — | Use as the canonical example of an inadmissible value domain | I |

Representative-sensitive set operations: this survey found no dedicated paper. The failures Frost lists (XOR reassociation and absorption orientation not holding at the representative level, while holding on the quotient) follow directly from the fact that left-biased union is associative but not commutative at the representative level. Treat this as Frost's own contribution (**I**).

### 4. Validated value domains

| Prior art | Type | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|
| Alexis King, "Parse, don't validate" (2019 blog) | Essay | D=(T,P) admitted values as a refined type produced by a parser | Informal | Cite for the "construct once, carry proof in the type" design | I |
| Liquid Haskell; F* refinement types | Languages | {v:T \| P v} is literally D=(T,P) | Checked statically | Use refinement-type notation for D | I |
| nutype, garde, validator (Rust) | Crates | nutype = newtype + sanitize + validate at construction | Sanitize-then-validate conflates normalization and validation | Keep Frost's separation; note nutype's order (sanitize first) | I |
| Unicode UAX #15 normalization, case folding (versioned Unicode data), path canonicalization | Standards | Normalization is a projection onto representatives | Results depend on the Unicode version | Record the normalization version in θ | I |

### 5. Image/preimage over Boolean operations

| Prior art | Type | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|
| Direct image ⊣ preimage Galois connection; preimage f⁻¹ is a complete Boolean-algebra homomorphism; direct image preserves ∪ always and preserves ∩ and − iff f is injective | Standard set theory | Exactly Frost's key-conversion laws | — | State as "f[−] ⊣ f⁻¹[−]" and cite any lattice-theory text (e.g., Davey & Priestley) | I (standard result) |
| Dictionary encoding / order-preserving dictionaries in column stores (e.g., Binnig, Hildenbrand, Färber, SIGMOD 2009) | Paper | Translating codes between independent dictionaries is a key conversion; order-preserving encoding lets range predicates run on codes | Engineering focus | Treat cross-dictionary translation as a non-injective-risk conversion requiring θ | I |

### 6. Bag/multiset algebra

| Prior art | Type | Link | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|---|
| Albert 1991, "Algebraic properties of bag data types", VLDB | Paper | http://www.vldb.org/conf/1991/P211.PDF | Laws of additive union, max-union, min-intersection, difference | — | Cite as the classical law catalogue | Bibliography V; content I |
| Grumbach & Milo, "Towards tractable algebras for bags", PODS 1993 / JCSS 52(3) 1996 | Paper | doi:10.1006/jcss.1996.0042 | Bag algebra expressiveness and complexity | Nested bags | Cite for complexity of nesting | Bibliography V |
| Geerts & Poggi, m-semirings (monus) | Paper | https://adrem.uantwerpen.be/bibrem/pubs/GeertsA.pdf | Difference via monus; for ℕ, monus = max(0, a−b) | Not every semiring admits a monus | Name Frost's bag difference "monus" | V |
| Suciu, "Different Differences in Semirings" (Tannen Festschrift, OASIcs vol. 119) | Paper | https://drops.dagstuhl.de/storage/01oasics/oasics-vol119-tannens-festschrift/OASIcs.Tannen.10/OASIcs.Tannen.10.pdf | Recounts Tannen's proof that m-semirings fail axiom (A5), which optimizers need; shows adding A5 is not enough, because keeping all identities of (ℕ,+,·,0,1,∸) means keeping a set that is "co-r.e. complete … not finitely axiomatizable" | — | Do **not** carry set-difference rewrite laws over to bag monus without re-proof | V |
| Python `collections.Counter` | Library | https://docs.python.org/3/library/collections.html | `+` sum, `-` truncated (monus), `&` min, `\|` max; multiset ops "exclude results with counts of zero or less"; `subtract()` allows negatives; `^` symmetric difference added in 3.15 | One type mixes ℕ-bag and ℤ-weight semantics | Cite as the counterexample motivating separate Bag and Z^(K) carriers | V |
| Guava `Multiset`, Eclipse Collections `Bag`, Rust `hashbag`/`multiset` | Libraries | — | Count-based bags | `int` counts; overflow behavior varies | Specify checked/saturating count arithmetic explicitly | I |
| SQL UNION ALL / INTERSECT ALL / EXCEPT ALL | Standard | — | Sum / min / monus | NULL semantics | Map Frost bag ops to these names in docs | I |

### 7. Relations, weighted composition, recursion, signed weights

| Prior art | Type | Link | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|---|
| Green, Karvounarakis & Tannen, "Provenance semirings", PODS 2007 | Paper | doi:10.1145/1265530.1265535 | K-relations: 𝔹 = sets, ℕ = bags, ℕ[X] universal; composition = semiring matrix product | Positive RA only | Define Frost Relation/MultiMap weights as K-relations | Bibliography V; content I |
| Abo Khamis, Ngo, Pichler, Suciu, Wang, "Convergence of Datalog over (Pre-)Semirings", PODS 2022; JACM 71(2) 2024 | Paper | https://arxiv.org/pdf/2105.14435 | Datalog° = least fixpoint over K-relations; "termination is guaranteed iff the POPS enjoys a certain algebraic property called stability" (Theorem 1.2, a condition on the core semiring P⊕⊥); generalized semi-naïve evaluation | Finite convergence only (not ω-limits) | Require a stability/boundedness precondition for transitive closure over ℕ-weighted relations, or restrict closure to 𝔹 | V |
| Budiu, Chajed, McSherry, Ryzhyk, Tannen, "DBSP", PVLDB 16(7) 2023 (Best Paper) | Paper + Feldera | https://www.vldb.org/pvldb/vol16/p1601-budiu.pdf | Z-sets as an abelian group; "Bags are Z-sets where all weights are positive"; Z-sets represent changes | Streams/incrementalization focus | Cite for keeping Z^(K) separate, and for deltas as Z-sets | V |
| McSherry et al., differential dataflow (CIDR 2013); Abadi, McSherry, Plotkin, "Foundations of Differential Dataflow" (FoSSaCS 2015) | Papers | — | Signed multiplicities, partially ordered times | — | Use for revision-indexed deltas in θ | I |
| Stratified negation (Apt, Blair, Walker 1988) | Theory | — | Difference inside recursion needs stratification | — | Forbid monus inside recursive plans unless stratified | I |

### 8. Multimaps

| Prior art | Type | Link | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|---|
| Guava `Multimap` / `SetMultimap` / `ListMultimap` | Library | https://guava.dev/releases/23.0/api/docs/com/google/common/collect/Multimap.html | `asMap()` is "a Map from each distinct key to the nonempty collection"; `get` never returns null; empty collections are not retained; "any two empty multimaps are equal"; a non-empty SetMultimap never equals a non-empty ListMultimap | Fiber kind is part of equality | Adopt the empty-fiber-means-absent invariant verbatim; note the empty-equality corner case across fiber kinds | V |
| Multimap ≅ relation (Set fibers) / ℕ-relation (Bag fibers) | Theory | — | K → 𝒫(V) ≅ 𝒫(K×V); K → ℕ^(V) ≅ ℕ^(K×V) | Seq fibers have no relation counterpart (order) | State the isomorphisms; Seq-fibered MultiMap is outside the K-relation framework | I |
| Eclipse Collections multimaps; Rust `multimap` crate | Libraries | — | Similar | The Rust `multimap` crate stores `Vec` fibers (Seq) | — | I |

### 9. BiMaps as finite partial bijections

| Prior art | Type | Link | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|---|
| Lawson, *Inverse Semigroups: The Theory of Partial Symmetries* (World Scientific 1998); "Primer on inverse semigroups" (arXiv 2006.01628); Wagner–Preston theorem | Book/papers | https://arxiv.org/pdf/2006.01628 ; https://www.macs.hw.ac.uk/~markl/3-inverse.pdf | Compatibility a~b iff a⁻¹b and ab⁻¹ are idempotent; in I(X), compatible iff f∪g is a partial bijection; partial identities 1_A with 1_A1_B = 1_{A∩B}; orthogonal join (disjoint) | Algebraic definition via idempotents rather than "functional both ways" | Rename checked union to "compatible join"; add "orthogonal join" as the disjoint special case; cite Wagner–Preston for every inverse semigroup embedding in some I(X); write R⁻¹∘R = 1_dom(R) | V |
| Guava `BiMap` | Library | https://guava.dev/releases/33.0.0-jre/api/docs//com/google/common/collect/BiMap.html | `put` throws IllegalArgumentException if the value is bound to a different key (map unmodified); `forcePut` silently evicts; `putAll` may partially apply before throwing | `putAll` is not atomic | Make Frost's checked union atomic, and cite Guava's `putAll` as the non-atomic contrast | V |
| Rust `bimap` crate | Library | https://docs.rs/bimap/latest/bimap/hash/struct.BiHashMap.html | `insert` returns `Overwritten::{Neither, Left, Right, Pair, Both}` and can shrink the map; `insert_no_overwrite` returns `Err((l,r))` and leaves it unchanged | — | Map `Overwritten::Both` to "insertion evicts two pairs" in Frost's overwrite contract | V |
| Apache Commons `BidiMap`, Eclipse `BiMap`, Haskell `bimap` | Libraries | — | Similar overwrite/throw choices | — | — | I |
| Bidirectional transformations / lenses (Foster et al., TOPLAS 2007) | Paper | — | GetPut/PutGet laws | Lenses are asymmetric views, not bijections | Only relevant if Frost adds BiMap "views" | I |

### 10. Map merge and alignment

| Prior art | Type | Link | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|---|
| Haskell `Data.Map.Merge.Strict` | Library | https://hackage.haskell.org/package/containers/docs/Data-Map-Merge-Strict.html | `merge` with two WhenMissing tactics + one WhenMatched; `preserveMissing`, `dropMissing`, `zipWithMaybeMatched`; union/intersection/difference/symmetric difference all expressed as tactic choices; `mergeA` for effectful merges | — | Adopt "merge tactic" vocabulary and the derivation of all Boolean map operations from three tactics | V |
| `semialign` (`Semialign`/`Align`/`Zip`, `These`) | Library | https://hackage.haskell.org/package/semialign-1.3.1/docs/Data-Semialign.html | Laws: `align x y ≡ swap <$> align y x`, `align x (align y z) ≡ assoc <$> align (align x y) z`; "laws resemble lattice laws"; explicitly *not* a law: `zip` distributing over `align` | — | Import the associativity/commutativity laws of align and cite the explicit non-law | V |
| Scala cats `Align`/`Ior`; Rust itertools `EitherOrBoth`, `merge_join_by` | Libraries | — | Same three-way case type | — | Mention as cross-language names for `These` | I |
| Option monoid lifting a semigroup | Theory | — | Option<S> with None as identity is a monoid iff S is a semigroup, so lifted partial resolvers are associative through absence only if the resolver is associative on present values | — | State the resolver-associativity precondition explicitly | I (standard) |

### 11. Sequences

| Prior art | Type | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|
| Free monoid (Boom hierarchy) | Theory | Seq laws | — | — | V (via §2) |
| `semialign` zip/align non-law | Library | zip is not a homomorphism over concatenation | — | Cite | V |
| Stream fusion (Coutts, Leshchinskiy, Stewart, ICFP 2007); Java Streams ordering/side-effect rules; Rust iterator laziness | Paper/libraries | Rewrites that reorder callbacks change observable effects | — | Require effect-order preservation as a rewrite-admission condition | I |
| OT/edit scripts (Ellis & Gibbs 1989) | Contrast | Sequence diff as operations, not algebra | — | Mark as out of scope | I |

### 12. Frozen/immutable lifecycle

| Prior art | Type | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|
| Clojure transients (`transient`/`persistent!`) | Library | Typed phase transitions; the old transient becomes invalid after `persistent!` | Dynamic checks | Closest model for typed freeze/thaw | I |
| .NET 8 `FrozenSet`/`FrozenDictionary` | Library | Expensive construction, optimized reads; a separate type | Is a carrier, unlike Frost | Contrast: Frost treats Frozen as lifecycle, not a carrier | I |
| Guava Immutable*, kotlinx.collections.immutable builders, Rust `im`/`rpds`/`imbl`, Swift CoW | Libraries | Builders ↔ thaw; structural sharing | Shallow immutability | Document shallow vs deep explicitly | I |
| JavaScript `Object.freeze` | Language | Shallow freeze | Interior objects stay mutable | Use as the shallow-freeze cautionary example; Rust interior mutability (`Cell`, `RefCell`) is the analogue, and the `bimap` docs call mutating a hash through `Cell`/`RefCell` "a logic error" | V (bimap docs); rest I |

### 13. Computed sets with incomplete knowledge

| Prior art | Type | Link | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|---|
| Imieliński & Lipski, "Incomplete information in relational databases", JACM 31(4) 1984 | Paper | doi:10.1145/1634.1886 | Possible worlds; c-tables; basis of certain answers | Null-based incompleteness | Rename [L,U] to "certain/possible answers" | Bibliography V; content I |
| Libkin, "SQL's three-valued logic and certain answers", ICDT 2015 / TODS 41(1) 2016 | Paper | doi:10.1145/2877206 | SQL's results and certain answers "could be totally unrelated"; a modified 3VL guarantees no false positives | — | Adopt the "no false positives" correctness criterion for L | V (abstract) |
| Kleene K3 / Belnap 4-valued logic; Pawlak rough sets; interval abstract domains (Cousot) | Theory | Lower/upper approximations | — | Transfer rules: L(A∪B)=L_A∪L_B; L(A∩B)=L_A∩L_B; L(A−B)=L_A−U_B, U(A−B)=U_A−L_B; XOR has L = (L_A−U_B)∪(L_B−U_A) | I (standard) |
| Zanzibar (Pang et al., USENIX ATC 2019) | Paper | https://authzed.com/zanzibar | Userset rewrites (union/intersection/exclusion, computed_userset, tuple_to_userset); checks evaluated "at a consistent snapshot no earlier than" a zookie; Leopard index for nested-group set operations | No [L,U] partial results | Cite zookies as prior art for θ snapshots and Leopard for base+delta indexing | V |
| SpiceDB | Implementation | https://authzed.com/docs/spicedb/concepts/schema | `+`, `&`, `-`, intersection arrow `.all`; union binds tighter than `&`/`-` "for historical reasons"; caveats produce `CONDITIONAL_PERMISSION` (a third value); intersection/exclusion only partially short-circuit | Precedence quirk | Cite the CVEs below; avoid non-standard precedence | V |
| SpiceDB CVE-2026-55866 / GHSA-4vrg-r928-h5vv (NVD, published 14 Sep 2026; CVSS 3.1 3.7; CWE-863; affects 1.34.0–1.53.x, fixed in 1.54.0) | Advisory | https://github.com/NixOS/nixpkgs/issues/563480 | `checkRequestToKey()` and `checkRequestToKeyWithCanonical()` "omit CheckHints when constructing dispatch Check cache keys", so "a result computed for one hint set can poison the cache entry used by a semantically different authorization check". Intersection/exclusion with caveats then returned HAS_PERMISSION instead of CONDITIONAL | — | Direct evidence that θ must be in every memo/cache key | V |
| OpenFGA, Ory Keto | Implementations | — | Zanzibar-style union/intersection/exclusion | — | — | I |
| Cedar (Cutler et al., OOPSLA 2024; Disselkoen et al., FSE 2024) | Paper + implementation | https://arxiv.org/pdf/2403.04651 | Lean-formalized semantics; symbolic compiler to SMT that is "decidable, sound, and complete"; differential testing of Rust against the Lean model | Policy language, not set algebra | Adopt "verification-guided development": Lean model + differential testing of the Rust implementation | V |
| LSM trees (O'Neil et al. 1996); changelogs; cursor tokens bound to snapshots | Engineering | Base + ordered delta + compaction | — | Specify compaction as α-preserving | I |

### 14. Rewrite admission and query-equivalence proofs

| Prior art | Type | Link | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|---|
| Chaudhuri & Vardi, "Optimization of real conjunctive queries", PODS 1993 | Paper | doi:10.1145/153850.153856 | Bag equivalence of CQs = isomorphism; bag containment complexity left open | The claimed Π₂ᵖ-hardness was later withdrawn (Konstantinidis et al., ICDT 2025: "since the withdrawal of the Π^P_2-hardness claim by Chaudhuri and Vardi … progress in this area has stalled"). Decidability "has now been an open problem for 30 years" (Marcinkowski et al., arXiv 2503.18003) | Rewrite admission under bag semantics should be a whitelist of proved rules | V (via subagent) |
| HoTTSQL (Chu, Weitz, Cheung, Suciu, PLDI 2017) | Paper | doi:10.1145/3062341.3062348 | Coq-checked K-relation semantics for SQL with bags; rewrite rules proved | — | Model for mechanizing Frost rewrite proofs | V (via subagent) |
| Cosette (Chu, Wang, Weitz, Cheung, CIDR 2017) | Paper | http://cidrdb.org/cidr2017/papers/p51-chu-cidr17.pdf | SMT counterexample search + Coq proof | — | Counterexample-first admission workflow | Bibliography V |
| U-semiring / UDP (Chu, Murphy, Roesch, Cheung, Suciu, PVLDB 11(11) 2018) | Paper | doi:10.14778/3236187.3236200 | Axiomatization of SQL bag semantics; decision procedure for UCQs | — | Cite as the algebra for bag rewrites | Bibliography V; content partly |
| SPES (Zhou et al., ICDE 2022) and EQUITAS (Zhou et al., PVLDB 12(11) 2019) | Papers | doi:10.1109/ICDE53745.2022.00250 ; doi:10.14778/3342263.3342267 | SPES: bag equivalence via a bijection between outputs; proved 95/232 Calcite pairs vs 67 for EQUITAS (set semantics only) and 34 for UDP | — | The set-vs-bag gap is measurable: set-only provers are unsound for bag rewrites | V (via subagent) |
| egg (Willsey et al., POPL 2021), egglog (PLDI 2023) | Tools | — | Conditional rewrites, e-class analyses | Pure terms; effects/errors must be modeled | Use e-class analysis to carry admission preconditions | I |

### 15. Integer set representations

| Prior art | Type | Link | What matches | Key difference | Frost could adopt | Status |
|---|---|---|---|---|---|---|
| Chambi, Lemire, Kaser, Godin, SP&E 46(5) 2016 | Paper | doi:10.1002/spe.2325 | Array and bitmap containers; per the paper (arXiv 1402.6407), Roaring bitmaps "often compress significantly better (e.g., 2 times) and… are faster than the compressed alternatives (up to 900 times faster for intersections)" than WAH/Concise | Run containers came later | Cite for the container-selection invariant (part of I_R) | V (via subagent) |
| Lemire et al., "Roaring bitmaps: implementation of an optimized software library", SP&E 48(4) 2018 (CRoaring) | Paper + library | doi:10.1002/spe.2560 | SIMD intersection/union/difference/XOR; roaring-rs port | — | Treat container type as representation-only (invisible to α_R) | V (via subagent) |
| Demaine, López-Ortiz, Munro, "Adaptive set intersections, unions, and differences", SODA 2000 | Paper | https://erikdemaine.org/papers/SODA2000/ | Instance-optimal adaptive algorithms | Comparison model | Justify galloping merge as an execution-plan choice with identical α | V (via subagent) |
| Baeza-Yates (CPM 2004); Lemire, Boytsov, Kurz, "SIMD compression and the intersection of sorted integers" (SP&E 2016) | Papers | — | Skewed intersection, SIMD | — | — | I |

### 16. Law-testing practice

| Prior art | Type | What matches | Frost could adopt | Status |
|---|---|---|---|---|
| QuickCheck (Claessen & Hughes, ICFP 2000); `quickcheck-classes`, `hedgehog-classes`; Scala `discipline` | Libraries | Reusable class-law suites | Ship one law suite per carrier, parameterized by key domain | I |
| Rust `proptest`/`quickcheck`; model-based (state-machine) testing against a reference model (e.g., `proptest-state-machine`) | Libraries | Compare implementation against α_R(model) | Use a `BTreeMap`/`Vec` reference model as the executable α_R; test plans by checking trace-set membership, not equality | I |
| Cedar differential testing (Rust vs Lean model) | Practice | Same pattern at industrial scale | Cite as precedent | V |

## Recommendations

### (a) Ten citations to add first
1. Hoare, "Proof of correctness of data representations", Acta Informatica 1972, as the origin of I_R/α_R.
2. Lammich & Lochbihler, "The Isabelle Collections Framework", ITP 2010, plus Lammich's Refinement Framework / Autoref (ITP 2013).
3. Lean 4 `Std.HashMap.Raw.WF` and Verus `vstd` `View`, as the modern Rust/Lean practice for invariants and views.
4. Meertens 1986 / Bird PRG-69 on the Boom hierarchy.
5. Lawson, *Inverse Semigroups* (1998) / arXiv 2006.01628, for compatible and orthogonal joins.
6. Green, Karvounarakis & Tannen, "Provenance semirings", PODS 2007.
7. Geerts & Poggi (m-semirings), together with Suciu, "Different Differences in Semirings".
8. Abo Khamis et al., "Convergence of Datalog over (Pre-)Semirings", PODS 2022 / JACM 2024.
9. Budiu et al., "DBSP", PVLDB 2023.
10. Pang et al., "Zanzibar", USENIX ATC 2019, plus Imieliński & Lipski 1984 / Libkin 2016 on certain answers.

Close runners-up: Haskell `Data.Map.Merge` + `semialign`, Chambi/Lemire Roaring, and Cutler et al. (Cedar, OOPSLA 2024).

### (b) Terminology mismatches

| Frost term | Established term | Note |
|---|---|---|
| Checked union of BiMaps; "compatible iff pair union is functional both ways" | *Compatible join* (Lawson); *orthogonal join* if domains and ranges are disjoint | Lawson defines compatibility algebraically (a⁻¹b, ab⁻¹ idempotent); it coincides with Frost's in I(X) |
| R⁻¹∘R | *Partial identity* / idempotent 1_dom(R) | The idempotents of I(X) form a semilattice ≅ 𝒫(X) |
| Bag difference | *Monus* / truncated subtraction (m-semiring) | Not group subtraction |
| Signed-weight carrier Z^(K) | *Z-set* / z-relation (DBSP) | DBSP treats bags as the positive cone |
| Lower/upper bound [L,U] | *Certain / possible answers* | "No false positives" = soundness of L |
| Representative retained by union | *Left-biased* (Haskell containers) | — |
| Representation invariant I_R | *Well-formedness* `WF` (Lean); type invariant (Verus) | — |
| Abstraction function α_R | *View* `@` (Verus); refinement relation (Isabelle) | Isabelle's is a relation |
| Merge strategy per side | *Merge tactics* (`preserveMissing`, `dropMissing`, `zipWithMaybeMatched`) | — |
| Three-way presence case | `These` / `Ior` / `EitherOrBoth` | — |
| Snapshot/revision token in θ | *Zookie* (Zanzibar); consistency token | — |
| Computed set expressions | *Userset rewrites* (Zanzibar) | — |
| Unknown-due-to-missing-context | `CONDITIONAL_PERMISSION` (SpiceDB caveats) | A third truth value, as in Kleene K3 |
| Terminating weighted closure | *Stable* / p-stable semiring | ℕ is not stable |

### (c) Results that contradict or strengthen Frost laws

- **Contradicts any blanket bag-difference law transfer.** Suciu's "Different Differences in Semirings" recounts Tannen's proof that m-semirings (monus-based difference) fail axiom (A5), which standard optimizer rewrites need. Suciu then shows that adding A5 alone is not enough: keeping every bag identity means keeping all identities of (ℕ,+,·,0,1,∸), a set that is "co-r.e. complete, and, thus, it is not finitely axiomatizable." Every Frost law mixing monus with ⊎/∩ must be re-proved for ℕ specifically. Do not inherit laws from the set case.
- **Requires a precondition on recursive composition.** Datalog° convergence is guaranteed iff the semiring is stable. Closure over ℕ-weighted relations can diverge (count blow-up on cycles). Frost should restrict closure to 𝔹 / absorptive weights or require bounded/checked counts with an explicit overflow error.
- **Limits rewrite admission under bag semantics.** Bag equivalence of CQs is isomorphism, and decidability of bag containment remains open (Chaudhuri & Vardi's hardness claim was withdrawn). SPES's benchmark shows set-only provers miss or mis-handle bag cases. Admission should be a whitelist of mechanically proved rules.
- **Strengthens BiMap checked union.** Lawson's theorem (compatible iff f∪g is a partial bijection), and the fact that multiplication distributes over compatible joins in distributive inverse semigroups, give Frost a ready-made proof and extra laws to state.
- **Strengthens the zip non-homomorphism.** `semialign` documents that zip does not distribute over align.
- **Strengthens empty-fiber-means-absent.** Guava's `asMap` contract and "any two empty multimaps are equal". Frost should state the corner case that empty Set-fibered and empty List-fibered multimaps are equal while non-empty ones never are.
- **Strengthens θ-in-identity.** SpiceDB CVE-2026-55866 (GHSA-4vrg-r928-h5vv; NVD, 14 Sep 2026; CVSS 3.7; CWE-863) is a production failure. Dispatch Check cache keys omitted CheckHints, so "a result computed for one hint set can poison the cache entry used by a semantically different authorization check" under intersection/exclusion with caveats. Frost should state the law "plan cache key ⊇ θ".
- **Strengthens atomic checked operations.** Guava `BiMap.putAll` may partially apply before throwing, and `bimap::insert` can shrink the map. Both show why Frost's typed partial-function contracts (all-or-nothing) are worth stating.
- **Strengthens borrowed-lookup coherence.** Verus's open issue shows that specifying a key model for `K` but not for the borrowed `Q` leaves lookup unspecified. Frost's key domain should cover the lookup type too.

## Caveats

- Rows marked **I** come from prior knowledge and were not re-read in this survey. In particular: Hoare 1972, He/Hoare/Sanders, Liskov/Guttag, Coq MSets, Creusot/Prusti/Kani, Okasaki Edison, the JVM/.NET/Swift/Kotlin libraries, Clojure transients, .NET Frozen collections, King's essay, nutype/garde, dictionary-encoding papers, stream fusion, egg/egglog, and QuickCheck-family tools. Verify them before citing.
- Several key-result sentences for Albert 1991, Grumbach & Milo, Imieliński & Lipski, Green et al. 2007, and the U-semiring paper are paraphrased from memory. Only their bibliographic data was confirmed.
- The subagent corrected some commonly repeated citation details:
  - SPES is ICDE 2022, not VLDB 2020.
  - EQUITAS is PVLDB 2019.
  - The U-semiring paper's authors are Chu, Murphy, Roesch, Cheung, Suciu.
  - The Chaudhuri–Vardi hardness withdrawal is confirmed by peer-reviewed sources: Konstantinidis et al., ICDT 2025 (LIPIcs vol. 328), refers to "the withdrawal of the Π^P_2-hardness claim by Chaudhuri and Vardi". Marcinkowski et al. (arXiv 2503.18003) note that the full version of the 1993 paper "never appeared".
- The Verus issues and the SpiceDB CVE are recent GitHub/advisory pages. They reflect the state at the time of this survey and may have been resolved since.
- Representative-sensitive set-operation laws (which equal element survives; why XOR reassociation and absorption orientation fail on representatives) had no dedicated prior paper in this survey. This appears to be a genuine Frost contribution worth stating carefully.

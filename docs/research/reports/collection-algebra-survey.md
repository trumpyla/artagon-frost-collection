---
id: frost-research-report-collection-algebra-survey
kind: report
level: landscape
research_status: curated
implementation_status: mixed
source_validation: partial
input_files: [docs/reviews/supplied/frost-collection-algebra-research-1.md]
input_members: []
input_sha256: [543d67b2f7c3724f9c96849ea77c8dd24c3a36792395c46e6050d1383ebd8f6d]
origin_created_at: unknown
imported_at: 2026-11-09
repo_baseline: 82becb998af5627cc1e58118ae01515d8a3559ca
parent_ids: []
related_ids: [frost-research-report-collection-algebra-implementation-notes, frost-research-synthesis-foundations-and-engine-roadmap]
supersedes: []
open_questions: [Q-ALG-1, Q-MAP-1, Q-BAG-1, Q-MM-1, Q-BIMAP-1, Q-PREFIX-1]
---

# Collection algebra survey (curated from Frost1)

**What this is.** A curated derivative of the supplied report *Algebra and
Collection Abstractions for Frost: Maps, Lists, Bags, Multimaps, and Bimaps*
(`Frost1.md.md`, 920 lines, author and date unknown). The raw original is
preserved byte for byte as
[`frost-collection-algebra-research-1.md`](../../reviews/supplied/frost-collection-algebra-research-1.md).
This document keeps the survey's structure, corrects the examples and claims
listed in the [integration audit](../integration-audit.md), replaces opaque
export markers with catalog ids from [SOURCES.md](../SOURCES.md), and adds a
per-claim status. It overlaps heavily with the
[implementation notes curated from Frost2](collection-algebra-implementation-notes.md);
read the two together, not as independent confirmation.

**What is implemented today.** Frost is one package. Its implemented algebra is
the borrowed set algebra of `FrozenOrderedSet` and `FrozenHashSet`, the
bitmap wrappers' native operations, and `FrozenIntSet`'s eager algebra
(`SRC-FROST-DOC-SET-OPERATIONS`, `SRC-FROST-DOC-INTEGER-SETS`). Maps have
borrowed lookup and traversal capabilities but no merge or join algebra.
There are no bag, multimap, bimap, sequence-algebra, or prefix-index types.
Every API spelled below is a proposal unless a row in the claim table says
otherwise.

## 1. Executive synthesis

The report's central recommendation survives review: **do not design one
universal collection algebra**. Sets, maps, sequences, bags, multimaps, and
partial bijections have different laws, conflict policies, representative
rules, and physical implementations. The architecture it recommends,
"capability-oriented at the read boundary, algebra-specific above it,
backend-specialized below it", matches Frost's existing capability traits
(`SRC-FROST-SRC-READ`) and the general set-algebra design
(`SRC-FROST-DOC-SET-ALGEBRA`).

Three corrections change what a reader may take from it:

1. Its bimap sketch puts `try_insert(&mut self, …)` on a *frozen* type. A
   frozen value exposes no structural mutation; mutation belongs on a mutable
   builder or family, or the operation consumes and produces a new frozen
   value.
2. Its representative-identity example derives `Eq`/`Hash` while claiming
   equality ignores a field; a derive compares every field. The corrected,
   compiled example is in section 3.
3. Its "total linear" claim for n-ary sorted merges is a documentation
   statement about one library counted in elements. The inspected
   implementation is a binary heap; see section 6 and the audit.

The report also assumes a `crates/frost-*` workspace that does not exist and
proposes plural crate names (`frost-bags`, `frost-multimaps`, `frost-bimaps`)
where Frost2 proposes singular ones. Both are recorded as naming alternatives;
no package is created.

## 2. Logical models and laws

A set is a characteristic function `T → {0,1}`; a bag a finite-support
function `T → ℕ`; a map a finite partial function `K ⇀ V`; a set-valued
multimap a relation `K × V`, with sequence- and bag-valued variants adding
order or multiplicity; a bimap a partial bijection whose two projections are
both functional; a sequence a finite word with concatenation as its
associative operation and the empty word as identity. An implementation is
correct when its observable operations refine the model. (Class: derived
reasoning; the report's attribution of these definitions to literature is
unverified, see the claim table.)

### Bag operations

| Operation | Multiplicity of `x` |
| --- | ---: |
| `sum(A,B)` | `a(x) + b(x)` |
| `max_union(A,B)` | `max(a(x), b(x))` |
| `intersection(A,B)` | `min(a(x), b(x))` |
| `difference(A,B)` | `a(x).saturating_sub(b(x))` |
| `symmetric_difference(A,B)` | `abs_diff(a(x), b(x))` |
| `contains(x)` | `a(x) != 0` |
| `count(x)` | `a(x)` |

Calling both `sum` and `max_union` "union" would make an API ambiguous. The
report supports the distinction by appeal to C++ `set_union` and SQL
`UNION ALL`; those attributions are plausible and unverified here. Additive
sum can overflow a count type; max-union, intersection, and saturating
difference cannot overflow when their inputs are representable. The overflow
must surface as an explicit error, not a silent saturation, because
saturation is part of the *difference* definition only
(`SRC-FROST-DOC-COLLECTION-ALGEBRA`).

A compressed bag stores one representative and a count. If five equal but
distinct objects were inserted, it cannot later yield five distinct references.
A bag models logical multiplicity, not occurrence identity; a sequence or a
sequence-valued multimap models occurrences.

### Map conflict states

| Key state | `merge_with` | `inner_join` | `difference_by_key` | `symmetric_difference_by_key` |
| --- | --- | --- | --- | --- |
| left only | left entry | absent | left entry | left entry |
| right only | right entry | absent | absent | right entry |
| both | resolver decides the value | both values retained | absent | absent |

A join keeps both values, conceptually `(&K, &VL, &VR)`; a merge reduces to
one value through an explicit resolver. The report attributes the
"function at the conflict site" design to Haskell `unionWith`, OCaml
`Map.merge`, Clojure `merge-with`, and Java `Map.merge` (unverified
cross-language claims). What survives without those sources: a map union has
no uniquely correct value for a key present on both sides, so the operation
must take a policy.

### Bimap closure

Intersection and difference of two partial bijections remain partial
bijections. Union and symmetric difference do not:

```text
A = { alice ↔ 1 }      B = { alice ↔ 2 }      A ∪ B is not a bimap (alice twice)
A = { alice ↔ 1 }      B = { bob   ↔ 1 }      A △ B is not a bimap (1 twice)
```

Therefore `try_union` (checked), `union_relation` (relation-valued, always
defined), and resolver-based merge are distinct operations. Composition
`A: L ↔ M` with `B: M ↔ R` yields a partial bijection `L ↔ R` and is a
natural bimap operation. (Class: derived reasoning, by construction.)

## 3. Representative identity

Two keys can be equal while being distinct stored objects. Rust's
`HashMap::get_key_value` exists for this case and its own documentation
example defines a key whose `Eq`/`Hash` ignore a `name` field
(`SRC-RUST-HASHMAP-GET-KEY-VALUE`). Frost already keeps the first equivalent
key, the last map value, and the left representative on set overlap
(`SRC-FROST-DOC-PHILOSOPHY`, `SRC-FROST-DOC-SET-OPERATIONS`).

The raw report's example reads:

```text
#[derive(Hash, Eq, PartialEq)]
struct Key { logical_id: u64, display: String }   // "equality intentionally depends only on logical_id"
```

That derive compares and hashes `display` too; the comment and the code
contradict each other (audit hazard H1). The corrected example uses lawful
manual implementations. It was compiled and run on Rust 1.99.1 on 2026-11-09
with `rustc +1.99.1 --edition 2024 -D warnings` (command and output recorded
in the audit). It uses only the standard library, so it demonstrates the law
without claiming any Frost API:

```rust
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug)]
struct Key {
    logical_id: u64,
    display: String,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.logical_id == other.logical_id
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.logical_id.hash(state);
    }
}

fn main() {
    let stored = Key { logical_id: 7, display: "stored".to_owned() };
    let lookup = Key { logical_id: 7, display: "lookup".to_owned() };
    assert_eq!(stored, lookup);

    let mut map = HashMap::new();
    map.insert(stored, "value");

    let (key, value) = map.get_key_value(&lookup).expect("equal key is present");
    assert_eq!(key.display, "stored");
    assert_eq!(*value, "value");

    // A second insert with an equal key replaces the value but keeps the
    // first stored key representative.
    map.insert(Key { logical_id: 7, display: "second".to_owned() }, "replaced");
    let (key, value) = map.get_key_value(&lookup).expect("still present");
    assert_eq!(key.display, "stored");
    assert_eq!(*value, "replaced");
}
```

Recommended representative rules (proposal, consistent with the implemented
set rules):

| Abstraction and operation | Representative rule |
| --- | --- |
| Set intersection, set union overlap | left stored representative (implemented for the two frozen families) |
| Map inner join | left stored key, both values |
| Map merge overlap | left stored key; resolver chooses or creates the value |
| Map difference by key | left key and value |
| Bag element present on left | left representative; right-only results keep right representatives |
| Set multimap pair intersection | left key and left value representatives |
| Bimap intersection | left pair representatives |
| Sequence operations | occurrence identity and position preserved; no equivalence collapse unless requested |

These rules make borrowed results deterministic without requiring `Clone`.
They constrain optimizers: a plan that probes from the smaller right operand
must still return the left representative, as Frost's hash intersection
already does (`SRC-FROST-DOC-SET-OPERATIONS`). Rust's `HashSet::intersection`
may return either side's object (`SRC-RUST-HASHSET-INTERSECTION`), so
delegating to it would weaken the contract.

## 4. Ecosystem evidence

The report's Rust-crate table is reproduced with status. "Verified" means a
primary source in the catalog supports the row at the version named there;
the report itself names no versions.

| Rust source | Report's observation | Status |
| --- | --- | --- |
| `std::HashMap` | arbitrary order; borrowed lookup; stored key recoverable | Verified (`SRC-RUST-HASHMAP-GET-KEY-VALUE`); order claim standard but not separately checked |
| `std::BTreeMap` | key-order traversal; stored key recoverable | Unverified here (plausible; `get_key_value` exists on `BTreeMap` per the input) |
| `hashbrown::HashMap` | raw-entry control; hash invariants are the caller's | Unverified; the raw-entry API warning is not checked |
| `indexmap` | order follows insertion/removal history, not sorting | Secondary: checked at 2.14.0 in the set-algebra design (S8) |
| `imbl::HashMap` / `OrdMap` | persistent, structurally shared; HAMT and B+ tree | Secondary: checked at 7.0.2 in the set-algebra design (S4, S5); HAMT origin claim unverified |
| `fst::Map` | sorted byte-string streams, n-ary key operations, conflicting values exposed | Verified (`SRC-FST-0_4_7-OPBUILDER-DOCS`, `SRC-FST-0_4_7-RAW-OPS`) |
| `roaring::RoaringBitmap` | count kernels, rank/select, ascending iteration | Count kernels and rank/select verified in locked 0.12.0 source (`SRC-ROARING-0_12_0-SRC`); ascending iteration not checked |
| `multimap::MultiMap` | `K → Vec<V>`-like; duplicates allowed; insertion-ordered values | Unverified; not a Frost dependency |
| `bimap` | two indices; conflicts from either side | Unverified; not a Frost dependency |
| `multi_bimap` | many-to-many via antiparallel multimaps | Unverified; not a Frost dependency |

The cross-language table (C++, Java, Guava, Haskell, OCaml, Clojure, Scala)
is retained in the raw file only; every row is an unverified claim with
unknown version and date. The design lesson that survives independently of
those rows is that mature libraries expose conflict policy at the call site.

## 5. Abstraction-by-abstraction sketches (pseudocode)

All code in this section is **pseudocode**: not compiled, not an API, and in
several cases not compilable as written (missing lifetimes or bounds). The
raw file retains the originals; the corrections are noted inline.

**Map capabilities.** The report proposes `MapGet<Q>` with `get_key_value`.
Frost already has this capability as `LookupRef<Q>` (returns the stored
representative key) and separate `Entries` traversal; the query relation is a
per-implementation bound, not a trait bound (`SRC-FROST-SRC-READ`). The
proposal maps onto existing names; nothing is renamed. An `OrderedEntries`
endpoint trait exists; a sorted-traversal promise for merges is a separate
capability the set-algebra design also requires.

**Map algebra.** `merge_with(self, rhs, resolve)`, `inner_join`,
`left_join`, `full_join`, `difference_by_key`, `symmetric_difference_by_key`
as typed views over `frost::map::view(&m)`. Resolver type, call count, call
order, panic behaviour, and owned output are open (`Q-MAP-1`).

**Sequences.** `concat`, `zip`, `cartesian_product`, `merge_sorted`, `map`,
`filter`, `flat_map`; explicit projections `as_set_view`, `as_bag_view`,
`group_by`. No unqualified `union`/`intersection` on sequences.

**Bags.** `BagCount<Q>` with `count(&Q) -> Count`; `BagEntries` yielding
`(&Item, Count)` over distinct members; `sum`, `max_union`, `intersection`,
`difference`, `symmetric_difference`; checked `try_sum` or terminal-deferred
`try_count`/`try_cardinality` for overflow (`Q-BAG-1`).

**Multimaps.** Separate types `SeqMultiMap`, `SetMultiMap`, `BagMultiMap`
(Frost2 spells the first `ListMultiMap`); relation algebra on `SetMultiMap`
(`pairs_union`, …); projections `distinct_keys`, `key_occurrences`, `pairs`,
`values_for` (`Q-MM-1`).

**Bimaps (corrected).** The raw sketch:

```text
impl<L, R> FrozenBiMap<L, R> {
    pub fn try_insert(&mut self, left: L, right: R) -> Result<(), BiConflict<L, R>>;
}
```

is rejected as written: a frozen type has no `&mut self` structural
mutation (`SRC-FROST-SRC-FROZEN`, `SRC-FROST-DOC-PHILOSOPHY`). The corrected
shape keeps the conflict model and moves mutation to a mutable family or a
consuming operation:

```text
impl<L, R> MutableBiMap<L, R> {
    pub fn try_insert(&mut self, left: L, right: R) -> Result<(), BiConflict<L, R>>;
    pub fn freeze(self) -> FrozenBiMap<L, R>;
}

impl<L, R> FrozenBiMap<L, R> {
    pub fn get_by_left<Q>(&self, q: &Q) -> Option<&R>;   // Q relation per implementation
    pub fn get_by_right<Q>(&self, q: &Q) -> Option<&L>;
    pub fn thaw(self) -> MutableBiMap<L, R>;
    // Algebra (closed): intersection, difference return bimaps.
    // Algebra (checked): try_union -> Result<FrozenBiMap, BiMergeConflict>; union_relation -> relation.
}
```

`BiConflict` with `Left`, `Right`, and `Both` cases, and a borrowed
`BiMergeConflict` for non-destructive checked merges, remain useful proposals
(`Q-BIMAP-1`). A checked merge must preserve both inputs on error.

**Prefix index.** `get`, `entries_with_prefix`, `matching_prefixes`,
`longest_prefix`; byte, UTF-8, path-component, and bit-prefix domains are
distinct policies (`Q-PREFIX-1`). The ART attribution is unverified.

## 6. Specialized execution, persistence, and concurrency

**Sorted inputs.** A two-way merge is `O(n + m)` comparisons; Frost's ordered
kernel already uses a merge or a skew-sensitive seek with an internal 32:1
heuristic (`SRC-FROST-DOC-SET-OPERATIONS`).

**Hash inputs.** Scan the smaller side and probe the larger, counting table
capacity and hashing cost, not only logical length. Frost's hash
intersection already does this while returning left representatives.

**FST n-ary streams (corrected).** The raw report says FST runs n-ary
operations in "total linear stream complexity" with workspace proportional
to the stream count, and later that Frost "can wrap or integrate this
directly". The documentation does say `O(n1 + n2 + …)` in elements
(`SRC-FST-0_4_7-OPBUILDER-DOCS`). The inspected implementation is a
`BinaryHeap` of per-stream slots, each owning a key buffer, popped and
refilled per emitted key (`SRC-FST-0_4_7-RAW-OPS`). With `k ≥ 0` streams,
`N` total keys, and keys of at most `L ≥ 0` bytes, a conservative bound on
the merge's own work is `O(k + N · (L + 1) · log(k + 1))`: priming every
stream, one push and one pop per key through a heap of at most `k` slots,
and byte comparisons and key copies that cost constant work even for empty
keys. Per-advance stream decoding and I/O are additional. The merge
workspace is `O(k · (L + 1))` bytes of slots and key buffers plus up to `k`
indexed values, excluding each stream's internal state. Treating `k` and `L`
as constants gives the upstream `O(N)`; that fixed-fan-in reading is this
corpus's interpretation, not upstream wording. None of this is a measurement,
and none of it is a bound for any Frost algorithm. Full derivation and
assumptions: the [integration audit](../integration-audit.md).

**Roaring.** Count-only kernels and rank/select exist in the locked 0.12.0
source; Frost's wrappers expose `u64` counts (`SRC-FROST-SRC-BITMAP-ADAPTER`).
The report's "presence bitmap plus values by rank" frozen integer-map layout
is explicitly an inference, not an implementation; it remains a proposal.

**Persistence** is a storage strategy. `FrozenIntSet` already shares a root
and, with explicit Chunked storage, untouched containers, while copying a
flat directory for changed results; it is not a persistent tree
(`SRC-FROST-DOC-PERSISTENT`). Generic persistent families need an explicit
element-ownership model (`SRC-FROST-DOC-SET-ALGEBRA`, section 13).

**Concurrency.** Per-key atomic update and whole-collection snapshot algebra
are different contracts; Frost separates guarded live access from snapshot
publication (`SRC-FROST-SRC-SNAPSHOT`). The report's `live.snapshot()` sketch
corresponds to loading one version from a `SnapshotStore`, not to a method on
a live map. A concurrent bimap that updates two indexes should publish
immutable snapshots unless a backend proves an atomic two-index protocol.

**Benchmark scenarios.** The report's table is a list of scenarios, not
results; no measurement is inherited. Frost's benchmark methodology applies
(`docs/maintainers/benchmarks.md`).

## 7. Architecture and migration (proposal, reconciled)

The report assumes a finished `crates/frost-*` split with `frost-api`
re-exporting `frost::map`, `frost::bag`, and so on, and proposes a feature
table (`set-algebra`, `map-algebra`, `bags`, `hashbrown-0_17`, `imbl-7`,
`fst-0_4`, `roaring`, …). Reality at the baseline: one package with features
`hashbrown-0_17`, `indexmap-2`, `imbl-7`, `fst-0_4`, `roaring-0_12`,
`int-set`, `arc-swap-1`, and others (`SRC-FROST-CARGO`). The proposed table
mixes names that exist (`hashbrown-0_17`, `imbl-7`, `fst-0_4`) with names that
do not (`roaring` without a line, `set-algebra`, `bags`); it is historical
and must not be copied into `Cargo.toml` (hazard H7).

The ownership split it proposes (capabilities and families in a core;
algebra layers above; specialized kernels below; a facade on top) is a
responsibility map that can be honoured inside one package by module
boundaries. The crate-name sets differ between inputs (plural here, singular
in Frost2, and a third set in Frost3); the synthesis records one naming
policy as a proposal for a future decision.

The migration table's "finish current `crates/frost-*` workspace split" row
has no corresponding repository state; the current sequence is the one in
`docs/design/collection-algebra.md`: contracts first, reference models,
one domain at a time within the package, kernels only with differential
evidence, persistence and concurrency last.

## 8. Verification roadmap (mapped to actual tooling)

Per-domain property laws against tiny reference models (`BTreeMap<T, u64>`
for bags, `BTreeSet<(K, V)>` for set multimaps, two maps plus an inverse
checker for bimaps) are the right first tests and match Frost's existing
practice of differential and exhaustive finite tests
(`SRC-FROST-DOC-VERIFICATION`). The report's `model_inner_join` sketch omits
lifetimes and is pseudocode (hazard H15). Kani, Loom, and Miri are available
as isolated packages and Make targets in this repository; their scopes are
documented, and a harness's existence is not a result. The report's
tool-attribution claims (`proptest` shrinking, Kani bounded proofs, Loom
interleavings) are unverified as cited and unnecessary: the repository's own
verification document is the authority for what each tool does here.

## 9. Claim and evidence ledger

| Claim ID | Claim | Class | Evidence | Disposition |
| --- | --- | --- | --- | --- |
| S1-1 | No universal collection algebra; distinct laws per domain | derived reasoning | Section 2; `SRC-FROST-DOC-COLLECTION-ALGEBRA` | Accepted as design direction |
| S1-2 | Stored representatives differ from equality; `get_key_value` exists for this | documented upstream | `SRC-RUST-HASHMAP-GET-KEY-VALUE` | Accepted; example corrected and compiled |
| S1-3 | Frost keeps left representatives on set overlap | implemented and inspected | `SRC-FROST-DOC-SET-OPERATIONS`, `SRC-FROST-TESTS` | Current behaviour |
| S1-4 | Bag sum and max-union are distinct; literature warns against copying set laws | unverified (literature); derived (definitions) | `SRC-INPUT-ALBERT-1991` not read | Definitions accepted; attribution open (`Q-BAG-1`) |
| S1-5 | Bimap union is not closed | derived reasoning | Counterexample in section 2 | Accepted |
| S1-6 | `FrozenBiMap::try_insert(&mut self)` | proposal, rejected as written | `SRC-FROST-SRC-FROZEN` | Corrected to mutable builder or consuming operation |
| S1-7 | FST n-ary operations are total-input-linear with `O(k)` memory | documented upstream, qualified | `SRC-FST-0_4_7-OPBUILDER-DOCS`, `SRC-FST-0_4_7-RAW-OPS` | Qualified: heap-based; conservative derived bound with key length, decoding, and workspace stated separately; not measured; not transferable |
| S1-8 | Roaring exposes count kernels and rank/select | documented upstream (source) | `SRC-ROARING-0_12_0-SRC` | Verified at 0.12.0 |
| S1-9 | `crates/frost-*` workspace exists and should be finished | historical assumption, false at baseline | `SRC-FROST-CARGO` | One package; crate names are proposals |
| S1-10 | Proposed feature table | historical proposal | `SRC-FROST-CARGO` | Not copied; names partly nonexistent |
| S1-11 | Cross-language and crate-ecosystem attributions | unverified | Catalog gaps | Retained in raw file only |
| S1-12 | Rank-indexed frozen integer map layout | proposal (inference) | `SRC-ROARING-0_12_0-SRC` for rank/select | Open (`Q-PREFIX-1` adjacent; separate question if pursued) |
| S1-13 | Per-key atomicity is not snapshot consistency | derived; consistent with implementation | `SRC-FROST-SRC-SNAPSHOT` | Accepted |
| S1-14 | Verification tool attributions (proptest, Kani, Loom) | unverified as cited | `SRC-FROST-DOC-VERIFICATION` for actual scope | Use the repository's verification document |

## 10. References

All ids resolve in [SOURCES.md](../SOURCES.md). Opaque export markers from
the raw file are scoped and recorded as unresolved in
[citation-map.json](../citation-map.json); none is reproduced here.

## 11. Change and lineage record

- 2026-11-09: curated from `Frost1.md.md` at baseline `82becb9`; corrections
  H1, H2, H3, H5, H7, H13, H15 applied (ids from the integration audit).
- Parent: none (imported input). Children: questions `Q-ALG-1`, `Q-MAP-1`,
  `Q-BAG-1`, `Q-MM-1`, `Q-BIMAP-1`, `Q-PREFIX-1` in the
  [synthesis](../synthesis/foundations-and-engine-roadmap.md).
- Next review trigger: any design decision on a new collection domain, or
  recovery of the original citation mappings.

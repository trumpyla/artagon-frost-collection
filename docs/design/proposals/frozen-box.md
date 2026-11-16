# Proposal: `Domain` value domains for interval collections

Status: accepted, revision 8. The design is implemented as the workspace
package [`frost-interval`](../../../crates/frost-interval/README.md), by the
[decision of 2026-11-16](../decisions.md#interval-package-and-project-direction-2026-11-16).
Revisions 1 to 7 were developed in a standalone prototype
(`frost-domain-proto` 0.7.0); the package carries its code.

"FrozenBox" is the name of this proposal. The base trait is `Domain`.
Revision 4 rebuilt the design around one concept: every ordered domain has an
order-preserving **encoding** into words. Revisions 5 and 6 change names
only. Revision 6 applies one set of naming rules (section 3); the names do not
depend on the names of other Frost modules. Revision 7 adds a mutable set,
`IntervalSet<V>`, with single-value insert and remove (section 6). Revision 8
moves the prototype into the workspace (section 13). Section 12 gives the
history.

## 1. Decision

Introduce FrozenBox: `Domain`, a family of capability traits that a value type
implements to declare its logical domain, and two immutable collections,
`FrozenIntervalSet<V>` and `FrozenIntervalMap<V, T>`.

## 2. The design in five rules

1. **The domain is the type.** A value type declares its extrema, density, and
   encoding as associated items. A collection stores no domain data.
   Two domains with equal words (`UserId`, `u64`) are different types.
2. **Every ordered domain has an encoding.** `encode: V -> V::Word` is an
   order embedding: it is strictly monotone. `decode` is its partial inverse:
   it gives `None` for a word that encodes no value. `i64`, `NotNanF64`, `UserId`,
   and a time of day encode into `u64`. `u64`, `String`, and
   `i128` use the identity encoding.
3. **Collections work on words only.** The set and map kernels are
   domain-free: they see sorted boundaries of words. Every domain with the
   same word type and density uses the same compiled kernel. The typed
   collections are thin wrappers.
4. **The layout is computed, not chosen.** The boundary layout is a function
   of `(Density, Word)`. An implementor never names a layout.
5. **Capabilities are traits, not runtime flags.** `count()` needs `Ranked`.
   Member iteration needs `DiscreteDomain`. The bounds of a method state its
   cost, and a missing capability is a compile error.

These rules extend what the Frost integer profile already enforces: a lossless
order-preserving mapping to machine words, separate logical domains, no fill
into gaps, complement only against an explicit universe, and `u128` counts.

## 3. Terminology and naming rules

Seven rules give every name. An earlier rule wins over a later rule.

1. One concept has one name, and one name has one concept.
2. The mathematical noun is used when a standard one exists (cut, interval,
   lattice, poset, down-set, hull, rank).
3. No name repeats a `std` name for another concept (`Range`, `Box`,
   `Product`, `Index`, `Bound`).
4. The grammar shows the role:

   | Role | Form | Examples |
   |---|---|---|
   | Kind of domain | Math noun, or adjective + `Domain` | `Lattice`, `Poset`, `OrderedDomain` |
   | Property or capability | Adjective | `Ranked`, `CoverPreserving`, `Convex` |
   | Immutable collection | `Frozen` + noun; the plain noun stays free for a mutable form | `FrozenIntervalSet` |
   | Domain-free kernel type | `Raw` + the typed name | `RawSet`, `RawMap`, `RawCut`, `RawInterval` |
   | Methods | Rust API conventions: `to_`, `into_`, `from_`, `with_`, `is_`, `wrapping_` | `to_words`, `with_word`, `wrapping_succ` |

5. A name is abbreviated only where mathematics abbreviates it (`succ`,
   `pred`, `min`, `max`, `leq`).
6. A wrapper type names the invariant that it adds (`NotNanF64`).
7. Each name passes rules 1 to 6 with no reference to the names of other
   Frost modules. Those names can change with no effect here.

| Term | Meaning |
|---|---|
| domain | The set of values of one type |
| cut | A position between values (a Dedekind cut): `BelowAll`, `Below(v)`, `Above(v)`, `AboveAll` |
| canonical | One representation for each position or set |
| interval | Two canonical cuts in a total order; also one maximal group of members of a set. An interval set or map here always holds disjoint intervals |
| encoding | The map `encode: V -> V::Word`. It is an order embedding: a strictly monotone injection. In coding theory: an order-preserving code |
| word | `encode(v)`, the codeword of `v`: what collections store and compare |
| decode | The partial inverse of the encoding: `None` for a word that encodes no value |
| identity encoding | `Word = Self` |
| cover-preserving | `encode(succ v) == encode(v) + 1`: the words of the domain are an interval of integers |
| rank | The number of values below `v`. Only the rank is an index: words can have gaps and need not start at 0 |
| machine word | A word of the sealed type `MachineWord`: `u32` or `u64` |
| boundary | A cut as stored: a word, plus a side bit in a dense domain |
| segment | The part of the domain between two adjacent boundaries |
| layout | The storage form of the boundaries |
| kernel | Domain-free code that works on boundaries |
| toggle | Remove a boundary if it is present, add it if it is absent |
| density | `Discrete` (each value has a next value) or `Dense` |
| extrema | `MIN` and `MAX` of a domain: its least and greatest values |
| region | A describable subset, closed under intersection |
| hull | The smallest region that holds two regions |
| cyclic interval | An interval that can pass the end of a cycle |
| entry | One `(interval, value)` pair of a map |
| law | A rule of a trait that the compiler cannot check |

"Key" is not used for stored positions: a key is a logical value. "Index" is
not used for a word: an index is a position in a sequence (a segment index, a
rank). "Range" is not used for an interval: in mathematics a range is the
image of a function, and `Range` is a `std` type. "Limit" is not used for an
extremum. "Code", "image", "store", "box" (for a region), "span", "run", and
"piece" are not used.

## 4. Traits

An implementor writes only the required items. Derived API is in the blanket
trait `OrderedDomainExt` and in `Cut::canonical`; it cannot be overridden.

| Trait | Required items | Gives |
|---|---|---|
| `Domain: Clone + Eq` | none | A claim of one logical identity |
| `OrderedDomain: Domain + Ord` | `type Density`, `type Word`, `MIN`, `MAX` (default `None`), `encode`, `decode` | `FrozenIntervalSet`, `FrozenIntervalMap`, `min_cut`, `max_cut`, interval factories |
| `DiscreteDomain: OrderedDomain<Density = Discrete>` | `succ`, `pred` | Canonical half-open cuts, `iter`, `closed_intervals` |
| `CoverPreserving: DiscreteDomain + Copy` | none (marker) | `Ranked` for free with machine words; `to_words()` and `from_words()` of sets |
| `Ranked: DiscreteDomain + Copy` | `rank` | `count()` in one step per interval |
| `CyclicDomain: OrderedDomain` | none | `CyclicInterval`, `cyclic_intervals` |
| `FiniteDomain: Domain` | `CARDINALITY`, `index`, `from_index` | `FiniteSet<V>` |
| `PrefixDomain: Domain` | `type Key`, `to_interval` | Prefix sets as `FrozenIntervalSet<Key>` |
| `Poset`, `Lattice`, `BoundedLattice` | `leq`; `join`, `meet`; `bottom`, `top` | `DownSet<V>`, `LatticeInterval<V>` |

`Density` is sealed (`Dense`, `Discrete`). `Discrete: Density<V>` holds only
when `V: DiscreteDomain`, and `DiscreteDomain` requires `Density = Discrete`,
so the density claim and the `succ`/`pred` impl cannot disagree.

### Canonical cuts and the `Interval` invariant

`Cut::canonical` applies three rules:

1. `Below(v)` with `v == MIN` becomes `BelowAll`.
2. `Above(v)` with `v == MAX` becomes `AboveAll`.
3. In a discrete domain, any other `Above(v)` becomes `Below(succ v)`.

`Interval<V>` holds two canonical cuts, and the empty interval is the one value
`(AboveAll, AboveAll)`. So two intervals are equal exactly when they hold the
same values, and `is_empty`, `is_connected`, `is_superset`, `is_disjoint`,
`intersection`, `hull`, and `gap` are correct in discrete domains.

## 5. The encoding

```rust
pub trait OrderedDomain: Domain + Ord {
    type Density: Density<Self>;
    type Word: Ord + Clone;
    const MIN: Option<Self> = None;
    const MAX: Option<Self> = None;
    fn encode(&self) -> Self::Word;
    fn decode(word: &Self::Word) -> Option<Self>;
    fn into_word(self) -> Self::Word { self.encode() }
    fn with_word<R>(&self, f: impl FnOnce(&Self::Word) -> R) -> R { f(&self.encode()) }
}
```

Laws: `a.cmp(&b) == a.encode().cmp(&b.encode())`;
`decode(&v.encode()) == Some(v)`; if `decode(&i) == Some(v)` then
`v.encode() == i`.

- **Identity encoding.** `identity_encoding!()` inside the impl sets
  `Word = Self`. `with_word` then passes the value by reference, so a query
  with a `String` probe never clones it (a test counts clones).
- **Newtypes.** `newtype_domain!(UserId(u64): discrete, cover_preserving)`
  gives a newtype the traits of its field. It is a separate logical domain
  with the same word and the same kernel.
- **Cover-preserving encodings** have an interval of integers as their words.
  With machine words they get `Ranked` from a blanket impl:
  `rank(v) = encode(v) - encode(MIN)`. A domain with gaps (`char`, a sparse
  enum) writes `rank` by hand. A domain with no `rank` (`i128`) has no
  `count()`, so no call walks every member by accident.
- **The encoding lifts to sets.** For a cover-preserving encoding the words
  of an interval are an interval. `set.to_words()` gives the `FrozenIntervalSet<V::Word>`
  of words, and `FrozenIntervalSet::<V>::from_words(&words)` goes back and drops
  words that encode no value. So a set of ids and a set of plain numbers can
  meet in word space.

| Domain | Word | Cover-preserving |
|---|---|---|
| `u32`, `u64` | identity | yes |
| `u8`, `u16`, `i8`, `i16`, `i32`, `NonZero<u8/u16/u32>`, `NotNanF32`, dates, `Ipv4Addr`, `Month` | `u32` | yes |
| `i64`, `usize`, `isize`, `NonZero<u64>`, `NotNanF64`, time of day, `UserId` | `u64` | yes |
| `char`, sparse `Tier` | `u32` | no; `rank` by hand |
| `u128`, `i128`, `Duration`, timestamps | identity | no rank |
| `String`, `SystemTime`, chrono date-times, `Unit` | identity (dense) | — |

## 6. Kernel and layouts

`kernel` holds `RawSet<B>` and `RawMap<B, T>`, generic only over the boundary
sequence `B`. They do not know `V`.

| Density | Layout `B` | Bytes per boundary |
|---|---|---|
| `Discrete` | `Vec<V::Word>` | `size_of::<Word>()` |
| `Dense` | `SideBitVec<V::Word>`: words plus one `Below`/`Above` bit | `size_of::<Word>()` + 1 bit |

- A set is a toggle set: membership flips at each boundary, and
  `starts_inside` is the state below the first boundary. `contains` is one
  search for the segment of the probe. Complement flips `starts_inside`.
- Union, intersection, difference, and symmetric difference are one linear
  merge. `is_subset` and `is_disjoint` are one pass with no allocation.
- A map has `n` boundaries and `n + 1` segment values; adjacent segments
  differ. `merge_with(other, f)` is one linear pass, and `overlay`,
  `union_with`, `intersection_with`, `restrict`, and `remove` are instances.
- The operation closures are defined in the kernel. If they were defined in
  the typed wrapper, each value type would get its own copy of the merge.

### Mutable sets: the boundary tree

`IntervalSet<V>` is the mutable counterpart of `FrozenIntervalSet<V>`. It
holds the same canonical boundaries in `BoundaryTree<E>`, a B+tree in the
module `tree`. Like the frozen kernels, the tree is generic only over the
boundary type `E` (a word, or a word and a side bit), so every domain with a
`u64` word uses one compiled tree.

- **One update primitive.** `toggle(e)` removes boundary `e` if it is
  present and adds it if it is absent. A value `v` is the interval
  `[Below(v), Above(v))`, and the symmetric difference of a toggle set with
  an interval toggles its two cuts. So `insert(v)` on a non-member and
  `remove(v)` on a member are two toggles of the canonical cuts of `v`. The
  result is canonical with no extra step: adjacent intervals join, and a
  removed value splits its interval.

  | Case, on `[10, 20) ∪ [30, 40)` | Boundaries after | Change |
  |---|---|---|
  | `insert(25)`: isolated value | `10 20 25 26 30 40` | +2 |
  | `insert(20)`: extends an interval | `10 21 30 40` | 0 |
  | `insert(20)` on `[10, 20) ∪ [21, 30)`: joins two intervals | `10 30` | −2 |

- **Edges.** A toggle of `BelowAll` flips `starts_inside`, and a toggle of
  `AboveAll` changes nothing. So `{MIN}` and `{MAX}` need no special code.
  The upper cut comes from `succ(v)`, not from `encode(v) + 1`, so gaps in
  the encoding (`char`) are correct.
- **Rank.** Each inner node stores the boundary count of each child. One
  descent gives the number of boundaries at or below a probe, and its parity
  gives membership. A dense probe compares `(word, side)` pairs with
  `(word, Below)`.
- **Structure.** Nodes live in an arena (`Vec<Node<E>>`) with `u32` links and
  a free list. There is no `unsafe` code. A leaf holds at most `L`
  boundaries and an inner node at most `F` children (defaults 64 and 64);
  every node except the root is at least half full. Separators are lower
  bounds and can be stale after a delete; routing stays correct.
- **Conversion.** `IntervalSet::from(&frozen)` builds the tree from the sorted
  boundaries in O(n), and `freeze()` copies them back in O(n).

## 7. Interval layer

- **`IntoInterval<V>`**: every API takes `Interval<V>`, `a..b`, `a..=b`, `a..`,
  `..b`, `..=b`, `..`, `(Bound, Bound)`, or a `core::range` type.
- **`Region`** (`empty`, `all`, `is_empty`, `contains`, `is_superset`,
  `is_subset`, `is_disjoint`, `intersection`), **`Convex`** (`hull`), and
  **`BooleanRegion`** (`union`, `difference`, `complement`).

| Domain kind | Region type | `Convex` | `BooleanRegion` |
|---|---|---|---|
| Total order | `Interval<V>` | yes | no |
| Total order, many intervals | `FrozenIntervalSet<V>` | no | yes |
| Finite | `FiniteSet<V>` (`u128` bitset) | no | yes |
| Lattice | `LatticeInterval<V>` | yes | no |
| Product | `ProductRegion<A, B>` | if both sides are | no |
| Cyclic | `CyclicInterval<V>` is not a `Region` | — | — |

Two cyclic intervals can intersect in two parts, so
`CyclicInterval::intersection` returns a `FrozenIntervalSet<V>`. `Cut` does not
generalize to partial orders, so a `LatticeInterval` has value bounds, both
inclusive. Sets over partial orders are `DownSet<V>` (an antichain of maximal
elements).

## 8. Domain coverage

| Type | Density | Word | Verification |
|---|---|---|---|
| `u8`–`u64`, `i8`–`i64`, `usize`, `isize`, `NonZero<u8>`–`NonZero<u64>` | discrete | `u32` or `u64` | Full walk for 8- and 16-bit types; edge walks for the rest |
| `u128`, `i128` | discrete | identity | Edge samples |
| `char` | discrete, gap | `u32` | Full walk |
| `NotNanF32` | discrete, 1 ULP | `u32` | **Full walk of all 4,278,190,081 values** |
| `NotNanF64` | discrete, 1 ULP | `u64` | Edge walks; proptest against `partial_cmp`, `next_up`, `next_down` |
| `Duration` | discrete, 1 ns | identity | Edge samples |
| `SystemTime`, `Instant` | dense, no extrema | identity | Samples |
| chrono `NaiveDate` | discrete | `u32` | **Full walk of all 191,491,529 days** |
| chrono `NaiveTime`, `NaiveDateTime`, `DateTime<Utc>` | dense | identity | Samples, including leap-second forms |
| jiff `civil::Date`; time `Date` | discrete | `u32` | **Full walk** (7,304,484 days for jiff) |
| jiff `civil::Time`; time `Time` | discrete, 1 ns | `u64` | Edge walks |
| jiff `Timestamp`, `civil::DateTime`, `SignedDuration`; time `PrimitiveDateTime`, `UtcDateTime`, `Duration` | discrete, 1 ns | identity | Edge samples |

chrono, jiff, and time are behind features of the same names.

- **Floats.** `NotNanF32`/`NotNanF64` are `#[repr(transparent)]`, exclude NaN, and store
  `-0.0` as `+0.0`. The encoding is offset binary (`SIGN + magnitude` or
  `SIGN - magnitude`), computed without a branch. These are sets of floats,
  not of real numbers: `[0.0, 1.0)` equals `[0.0, 1.0.next_down()]`, and
  `count()` is exact. The `Unit` demo shows a dense float domain.
- **chrono time-of-day types are dense.** chrono stores a leap second as a
  nanosecond field of `1_000_000_000..2_000_000_000` in second 59, and its
  arithmetic skips those values, so `+ 1 ns` is not a lawful `succ`.
- **For a dense type**, `[a, b]` and `[a, b + 1 ns)` hold the same values but
  compare unequal. Membership is always correct.

## 9. Evidence (rustc 1.97.0, x86-64)

These measurements are historical. They were made on the prototype before
the move and were not rerun in the workspace. The prototype's benchmarks and
its code-generation examples are not in the package (section 13).

### Kernel sharing

One example program runs the same set operations over 20 domains (8 with a
`u64` word, 12 with a `u32` word). Revision 3 against revision 4:

| Measure | Revision 3 | Revision 4 |
|---|---|---|
| Copies of the merge kernel, debug build | 81 | 9 (4 operations × 2 machine word types, plus the generic entry) |
| Copies of the subset/disjoint kernel, debug build | 40 | 4 |
| LLVM functions, debug build | 2,420 | 1,745 |
| Text size, debug build | 1,185,074 bytes | 648,338 bytes |
| Text size, release build | 490,235 bytes | 375,571 bytes |
| Release build time of the example | 3.9–4.2 s | 1.2–1.3 s |

In revision 3, equal machine code for `UserId` and `u64` depended on LLVM
merging identical functions. In revision 4 it holds by construction:
`union::<UserId>` compiles to one jump to `RawSet<Vec<u64>>::union`.

### Size and machine code

- `size_of::<FrozenIntervalSet<T>>()` equals `size_of::<(bool, Vec<Word>)>()`
  for every integer, `NonZero`, float, and `char` type (test).
- `contains::<u64>` is 24 instructions with one `cmova` search loop, against
  23 for a hand-written `partition_point` over `&[u64]`. `contains::<i64>`
  adds the sign flip. `contains::<NotNanF64>` has a branch-free encoder and no
  float compare.

### Time

The machine is a noisy 2 vCPU VM. Confidence intervals reach ±20%.

- **Set union, alternated three times against revision 3:** revision 3
  1.36–1.87 ms, revision 4 1.12–1.67 ms. The first version of the new kernel
  was about 1.5 times slower (2.2–2.9 ms), because its merge loop held a
  selected reference across the compare. The A/B run found it. The final loop
  turns the three-way compare into two flags.
- **`contains` × 1024 (two runs):** `u64` 32.4–33.7 µs, `UserId`
  31.2–36.6 µs, `i64` 30.7–37.7 µs, `NotNanF64` 27.2–33.7 µs, hand-written `&[u64]`
  30.4–32.5 µs. The domains are in one band.
- **Maps against `rangemap` 1.8.0 (100k entries, two runs):**

  | Workload | `FrozenIntervalMap` | `rangemap` | Ratio in the same run |
  |---|---|---|---|
  | Build | 9.4–11.7 ms | 37.3–46.7 ms | 4.0x |
  | `get` × 1024 | 35.1 µs | 89.1–89.8 µs | 2.5–2.6x |
  | Overlay of two maps | 3.6–3.8 ms | 40.5–41.4 ms | 11.0–11.2x |

  Both maps return the same values at all 1,024 probes. The comparison is not
  complete: `rangemap` is a mutable `BTreeMap` with single inserts, which this
  prototype does not have, and its overlay is an insert loop.
- **Cost that is not zero:** `encode` runs once per query. It is free for
  `u32`/`u64`, one `xor` for signed integers, a few instructions for floats,
  and the library's conversion for dates.

### Mutable sets

Workload: 100,000 random `u64` values in `0..1,000,000` (172,056
boundaries, tree height 3). Two runs in one session; the ranges show both.

| Workload | `IntervalSet` | `rangemap` 1.8.0 `RangeInclusiveSet` | Other |
|---|---|---|---|
| Build by 100,000 single inserts | 22.5–25.5 ms | 29.9–35.1 ms | `BTreeSet<u64>`: 2.4–2.5 ms |
| 256 inserts plus 256 removes in the large set | 88.8–89.8 µs | 122.5–125.7 µs | — |
| One insert by rebuild of the frozen set | — | — | 448–476 µs |
| `contains` × 1024 | 76.3–80.1 µs | 83.2–91.0 µs | `FrozenIntervalSet`: 33.4–35.0 µs |
| Heap memory | 2,485,576 bytes | not measured | `FrozenIntervalSet`: 1,376,448 bytes |

Earlier runs in the same session gave 60–64 µs for `IntervalSet::contains`
and 95–119 µs for `rangemap`. The machine is noisy; read these as ranges.

- **Against a rebuild:** one insert is about 0.17 µs (half of a pair in the
  second row), against 448–476 µs for a frozen rebuild.
- **Against `BTreeSet`:** a build by inserts is about 10 times slower. A
  `BTreeSet` insert is one descent; an `IntervalSet` insert is three (one
  `contains` and two toggles), and a leaf insert moves up to 64 words.
- **Kernel sharing:** `examples/tree_mono.rs` updates sets of `u64`, `i64`,
  `UserId`, `NotNanF64`, and `u32`. The debug binary has one
  `BoundaryTree<u64>::toggle_at` and one `BoundaryTree<u32>::toggle_at`.

Leaf capacity and fan-out (`L/F`), one run on trees built by toggles:

| `L/F` | Build by 100,000 toggles | `rank_le` × 1024 | 256 toggle pairs |
|---|---|---|---|
| 32/32 | 8.48 ms | 65.2 µs | 41.1 µs |
| **64/64 (default)** | 8.14 ms | 55.1 µs | 35.2 µs |
| 128/64 | 9.15 ms | 47.1 µs | 37.9 µs |
| 256/64 | 9.65 ms | 43.1 µs | 39.2 µs |
| 512/64 | 10.73 ms | 48.4 µs | 48.0 µs |

Larger leaves make queries faster and updates slower. An insert is one query
and two toggles, so its estimated cost is the same for 64/64, 128/64, and
256/64 within the noise. The default stays 64/64.

Prefix sums in inner nodes were tried and rejected. They make a rank query
add one number per level instead of summing up to 63, but every update must
then change up to 63 sums per level. In an alternated A/B run they made
`rank_le` on a bulk-built tree 1.4 times faster (48–50 µs against 69–75 µs),
but toggles 1.3–1.6 times slower, and `contains` on a tree built by inserts
did not change (56–61 µs against 60–64 µs).

## 10. Tests

In the workspace, 66 tests pass with all features and 58 with no default
features on rustc 1.99.1. One test lost an assertion in the move: it checked
that `count()` on all of `char` took under 1,000 µs, and Frost forbids timing
assertions in correctness tests. In the prototype, 66 tests passed on rustc
1.97.0 and on 1.99.1 with all features:
26 domain tests, 17 interval-layer tests, 9 mutable-set tests, 7 third-party
temporal tests, and 7 doc tests (5 are `compile_fail`). Without features, 59
pass. One more test is ignored by default: the
full `NotNanF32` walk (about 1 minute). It passes when run. Default clippy is clean
on 1.99.1 with all features.

- **Law kit** (`laws`): encoding round trip and order, extrema, canonical
  cuts, `succ`/`pred`, words between neighbors encode no value,
  cover-preserving steps, rank steps, region, hull, and Boolean-algebra laws.
- **Unlawful impls are detected:** a skipping `succ`, an order-reversing
  encoding, a lossy encoding, a wrong `CoverPreserving` marker, a wrong
  `rank`, and a wrong `MIN`.
- **Compile-time guarantees** (`compile_fail`, error text checked by hand):
  mixed domains (E0308); intervals over a finite domain with no order (E0599);
  `Discrete` without `succ` (E0277); `count()` on a domain with no rank
  (E0599); a member query on a domain with a missing extremum (E0080).
- **Exhaustive models:** all 676 × 676 pairs of `Month` intervals; all 4,096
  subsets of `Month`, also through `to_words()` and `from_words()`; all 128
  subsets of `Weekday`; all 100 cyclic intervals of `Weekday`; 256 down-sets.
- **Maps:** 4,000 random cases against a 12-point model for every operation;
  the sweep build equals a fold of overlays.
- **Mutable sets:** all 4,096 subsets of `Month` against every single insert
  and remove, also on the complement; every subset built from empty by
  inserts and from `all()` by removes; 2,000 random toggle sequences on a
  tree with `L = F = 4` (height 5 or more), with every invariant checked
  after every step; 300 random sequences of up to 3,000 `i64` updates
  against a `BTreeSet` model; edges `{MIN}`, `{MAX}`, the `char` gap, and
  dense domains with and without extrema.
- **Mutation check for revision 7:** 19 injected bugs in the tree and
  `IntervalSet`. The tests caught 18. The other one picks the right sibling
  instead of the left one for a merge; both are correct.
- **Mutation check for revision 4:** 18 injected bugs. The tests caught 17.
  The other one is an equivalent change (on a tie it pushes the equal boundary
  from the other side), so it cannot be detected.

## 11. Open findings and decisions

| # | Finding | Status |
|---|---|---|
| R1 | Orphan rule: a user cannot implement the traits for a third-party type. | chrono, jiff, and time are shipped behind features. For other crates, `newtype_domain!` needs one line. |
| R2 | A missing extremum fails at `cargo build`, not at `cargo check`. | Open. |
| R3 | A logical trait named a storage layout. | **Closed in revision 4**: the layout is computed from density and word. |
| R4 | `Interval<u64>` is 32 bytes (two cuts). | Open. Sets and maps do not store `Interval`. |
| R5 | `FiniteSet` holds at most 128 values. | Open. |
| R6 | `DownSet` has no up-set and scans all generators. | Open. |
| R7 | An unlawful impl can give wrong sets, or panic in one place (a stored word that does not decode). It cannot cause undefined behavior: the crate has `#![forbid(unsafe_code)]`. | Run the law kit for every impl. |
| R8 | The base trait was `FrozenBox`, but the capability traits were `*Domain`. | **Closed in revision 6**: the base trait is `Domain`. |
| R9 | The float types `F32`/`F64` were easy to confuse with `f32`/`f64`. | **Closed in revision 6**: `NotNanF32`, `NotNanF64`. |
| R10 | A discrete domain with a non-`Copy` word (a big integer) compares boundaries by reference. It is supported by design but not tested. | Open. |
| R11 | `IntervalSet::contains` is about 2 times slower than `FrozenIntervalSet::contains`. Each node keeps its arrays in separate `Vec` buffers, so a descent reads several cache lines per level. | Open. Next candidate: fixed-size arrays in nodes for `Copy` words, and leaves in their own arena. |
| R12 | `insert` makes three descents: one `contains` and two toggles. | Open. A fused descent can find the state and toggle in one pass. |
| R13 | The tree uses 1.8 times the heap memory of the frozen set (half-full leaves and per-node buffers). | Open. |
| R14 | `insert` and `remove` clone the value once to build its lower cut. Free for `Copy` domains; one clone for `String`. `contains` does not clone. | Open. |

## 12. History

- **Revision 1:** type-level domains, cuts, toggle sets.
- **Revision 2:** adversarial review. 12 probes failed against revision 1
  (`docs/review/probe_v0_1.rs`); nine defects fixed, among them non-canonical
  intervals, a forgettable `succ`, and wrong empty-interval algebra.
- **Revision 3:** `IntoInterval`, `Region`, `CyclicInterval`, the map, and domains
  for all common integer, float, and temporal types.
- **Revision 4:** an order-preserving encoding as the core concept; domain-free
  kernels; computed layout; `CoverPreserving` and `Ranked` in place of a
  runtime flag and an optional rank; `newtype_domain!`; word sets;
  one terminology.
- **Revision 5:** names only. `embed`/`preimage`/`Image` became
  `encode`/`decode`/`Word`. Set-level `image`/`preimage` became
  `to_words`/`from_words`.
- **Revision 6:** names only, by the rules of section 3.

  | Before | Now |
  |---|---|
  | `Range`, `CyclicRange`, `IntoRange` | `Interval`, `CyclicInterval`, `IntoInterval` |
  | `FrozenRangeSet`, `FrozenRangeMap` | `FrozenIntervalSet`, `FrozenIntervalMap` |
  | `ranges()`, `range_count()`, `from_ranges()`, `closed_ranges()` | `intervals()`, `interval_count()`, `from_intervals()`, `closed_intervals()` |
  | `FrozenBox` (base trait) | `Domain` |
  | `F32`, `F64` | `NotNanF32`, `NotNanF64` |
  | sealed `Word`, then `IntWord` | `MachineWord` |
  | `PartialOrderDomain` | `Poset` |
  | `Product` | `ProductRegion` |
  | `Pos`, `Span` (kernel) | `RawCut`, `RawInterval` |
  | `Limits`, `limits(..)` in `newtype_domain!` | `Extrema`, `extrema(..)` |
  | `succ_wrapping`, `pred_wrapping` | `wrapping_succ`, `wrapping_pred` |
  | `FrozenRangeMap::len` | `FrozenIntervalMap::entry_count` |
  | `FiniteSet::len` (`u32`) | `FiniteSet::count` (`u128`, as for `FrozenIntervalSet::count`) |
- **Revision 7:** `IntervalSet<V>` with single-value `insert` and `remove`
  on `BoundaryTree<E>`, a B+tree of boundaries with child counts.
- **Revision 8:** the prototype became the workspace package `frost-interval`
  (section 13). Features take compatibility-line names; the demonstration
  domains and the law checks moved to `tests/support`.

## 13. Integration into Frost

The package is `crates/frost-interval`. It depends on no other Frost package,
so its names do not depend on theirs (section 3, rule 7). The facade
re-exports it as `frost::interval` behind the feature `interval`.

| Step of revision 7 | State in revision 8 |
| --- | --- |
| A module behind an additive feature, `alloc`, no new required dependencies, no `unsafe` | **Done.** A package, not a module. Facade feature `interval` (not `interval-set`). `#![no_std]`, always `alloc`; `std` adds the `SystemTime` and `Instant` domains. `#![forbid(unsafe_code)]`. |
| An `OrderedDomain` impl for every `IntKey` type | **Open.** It needs a dependency on the integer package or a shared trait; that choice belongs to the API refactor. |
| A law kit for implementors (feature `testing`) | **Open.** The law checks are in `tests/support/laws.rs`. [CRATES.md](../../../CRATES.md) CR-LAYOUT-1 keeps test helpers out of `src/`, so a shipped kit needs its own design. |
| Third-party domains by compatibility-line features | **Done:** `chrono-0_4`, `jiff-0_2`, `time-0_3`, in the package and forwarded by the facade. `rust_decimal-1` and `ipnet-2` are open. |
| Frost's lint set | **Done.** Clippy with `pedantic` and `-D warnings` is clean in the default, no-default and all-features profiles; every public item has rustdoc. Scoped `allow`s state their reason. |
| Catalog records, `make check`, the feature matrix and the embedded build | Catalog records are scaffolded and `unreviewed`; curating them is open. The gates that ran are listed in the change that added the package. |

Not moved from the prototype:

- The criterion benchmarks `zero_cost` and `tree`. Frost benchmarks use the
  shared harness and `FROST_BENCH_LIST` ([CRATES.md](../../../CRATES.md)
  CR-LAYOUT-5); porting them is open.
- The examples `codegen`, `mono` and `tree_mono`, which produce binaries for
  the kernel-sharing evidence of section 9.
- The review probe `docs/review/probe_v0_1.rs` of revision 2.

## 14. Not covered

- Interval insert and remove on `IntervalSet`, set operations between two
  trees, and a mutable map. The map tree stores the value of each segment at
  its start boundary.
- `FrozenIntervalMap` clones values in every merge.
- Decimals, signed `NonZero`, jiff `Zoned`, time `OffsetDateTime`, and
  runtime domains.
- An automatic link from `Ranked` to `FiniteDomain` (it needs a constant
  cardinality, which a trait method cannot give on stable Rust).

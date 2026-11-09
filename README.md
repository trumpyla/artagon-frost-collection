# frost-collections

Mutable and frozen collection families with stable public names, small
capability traits, and optional adapters for native Rust collections. Families
own their mutation and freeze contracts; native backends retain their specialized
APIs. Concurrent collections use a separate access model.

**Target:** **Rust 1.99.1, edition 2024**. `rust-toolchain.toml` pins 1.99.1;
Cargo declares `rust-version = "1.99.1"`, and required CI jobs use that exact
release. Version **0.4.0** adds fast frozen set algebra to the reviewed family,
adaptive-storage, framework and concurrency design. The library is
`no_std`; `alloc` and `std` add capabilities. The default enables only `std`.

This unreleased revision integrates guarded snapshot reads and conditional
publication, retains the reviewed kernel improvements, and adds deterministic
concurrency models, finite exhaustive checks, and isolated proof harnesses.
[Current validation](docs/integer-output-validation.md) distinguishes executed gates
from the remaining release requirements. The [previous performance review](docs/reviews/performance-review.md)
and [incoming patch assessment](docs/reviews/incoming-assessment.md) preserve the
measurements, tradeoffs, and reasons for each integration decision.

## Core API

This example works even with every feature disabled:

```rust
use frost::prelude::*;

let frozen = [3, 1, 2].seal();
assert_eq!(frozen.get(1), Some(&1));
assert_eq!(frozen.len(), 3);
let mut array = frozen.into_inner();
array.sort_unstable();
assert_eq!(array, [1, 2, 3]);
```

With the default features:

```rust
# #[cfg(feature = "std")]
# {
use frost::prelude::*;
use std::collections::{BTreeMap, HashMap, HashSet};

let retained: Frozen<Vec<i32>> = vec![1, 2, 3].seal();
let compact: FrozenVec<i32> = vec![1, 2, 3].compact();
let ordered: FrozenOrderedMap<_, _> = BTreeMap::from([(2, 'b'), (1, 'a')]).compact();
let hashed: FrozenHashMap<_, _> = HashMap::from([("task", 7)]).compact();
let members: FrozenHashSet<_> = HashSet::from(["ready", "done"]).compact();

assert_eq!(retained.as_ref(), compact.as_ref());
assert_eq!(ordered.iter().map(|(k, _)| *k).collect::<Vec<_>>(), [1, 2]);
assert_eq!(hashed.get("task"), Some(&7));
assert!(members.contains("ready"));
# }
```

A frozen collection has no structural mutation API or mutable element access.
Leaving that state consumes the frozen value. Interior mutability **inside an
element** is still possible; frozen does not imply deeply immutable or pinned.

```compile_fail,E0594
use frost::prelude::*;
let mut frozen = [1, 2].seal();
frozen[0] = 9;
```

```compile_fail,E0451
let _ = frost::Frozen { inner: [1, 2] };
```

## Mutable families and fixed partners

Use a family name in application signatures when the backend should remain an
implementation detail. Entry and map-iterator handles are crate-owned, and map
families do not dereference to their native backends.

```rust
# #[cfg(feature = "std")]
# {
use frost::prelude::*;

let mut sequence = MutableVec::new();
sequence.push(1);
let mut sequence: MutableVec<_> = sequence.freeze().thaw();
sequence.push(2);
assert_eq!(sequence.freeze().as_slice(), &[1, 2]);

let mut counts = MutableHashMap::new();
counts.entry(String::from("verified")).and_modify(|n| *n += 1).or_insert(1);
let frozen: FrozenHashMap<_, _> = counts.freeze();
assert_eq!(frozen.get("verified"), Some(&1));
let mut counts: MutableHashMap<_, _> = frozen.thaw();
assert_eq!(counts.remove("verified"), Some(1));

let mut ordered = MutableOrderedMap::new();
ordered.insert("b", 2);
ordered.insert("a", 1);
assert_eq!(ordered.freeze().keys().copied().collect::<Vec<_>>(), ["a", "b"]);

let members: MutableOrderedSet<_> = [3, 1, 3].into_iter().collect();
assert_eq!(members.freeze().iter().copied().collect::<Vec<_>>(), [1, 3]);
let members: MutableHashSet<_> = ["ready", "done"].into_iter().collect();
assert!(members.freeze().contains("ready"));
# }
```

Each family has one named frozen partner:

| Mutable | Frozen | Consuming native extraction |
| --- | --- | --- |
| `MutableVec<T>` | `FrozenVec<T>` | `into_vec()` |
| `MutableHashMap<K, V, S>` | `FrozenHashMap<K, V, S>` | `into_map()` |
| `MutableOrderedMap<K, V>` | `FrozenOrderedMap<K, V>` | `into_map()` |
| `MutableHashSet<T, S>` | `FrozenHashSet<T, S>` | `into_set()` |
| `MutableOrderedSet<T>` | `FrozenOrderedSet<T>` | `into_set()` |

Call `freeze()` to consume a mutable family and `thaw()` to recover its paired
mutable family. Explicit native conversions support integration with existing
libraries. There is no exact-backend `Mutable<B>` wrapper or universal map enum.
The canonical modules are `family::vec`, `family::hash_map`, `family::ordered`,
`family::hash_set`, and `family::sets`. `frost::mutable` provides convenient
reexports. Incoming native extraction names such as `into_hash_map` and
`into_btree_map` remain available alongside `into_map` aliases.

## Explicit adaptive profiles

Adaptive profiles are separate public types with closed representations. Cargo
features never add a variant to an existing type. Removal does not automatically
demote a mutable profile; `shrink_to_fit` makes that decision explicit.

| Mutable profile | Storage choices | Frozen result and recovery |
| --- | --- | --- |
| `InlineVec<T, N>` | inline buffer / Vec | `InlineFrozenVec<T, N>`; `thaw` returns `InlineVec` |
| `AdaptiveHashMap<K, V, S>` | linear entries / hash table | `AdaptiveFrozenHashMap<K, V, S>`; `thaw_with(policy)` restores a chosen growth policy |
| `AdaptiveOrderedMap<K, V>` | sorted vector / B-tree | `FrozenOrderedMap<K, V>`; explicit `AdaptiveOrderedMap::from_frozen(frozen, policy)` reconstructs the profile |

`AdaptiveFrozenHashMap::thaw()` uses the default growth threshold. It preserves
its hashing policy and current representation, but does not recover a former
nondefault threshold. Use `thaw_with` when that threshold matters.
`FrozenOrderedMap::thaw()` always returns `MutableOrderedMap`; there is no
ambiguous inverse selected by a feature or runtime tag.

```rust
# #[cfg(feature = "std")]
# {
use frost::prelude::*;
use frost::adaptive::HashRepr;

let policy = AdaptiveHash::linear_up_to(2);
let mut map = AdaptiveHashMap::with_policy(policy);
map.extend([("a", 1), ("b", 2), ("c", 3)]);
assert_eq!(map.representation(), HashRepr::Table);
let mut map = map.freeze().thaw_with(policy);
map.remove("c");
assert_eq!(map.representation(), HashRepr::Table);
map.shrink_to_fit();
assert_eq!(map.representation(), HashRepr::Linear);
# }
```

## Frozen set operations

`FrozenOrderedSet` and `FrozenHashSet` provide lazy borrowed `union`,
`intersection`, `difference`, and `symmetric_difference` iterators. They allocate
no intermediate set and require no element cloning. Union and intersection keep
the left set's stored representative when two elements are equivalent.

Use `is_subset`, `is_superset`, and `is_disjoint` for relations, or
`intersection_len`, `union_len`, `difference_len`, and
`symmetric_difference_len` when only the result count is needed. The scalar
queries allocate no result collection.

```rust
# #[cfg(feature = "alloc")]
# {
use frost::family::FrozenOrderedSet;

let left: FrozenOrderedSet<_> = [1, 2, 4, 7].into_iter().collect();
let right: FrozenOrderedSet<_> = [2, 3, 4, 8].into_iter().collect();
assert_eq!(left.intersection_len(&right), 2);
assert_eq!(left.union_len(&right), 6);
assert_eq!(left.difference(&right).copied().collect::<Vec<_>>(), [1, 7]);

// Materialize when an independently owned result is required.
let common: FrozenOrderedSet<_> = left.intersection(&right).copied().collect();
assert!(common.is_subset(&left));
# }
```

Frozen Roaring bitmaps/treemaps and `FixedBitSet` backends expose the same named
algebra and scalar queries under their existing features. Their algebra methods
produce a frozen bitmap using the native representation's word/container
kernels. `FixedBitSet` operations ignore raw padding and document the resulting
logical universe for differently sized operands.

See `docs/set-operations.md` for result ownership, complexity and boundary contracts,
and `examples/set_operations.rs` for the tested complete scenario.

## Feature catalog

Every feature is additive. Enabling one never changes an existing public type's
representation or selects a different hasher. Backends are reexported from
`frost::backend`, with their native specialized APIs intact.

| Feature | Backends | Environment |
| --- | --- | --- |
| none | arrays and slice capabilities | `core` |
| `alloc` | Vec, boxed slices, deque, linked list, heap, B-tree map/set; ordered and sequence families | `alloc` |
| `std` (default) | standard hash map/set and their mutable/frozen families | `std` |
| `hashbrown-0_17` | `SwissTable` hash map/set | `alloc`; supply a hasher explicitly |
| `indexmap-2` | insertion-order map/set | `alloc`; `std` enables its default hasher |
| `smallvec-1` | inline vectors that can spill | `alloc` |
| `arrayvec-0_7` | fixed-capacity inline vectors | `core`; compact conversion needs `alloc` |
| `imbl-7` | persistent vector, hash map/set, ordered map/set | `std` |
| `roaring-0_12` | compressed `u32` and `u64` integer sets | `std` |
| `int-set` | typed frozen integer profile with fixed sorted/dense/Roaring candidates | `alloc` |
| `fixedbitset-0_5` | dense bitsets | `alloc` |
| `slotmap-1` | generational slot maps | `alloc` |
| `phf-0_14` | compile-time perfect-hash maps/sets, including declaration-order variants | `core` at runtime |
| `fst-0_4` | compressed byte-string maps/sets and lending streams | `std` |
| `dashmap-6` | sharded concurrent maps/sets | `std` |
| `scc-3` | concurrent hash maps/sets | `std` |
| `crossbeam-skiplist-0_1` | concurrent ordered skip maps/sets | `std` |
| `arc-swap-1` | atomic publication of whole frozen snapshots | `std` |
| `serde-1` | logical serialization/deserialization and strict decoding helpers | `alloc` |
| `rayon-1` | parallel traversal, construction and extension | `std` |
| `arbitrary-1` | generation for family and adaptive types | `std` |
| `borsh-1` | canonical family serialization and strict family decoding | `alloc` |
| `schemars-1` | schemas for the logical Serde shape | `alloc` |
| `frameworks` | all five framework integrations | `std` |
| `advanced` | the eight advanced nonconcurrent backend features above | `std` |
| `concurrent` | the four concurrent/publication features above | `std` |

For this source distribution, use a path dependency:

```toml
[dependencies]
frost = { package = "frost-collections", path = "../frost", features = ["smallvec-1", "roaring-0_12", "concurrent"] }
```

This archive is not a claim that version 0.4.0 has been published to crates.io.

## Typed frozen integer sets

Enable `int-set` for `FrozenIntSet<K>`, a separate profile for lossless integer
codecs. `IntKey::Word` selects `u32` or `u64` at compile time. The four ordinary
signed/unsigned primitives and `NonZeroU32`/`NonZeroU64` are supported; applications
can implement `IntKey` for their own domain types. Signed encodings preserve
numeric order by flipping the sign bit.

```rust
# #[cfg(feature = "int-set")]
# {
use frost::integer::{FrozenIntSet, IntSetPolicy};
use frost::TryFreezeWith;

let left = vec![-4_i64, -1, 0, 7].try_freeze_with(IntSetPolicy::auto()).unwrap();
let right: FrozenIntSet<i64> = [-1, 7, 8].into_iter().collect();
assert_eq!(left.intersection(&right).iter().collect::<Vec<_>>(), [-1, 7]);
assert_eq!(left.union_len(&right), 5);
# }
```

The complete profile has fixed sorted, dense-window, and width-specific Roaring
candidates. Automatic construction makes a bounded, memory-oriented choice;
explicit policies and representation diagnostics support workload testing. Dense
windows use absolute encoded coordinates, including for differently based sets.
Cardinality is `u64`; inclusive spans use `u128` so the full `u64` domain does not
overflow. This profile supports `no_std + alloc`; enabling `std` also enables
the dependency's supported runtime CPU dispatch. The existing `roaring-0_12`
integration keeps its prior `std` requirement and native adapter API.
For a manifest that disables default features, request `features = ["int-set", "std"]`
if it also needs standard-library families or the dependency's std APIs. The
earlier integer-profile revision enabled `std` transitively; this revision makes
that choice explicit.

Algebra preserves compact outputs: small updates can clone and edit Dense bits,
while distant Dense windows can promote to Roaring. Each set retains its Dense
payload and Roaring partition limits. Union, intersection, and symmetric
difference use the smaller limit from each operand; difference keeps the left
limits. A forced construction backend is a preference for that construction,
not a permanent result type. Use `union_with_policy` and the corresponding
methods for the other operations to choose an explicit result policy. Limits do
not bound sorted buffers, temporary allocations, or total process memory.

Same-representation equality uses native or contiguous comparisons. Hashing
selects a canonical member or run stream from logical contents, so equal sets
hash identically across storage types and construction histories. The hash bytes are not a serialization format.
Count operations keep values encoded; some Dense/Roaring counts allocate
temporary conversion storage to avoid individual membership probes.

Iteration yields decoded keys by value. Bulk set operations use canonical codes
without reconstructing keys. Checked imports reject invalid and noncanonical
codes and test ordering on the observed input. Custom codecs must still preserve
the complete value and remain stable; equality alone cannot verify discarded
metadata. Ordinary ordered sets retain their existing `freeze()` result.

See [the integer design](docs/design/integer-sets.md) and
[adversarial review](docs/reviews/integer-design-audit.md) for contracts, planner
limits, and implementation decisions. The [output review](docs/reviews/integer-output-review.md),
[current validation](docs/integer-output-validation.md), and
[comparative benchmarks](docs/integer-output-benchmarks.md) record the follow-up
corrections and the evidence actually produced. Proposed new collection families
remain in the [integer roadmap](docs/design/integer-roadmap.md).

The [lazy-views research review](docs/reviews/lazy-views-adversarial-review.md)
proposes explicit borrowed [integer expressions](docs/design/integer-views.md)
and a separate [persistent integer profile](docs/design/persistent-integer-storage.md).
Those types are design proposals, not implemented APIs. Current integer algebra
remains eager. The review preserves the supplied research, checks primary
sources, and includes an independent finite model of cardinality bounds.

## Framework contracts

The five framework integrations cover the ordinary mutable/frozen families and
the applicable adaptive profiles, including the mutable ordered set and both
hash-set families. Frozen adapters expose only read-only traversal.

- Serde encodes logical elements or map entries. Family decoding applies the
  family duplicate policy; strict helpers reject duplicates or unsorted input.
- Borsh family encoders use ascending keys/elements for maps/sets, and family
  decoders reject nonascending or duplicate input. Exact `Frozen<B>` delegates
  to the native backend's wire contract. Different stored representatives may
  serialize differently even when a custom key equality ignores their metadata.
- Decoding reconstructs default hashers and adaptive policies. Runtime policies,
  capacities, and storage variants are not serialized.
- Rayon results follow the family duplicate rule when the parallel source has a
  defined encounter order. Unordered sources do not acquire deterministic winners.
  Slice-backed profiles support indexed parallel traversal. Native hash/tree
  parallel traversal may allocate a temporary buffer; this is not hidden by an
  allocation-free claim.
- Arbitrary generation exercises adaptive variants with their invariants intact.
  Schemars describes the logical data shape, not internal representation fields.

```rust
# #[cfg(feature = "serde-1")]
# {
use frost::prelude::*;
// serde_json is the format crate used by this example.
let map: FrozenOrderedMap<String, u32> =
    serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
assert_eq!(map.keys().map(String::as_str).collect::<Vec<_>>(), ["a", "b"]);
assert_eq!(map.thaw().get("b"), Some(&2));
# }
```

## Contracts that matter

- `LookupRef` returns a stored key and value; `ValueRef` lends only a value;
  `LookupValue` decodes a value. Compressed storage does not fabricate references.
- `Elements` lends stored elements; `ValueElements` yields reconstructed values.
  `Cardinality` preserves a bitmap's natural `u64` count.
- Frozen families keep the **first equivalent key/element representative** and,
  for maps, the **last value**. Strict constructors reject duplicates.
  Exact `Frozen<B>` wrappers preserve the native backend's construction rules.
- `Retain` does not rebuild or allocate. Moving inline storage costs up to the
  inline representation's size. `Compact` may allocate, move, shrink or rehash;
  it does not promise that every backend has zero spare capacity.
- Hashers are preserved. Feature activation never silently switches to a faster
  non-cryptographic hasher. Choose a hasher for your input and threat model.
- Adaptive thresholds are caller policies, not benchmark-derived recommendations.
- Concurrent callbacks keep guards scoped. They are synchronous; do not block,
  re-enter the same collection, or await while accessing it under a guard.
  Live traversal/length observations do not constitute a consistent snapshot.
- Publish a complete frozen value when readers need a stable whole-collection
  view. Element interior mutability remains the caller's responsibility.

## Snapshot reads and optimistic updates

With `arc-swap-1`, load one version for a consistent structural view. Use a
short-lived guard or `peek` for scoped reads; use `load` or `guard.into_owned()`
to retain an owned Arc. Conditional publication compares allocation identity.
`update` retries after conflict and returns the displaced predecessor; its
callback may run more than once.

```rust
# #[cfg(feature = "arc-swap-1")]
# {
use frost::concurrent::SnapshotStore;
use frost::family::FrozenVec;

let store = SnapshotStore::new(FrozenVec::from([0_usize]));
let before = store.load_guard();
let displaced = store.update(|version| FrozenVec::from([version.as_slice()[0] + 1]));
assert_eq!(displaced.as_slice(), &[0]);
assert_eq!(before.as_slice(), &[0]);
assert_eq!(store.peek(|version| version.as_slice()[0]), 1);
# }
```

Guarded reads can fall back to reference counting. Contention, allocation,
callbacks, and destructors prevent an unconditional bounded-latency guarantee.
[The snapshot contract](docs/design/snapshots.md) explains ownership, retries,
panics, progress, and old-version retention.

## Examples and verification

The `examples/` programs contain assertions and tests that execute their actual
scenarios. Cargo declares their feature requirements. This README is included
in rustdoc, so its Rust blocks are doctested.
The verification script requires Bash and Python 3; it uses only Python's
standard library.

```sh
rustup toolchain install 1.99.1 --profile minimal --component clippy,rustfmt
cargo test --all-features --locked
cargo test --examples --all-features --locked
./scripts/verify.sh
```

See [the documentation index](docs/README.md),
[design philosophy](docs/design/philosophy.md),
[contributor guide](CONTRIBUTING.md), and
[maintainer notes](docs/maintainers/guide.md). [AGENTS.md](AGENTS.md) records the
same invariants and verification expectations for automated changes.
[Current validation](docs/integer-output-validation.md) records executed results;
[verification scope](docs/verification.md) states model assumptions and bounds.
No throughput or latency advantage is claimed without workload-specific evidence.

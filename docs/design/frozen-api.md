# Frozen API guide

Status: **editorial guide to implemented contracts**, inspected at source
revision `82becb9` on 2026-11-09, plus a separately marked proposal section.
Each implemented claim links to the source that defines it and to tests that
exercise it. When this guide and rustdoc disagree, the source and its tests
win; report the drift.

Read [philosophy](philosophy.md) for the principles and
[architecture](architecture.md) for the complete structure. This guide answers
one question: what does "frozen" promise in Frost today, and what does it cost?

Claim labels used below:

| Label | Meaning |
| --- | --- |
| Implemented | Present in `src/` at the inspected revision and exercised by a linked test or doctest |
| Proposal | Described in a design or research document; no production API exists |
| Outside scope | Not a goal of this library |

## 1. Four kinds of frozen value

Frost has no single `FrozenValue` type and no universal `thaw()` method. It has
four distinct kinds of frozen value, each with its own contract.

| Kind | Types | Semantics come from | Leave by |
| --- | --- | --- | --- |
| Exact-backend wrapper | `Frozen<B>` for an audited backend `B` | The native backend `B` | `into_inner()`, which returns `B` |
| Ordinary frozen family | `FrozenVec`, `FrozenHashMap`, `FrozenOrderedMap`, `FrozenHashSet`, `FrozenOrderedSet` | Frost's family contract | `thaw()` to the fixed mutable partner, or native extraction |
| Adaptive frozen profile | `InlineFrozenVec<T, N>`, `AdaptiveFrozenHashMap<K, V, S>` | Frost's family contract plus an explicit policy | `thaw()` on both; `AdaptiveFrozenHashMap` also offers `thaw_with(policy)` to restore a chosen growth policy |
| Typed integer profile | `FrozenIntSet<K>` (`int-set`) | The `IntKey` codec laws | No consuming exit or thaw: `iter()` borrows the set and yields decoded keys to collect elsewhere |

The frozen ordered map is shared by the ordinary and adaptive ordered families.
`FrozenOrderedMap::thaw()` always returns `MutableOrderedMap`;
`AdaptiveOrderedMap::from_frozen(frozen, policy)` is the explicit alternative.

Sources: [exact wrapper](../../crates/frost-collections-core/src/frozen.rs), [family modules](../../crates/frost-collections-core/src/family/mod.rs),
[adaptive profiles](../../crates/frost-collections-core/src/adaptive/mod.rs), [integer profile](../../crates/frost-collections-integer/src/integer/mod.rs).
Tests: [families](../../crates/frost-collections/tests/families.rs), [mutable sets](../../crates/frost-collections/tests/mutable_sets.rs),
[contracts](../../crates/frost-collections/tests/contracts.rs).

### Exact-backend `Frozen<B>`

`Frozen<B>` keeps backend `B` and all of its native rules: construction,
comparison, traversal order, duplicate handling, and wire format. For example,
`Frozen<BTreeMap<K, V>>` built with `collect()` uses `BTreeMap`'s own duplicate
behavior, not the family's first-key rule.

- **Sealed eligibility.** `Eligible` has a crate-private supertrait. Only
  backends audited by this crate qualify: no operation reachable through `&B`
  may change structure, and `Clone` must not share structure with a mutable
  alias. Downstream crates cannot add backends; they define their own frozen
  type and implement the open `read` traits.
- **Curated construction.** `Eligible` gates `Default` and `FromIterator`.
  Every other path is an explicit freeze plan.
- **No backend escape.** There is no `Deref<Target = B>` and no shared backend
  accessor, because a shared reference to some backends exposes mutation.
  Contiguous sequences dereference to `[T]` only.
- **Explicit exit.** `into_inner()` consumes the wrapper and returns `B`
  without allocating or rebuilding. Layout and `repr(transparent)` are not
  part of the API.
- Live concurrent collections are not eligible. Exact FST freezing accepts
  only audited immutable byte storage, not any user `AsRef<[u8]>` type.

Sources: [`Eligible` and `Frozen`](../../crates/frost-collections-core/src/frozen.rs),
[FST eligibility](../../crates/frost-collections-core/src/backends/fst.rs). The compile-fail doctests in
`src/frozen.rs` and the README check the missing `Deref`, mutable access, and
private construction.

### Ordinary families and fixed partners

| Mutable | Frozen | `thaw()` cost | Native extraction |
| --- | --- | --- | --- |
| `MutableVec<T>` | `FrozenVec<T>` | Recovers the vector | `into_vec()`, `into_boxed_slice()` |
| `MutableHashMap<K, V, S>` | `FrozenHashMap<K, V, S>` | O(1); same table and hasher | `into_map()`, `into_hash_map()` |
| `MutableOrderedMap<K, V>` | `FrozenOrderedMap<K, V>` | Rebuilds a B-tree | `into_map()`, `into_btree_map()` |
| `MutableHashSet<T, S>` | `FrozenHashSet<T, S>` | Recovers the table | `into_set()`, `into_hash_set()` |
| `MutableOrderedSet<T>` | `FrozenOrderedSet<T>` | Rebuilds a B-tree | `into_set()`, `into_btree_set()` |

Each mutable family's `freeze()` returns exactly its named partner, and each
frozen family's `thaw()` returns exactly its named mutable partner. Features
never change either result. Owning iteration also consumes a frozen family and
ends the frozen state.

Sources: [vector](../../crates/frost-collections-core/src/family/vec/frozen.rs), [hash map](../../crates/frost-collections-core/src/family/hash_map/frozen.rs),
[ordered map and set](../../crates/frost-collections-core/src/family/ordered/frozen.rs), [hash set](../../crates/frost-collections-core/src/family/hash_set.rs),
[set families](../../crates/frost-collections-core/src/family/sets.rs).

## 2. Construction paths and plans

| Path | Plan or method | Output | Cost contract |
| --- | --- | --- | --- |
| Keep the representation | `seal()` = `freeze_with(Retain)` | `Frozen<B>` | No allocation or rebuild. Moving an inline backend moves its inline elements. |
| Compact representation | `compact()` = `freeze_with(Compact)` | The family's compact type, or a specialized exact wrapper | May allocate, shrink, rehash, or move elements. It does not promise zero spare capacity. |
| Mutable family | `freeze()` | The fixed frozen partner | Family-specific; ordered families flatten their already ordered tree into sorted boxed storage in O(n) with no comparisons |
| Heap to sequence | `freeze_with(Sorted)` | Ascending sequence | O(n log n), in place. A heap has no ambiguous `Compact` plan. |
| Fixed length | `try_freeze_with(ExactLen::<N>)` | `Frozen<[T; N]>` | Fallible; returns the input on length mismatch |
| Inline profile | `freeze_with(Inline::<N>)` | `InlineFrozenVec<T, N>` | Moves up to `N` elements inline, otherwise boxes |
| Adaptive hash | `freeze_with(AdaptiveHash)` | `AdaptiveFrozenHashMap` | Linear entries below a threshold, otherwise a shrunk table |
| Integer profile | `try_freeze_with(IntSetPolicy)` | `FrozenIntSet<K>` | Validates codes, normalizes with O(n) staging (no comparison sort for ascending or descending input; a temporary bitmap for dense unsorted input; otherwise one comparison sort), selects bounded storage |

`FreezeWith<P>` gives each source and plan pair one associated `Output`.
`TryFreezeWith<P>` adds an associated `Error`. A missing plan is a compile-time
error, never a silent fallback. `FreezeExt` is a blanket trait with a closed
method set, `seal` and `compact`; new plans use `freeze_with` or
`try_freeze_with` so that blanket methods cannot collide with downstream
extension traits.

Sources: [plan definitions](../../crates/frost-collections-core/src/plan.rs), [vector plans](../../crates/frost-collections-core/src/backends/std_seq.rs),
[integer plans](../../crates/frost-collections-integer/src/integer/set.rs). Tests: [contracts](../../crates/frost-collections/tests/contracts.rs),
[core review](../../crates/frost-collections/tests/core_review.rs), [inline ownership](../../crates/frost-collections/tests/inline_safety.rs).

## 3. Errors decide what you can recover

Not every error returns its input. The error **type** tells you what you get
back. Panics and allocation aborts are never reported as `Err`.

| Operation | Error type | What the caller recovers |
| --- | --- | --- |
| `try_freeze_with(ExactLen::<N>)` on a vector | `Rejected<Vec<T>, LengthMismatch>` | The unchanged input |
| `try_freeze_with(IntSetPolicy)` on `Vec<K>`, `MutableOrderedSet<K>`, or `FrozenOrderedSet<K>` | `Rejected<S, IntSetError>` | The unchanged input |
| `try_from_sorted_unique` on the frozen ordered map or set | `Rejected<Vec<_>, NotSortedUnique>` | The unchanged input; validation happens before commitment |
| Family `try_from_unique` builders | `DuplicateKey<K, V>` or `DuplicateElement<T>` | Only the rejected duplicate; other consumed entries are dropped |
| `FrozenIntSet::try_from_iter` and `try_from_encoded` | `IntSetError` | Nothing; the consumed iterator is gone |

Use a plan that returns `Rejected` when the caller needs the source back after
a failure. Strict `try_from_unique` builders follow two different protocols:

| Builders | Validation order | Duplicate returned |
| --- | --- | --- |
| `MutableHashMap`, `FrozenHashMap`, `MutableHashSet`, `FrozenHashSet`, `MutableOrderedMap` | Input order, by insertion | The first duplicate encountered in input order |
| `FrozenOrderedMap`, `FrozenOrderedSet`, and `MutableOrderedSet` (which delegates to the frozen set) | Collect, stable-sort, then scan | The later occurrence of the smallest duplicated key or element |

With several duplicates, the two protocols can reject different items.

Sources: [`Rejected`](../../crates/frost-collections-core/src/plan.rs), [duplicate errors](../../crates/frost-collections-core/src/family/mod.rs),
[ordered validation](../../crates/frost-collections-core/src/family/ordered/frozen.rs), [integer errors](../../crates/frost-collections-integer/src/integer/set.rs).

## 4. Duplicate and representative laws

- **Families:** construction keeps the **first** equivalent key or element and
  the **last** map value. Replacing a map value never replaces the stored key.
  Strict builders reject duplicates instead.
- **Set algebra:** union and intersection keep the **left** operand's stored
  representative, even when a hash intersection scans the smaller right set.
  Symmetric difference yields each member from the only operand containing it.
- **Exact wrappers:** native backend rules apply, including unspecified ones.
- **Integer profile:** duplicates collapse by canonical code. The codec must
  reconstruct the complete key, so no representative metadata exists to keep.
  Use an ordinary family when stored metadata matters.

These rules rely on lawful user traits. Equivalent keys must hash alike,
borrowed queries must agree with their keys, ordering must be coherent, and
interior mutation must not change a key's equivalence or order. Freezing does
not repair inconsistent trait implementations. The README's
[representative example](../../crates/frost-collections/README.md#contracts-that-matter) is a doctest
with hand-written `PartialEq`, `Eq`, and `Hash` that use the same field.

Sources: [family laws](../../crates/frost-collections-core/src/family/mod.rs), [hash set algebra](../../crates/frost-collections-core/src/family/hash_set.rs).
Tests: [contracts](../../crates/frost-collections/tests/contracts.rs), [hash set operations](../../crates/frost-collections/tests/hash_set_ops.rs),
[ordered set operations](../../crates/frost-collections/tests/ordered_set_ops.rs), [set review](../../crates/frost-collections/tests/sets_review.rs).

## 5. Three read access models

No single iterator contract fits every representation. Frost keeps three
access models separate, and a backend implements only the traits it can honor.

| Model | Traits or API | What a caller receives | Example backends |
| --- | --- | --- | --- |
| Borrowed stored data | `Elements`, `Entries`, `LookupRef`, `SetGet`, `ValueRef` | References into storage. `Elements` and `Entries` traverse stored elements and keys; `LookupRef` and `SetGet` return the stored representative for a query; `ValueRef` lends a value only, with no stored key | Families, `Frozen<B>` over std and most native backends |
| Values by copy or decoding | `ValueElements`, `LookupValue`, `SetContains`, `Cardinality` | Reconstructed values, booleans, `u64` counts | `FrozenIntSet`, Roaring, `FixedBitSet`, FST lookup |
| Lending streams | Native `fst` `Streamer` | A key borrowed from a buffer that is reused on the next advance | FST maps and sets |

- `LookupRef<Q>` returns the stored key and value. The relation between `Q` and
  the key (`Borrow` with `Hash + Eq`, `Ord`, or a backend's `Equivalent`) is a
  bound on each implementation, never a blanket bound on the trait.
- `ValueRef` lends a value without inventing a stored key reference, as for
  generational slot maps. `LookupValue` returns a decoded value, as for FST.
- `Cardinality` keeps a natural `u64` count; it is never truncated to `usize`.
  A bitset's member count is not its addressable universe.
- FST streams are not exposed as `Entries` or `Elements`, because a lent key is
  valid only until the next advance. Use the native streaming API.

`LookupRef`, `SetContains`, and `SetGet` have no generic associated types or
generic methods, so they are dyn-compatible. Traversal traits use generic
associated types and are not.

Sources: [read capabilities](../../crates/frost-collections-core/src/read.rs), [FST](../../crates/frost-collections-core/src/backends/fst.rs),
[bitmaps](../../crates/frost-collections-core/src/backends/bitmap.rs), [slot maps](../../crates/frost-collections-core/src/backends/slotmap.rs).
Tests: [backend review](../../crates/frost-collections/tests/backend_review.rs), [specialized](../../crates/frost-collections/tests/specialized.rs).

## 6. What frozen does and does not mean

Frozen is **structural** immutability: no insertion, removal, reordering, or
mutable element access while the value stays frozen.

It does **not** mean:

- **Deep immutability.** Cells, atomics, locks, and shared pointers inside
  elements keep their interior mutability.
- **Pinning.** A frozen value can move. Moving inline storage moves elements.
- **Cheap cloning.** Cloning an ordinary family copies its storage.
  `FrozenIntSet` shares storage on clone; an exact wrapper clones as its
  native backend does, which for persistent backends can share nodes.
- **Serialization compatibility.** Wire formats encode logical contents; equal
  collections can serialize differently when equality ignores metadata.
- **Freshness.** A frozen value says nothing about whether an outside source
  has changed since it was built.

Auto traits follow the payload. Frost has no manual `Send` or `Sync`
implementations. `FrozenIntSet<K>` keeps `K`'s restrictions through
`PhantomData<K>` and the selected shared pointer's own auto traits.

## 7. Costs that stay visible

| Operation | Cost |
| --- | --- |
| `seal()` / `Retain` | No allocation or rebuild; moves the backend value |
| `compact()` / `Compact` | May allocate, shrink, rehash, or move; spare capacity can remain |
| Mutable ordered map or set `freeze()` | O(n) flatten of the already ordered tree; no comparisons |
| Frozen ordered map or set built from an unsorted iterator (`collect`) | Collects into a vector, stable-sorts (which can allocate temporary storage), then deduplicates in place |
| `try_from_sorted_unique` | O(n) validation of already ascending input; adopts the vector |
| Ordered `thaw()` | Rebuilds a B-tree |
| Hash `thaw()` | O(1) for the map; keeps the table and hasher |
| `Frozen<B>::into_inner()` | Moves `B`; no allocation |
| `FrozenIntSet` clone | One reference-count increment |
| `FrozenIntSet::new()` | Allocates a shared storage header, even when empty |
| Changed Chunked result | Shares untouched containers; copies the flat directory in O(C) for C containers |
| Final owner of shared integer storage | Releases every reachable container |

Sources: [plans](../../crates/frost-collections-core/src/plan.rs), [ordered family](../../crates/frost-collections-core/src/family/ordered/frozen.rs),
[integer storage](../../crates/frost-collections-integer/src/integer/set.rs). The [set operations](../set-operations.md)
guide records algebra complexity; the [benchmark method](../maintainers/benchmarks.md)
governs any timing claim.

## 8. Typed integer profile

`FrozenIntSet<K>` (feature `int-set`, requires `alloc`) is a separate profile,
not a variant of an ordinary family.

- **Safe, extensible codec.** Applications implement `IntKey`. The trait is
  safe; its laws require a lossless, canonical, order-preserving, stable code.
  Violations can produce errors, checked panics, or wrong logical answers, but
  Frost never uses codec laws to justify unchecked memory access.
- **Sealed widths.** `IntKey::Word` must be `u32` or `u64`; `IntWord` is sealed
  by a private storage bound.
- **Checked imports.** Input order and duplicates do not matter: construction
  sorts and deduplicates codes. It rejects invalid or noncanonical codes, keys
  that do not round-trip, and any observed case where code order disagrees
  with decoded-key order. It cannot prove the laws for unobserved values.
- **Fixed candidates.** Sorted, Dense, Roaring, and Chunked storage. Automatic
  construction chooses among the first three with the retained memory score;
  `IntSetPolicy::chunked()` is the only way to select Chunked.
- **Absolute Dense coordinates.** Dense windows use absolute aligned encoded
  offsets, including for operands with different bases. Inclusive spans use
  `u128`; cardinality is `u64`.
- **Limits.** `IntSetLimits` bound Dense payload and Roaring partitions, not
  Sorted or Chunked storage, staging, or process memory. Commutative operations
  take the smaller limits; difference keeps the left limits. All four
  `*_with_policy` methods remain.
- **Shared storage.** The private `Shared` alias is `Arc` on targets with
  pointer-width atomics and `Rc` otherwise. Reference counts and final-owner
  release are real costs.
- **Eager algebra.** Integer algebra materializes a new `FrozenIntSet`. Count
  and predicate kernels stay on canonical codes; some mixed pairs allocate
  temporary conversion storage. There is no lazy integer expression engine.

Sources: [codec laws](../../crates/frost-collections-integer/src/integer/key.rs), [policies](../../crates/frost-collections-integer/src/integer/policy.rs),
[set and freeze plans](../../crates/frost-collections-integer/src/integer/set.rs), [shared alias](../../crates/frost-collections-integer/src/integer/mod.rs).
Tests: [integer sets](../../crates/frost-collections/tests/integer_sets.rs), [integer output](../../crates/frost-collections/tests/integer_output.rs).
Design: [integer profile](integer-sets.md).

## 9. Snapshot admission and publication

With `arc-swap-1`, `SnapshotStore<F>` publishes whole frozen versions. `F` must
implement the sealed `SnapshotValue` marker. Eligible `Frozen<B>` wrappers, the
five ordinary frozen families, `InlineFrozenVec`, `AdaptiveFrozenHashMap`, and
(with `int-set`) `FrozenIntSet` qualify. Mutable families and live concurrent
collections do not.

| Operation | Contract |
| --- | --- |
| `load()` | Returns an owned `Arc<F>`. Load once and reuse it when several reads must see one structure. |
| `load_guard()`, `peek(f)` | Scoped protected access; a guard can become an owned `Arc` with `into_owned()` |
| `publish(value)` | **Unconditional** replacement; returns the displaced version |
| `publish_if_current(&expected, value)` | Replaces only if the current version is the same **allocation** as `expected`. On failure returns the rejected candidate and an observed version that may already be superseded. |
| `update(f)` | Repeats derivation and conditional publication after conflicts; returns the **displaced** version, not the new one. The callback can run many times. No fairness or bounded-retry promise. |

`update` uses the shared retry helper that the scheduler model executes. Old
versions stay alive while any guard or `Arc` holds them. Publication allocates,
and displaced or rejected values run their destructors wherever their last
owner drops them. A snapshot freezes structure, not
interior state shared by elements.

Sources: [snapshot API](../../crates/frost-collections-concurrent/src/concurrent/snapshot.rs),
[retry helper](../../crates/frost-collections-concurrent/src/concurrent/publication.rs). Tests:
[snapshot behavior](../../crates/frost-collections/tests/concurrent/snapshot.rs),
[retry schedules](../../crates/frost-collections/tests/publication_model.rs). Contract:
[snapshot publication](snapshots.md); model bounds: [verification](../verification.md).

## 10. Feature boundaries

| Capability | Feature | Notes |
| --- | --- | --- |
| `Frozen<B>` over arrays and slices | none | `core` only |
| Sequence, ordered, adaptive inline and ordered families | `alloc` | |
| Hash families and adaptive hash profile | `std` (default) | Default hasher is std's `RandomState` |
| Typed integer profile | `int-set` | Requires `alloc`; `std` enables the dependency's runtime CPU dispatch |
| Snapshot publication | `arc-swap-1` | Requires `std` |
| Exact wrappers over optional backends | The backend's compatibility-line feature | Native rules and APIs remain |

Features are additive. Enabling one never changes an existing type's
representation, hasher, public bounds, or freeze/thaw partner. The README
[feature catalog](../../crates/frost-collections/README.md#feature-catalog) is the complete list.

## 11. Proposals, not current API

Everything in this section is a **proposal**. None of it compiles against the
current crate, and nothing here is a usage example.

| Proposal | Design document |
| --- | --- |
| Typed borrowed set views over families, wrappers, bitmaps, and FST streams | [General set algebra](set-algebra.md) |
| Borrowed integer expressions (`IntSetExpr`) | [Integer views](integer-views.md) |
| Persistent tree directories; a separate persistent integer family | [Persistent integer storage](persistent-integer-storage.md) |
| Map merge and join, bags, multimaps, bimaps, sequence algebra, prefix indexes | [Collection algebra](collection-algebra.md) |
| Computed sets over durable or revisioned sources | [Research synthesis](../research/synthesis/foundations-and-engine-roadmap.md) |

Any new frozen type must follow the rules above: structural mutation belongs to
a mutable family or builder. A consuming operation may return a new frozen value.
The supplied research placed `try_insert(&mut self, ...)` on a frozen bimap;
that contradicts frozen semantics. The corrected shape, as pseudocode:

```text
MutableBiMap::try_insert(&mut self, left, right) -> Result<(), BiConflict>
MutableBiMap::freeze(self) -> FrozenBiMap
FrozenBiMap::thaw(self) -> MutableBiMap
FrozenBiMap::try_union(&self, &other) -> Result<new FrozenBiMap, conflict>
```

The conflict type, which inputs a failure returns, panic behavior, and
allocation-failure behavior are open questions. A proposed frozen type must
also state its snapshot eligibility, auto traits, and costs before review.

The [Frozen API research prompt](../research/prompts/frozen-api-contracts.md)
frames follow-up research. Its findings become design only through a reviewed
design change, and become implementation only with tests.

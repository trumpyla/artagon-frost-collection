> **0.4.0 follow-up:** frozen ordered/hash set algebra and Roaring/dense bitmap
> algebra are implemented and verified. The implementation and benchmark
> contracts are in [SET_OPERATIONS.md](../set-operations.md); current execution
> results are in [VERIFICATION.md](historical-verification.md). Independent review checked
> left-side representatives, iterator progress and size hints, unequal bitmap
> universes, dirty padding, and cardinality arithmetic. No additional unsafe
> implementation was introduced. The original 0.3.0 review below remains the
> historical record of the architecture refactor and its reproduced defects.

# Adversarial review: incoming prototype and selected 0.3.0 design

Review date: 2026-11-08. Selected compiler: **Rust 1.99.1**, edition **2024**.

## Assessment and scope

The incoming `0.1.0-proto` attachment has useful additions: mutable adaptive
profiles, richer crate-owned entry handles, and five framework integrations. It
also omits substantial parts of the earlier reviewed design. The selected 0.3.0
implementation keeps the earlier architecture and incorporates those additions
where their contracts fit. It does not replace the complete family/capability
model with the attachment's smaller backend surface. The comparison and final
architectural choices are recorded in [DESIGN_DECISIONS.md](../design/decisions.md).

The unchanged attachment passed **59 integration/property tests and 24 doctests**
in the baseline run. That includes its existing inline ownership tests. The
review then added adversarial cases outside that coverage. Two new cases were
reproduced against incoming code: a zero-sized element reservation contract
failure, and a defensive strict-decoding failure for inconsistent user key
traits. Neither reproduction established undefined behavior.
[Baseline evidence](../../verification/history/incoming-baseline.log).

This review distinguishes a reproduced behavior defect from an unsupported
guarantee, an omitted requested integration, and an API/performance improvement.
Severity below describes the demonstrated consequence; it is not a security
rating. Final build/test totals, toolchain details, and execution limits belong
to [VERIFICATION.md](historical-verification.md), rather than an inferred sum of intermediate
logs.

## Findings at a glance

| Finding | Classification and severity | Selected response |
| --- | --- | --- |
| `InlineVec::reserve` accepts an impossible logical length for zero-sized elements | Reproduced correctness defect; medium | Check `len + additional` before choosing representation or capacity |
| Strict Borsh hash decoding can silently collapse distinct ordered entries into one equivalent hash key | Reproduced defensive contract failure; low, requires inconsistent user key traits | Require decoded cardinality to match the validated input cardinality |
| Missing mutable set pairs and advanced/concurrent integrations | Requested-scope gaps; not runtime defects | Restore the earlier five-family model and fourteen optional backend integrations |
| Framework tests require every framework feature simultaneously; no executable examples directory | Verification gaps | Test features independently and provide explicit tested example targets |
| Family duplicate behavior depends on native bulk construction | Contract-maintenance risk; no failing valid-key counterexample claimed | Make first representative/last value handling explicit in family builders |
| Small frozen inline construction stages through a `Vec`; some operations require unnecessary type bounds | Performance/API limitations | Build fitting sequences inline and place bounds on operations that need them |
| Canonical encoding, parallel duplicate winners, and allocation costs are described too broadly | Guarantee/documentation limitations | Specify ordering, representative metadata, staging, and source-order limits |

## 1. Reproduced correctness defect: zero-sized reservation overflow

The incoming `reserve` used saturating addition and relied on a later
`Vec::with_capacity` call to reject an impossible request. That reasoning works
for an enormous byte allocation but does not cover zero-sized element types.
A `Vec<()>` can represent capacity `usize::MAX` without allocating that many
bytes. Saturation therefore hid an overflow of the requested logical length.

The essential reproduction is:

```rust
use frost::prelude::*;

let mut values: InlineVec<(), 4> = InlineVec::new();
values.push(());
values.reserve(usize::MAX); // Must reject 1 + usize::MAX.
```

Incoming code returned normally and changed representation. The selected code
checks addition before spilling. The regression catches the panic, verifies that
the existing element remains owned, and verifies that the value remains inline.
It also tests the converse: reserving `usize::MAX` for an **empty** zero-sized
sequence is permitted. Rejecting every large zero-sized reservation would be an
incorrect fix.

This was a violated reservation contract, not an out-of-bounds write or an
established memory-safety defect. The implementation still had checked storage
operations; the failure was accepting a logical request whose required length
could not be represented.

Evidence: `inline_zero_sized_reservation_checks_length_overflow` in
[tests/core_review.rs](../../tests/core_review.rs), with the corrected allocation-only
run in [verification/core-alloc.log](../../verification/history/core-alloc.log). A separate run
against unmodified archived source with Rust 1.99.1 failed with the assertion
`len + additional must not overflow for ZSTs`;
[the reproduction log](../../verification/history/incoming-inline-reproducer.log) records the
command, isolated source, and expected failing exit status.

## 2. Reproduced defensive failure: Borsh strict cardinality

The incoming family decoder validated strict key ordering, then collected the
entries into a hash family. Ordering and hash equivalence are separate operations.
With a deliberately inconsistent user key implementation, ordering can accept two
entries that hashing subsequently merges:

- `Eq` and `Hash` use an `id` field.
- `Ord` compares `(id, metadata)`.
- The wire entries use keys `(1, 1)` and `(1, 2)`, with values `10` and `20`.

Those keys are strictly ascending under the supplied `Ord`, but equivalent under
the supplied hash-key relation. The incoming strict decoder returned `Ok` with
one entry. The archived-source reproduction records the observed result as
`Ok(1)` where rejection was expected.
[Before-fix evidence](../../verification/history/incoming-borsh-reproducer.log).

The selected decoder records the validated entry count, builds the family, and
returns `InvalidData` if construction reduced that count. The correction covers
ordinary and adaptive hash maps and is also applied to the added hash-set
families. Ordinary duplicate or descending keys are still rejected by the
ordering check. Tests cover both classes of rejection in
[tests/ecosystem_review.rs](../../tests/ecosystem_review.rs).

The severity is deliberately limited: this reproduction violates the key type's
`Ord`/`Eq` consistency requirements. It is defensive strengthening of a strict
wire contract, not evidence that law-abiding keys can bypass uniqueness or that
the decoder is generally unsafe. A cardinality check also cannot validate every
possible faulty `Eq`, `Hash`, or `Ord` implementation.

The scope of strictness remains explicit. Family Borsh decoding rejects
noncanonical ordering and collapse. Exact `Frozen<B>` decoding retains the
backend's rules. The `exact_wrapper_preserves_native_decoding_rules` regression
compares the wrapper with native `BTreeMap` behavior rather than assuming the
backend has enabled Borsh's optional strict-order feature.

## 3. Architectural selection and restored integration coverage

### Keep the complete semantic families

The earlier design's five mutable/frozen pairs are retained: vector, hash map,
ordered map, hash set, and ordered set. The incoming family modules and richer
occupied/vacant entry handles remain the implementation home for these types.
Convenience reexports expose the same types; they do not create competing
implementations.

The missing mutable sets and hash-set family were requested-scope gaps in the
attachment. Their addition is not presented as repairing a failed existing set
implementation. Tests now cover representative preservation, borrowed queries,
strict construction, set algebra, ordered ranges, fallible reservation, and
freeze/thaw ownership. See [tests/mutable_sets.rs](../../tests/mutable_sets.rs),
[tests/sets_review.rs](../../tests/sets_review.rs), and
[tests/root_regressions.rs](../../tests/root_regressions.rs).

An exact-backend `Mutable<B>` wrapper is still excluded. It would retain the
backend in the application type while requiring the crate to forward its mutable
surface. Native mutable collections instead implement the open capabilities
directly. Family types justify their wrapper by owning stable semantics and
private storage.

Ordinary family storage is not selected by a Cargo feature. Optional features add
integrations and separate collection types; they cannot silently impose `Ord` on
hash users or change an ordinary family's ordering. Adaptive enums are retained
only within one semantic family, with explicit policy and representation costs.

### Retain the earlier advanced capabilities

The incoming backend surface contained hashbrown and indexmap. The selected
implementation retains all fourteen reviewed backend features, including inline
and fixed-capacity vectors, persistent collections, bitmaps, slot maps, static
hash tables, finite-state transducers, live concurrent collections, and snapshot
publication. Capability design follows what storage can honestly return:

| Storage property | Capability consequence | Regression evidence |
| --- | --- | --- |
| Compressed bitmaps reconstruct integer members | Yield values; keep `u64` cardinality where needed | `bitmap_iterates_values_and_keeps_cardinality_wide`, `treemap_preserves_full_width_keys_and_snapshot` |
| FixedBitSet has a universe length and may contain nonzero padding blocks | Count and iterate only logical members; preserve raw retained storage | `padding_bits_are_not_members_and_retention_preserves_storage` |
| Slot maps synthesize generational keys | Copy keys instead of promising stored-key references | `slotmap_rejects_stale_keys_and_clones_do_not_mutate_snapshot` |
| SecondaryMap membership is independent of later primary removal | Do not infer secondary membership from the primary map | `secondary_membership_is_independent_of_primary_removal` |
| FST streams lend keys from reusable storage and decode values | Keep lending traversal and value-returning lookup distinct | `fst_lookup_decodes_values_and_streams_lend_keys` |
| ArrayVec capacity and requested output length are different quantities | Validate current length and return the owned source on failure | `arrayvec_exact_length_failure_preserves_owned_elements` |

These tests are in [tests/advanced.rs](../../tests/advanced.rs),
[tests/specialized.rs](../../tests/specialized.rs), and
[tests/backend_review.rs](../../tests/backend_review.rs). The FixedBitSet and
SecondaryMap cases are retained review regressions from the earlier design,
not newly discovered defects in the attachment, which did not contain those
adapters.

Persistent mutation adapters also respect the family capability laws. imbl set
insertion checks for an existing equivalent member before native insertion,
because the native replacement behavior would otherwise replace the stored
representative. Borrowed map removal returns the stored owned entry while
previous persistent clones retain their version. IndexMap/IndexSet removal keeps
the native shift/swap choice explicit because those operations have different
ordering and cost behavior.

### Keep live access separate from snapshot publication

The selected concurrent API retains scoped synchronous callbacks for live
DashMap, scc, and skip-list collections. It does not manufacture ordinary
unguarded references from protected entries. Mutable-value access is exposed only
where the native collection supplies exclusive protection. Native async,
insertion, and traversal APIs remain available on the backend.

`SnapshotStore` accepts a sealed set of frozen payloads. Mutable families, mutable
adaptive profiles, native mutable collections, and live concurrent collections
are ineligible. Tests exercise old-version retention, whole-version publication,
and the new frozen adaptive profiles; compile-fail examples reject mutable
payloads and guard-lifetime escape. See
[src/concurrent.rs](../../src/concurrent/mod.rs) and
[tests/concurrent.rs](../../tests/concurrent.rs).

This guarantees publication of a whole owned version. It does not make a
multiwriter read/derive/publish sequence transactional, and structural freezing
does not remove interior mutability from stored elements.

## 4. Contract, API, and performance corrections

### Define representative ownership in crate code

Family maps promise the first equivalent key representative and the last value;
sets promise the first representative. Native bulk-constructor behavior is not
the appropriate foundation for that public promise. Builders and extension now
apply the family insertion rule explicitly. Ordered duplicate reduction uses a
stable sort and in-place reduction, swapping values while retaining the first
key. The parallel ordered builder follows the same rule after staging input.

The review does not claim a newly reproduced valid-key failure in the prior
native constructor on the tested compiler. This change makes the contract
explicit and removes reliance on a less specific upstream bulk-construction
guarantee. `every_hash_builder_preserves_first_key_and_last_value` and
`ordered_parallel_input_retains_first_representative_and_last_value` verify
representatives whose equality ignores metadata.

Strict constructors return the rejected owned entry or the unchanged rejected
input where documented. Tests check that the rejected key is the input key,
not the previously stored representative. Different strict constructors may
detect duplicates in input order or after sorting; their documentation states
which entry is returned. See [tests/core_review.rs](../../tests/core_review.rs) and
[tests/ecosystem_review.rs](../../tests/ecosystem_review.rs).

### Remove bounds that an operation does not need

Already valid ordered storage can freeze without asking the key to implement
`Ord` again. Pure traversal does not need key hashing. Consuming a stored hasher
does not require cloning it. Explicit hash builders accept caller-supplied
hashers without requiring `Default` or `Clone` merely for construction.

Rayon wrappers likewise require `Send`/`Sync` for the data actually transferred or
borrowed across workers. Native hash traversal stages entries/references; its
iterator payload does not contain the hasher. A hasher containing `Rc` can remain
on the collecting thread. Regressions test a non-`Send` hasher, empty traversal
with otherwise traitless key/value types, and a non-cloneable hasher during
frozen adaptive extraction. These are API improvements, not findings of
previously unsafe threading.

The incoming `MapTypes` supertrait is retained for map mutation, and `SetInsert`
shares `ElementType`. This prevents independently declared associated types from
disagreeing across capabilities.

### Make inline construction actually stay inline when it fits

The incoming frozen inline collection built through a temporary `Vec`, and
mutable extension consulted the input's reservation hint before consuming it.
Fitting sequential collection now fills inline storage directly and reaches
heap reservation only after an actual overflow of the inline capacity.

Tests cover a panicking size hint that must never be consulted while input fits,
a non-fused iterator that must stop at its first `None`, and a hint that panics
after spilling without leaking produced values. This is a concrete construction
path improvement; it is not a benchmark claim that an adaptive profile is faster
for a particular workload. Parallel collection still stages a vector, and an
explicit `from_vec` receives storage that has already been allocated.

## 5. Ownership, panic, and lifetime review

The unsafe code in the incoming inline-buffer design was reviewed independently
and retained with its ownership model. No new undefined-behavior counterexample
was found. The relevant invariants are concrete:

- `InlineBuf` owns only its initialized prefix. Pop reduces that prefix before
  reading the removed element; cloning grows a separate initialized prefix.
- Spilling allocates before relinquishing inline ownership. The later transfer
  moves initialized elements into sufficient, nonoverlapping storage.
- An owning iterator owns only its remaining `[front, back)` range. Both ends
  advance ownership boundaries before returning values. Slice drop glue disposes
  of the remaining range, including when one destructor panics.
- The retain guard moves the unvisited suffix back after a predicate panic,
  including the current element. After a rejected element's destructor panics,
  that already-consumed element is excluded. Restoration cannot exceed capacity:
  kept elements plus remaining elements are bounded by the original length.

Additional regressions exercise rejected-element destructor panic, mixed-end
iterator consumption followed by destructor panic, over-aligned elements,
zero-sized destructors, partial cloning, and zero capacity.
[tests/inline_safety.rs](../../tests/inline_safety.rs). Focused Miri evidence is recorded
in [verification/core-miri-stacked.log](../../verification/history/core-miri-stacked.log);
the exact final Miri scope belongs to [VERIFICATION.md](historical-verification.md).

Adaptive hash/tree promotion is deliberately weaker than transactional rollback.
If user hashing or comparison panics while rebuilding a representation, moved
entries can be dropped. The object remains valid and owned values are dropped
once. The new promotion tests verify subsequent usability and drop accounting;
they do not assert preservation of all pre-panic entries.
[tests/core_review.rs](../../tests/core_review.rs).

The framework review found no new Rayon lifetime or aliasing counterexample.
Borrowed wrappers retain the source lifetime, including boxed-slice storage.
Mutable map iteration yields immutable key references and disjoint mutable value
references. Owned iteration transfers values into the owning native iterator.
There is no lifetime widening or new unsafe code in these wrappers.
[src/adapters/rayon.rs](../../src/adapters/rayon.rs).

## 6. Framework guarantees and verification limits

All five incoming frameworks are retained and extended to the added sets. Their
contracts are narrowed where representation independence alone was insufficient:

- **Serde:** hashes have unspecified wire traversal order. Default decoding
  applies family duplicate rules; strict helpers opt into duplicate/order
  rejection. Runtime hashers and thresholds are reconstructed from defaults.
- **Borsh:** family key/member ordering is canonical, but equality that ignores
  serialized metadata can still produce different bytes for equal collections.
  Custom serializers remain responsible for their own determinism.
- **Decoding resources:** cautious initial reservation prevents trusting an
  excessive size hint. It is not a total collection-size limit. Strict decoding
  stages input before validating it; callers need appropriate input budgets.
- **Rayon:** defined encounter order supports a deterministic first/last
  duplicate rule. Unordered sources cannot supply that guarantee. Tree/hash
  traversal and parallel construction may allocate staging vectors.
- **Schemas and generators:** schemas describe the logical wire shape and omit
  private representation fields. Generators deliberately vary adaptive
  thresholds and growth states to exercise invariants; those choices are not
  performance recommendations. See
  [src/adapters/arbitrary.rs](../../src/adapters/arbitrary.rs).

The attachment's framework test file was gated on all five framework features
together. A successful all-features run therefore did not demonstrate that each
optional adapter was independently exercised. Test modules now have their own
feature gates; no-std-capable adapters are also exercised without the crate's
`std` feature. Intermediate evidence appears in
[verification/ecosystem-isolated.log](../../verification/history/ecosystem-isolated.log).

Nine executable examples now have explicit Cargo test targets that invoke their
actual scenarios, alongside doctests and negative capability checks. Required
toolchain/build gates use the selected Rust 1.99.1 and edition 2024. Final results
and any outstanding limitations are recorded in
[VERIFICATION.md](historical-verification.md). Passing these checks establishes the tested
cases; it does not prove all unsafe paths, arbitrary user callbacks, or workload
performance.

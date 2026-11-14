# Revision 2 integration review

Review date: 2026-11-09. Target: Rust 1.99.1, edition 2024.

## Decision

Integrate revision 2 selectively into the supplied reviewed main project. Keep
one production library package, `frost-collections`, exposed as `frost`. The
new integer storage belongs in `src/integer`; the ordinary collection families,
backends, adapters, and concurrency modules retain their existing identities.

The revision archive and both patches are consistent with each other, but they
were authored against an older common ancestor. Applying their file replacements
verbatim to reviewed main would discard newer result-policy, promotion, hashing,
portability, and general set-algebra work. The code-only patch is an exact subset
of the full patch, not a second independent change to apply afterward.

The merged implementation adds shared integer storage, explicitly selected
chunked containers, more built-in codecs, and the revised pair dispatch. It
preserves the stronger main-project contracts and fixes the defects below.
An additional independent policy oracle completed 507,840 checks on the initial
merge, before the final Roaring optimization placement refinement. It compared
against standard-set expected results, including zero limits, forced layouts,
empty sets, extreme codes, and exact Hash events. Its source, exact provenance,
and reproduction instructions are included with the execution evidence. The
final source subsequently passed the native, package, affected-feature,
embedded, and Miri gates described in the validation report.

Execution results for this source integration are in
[the validation report](../revision2-integration-validation.md). The later
[GitHub import and Make checks](../../verification/revision2-integration/github-import-ci/README.md)
cover its adaptation to the live repository's newer build tooling separately.

## Inputs and provenance

Inputs were `frost-collections-reviewed(1).zip`, `frost-revision2.zip`,
`integer-revision2.diff`, and `integer-revision2-code.diff`.

Both patches reverse-apply cleanly to revision 2. The full patch changes 29 files;
the code-only patch contains 12 of those changes byte for byte. Reconstructing
the parent confirms that its integer sources and manifest match the older
baseline embedded in reviewed main's integer-output evidence. The reviewed main
archive contains later source fixes and additional design documents.

Input digests, patch checks, and the baseline results are preserved under
[`verification/revision2-integration/before/`](../../verification/revision2-integration/before/).
The original incoming measurement directory remains under
`verification/int-set/revision2/`; its measurements describe the supplied
revision, not the final merge.

## Findings and resolutions

### 1. Result-policy regression: rejected

Revision 2 removed `IntSetLimits`, `limits()`,
`contiguous_payload_bytes()`, and the four materializing `*_with_policy`
operations. Its replacement budget composition took the larger operand limits,
and some identity paths returned an operand without validating the result's
constraints.

Those changes would silently relax existing callers' limits. The integration
retains the main contract:

| Operation | Inherited result limits |
| --- | --- |
| Union | Componentwise minimum of both operands |
| Intersection | Componentwise minimum of both operands |
| Symmetric difference | Componentwise minimum of both operands |
| Difference | Left operand's limits |
| Explicit `*_with_policy` | Supplied policy replaces the inherited limits |

Every result path checks its actual representation, including empty inputs,
self operations, equal operands, and forced result layouts. Shared allocation
reuse occurs only when the allocation meets the resulting policy. When both
operands equal the result, the implementation considers both eligible layouts
before rebuilding. The policy caps dense payload bytes and native Roaring
upper-32-bit partitions; it does not cap total process memory, staging, sorted
storage, or chunked storage.

Evidence: [`tests/integer_output.rs`](../../crates/frost-collections/tests/integer_output.rs), including
`identity_sharing_obeys_forced_representations_and_inherited_limits`, and the
retained original result-policy regressions.

### 2. Partial chunked folds duplicated a value: fixed

The incoming `ChunkedIter::fold` used `next()` to finish its current container.
That call could advance into the next container and consume its first value.
The subsequent whole-container fold then emitted that value again.

The reproduction consumed one value and expected 69,639 remaining values;
the incoming implementation returned 69,640, duplicating code 65,536 at an
Array-to-Bitmap boundary. The fixed fold directly respects the saved Array,
Bitmap, or Full cursor for the first container, then traverses later containers.

Tests retain the larger native reproduction, multiple partially consumed and
exhausted states, cloned iterators, every container kind, and values near
`u64::MAX`. A separate bounded test constructs cursors near container ends so
Miri exercises the same failure without enumerating a full container.

Evidence: [original failure](../../verification/revision2-integration/before/chunked-fold-reproducer.log),
[`src/integer/chunked.rs`](../../crates/frost-collections-integer/src/integer/chunked.rs).

### 3. Targets without pointer atomics stopped compiling: fixed

The old allocating integer profile compiled for `thumbv6m-none-eabi`. Revision
2 introduced unconditional `alloc::sync::Arc` imports, and the same build failed
because that target has no pointer-width atomics.

The integrated profile uses a private shared-pointer alias: `Arc` when
`target_has_atomic = "ptr"`, and `Rc` otherwise. Auto-traits follow the selected
standard pointer and the logical key; there are no manual `Send` or `Sync`
implementations. Native and bare-metal builds exercise both branches. The
original integration added the target to its CI and `verify.sh embedded` scope;
the GitHub checkout carries that check in `make embedded` and CI.

Evidence: [baseline build](../../verification/revision2-integration/before/baseline-thumbv6m.log),
[incoming failure](../../verification/revision2-integration/before/revision2-thumbv6m.log),
[`src/integer/mod.rs`](../../crates/frost-collections-integer/src/integer/mod.rs).

### 4. Unchanged overlapping chunks lost sharing: fixed

The incoming chunk directory merge shared disjoint containers, but an overlapping
container could be rebuilt even when its logical result was unchanged. For
example, a union that changes only the second chunk could retain zero shared
chunks from its left input.

The integration reuses a container when the result cardinality proves it equals
an operand: union/intersection can equal either operand, and unchanged difference
can equal the left. This avoids a second member scan. It preserves the allocation
in the final result; discovering that identity can still create a temporary
candidate. The documentation makes that cost explicit.

Evidence: [original failure](../../verification/revision2-integration/before/chunked-sharing-reproducer.log),
`unchanged_overlapping_containers_keep_their_allocations` in the chunked unit tests.

### 5. Dense conversion hardening would be lost: preserved and extended

Reviewed main already imported dense words into Roaring using bounded,
little-endian chunks that do not cross an upper-32-bit partition or the native
byte import's full-width extent. Revision 2 replaced that implementation with
whole-window byte staging. Its 64-bit helper also computes an exclusive endpoint
before subtracting one, which needs an empty-input guard.

The integration retains the tested `storage/word.rs` conversion, including empty,
partition-edge, endian, and maximum-code checks. The new dense kernel uses safe
inclusive end arithmetic. Removing member zero from a private empty dense window
now returns empty instead of indexing its empty word buffer. Public operations
already intercepted most empty cases; the kernel now upholds the invariant itself.

The main cached-cardinality path for small dense updates is retained. A tiny
change need not add another full bitmap popcount after cloning its words.

### 6. Hash work and planner behavior would regress: preserved

The incoming compressed-storage hash paths enumerated every code and allocated
a temporary buffer; the Sorted path hashed its code slice directly. Main already used a canonical, content-selected stream of normalized
integer events, with fast whole-run hashing and a bounded run scout. That stream
is preserved and extended to Chunked. Full chunks can emit a run directly;
physical boundaries are coalesced before selecting the hash format.

Tests compare exact Hasher events, not merely final hash digests, across all
four representations, signed/restricted domains, word boundaries, and result
histories. Limits and storage identity remain excluded from logical equality,
ordering, and hashing. The hash protocol is not a persistent serialization format.

Revision 2 also unconditionally chose Dense for eligible windows up to 4 KiB.
That changes tiny sparse sets into unnecessarily larger allocations. The merged
constructor retains main's deterministic memory scoring and its 128-byte Dense
preference. Four values in distant windows may therefore use a 32-byte sorted
payload; large distant runs still promote to compact Roaring. Chunked remains an
explicit construction policy, not an automatic hidden choice.

### 7. Diagnostic estimates needed a clear meaning: corrected

Revision 2 approximated Roaring payload as two bytes per member. That ignores
bitmap and run compression and can badly misdescribe either sparse or dense
shapes. The integrated diagnostic uses the same statistics-based container and
partition proxy as the result planner. Sorted and Dense member-buffer sizes remain exact;
Chunked counts its directory entries and container payloads. These are not RSS,
allocator-header, retained-capacity, or total-live-version measurements.

## Measured refinement before delivery

The first same-executable comparison found an unnecessary full native Roaring
optimization scan after small cloned updates in the initial merge. That scan
was removed from the generic storage constructor and kept at the native binary
operation boundary. Small clone-and-edit operations now preserve the reviewed
main implementation's direct update behavior. The complete first measurements
and both affected source files are retained under
`verification/revision2-integration/first-pass/`. The final source is rechecked
and remeasured; slower observations are retained in both records.

## Integrated design and costs

`FrozenIntSet<K>` holds one shared immutable storage root. Clone is constant-time
reference counting on that root. Empty construction allocates a shared header,
even though it allocates no member buffer. Final-owner release still destroys
owned storage and is not constant-time. The logical key phantom continues to
preserve lifetimes and auto-trait restrictions.

Chunked storage partitions encoded values by their high bits into 2^16-code
containers. The current container kinds are a sorted `u16` array, a fixed bitmap,
and a Full marker. Container payloads are individually shared. The directory is
flat and sorted, backed by key and reference arrays; a changed operation can copy or merge O(C) directory
entries and update reference counts, where C is the chunk count. This is an
initial persistent-payload implementation, not the proposed persistent radix
or B+ tree directory. Run containers and deeper path copying remain future work. Chunked/Dense scalar
predicates can allocate a temporary chunked view of the dense window; converting
Chunked to Roaring currently enumerates encoded members. Policy conversion can
also clone before rejecting a requested layout. These are documented eager
operation costs and opportunities for a later targeted optimization.

New built-in codecs include narrow signed and unsigned integers, `char`, address
sized integers, and narrow nonzero unsigned integers. Checked imported codes still
reject invalid logical values and preserve typed domains. The ordinary collection
families remain available for arbitrary keys and observable stored representatives.

The general set-algebra design, integer expression proposal, and their reviews
remain in the project. This integration implements the integer storage revision;
it does not claim that those broader expression APIs have been implemented.

## Validation and measurement interpretation

The merged tests cover 16 ordered representation pairs. The finite five-member
oracle exercises both unsigned widths and all 32 × 32 subset pairs: 32,768
input/representation combinations. Forced result policies and exact Hash events
also include all four candidates. Native property, boundary, snapshot, and
concurrency tests remain in the suite.

The new comparison harness compiles reviewed main and the integrated code into
the same executable with identical dependency versions and feature settings.
It alternates execution order by round and retains every raw sample. Its baseline
fixture is the actual reviewed input, not the older baseline used by the incoming
report. Source-level speed claims must come from that comparison and retain
regressions and measurement limitations.

See the [current validation and measurement record](../revision2-integration-validation.md)
for executed commands, counts, timing results, and any unexecuted gates. Historical
measurements remain historical evidence. No crate publication or remote repository
push was part of the source integration described in this report. The subsequent
GitHub import has its own evidence record linked above.

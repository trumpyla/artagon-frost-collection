# Notes to maintainers

Preserve the simple public story: one ordinary family, one frozen partner, and
explicit alternatives for adaptive or native storage. Read the
[philosophy](../design/philosophy.md) and [architecture](../design/architecture.md)
before adding a new profile or changing a feature boundary.

[Code search tools](../../TOOLS.md) documents project search scopes, tgrep
index refresh, and the optional ast-grep rule checks.

## Code and invariant map

| Area | Files | Invariants to review |
| --- | --- | --- |
| Public surface | `src/lib.rs`, `read.rs`, `write.rs`, `plan.rs`, `frozen.rs` | Small capabilities, sealed eligibility, explicit ownership plans |
| Ordinary families | `src/family/` | Fixed pairings, representative policy, borrowed keys, owned iterator contracts |
| Ordered set kernels | `src/family/ordered/set_ops.rs` | Sorted unique input, left representatives, monotonic cursors, balanced/skewed paths |
| Dense and compressed bitmaps | `src/backends/bitmap.rs` | Natural cardinality width, logical universe, dirty-padding masks, empty/exhausted iteration |
| Adaptive maps | `src/adaptive/{hash_map,ordered_map}/` | Cache lookup only until representation changes; explicit promotion/demotion policy |
| Inline storage | `src/adaptive/seq/buf.rs` | Initialized prefix, one owner per initialized element, transfer-before-drop, ZST/zero capacity |
| Integer profile | `src/integer/{key,policy,set,storage}.rs` and `storage/word.rs` | Canonical codecs, checked imports, wide counts/spans, compact pair outputs, retained minimum/left limits, canonical Hash events, policy-compatible root reuse |
| Integer Dense/Chunked storage | `src/integer/{dense,chunked}.rs` | Absolute dense coordinates, immutable shared containers, sorted flat directory, unchanged old versions, partial cursors, bounded conversions |
| Frameworks | `src/adapters/` | Logical encoding, canonical strict decoding, deterministic winners only with defined encounter order |
| Live concurrency | `src/concurrent/{live,dashmap,scc,skiplist}.rs` | Guard scope, no escaping references, native reentrancy constraints |
| Snapshots | `src/concurrent/{snapshot,publication}.rs` | Identity CAS, candidate ownership, exact displaced version, retry callback semantics |

The internal file split does not rename the public `frost::concurrent` module or
ordinary family paths. Integration tests are organized by contract; files under
`tests/concurrent/` are submodules of the single Cargo target `concurrent`.

## Match verification to the risk

Run the affected test first. For new set kernels, include arbitrary input
comparison, skew and ratio boundaries, partial hints/count/fold/clone/fusion,
and observable representatives. For adaptive entry changes, include hit/miss
comparison counts, one callback invocation when appropriate, promotion, and
panic/drop cases. For ownership changes run both Miri aliasing configurations.
For concurrency, combine actual ArcSwap tests with the shared-helper scheduler;
the latter is not a model of ArcSwap internals.

Use `make check` for normal review and `make verify` for a release
candidate's host matrix. Run `make embedded` separately to compile the library
for bare-metal targets, so host dev-dependency feature unification cannot conceal
a std dependency. The integer checks cover targets with and without pointer
atomics. `make miri` and the formal-tool targets are also separate gates.
Independent feature checks and all-features tests are complementary; neither
enumerates every possible subset of features.

## Costs applications should see

`AdaptiveOrderedMap` uses a sorted Vec while under its configured threshold.
Inserting into the middle or `pop_first` shifts entries; repeatedly draining the
front is quadratic for an intentionally large sorted threshold. Prefer
consuming `into_iter()` for an ascending full drain, `pop_last()` when reverse
order is acceptable, or a tree policy for repeated endpoint mutation. Extension
currently applies insertion semantics entry by entry. Do not silently change
its panic behavior or representative policy to get a prettier bulk benchmark.

A deque conversion needs a full iterator/range/contiguity design; immutable
ranges cannot call `make_contiguous(&mut self)`. A shared-slice profile needs a
separate cloning/thaw contract; an unsized `Arc<[T]>` cannot be uniquely extracted
using the sized `Arc::try_unwrap` API. Neither is a drop-in storage replacement.
Other proposed frozen layouts remain experiments until correct prototypes beat
native baselines on relevant workloads. See the
[incoming review assessment](../reviews/incoming-assessment.md).

For the integer profile, test forced and inherited result policies on all
representation pairs, including empty/self identities and near-maximum codes.
A tiny snapshot update must not silently expand a large compressed result into
Sorted storage. Equality and Hash must agree across physical layouts and update
histories. Keep the exact Hash event test; a final hash comparison alone can
hide differences in Hasher method calls. Run isolated alloc-only tests and
bare-metal integer builds with and without pointer atomics after changing
ownership or dependency features. Private Shared imports must stay consistent
across the root and chunk implementations. Do not add manual Send/Sync impls.

The implemented Chunked candidate shares container payloads but copies its flat
directory for changed versions. Test every retained version, container identity,
forced result policies, and partial iterator fold states. Measure directory
copying, reference-count traffic, retained memory, and final-owner release. Empty
sets allocate a shared header; clone means a constant-time reference increment,
not zero work. Payload diagnostics are not resident-memory bounds.

The [integer expression proposal](../design/integer-views.md) and the separate
family/tree-directory parts of the
[persistent-storage proposal](../design/persistent-integer-storage.md) remain
unimplemented. Keep their acceptance gates separate from current test totals.
In view work, distinguish proven bounds from planner estimates, preserve mixed
Boolean structure, and test partial cursor states. The
[earlier research review](../reviews/lazy-views-adversarial-review.md) is historical;
the [integration review](../reviews/revision2-integration-review.md) explains the
later explicit Chunked decision. No automatic result cache or lazy evaluation
engine is introduced.

## Dependencies and compatibility

Keep direct compatibility-line features and the lockfile together. Review
upstream release notes before updating a backend: representative handling,
wire formats, safety, and minimal supported Rust versions may change. Do not
add a weak hasher globally. Avoid changing optimizer flags for the library to
make one benchmark win. Default Cargo profiles remain the baseline.

`verification/loom` and `verification/kani` are isolated tooling packages.
Resolve and record their tool/dependency versions before claiming their results.
The package archive excludes verification artifacts; the source distribution
contains them. `make package` tests the extracted Cargo package to
catch missing source files or accidentally coupled verification dependencies.

## Release ownership

The owner must supply the canonical repository, security contact, supported
platforms, and release policy. No publishing identity is assumed by this source
bundle. Follow [the release procedure](releasing.md); update
[the integration validation](../revision2-integration-validation.md) from actual
gate output rather than copying an earlier release's test count. Keep the
[GitHub import and Make checks](../../verification/revision2-integration/github-import-ci/README.md)
distinct from the original integration's script-run evidence.

# Notes to maintainers

Preserve the simple public story: one ordinary family, one frozen partner, and
explicit alternatives for adaptive or native storage. Read the
[philosophy](../design/philosophy.md) and [architecture](../design/architecture.md)
before adding a new profile or changing a feature boundary.

## Code and invariant map

| Area | Files | Invariants to review |
| --- | --- | --- |
| Public surface | `src/lib.rs`, `read.rs`, `write.rs`, `plan.rs`, `frozen.rs` | Small capabilities, sealed eligibility, explicit ownership plans |
| Ordinary families | `src/family/` | Fixed pairings, representative policy, borrowed keys, owned iterator contracts |
| Ordered set kernels | `src/family/ordered/set_ops.rs` | Sorted unique input, left representatives, monotonic cursors, balanced/skewed paths |
| Dense and compressed bitmaps | `src/backends/bitmap.rs` | Natural cardinality width, logical universe, dirty-padding masks, empty/exhausted iteration |
| Adaptive maps | `src/adaptive/{hash_map,ordered_map}/` | Cache lookup only until representation changes; explicit promotion/demotion policy |
| Inline storage | `src/adaptive/seq/buf.rs` | Initialized prefix, one owner per initialized element, transfer-before-drop, ZST/zero capacity |
| Integer profile | `src/integer/` | Canonical stable codecs, typed domains, checked imports, absolute dense coordinates, wide counts/spans, compact pair outputs, retained limits, canonical Hash events |
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

Use `./scripts/verify.sh check` for normal review and `all` for a release
candidate. The matrix deliberately compiles the library for a bare-metal target
so host dev-dependency feature unification cannot conceal a std dependency.
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
hide differences in Hasher method calls. Run both isolated alloc-only tests and
a bare-metal integer build after changing dependency features.

The [integer expression proposal](../design/integer-views.md) and
[persistent-storage proposal](../design/persistent-integer-storage.md) are
unimplemented research designs. Keep their acceptance gates separate from
current test totals. In view work, distinguish proven bounds from planner
estimates, preserve mixed Boolean structure, and test partial cursor states.
In persistent work, test every retained version and measure directory copying,
atomic traffic, and final-owner release. The
[research review](../reviews/lazy-views-adversarial-review.md) records why neither
automatic result caching nor a hidden new FrozenIntSet candidate was accepted.

## Dependencies and compatibility

Keep direct compatibility-line features and the lockfile together. Review
upstream release notes before updating a backend: representative handling,
wire formats, safety, and minimal supported Rust versions may change. Do not
add a weak hasher globally. Avoid changing optimizer flags for the library to
make one benchmark win. Default Cargo profiles remain the baseline.

`verification/loom` and `verification/kani` are isolated tooling packages.
Resolve and record their tool/dependency versions before claiming their results.
The package archive excludes verification artifacts; the source distribution
contains them. `scripts/verify.sh package` tests the extracted Cargo package to
catch missing source files or accidentally coupled verification dependencies.

## Release ownership

The owner must supply the canonical repository, security contact, supported
platforms, and release policy. No publishing identity is assumed by this source
bundle. Follow [the release procedure](releasing.md); update
[current validation](../integer-output-validation.md) from actual gate output rather
than copying an earlier release's test count.

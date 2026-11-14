# Prior integration validation and release readiness

This record describes the snapshot/project-hardening baseline. The subsequent
integer profile, output-policy work, and shared-storage integration have their
own records; see the latest
[integration validation](revision2-integration-validation.md) and the separate
[GitHub import and Make checks](../verification/revision2-integration/github-import-ci/README.md).
The counts and script-based execution claims below remain historical evidence
for their original source revision. Use the root [Makefile](../Makefile) for
current verification commands.

## Assessment

The supplied patch and review have been integrated selectively into the prior
optimized Frost implementation. The result is a production-hardened **release
candidate**, with explicit ownership contracts, an organized Rust project,
repeatable quality gates, and executed concurrency/ownership verification.
It is **not an unconditional production certification**: the optional Loom and
Kani checker runs, a current dependency advisory audit, and execution of the
remote platform CI jobs remain outstanding. Publishing also requires the
owner's repository and security/support metadata.

The crate remains an unreleased **0.4.0**, on **Rust 1.99.1 / edition 2024**.
Its ordinary runtime dependency declarations, backend lockfile, default hasher,
feature model, family identities, and freeze/thaw pairings are retained. No new
unsafe runtime code or unsafe Send/Sync implementations were introduced.
The library remains core-only without features, alloc-enabled when requested,
and std-enabled by default.

Verification was performed on x86_64 Linux reporting an AMD EPYC 9V74, with
normal Cargo profiles and no task-specific RUSTFLAGS. The diagnostic nightly
manifest is 2026-11-08. See the [environment record](../verification/results/environment.json)
and [source fingerprint](../verification/results/source-manifest.json).

## What changed

### Snapshot publication

The integration adds `load_guard`, `peek`, `publish_if_current`, and `update`.
`SnapshotGuard<F>` is crate-owned, dereferences to the frozen payload, and can
retain exactly that version with `into_owned`. Upstream guard/strategy types
are private implementation details.

Conditional publication compares allocation identity. On failure it returns
the original non-Clone candidate and an observed version, which may already
have been superseded. On success it returns the exact displaced predecessor.
`update` re-derives after conflict and also returns the displaced predecessor;
its callback may execute repeatedly. Plain `publish` remains unconditional.

The successful atomic replacement is the publication point. Old guards/Arcs
retain their versions across writes and store destruction. A derivation panic
does not publish that invocation's candidate, but does not undo another writer
or element interior mutation. Allocation, destructors, finite guard fast paths,
and arbitrary retries rule out a blanket bounded-latency or lock-free workflow
claim. The [snapshot contract](../crates/frost-collections-concurrent/docs/snapshots.md) documents these boundaries.

`src/concurrent.rs` became a directory with separate live traits, backend
adapters, snapshot types, and a shared retry helper. Public paths remain stable.
The real store, native scheduler, and Loom model use the same retry helper;
they do not maintain separate implementations of the retry algorithm.

### Performance integration

The prior ordered seeking/balanced kernels, dense bitmap loops, cached adaptive
entry searches, and owning Rayon buffer transfers are retained. The incoming
ordered replacement targeted an older linear-merge baseline and would regress
several controls in the already optimized implementation.

A 302-row comparison compiled the exact prior and incoming kernels together
with identical Rust 1.99.1, optimization level 3, one codegen unit, and thin LTO.
Execution was serial on CPU 0 while other project builds were held. BTreeSet
provided independent expected answers. Each row retains seven raw samples;
workloads were run in both implementation orders except the same-storage
scalar control. These flags are comparison controls, not a changed Cargo policy.

| Workload, scalar count unless stated | Prior trial medians, microseconds | Incoming trial medians, microseconds |
| --- | ---: | ---: |
| Balanced 65,536 / 65,536 | 62.657 / 63.467 | 99.535 / 161.598 |
| Spread 64 / 65,536 | 2.674 / 2.783 | 2.841 / 3.023 |
| Spread 64 / 1,048,576 | 5.015 / 4.875 | 5.120 / 4.725 |
| Head cluster 64 / 65,536, next checksum | 0.075 / 0.079 | 0.246 / 0.250 |
| Initially balanced then long gap | 7.253 / 6.586 | 4.131 / 4.197 |
| Same long-gap input, next checksum | 9.248 / 8.316 | 17.715 / 16.057 |

The incoming scalar long-gap path improves, while several balanced and iterator
paths regress; the largest 64-key spread case is approximately tied. The shared
host has visible noise, particularly in the incoming balanced scalar case.
These are workload observations, not precise universal ratios. Preserve the
current kernel and investigate a narrower long-gap optimization only with
balanced/clustered/partial-iterator controls intact.

Useful incoming regression scenarios were imported and strengthened: skewed
and shared-prefix property tests, comparison budgets across 14 entry points,
and predicates forced to consume their full small operand. No speculative
storage rewrite was introduced to satisfy an unvalidated benchmark claim.

The benchmark targets now use native controls, independent expected answers,
calibrated batches, and chronological raw samples. Adaptive destructive cases
exclude fixture setup. Snapshot readers persist across synchronized batches,
with warmup and checked checksums; those results measure aggregate throughput,
not single-read latency. The [incoming assessment](reviews/incoming-assessment.md)
records the source probe's setup/thread/counting defects and all accept, adapt,
and defer decisions. Exact historical comparison sources and commands are in
[the reproduction directory](../verification/comparisons/ordered/README.md).

### Project organization and maintenance

The root now contains AGENTS.md, CONTRIBUTING.md, SECURITY.md, editor/line-ending
settings, Cargo metadata, licenses, and the README/changelog. Design philosophy,
architecture, snapshot semantics, maintainer notes, benchmark guidance, release
procedure, and review history live under `docs/`. Original supplied review and
patch files are preserved without changing their claims into project policy.

Verification scripts have composable scopes. CI specifies the pinned normal
compiler, Linux/macOS/Windows host jobs, isolated features, bare-metal builds,
a pinned nightly ownership lane, publication models, extracted-package tests,
and a dependency audit. Checkout actions use a verified immutable revision.
The symbolic-proof workflow requires an explicitly selected compatible verifier.
No remote workflow execution is claimed by this local report.

The maintainer guide records known costs and deferred designs. In particular,
a large sorted adaptive threshold can make repeated `pop_first` quadratic;
consuming iteration is the appropriate linear full-drain path. Deque storage,
batched extension, shared frozen slices, and alternative indexed/hash layouts
need separately reviewed contracts and measurements. The supplied designs
contain incorrect or unsupported contiguity, unsized-Arc, cache-size, MPHF, and
rank/select claims; the assessment explains each correction.

## Executed checks

All successful rows below describe this integration, not the older preserved
logs under `verification/history/`.

| Gate | Result | Evidence |
| --- | --- | --- |
| Formatting, local documentation links, Bash syntax, manifest/workflow structure | Passed | [Quality gate](../verification/results/current-check.log); final structural check in delivery log |
| Clippy, all targets/all features, warnings denied | Passed after correcting test/benchmark lint issues | Same quality log |
| Rustdoc, all features, warnings denied | Passed | Same quality log |
| Native all-feature suite | **258 distinct cases passed**: 3 library, 194 integration/property/model, 10 examples, 51 doctests | Same quality log |
| Core-only, alloc-only, default profiles | Passed | [Profile log](../verification/results/profiles.log) |
| All 19 optional integrations independently | Library Clippy, tests, and rustdoc passed for each | [Feature-matrix log](../verification/results/isolated-features.log) |
| Bare-metal target `thumbv7em-none-eabihf` | Core-only and full alloc-compatible feature builds passed | [Embedded log](../verification/results/embedded.log) |
| Ten example programs | Executed successfully; their tests also passed | [Example log](../verification/results/examples.log) |
| Cargo source package | Packaged, verified, and extracted all-feature/example tests passed | [Package log](../verification/results/source-package.log) |
| Exact retry-helper scheduler | 198 three-writer schedules and 6 equal-value/identity schedules passed | [Bounded-model log](../verification/results/bounded-models.log) |
| Kani property helpers as ordinary native tests | One smoke test passed against production code | [Native helper log](../verification/results/kani-native-smoke.log) |
| Miri, strict provenance with Stacked Borrows | 37 cases passed; 2 named long random cases intentionally filtered | [Current Miri log](../verification/results/miri-current.log) |
| Miri, strict provenance with Tree Borrows | 12 inline-safety cases and the separate 686-trace test passed | Same Miri log |
| Pinned nightly doctests | 51 passed; compiler checks rather than Miri executions | Same Miri log |
| Dependency graph boundary | Core-only runtime graph contains only Frost; proof tooling is isolated | [Core graph](../verification/results/core-dependency-graph.log), [all-feature graph](../verification/results/all-features-dependency-graph.log) |
| Exact ordered-kernel comparison | 302 measured rows, answers checked | [Raw CSV](../verification/results/ordered-strategy-comparison.csv), [environment](../verification/results/ordered-strategy-environment.json) |

Repeated feature/example/Miri runs are not additional distinct unit-test cases.
A property test counts as one case regardless of generated inputs. The two
filtered random-operation cases passed natively. Embedded results are
cross-target compilation checks, not execution on embedded hardware.

### Finite and concurrent domains

- **Ordered sets:** all 4,096 pairs of subsets of six values, all four operations,
  cardinalities and relations, and every consumed iterator prefix including
  count/fold, size hints, clone, and exhaustion.
- **Dense bitmaps:** 66,564 combinations covering six membership positions at
  word boundaries, eight logical lengths, unequal universes, and independent
  clean/all-one trailing padding. Results check members, counts, universes,
  and sanitized padding.
- **Inline storage:** 33,614 five-operation traces at capacities 0 and 2 with
  non-Clone values and individual ownership counters. A distinct 686-trace
  three-operation domain runs under Miri; the larger native domain is unchanged.
- **Native snapshots:** 21 total concurrency-target tests, including 15 snapshot
  cases. Eleven new cases cover stale/equal-content identity, candidate recovery,
  exact drops, forced retry, callback/peek panics, guard retention/fallback, and
  multiwriter history. Eight writers perform 250 updates each and validate the
  complete displaced-version history for gaps and duplicates.
- **Retry model:** the actual production helper is exercised under all 198
  three-writer schedules (maximum nine atomic steps and three failures) and six
  two-writer identity schedules (maximum five steps and one failure).
  Noncommuting transformations expose the serial commit order. The atomic
  publication primitive and immutable version lifetime are explicit assumptions.

These domains are finite. The models do not prove ArcSwap's weak-memory
implementation, reference counting, reclamation, unbounded progress, or all
possible user callbacks. Miri diagnoses executed paths, not universal soundness.
The full [verification document](verification.md) states each bound and assumption.

## Benchmark execution

All five integrated benchmark targets built and executed successfully with
Rust 1.99.1, all features, offline/locked dependency resolution, and the default
Cargo bench profile. No custom optimization or RUSTFLAGS settings were used.
Four single-threaded targets ran on CPU 0; snapshot workers could use CPUs 0–8.
No project builds or other project benchmarks ran during their timed regions.

| Target | Checked/timed cases | Raw samples |
| --- | ---: | --- |
| `ordered_sets` | 105 | [CSV](../verification/results/bench-ordered_sets.csv) |
| `adaptive_maps` | 8 | [CSV](../verification/results/bench-adaptive_maps.csv) |
| `snapshots` | 8 | [CSV](../verification/results/bench-snapshots.csv) |
| `set_operations` | 12 | [CSV](../verification/results/bench-set_operations.csv) |
| `performance_review` | 140 | [CSV](../verification/results/bench-performance_review.csv) |
| **Total** | **273** | Seven chronological samples per case |

Every harness's answer assertions passed. Output validation checked unique case
names, positive iteration counts, finite samples, and reported median/min/max
against those samples. The [benchmark environment](../verification/results/bench-environment.json)
records commands, source/executable/data hashes, compiler, profile, and affinity.
These are 273 measured workload cases, separate from the 302-row historical
kernel comparison above and from the native correctness-test count.

### Snapshot read throughput

Median aggregate elapsed nanoseconds per read:

| Operation | One reader | Four readers |
| --- | ---: | ---: |
| Owned `load`, immediately dropped | 9.275 | 32.545 |
| `load_guard` | 4.943 | 2.027 |
| `peek` | 4.565 | 1.354 |
| Raw ArcSwap guard | 5.556 | 1.363 |

Four-reader `peek` achieved about 24 times the aggregate throughput of repeatedly
loading and immediately dropping an owned Arc in this particular workload. It
was close to the raw guard control. The workload uses a warm, unchanged
64-element u64 snapshot, no writer, short-lived guards, and a trivial indexed
read. Batch barriers and result channels are timed; thread creation/join are
excluded. This is reciprocal aggregate throughput, not individual-read latency,
a mixed reader/writer result, or a general latency bound. Shared-host scheduling
and worker placement limit small differences.

### Large adaptive-policy stress cases

At 16,384 initial entries, a complete `pop_first` drain measured 26.276 ms for
the sorted adaptive profile versus 0.379 ms for BTreeMap. An interleaved
16,384-entry extension measured 26.245 ms versus 0.905 ms, with the adaptive
threshold deliberately set to 32,768. Fixture setup is excluded. These custom
large-threshold cases confirm the documented quadratic-shift limitation; they
do not describe the default threshold of 32. Prefer consuming iteration for a
full drain or a tree policy for repeated endpoint/middle mutation.


## Remaining release gates

| Gate | Current state | Concrete next action |
| --- | --- | --- |
| Loom model execution | **Not run:** package absent from the offline registry/cache | Resolve the isolated pinned package, retain its lockfile, and run `make loom` (which uses `--locked`) |
| Six Kani symbolic harnesses | **Not run:** cargo-kani unavailable | Install and validate a verifier compatible with the unchanged Rust requirement, then run `make kani` with unwind assertions enabled |
| Current dependency advisory audit | **Not run:** cargo-audit unavailable and no current database audit completed | Run the configured pinned audit job with a current RustSec database and review any findings |
| Remote platform CI | Configured, not executed in this workspace | Run the actual repository's Linux/macOS/Windows and remaining CI jobs before claiming those platform results |
| Release identity/support metadata | Canonical repository, reporting contact, and support policy not supplied | Owner supplies metadata and support decisions, chooses release version, and deliberately enables publishing when gates are accepted |

Exact tool-availability evidence is preserved for
[Loom](../verification/results/loom-availability.log),
[Kani](../verification/results/kani-availability.log), and
[the audit command](../verification/results/audit-availability.log).
The ordinary native Kani helper pass is not a symbolic proof pass. No missing
check was silently skipped and reported green.

The Cargo package gate reports the expected missing documentation/homepage/
repository metadata warning. Those URLs were not fabricated, and
`publish = false` remains intentional. CI host and proof workflows are concrete
reviewable configurations, not evidence of successful remote execution.
The [release procedure](maintainers/releasing.md) lists the commands and owner
choices needed to close these gates.

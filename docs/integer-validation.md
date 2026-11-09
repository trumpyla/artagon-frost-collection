# Integer profile: implementation and validation

> Historical evidence for the initial integer-profile delivery. Follow-up output
> policies, alloc-only support, and current results are recorded in
> [integer output validation](integer-output-validation.md).

The complete source distribution integrates the typed integer profile into the
previous Frost production candidate. It includes source, examples, tests,
benchmarks, CI configuration, design and maintainer documentation, optional
verification packages, supplied review evidence, and execution records. It does
not require applying a patch or obtaining the previous source archive.

The implementation uses **Rust 1.99.1, edition 2024**. It is a verified production
candidate with the explicit remaining gates below; it is not an assertion that
all possible codecs, dependency internals, platforms, or concurrent executions
have been formally proved correct. Nothing has been published to a registry.

## Implemented design and names

`frost::integer::FrozenIntSet<K>` preserves the logical key domain. The public
safe `IntKey` trait associates that domain with a sealed `IntWord` implemented
only by u32 and u64. Built-in codecs cover u32, i32, u64, i64, NonZeroU32, and
NonZeroU64. Downstream applications can implement their own integer newtypes.
Signed codecs flip the sign bit so encoded order agrees with numerical order.

The private fixed candidates are sorted words, aligned local dense windows,
and width-specific Roaring storage. `IntSetPolicy` controls explicit or automatic
construction, `IntSetBackend` describes the requested choice, and `IntSetRepr`
reports the actual choice. These names follow the existing profile vocabulary;
backend types do not enter application signatures.

The profile is organized into `src/integer/{key,policy,set,storage}.rs`, with a
small module entry point. The complete additive `int-set` feature enables all
three candidates and explicitly requires std. It reuses the existing Roaring
integration without adding a runtime dependency or changing the lockfile.
Ordinary mutable/frozen pairings retain their meaning.

Checked constructors validate observed round trips, canonicality, and order.
Checked word import rejects invalid domain codes. Fallible freeze plans for
Vec, MutableOrderedSet, and FrozenOrderedSet return their owned source on a
normal rejection. Allocation failures and user-code panics retain ordinary Rust
behavior. An extensible safe codec's laws never justify unchecked memory access.

All four materialized operations, scalar counts, and relations use encoded
storage. Iteration reconstructs logical values on demand. Cached cardinality is
u64; inclusive span arithmetic is u128; `try_len()` explicitly checks conversion
to usize. Logical equality, ordering, and hashing are independent of storage.
SnapshotStore accepts this frozen profile, retaining K's lifetime and auto-trait
restrictions through PhantomData.

The detailed [design](design/integer-sets.md) and
[adversarial review](reviews/integer-design-audit.md) describe the contracts,
planner estimates, result-representation rules, and deferred extensions.

## Adversarial findings addressed

- **Full-value reconstruction:** equality cannot reveal metadata discarded by
  a codec when equality itself ignores that metadata. The law requires the
  complete value; observed checks do not claim to prove arbitrary application
  code. Ordinary sets remain appropriate for representative metadata.
- **Wide arithmetic:** `[0, u64::MAX]` has two members and a span of 2^64. Dense
  budgeting uses u128 before allocation or narrowing. The full 2^64-member
  universe is outside this API's u64 cardinality domain and is checked.
- **Dense coordinate alignment:** bases align to absolute 64-bit word positions.
  Kernels combine matching coordinates, including around signed zero and the
  unsigned maximum. Widely separated dense union/XOR results fall back to
  sorted storage rather than allocating the intervening universe.
- **Roaring partition cost:** automatic planning accounts for run/container
  structure and upper-32-bit partitions. The default cap is 4,096 partitions;
  dense payload defaults to 8 MiB. Planner estimates are not heap measurements.
- **Encoded operation closure:** union, intersection, difference, and XOR select
  existing members. Checked restricted domains stay valid; a complement is
  expressed relative to an explicit valid universe.
- **Feature and abstraction boundaries:** external codec implementations compile
  in a separate package. Unsupported word widths, domain-mismatched algebra,
  and frozen mutation are rejected by compile-fail examples.
- **Measured mixed-probe cost:** cardinality alone chose repeated sorted binary
  searches for near-balanced operands. The final direction rule estimates
  binary-search work and dense zero-word traversal before the loop. It keeps
  compressed-side probing for genuine extreme skew. Both operand orders and
  several size ratios are checked against BTreeSet.

No unsafe code was added. Existing ordered kernels were reused by widening only
the necessary internal visibility. Their algorithm bodies were not changed by
this addition.

## Checks executed on the final implementation

The final complete gate was:

```sh
CARGO_NET_OFFLINE=true FROST_NO_STD=1 ./scripts/verify.sh all
```

It completed successfully after the mixed-probe improvement. The script uses
the pinned toolchain. Its [complete log](../verification/int-set/release-check.log)
contains the actual command output, including tests of the extracted Cargo
package. Host and tool details are in
[environment.json](../verification/int-set/environment.json).

| Check | Executed result |
| --- | --- |
| All-feature test run | **289 passed:** 3 library, 217 integration/property/model, 11 executable-example, and 58 rustdoc tests |
| Rustdoc contract tests | 33 runnable examples and 25 compile-fail examples, included in the 289 total |
| Integer integration target, all features | **23 passed**, included in the 217 integration total |
| Isolated integer profile | **21 passed** with no default features and only `int-set` enabled |
| Optimized integer target | **23 passed** with `int-set,arc-swap-1` |
| Separate downstream package | **2 runtime tests and 1 compile-fail doctest passed**, with its own retained lockfile |
| Quality | Formatting, local documentation links, all-target/all-feature Clippy with warnings denied, and rustdoc with warnings denied passed |
| Feature coverage | Core-only, alloc-only, default, and **20 isolated optional features**, including their Clippy, test, and rustdoc checks, passed |
| Examples | All **11 example programs** executed; their test forms also passed |
| Cargo package | **141 files** packaged; extracted-source all-feature tests and example tests passed |
| Embedded boundaries | Two library builds for `thumbv7em-none-eabihf` passed: core-only and the supported alloc/integration combination |
| Integer Miri | **4 focused tests passed** on `i686-unknown-linux-gnu` with strict provenance |
| Existing finite models | Ordered, bitmap, ownership, and publication models passed again against this source |
| Kani property helpers as ordinary Rust | **1 native helper test passed**; this is not a solver result |

Repeated profile and package runs exercise overlapping tests. The 289 count is
one complete all-feature run, not a sum inflated by those repetitions. Embedded
builds protect Frost's existing no_std boundaries; the new integer profile
itself requires std. Platform CI configurations are provided, but this execution
record does not claim native macOS or Windows runs.

### Integer exhaustive and generated coverage

The finite integer domain uses five values around word boundaries. It checks
all 32 by 32 subset pairs, all nine requested storage-policy pairs, and both
unsigned widths: **18,432 input/policy combinations**. Each combination checks
four materialized results, four cardinality queries, three set relations,
iteration order, membership, cardinality, and bounds against BTreeSet. Empty
representations may canonicalize, so these are not 18,432 distinct physical
layouts.

Additional regressions cover signed endpoints, u64 maximum windows, different
bases, full-width dense rejection, gaps between tiny dense operands, restricted
nonzero/enum domains, duplicate input, policy limits, and source recovery.
Malformed codecs exercise collisions, reversal, noncanonical decoding, and a
decoder that becomes invalid after construction. An explicit counter regression
checks that bulk operations and replanning perform no logical encoding or
decoding, while membership encodes its query once.

Three property tests each use 128 generated cases: signed ordering/round trips,
windowed u64 algebra across all policy pairs, and arbitrary u64 sorted/Roaring
algebra. The measured mixed-probe regression adds near-balanced 8,192:5,462 and
strongly skewed 8:4,096 cases in both operand orders and storage assignments,
using both widths and a large unaligned u64 origin. These are executable tests,
not an unbounded mathematical proof.

### Concurrency and model bounds

The integer snapshot regression uses four writers, each adding 64 disjoint typed
IDs, and a concurrent guarded reader. It checks complete ordered versions,
correct cached cardinality, a final 257-member set including the initial member,
and an unchanged retained initial snapshot. A separate regression checks
publication and displaced/retained versions directly.

The existing publication explorer executes the production retry helper and
passed all **198 three-writer schedules** plus **6 identity-changing two-writer
schedules**. Its trusted atomic-store assumptions and finite step bounds remain
explicit in [verification scope](verification.md). It does not model arc-swap's
internal memory reclamation or prove unbounded progress.

The other finite models again covered 4,096 ordered-set pairs, 66,564 bitmap
input/padding cases, and 33,614 five-operation ownership traces, with the
separate shorter ownership domain retained. Their
[current log](../verification/int-set/bounded-models.log) also records the native
Kani property-helper smoke test.

### Miri scope

The current integer Miri command was:

```sh
MIRIFLAGS=-Zmiri-strict-provenance cargo +nightly miri test --locked \
  --no-default-features --features int-set --target i686-unknown-linux-gnu \
  --test integer_sets dense_ -- --test-threads=1
```

The installed nightly alias corresponds to the pinned 2026-11-08 diagnostic
release; exact compiler and Miri commits are recorded in the environment file.
All four selected alignment/span/gap tests passed, with a successful process
exit; see [the log](../verification/int-set/miri-integer-32.log). This executes
32-bit address arithmetic on the production paths, including extreme u64
coordinates. It is not a native performance measurement or a check of every
integer test under Miri. The script includes this lane for future CI runs.

The prior candidate's unsafe-inline Miri results remain separately labeled
[historical evidence](production-readiness.md); this integer addition introduced
no new unsafe implementation or ownership-layout change.

## Performance evidence

The [integer benchmark report](integer-benchmarks.md) contains the native
controls, both serial runs, full sample links, and the measured mixed-probe
before/after comparison. Complete before-change source files and their matching
hashes are retained in
[the baseline directory](../verification/int-set/baselines/README.md), so the
comparison does not depend on an undocumented patch.

The benchmark validates complete outputs before timing. It measures counts,
all four materialized operations, mixed intersections, and construction on
identical logical datasets. Materialization includes output allocation and
destruction; construction includes validation, normalization, building and
teardown. Dataset generation is excluded. The construction fixtures are already
sorted, so these are not measurements of shuffled-ingest sorting throughput.

Automatic representation selection remains a deterministic memory-oriented
heuristic. It incurs a statistics pass and is not a universal speed selector.
Dense/Roaring results retain their native representation; mixed materialized
results use sorted codes. `with_policy` explicitly replans at an O(n) staging
cost. Constructor limits do not impose a total process-memory or perpetual
operation-output budget.

## Remaining release gates and limits

- **Loom was not executed:** the isolated package could not resolve Loom from
  the offline cache. The two instrumented explorations remain unexecuted; see
  [the availability log](../verification/int-set/loom-availability.log).
- **Kani was not executed:** `cargo kani` is unavailable. The six symbolic
  harnesses remain unexecuted. The passing native helper test is not a Kani
  proof; see [the availability log](../verification/int-set/kani-availability.log).
- **RustSec auditing was not executed:** `cargo-audit` is unavailable. The
  dependency versions are unchanged, but that is not evidence of a current
  vulnerability-database check; see
  [the availability log](../verification/int-set/audit-availability.log).
- **Deployment hardware remains a measurement gate:** timings come from one
  virtualized x86_64 host. Cold-cache workloads, application distributions,
  shuffled construction, allocator peaks, and native cross-platform execution
  require their own evidence before making deployment-specific guarantees.
- **Codec correctness remains an application contract:** observed checks cannot
  prove arbitrary code, detect equality-ignored data loss, or prevent future
  instability. The safe trait may fail logically or panic, but no codec law is
  used to justify unchecked memory operations.

Portable SIMD remains unavailable on the required stable compiler, confirmed
by the [actual 1.99.1 compile probe](../verification/int-set/portable-simd-probe.log).
The profile uses the existing Roaring implementation's supported SIMD dispatch.
Fresh intrinsics, C-backed Roaring, offset-u32 narrowing, alternate codecs,
mutable integer families, and serialization adapters are explicitly deferred.

## Evidence hygiene and delivery

Logs from development attempts remain in the archive. An initial benchmark
scope mistake was corrected before measurement. A subsequent gate rerun ran
out of build-cache disk; generated older build output was cleared and the full
gate and Miri command were rerun successfully. Those failed attempts are named
separately and are not counted as successful checks.

The project keeps AGENTS.md, contribution guidance, design philosophy, maintainer
notes, explicit release procedures, and reproducible scripts. The full ZIP
contains project files and verification records; generated Cargo target caches
are not project inputs. The archive is intended to be extracted and built on
its own, using its manifests, lockfiles, and pinned toolchain.

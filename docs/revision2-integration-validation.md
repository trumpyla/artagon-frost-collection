# Revision 2 integration validation

Execution date: 2026-11-09, UTC. Production toolchain: Rust 1.99.1,
edition 2024. The integrated production package is `frost-collections`,
with the library name `frost`.

The [integration review](reviews/revision2-integration-review.md) records the
merge decisions, reproduced defects, retained contracts, and implementation
costs. All final executed verification gates passed. The measurements show
large shared-clone improvements and mixed operation-dependent results; they do
not establish a universal performance improvement.

Evidence is retained under
[`verification/revision2-integration/`](../verification/revision2-integration/).
Historical reports and the supplied revision's evidence remain in the project,
with their source versions distinguished from this integration.

This report records the completed source integration before its import into the
GitHub checkout. The executed `scripts/verify.sh` commands below are retained as
provenance. Current development uses the root Makefile; the later
[GitHub import and Make checks](../verification/revision2-integration/github-import-ci/README.md)
are recorded separately and do not change the source scope of these results.

## Source and execution scope

Final production changes were complete before the final native, package,
affected-feature, embedded, Miri, and benchmark runs. The last refinement moved
native Roaring optimization from the generic storage constructor to the native
binary-operation boundary. This preserves the reviewed main's direct small
clone-and-edit path. It changed only `src/integer/storage.rs` and
`src/integer/storage/word.rs` after the complete isolated-feature matrix.

The final source/test/Cargo/script digest recorded by the affected-feature lane
is `276c6259a2de0f250c423f66f409b07c63012fd112bd96f273a3ff0c0093d549`.
The [source manifest](../verification/revision2-integration/post-benchmark-source-manifest.json),
[Miri status](../verification/revision2-integration/miri-status.json), and
[benchmark status](../verification/revision2-integration/benchmark-status.json)
provide per-file hashes or command provenance. Documentation, the integration
patch, and evidence indexes were finalized after those runs without changing
production Rust source.

## Executed verification

| Gate | Result and scope | Evidence |
| --- | --- | --- |
| Final `./scripts/verify.sh check` | Passed formatting, documentation links, all-target/all-feature Clippy with warnings denied, rustdoc with warnings denied, tests, and example checks | [check.log](../verification/revision2-integration/check.log) |
| Final native all-features tests | 271 unit, integration, and example tests; 59 doctests, comprising 34 executable and 25 compile-fail cases | [check.log](../verification/revision2-integration/check.log) |
| Final `./scripts/verify.sh package` | Packaged source built and passed its tests and examples after extraction | [package.log](../verification/revision2-integration/package.log) |
| Core, alloc, and default profiles | Passed the profile script; these configurations exclude the later two-file integer refinement | [profiles.log](../verification/revision2-integration/profiles.log) |
| All 20 isolated features | Clippy, tests, and rustdoc passed for each feature before the final two-file integer refinement | [features-final.log](../verification/revision2-integration/features-final.log), [status](../verification/revision2-integration/feature-platform-status.json) |
| Final alloc-only `int-set` recheck | Clippy passed; 17 library, 28 integer-set, and 18 integer-output tests passed on the refined source | [status and commands](../verification/revision2-integration/post-benchmark-validation.json) |
| Final integer and snapshot tests | 17 library, 30 integer-set, and 19 integer-output tests passed with `int-set,arc-swap-1` | [post-benchmark-integers.log](../verification/revision2-integration/post-benchmark-integers.log) |
| Integer release and downstream profile | Before the final refinement: 30 release integer-set and 19 release output tests; downstream custom-key fixture passed 2 tests and 1 compile-fail doctest | [integers.log](../verification/revision2-integration/integers.log), [counts](../verification/revision2-integration/integer-test-counts.json) |
| Final embedded integer builds | Alloc-only `int-set` built for both `thumbv6m-none-eabi` and `thumbv7em-none-eabihf`; this checks the Rc and Arc portability branches | [thumbv6m log](../verification/revision2-integration/post-benchmark-thumbv6m.log), [thumbv7em log](../verification/revision2-integration/post-benchmark-thumbv7em.log) |
| Final Miri | 70 Miri test executions passed: 50 host ownership checks and 20 i686 integer checks; 59 nightly doctests also passed | [miri.log](../verification/revision2-integration/miri.log), [status](../verification/revision2-integration/miri-status.json) |
| Independent policy oracle | 507,840 checks passed on the initial merge before the optimization-placement refinement; its precise earlier source scope is retained | [oracle source and reproduction](../verification/revision2-integration/policy-oracle-README.md), [provenance](../verification/revision2-integration/policy-oracle-provenance.json) |
| Final serial comparison | 634 benchmark cases completed with their answer checks: 420 paired cases and 214 new-only cases | [summary](../verification/revision2-integration/benchmark-summary.csv), [status](../verification/revision2-integration/benchmark-status.json) |

Counts describe distinct executions/configurations. Repeated examples and tests
across feature, package, release, and Miri runs are not added to the 271 native
tests as new unique tests. Cross-compilation does not constitute execution on
physical embedded hardware.

The final Miri run used `nightly-2026-11-08`, strict provenance, the existing
Tree Borrows ownership checks, and the `i686-unknown-linux-gnu` integer lane.
Its source comparison reports no file changes during the run. Six bounded
regressions exercise the partial chunked fold, full-container runs and early
breaks, overlapping-container sharing, Dense-to-Chunked boundaries, empty Dense
deletion, and dense run endpoints near the maximum code.

An earlier feature/Miri wrapper attempt was interrupted by an in-place shell
script edit after its file offsets moved. The original logs and statuses remain
available; it was not a Rust test failure. Both gates were rerun successfully
from captured stable script bodies while preserving their original working
directory, arguments, and command bodies. The completed final runs are the
ones summarized above.

### Content and policy coverage

The native finite five-value oracle checks all 32 by 32 subset pairs across
16 ordered representation pairs for both unsigned widths: 32,768
input/representation combinations. Other tests cover complete chunks, mixed
representations, signed and restricted codecs, maximum codes, Dense window and
Roaring partition boundaries, snapshots, and early termination.

The independent policy oracle uses standard `BTreeSet` results and checks exact
returned membership, representation, retained limits, equality, errors, and
Hasher event streams. It covers all four operations, all ordered representation
pairs, five output policy choices, and selected zero/small limits and extreme
inputs. It is finite selected-domain evidence, not a proof for all possible
sets, custom codecs, allocator behavior, or platforms. Its isolated Cargo
manifest is supplied for future reproduction; that manifest was not itself run
during this review. The recorded execution linked the already-built all-features
debug library as documented in its provenance.

## Final performance comparison

### Method

The [comparison harness](../verification/revision2-integration/ab-harness/src/main.rs)
compiles reviewed main and integrated source into one executable. The
[reviewed baseline fixture](../verification/revision2-integration/reviewed-baseline/)
preserves the attached reviewed source; only its Cargo package and library names
were changed to permit co-compilation. The compared dependencies use the same
versions present in the production lockfile, with `int-set,std` enabled.

The final run used Rust 1.99.1, Cargo release defaults, an empty `RUSTFLAGS`,
and no `target-cpu` override. The host was x86_64 Linux 6.18.57 on an
Intel Xeon Platinum 8573C; the executable was pinned to CPU 0. All project
compilation, native-test, and Miri lanes had finished before measurement.
[Environment metadata](../verification/revision2-integration/benchmark-environment.json)
records the available information and host limitations.

Each case used three rounds and five samples per version per round, with a
calibrated batch of at least 4 ms. The first implementation alternates by round.
The reported value is the median of the three round medians. Setup and answer
oracles are outside timing; the operation's allocations and destruction are
inside timing. Materialized answers are checked against standard-set expected
results, with additional constructor, count, and paired-result checks before
timing. There are 3,162 raw sample rows, each containing five sample times;
the raw file also retains its final completion marker.

The final run took 132.239 seconds and exited successfully. The
[full summary](../verification/revision2-integration/benchmark-summary.csv) and
[raw samples](../verification/revision2-integration/benchmark-raw.csv) retain
every case. The 214 new-only cases, mainly involving Chunked, have no old
implementation timing and are excluded from paired ratios.

### Representative observed timings

Times below are microseconds per named operation, rounded from the full CSV.
The lookup row is the complete 1,000-lookup batch. These examples deliberately
include improvements, approximately unchanged cases, and regressions.

| Case | Reviewed main, µs | Integrated, µs | Interpretation |
| --- | ---: | ---: | --- |
| `u32.skew.sorted.clone` | 16.2817 | 0.0392 | Shared root avoids copying the 65,536-member buffer |
| `u64.scattered_prefixes.roaring.clone` | 249.7369 | 0.0198 | Shared root avoids cloning the native container structure |
| `u64.scattered_prefixes.mixed_roaring_sorted.union_materialize` | 3,033.9350 | 416.9058 | Large improvement; both outputs are Sorted |
| `review.dense1m_union_roaring_singleton` | 8.0539 | 7.0066 | Small update retains Dense on both versions |
| `review.dense1m_difference_sorted4` | 7.7910 | 7.9616 | Similar observed time; both outputs are Dense |
| `review.far_apart_dense_union` | 150.9401 | 160.2700 | Modest observed slowdown; both outputs are Roaring |
| `review.construct_200k_sorted` | 594.1052 | 590.4290 | Similar observed construction time |
| `review.construct_200k_dense` | 771.8768 | 1,007.1450 | Observed construction regression |
| `review.construct_200k_roaring` | 1,457.1428 | 1,878.1998 | Observed construction regression |
| `review.run1000_auto_contains_x1000` | 1.5301 | 2.1219 | Observed lookup regression |
| `u32.skew.auto.snapshot_update` | 11.3786 | 12.9873 | Observed update slowdown remains |
| `u32.skew.roaring.snapshot_update` | 10.9854 | 39.1029 | Large observed slowdown with substantial sample dispersion |
| `review.construct_empty` | 0.0032 | 0.0370 | Empty construction now allocates a shared header |

Among the 420 paired cases, 136 used at least 20% less time, 22 used at least
20% more time, and 262 fell within those thresholds. The unweighted median
old/new ratio was approximately 1.033. These counts and ratios describe this
chosen workload collection; they are not a weighted application speedup or a
statistical confidence estimate. The [analysis JSON](../verification/revision2-integration/benchmark-analysis.json)
records the definitions and counts.

The execution host is shared, and several sample distributions are wide. In the
forced-Roaring snapshot row, for example, integrated raw samples range from
roughly 11 to 64 µs. Pinning and alternating the order reduce some confounding,
but do not remove scheduling, clock, allocator, or cache variation. The measured
slowdowns remain in the report; the run does not establish that each ratio is
stable across machines or attributable solely to a single source change.

The first complete comparison exposed an avoidable optimization scan after
small cloned Roaring updates. The final implementation removes that scan from
the shared storage constructor while retaining optimization at native binary
operation boundaries. The earlier measurements and both affected source files
are preserved under [first-pass/](../verification/revision2-integration/first-pass/).
The final record is a complete rerun after the fix, not a selection of each
case's best result across runs.

### Remaining cost boundaries

Clone performs constant-time reference counting, while final-owner destruction
still visits and frees owned storage. Chunked payloads are individually shared,
but the flat key/reference directory can require O(C) work for a changed result.
Chunked/Dense scalar predicates can allocate temporary chunks; Chunked-to-Roaring
conversion can enumerate codes. Some policy conversions or identity discoveries
can allocate before returning a shared result or an error. The representation
limits and payload diagnostics do not measure total process memory or temporary
allocation. These are implementation costs documented in the review, rather
than unimplemented persistent-tree or expression guarantees.

## Reproduction

For a fresh run from the current project root, use the Make targets below with
the pinned production toolchain and the documented nightly/target components
installed. These are the current equivalents of the original script scopes,
not a claim that Make produced the preserved logs above. The embedded target
requires both `thumbv7em-none-eabihf` and `thumbv6m-none-eabi`.

```sh
make check
make profiles
make features
make integers
make embedded
make miri
make package
```

For the same-executable comparison, build first, then measure without concurrent
project verification. Choose an available CPU if CPU 0 is not available on the
reproduction host. The commands below write fresh output in the current directory
and leave the supplied evidence intact:

```sh
cargo build --release --locked --manifest-path verification/revision2-integration/ab-harness/Cargo.toml
taskset -c 0 verification/revision2-integration/ab-harness/target/release/frost-integration-ab 3 > benchmark-summary.csv 2> benchmark-raw.csv
```

The [integration code patch](../patches/revision2-integration-code.diff) targets
the exact reviewed attachment. The complete source archive also includes the
updated documents, original diffs, fixtures, provenance, and logs. Build outputs
are excluded. This validation does not claim a crate publication, remote push,
hardware test campaign, exhaustive custom-codec proof, or universal speedup.

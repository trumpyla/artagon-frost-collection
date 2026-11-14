# Integer output validation

> Historical evidence for the source revision named below. The current merged
> implementation is documented in [revision-2 integration validation](../../../docs/revision2-integration-validation.md).

This revision integrates the follow-up output and wrapper-cost review into the
complete Frost source distribution. It independently reproduces the supplied
summary, corrects the implementation, preserves the first measured iteration,
and records verification separately from performance claims. The crate remains
an unpublished production candidate: `publish = false` is deliberate.

## Inputs and scope

The input was the review summary supplied in chat. Its full review document and
harness were not attached. The exact supplied text is retained in
[the supplied summary](../../../docs/reviews/supplied/integer-output-review-summary.md); our
[disposition and independent review](../../../docs/reviews/integer-output-review.md) distinguish
confirmed source behavior, independently measured observations, and proposals.
No claim is made to have run the supplier's unavailable harness.

The before implementation is the complete previously delivered archive, bundled
as [baseline.zip](../../../verification/integer-output/baseline.zip), with SHA-256:

```text
516a019e46cf7755bb495fbb92356ea3044a0560863d075ef4f2b4dea9195886
```

The current comparison compiles one common public-API harness unchanged against
that archive and the updated source. Its source digests, dependency pins, flags,
compiler records, raw samples and output checks are under
[`verification/integer-output/`](../../../verification/integer-output/README.md).
The [benchmark report](integer-output-benchmarks.md) owns the measured timing
results. Earlier integer-profile and production logs remain historical evidence;
their results are not silently relabeled as checks of this revision.

The later [views research review](../../../docs/reviews/lazy-views-adversarial-review.md) adds
proposed expression and persistent-storage designs, primary-source audits, and
an isolated [mathematical bounds model](../../../verification/integer-views/README.md).
It does not implement either proposed type. Their admission tests are not
included in the production implementation totals below.

After timing finished, final documentation review corrected one stale module
rustdoc statement that still said int-set required std. The retained
[post-measurement record](../../../verification/integer-output/post-measurement-changes.json)
checks every measured input digest, preserves the measured module, and records
that the only changed measured file differs in module documentation. Runtime
logic, benchmark source, lockfile, and compiler configuration remain as measured.

## Implemented contracts

The public profile stays `FrozenIntSet<K>` over safe extensible `IntKey` and
sealed u32/u64 storage words. The ordinary mutable/frozen family pairings remain
unchanged. This correction adds `IntSetLimits`, four fallible operation-policy
methods, and an exact contiguous payload diagnostic. The private word/backend
mapping now has its own `crates/frost-collections-integer/src/integer/storage/word.rs` module.

Materialized operations preserve suitable compressed output. Small updates can
clone Dense storage and modify changed bits while maintaining cardinality and
bounds. Distant Dense windows can promote through safe little-endian byte chunks
to native Roaring operations. Large Sorted/Roaring operations use sorted native
construction; the clone-and-edit path is bounded to 64 Sorted updates. Skew-aware
intersection direction is retained for counts, materialization and explicit
Sorted results.

Every result retains representation limits. Union, intersection and symmetric
difference compose the componentwise minimum; difference keeps the left limits.
Explicit `*_with_policy` calls override both the result limits and requested
backend. Identity, empty and separated-operand shortcuts obey the same rules.
Limits cover Dense payload bytes and Roaring partitions, not Sorted buffers,
temporary conversion storage, total resident memory, or allocator failure.

Auto construction uses a transparent memory estimate with a 128-byte preference
for Dense when the alternative saving is small. Auto operations can retain legal
Dense or Roaring results competitive with Sorted. Consequently, Auto does not
promise a unique representation or global minimum memory for identical contents
reached through different operation histories.

Equality uses native or contiguous comparisons for matching representations.
Hash chooses a canonical content-derived member or maximal-run stream. Exact
Hasher-call consistency is tested across representations, policy limits and
operation histories. A bounded scout avoids two endpoint writes for every sparse
member; full contiguous spans hash cached bounds directly. Hash output is not a
persistent serialization contract. Ordered comparison remains over ascending
canonical words.

The complete `int-set` feature now supports `no_std + alloc`. It does not enable
`std` transitively. Default-feature users retain std; manifests disabling defaults
must request `std` explicitly when they need standard-library families, supported
runtime CPU dispatch, or native dependency std APIs. The existing `roaring-0_12`
integration keeps its prior std requirement. Dependency versions were not upgraded.

## Executed release checks

Normal checks used Rust **1.99.1**, edition **2024**, on Linux x86-64. Commands
used the committed lockfiles and offline dependency resolution. The primary
release gate was:

```sh
FROST_NO_STD=1 CARGO_NET_OFFLINE=true ./scripts/verify.sh all
```

It completed with **exit 0**. See the full
[release-check log](../../../verification/integer-output/release-check.log) and
[execution status](../../../verification/integer-output/release-check-status.json).

| Gate | Executed result |
| --- | --- |
| Formatting and repository document links | Passed |
| All-target, all-feature Clippy | Passed with `-D warnings` |
| Public documentation | Passed with `RUSTDOCFLAGS="-D warnings"` |
| Primary all-feature suite | **312 passed**: 8 library unit tests, 235 integration tests, 11 example tests, and 58 doctests; none failed or ignored |
| Core-only, alloc-only and default profiles | Tests and applicable strict lint/doc checks passed |
| Individual integrations | All **20 isolated features** passed library Clippy, tests and docs with default features disabled |
| Integer output regressions | **18 passed** with snapshot support; **17 passed** in the alloc-only integer lane |
| Existing integer regression suite | **23 passed** with snapshot support; **21 passed** in the alloc-only integer lane |
| Optimized integer tests | All **41** snapshot-enabled output and integer cases passed in release mode |
| Separate downstream codec crate | Locked compilation, tests and doctests passed |
| Examples | All **11** example programs executed their assertions; example test targets also passed |
| Source package | Cargo package verification and tests of the extracted package passed |
| Bare-metal builds | Core-only, alloc/integration aggregate, and the complete int-set profile compiled for `thumbv7em-none-eabihf` |

The primary 312 count is one all-feature suite. Repeated profile, package, release,
example and interpreter executions are not added to that number. The feature
matrix checks individual integrations and their union; it does not enumerate the
entire feature powerset. Bare-metal results establish compilation, not execution
on a physical microcontroller.

The downstream lockfile required an explicit refresh after the std feature
boundary changed. Offline workspace resolution removed 19 unused dependency
entries, added none and changed no retained dependency versions or checksums.
The [lock refresh record](../../../verification/integer-output/downstream-lock-refresh.json)
and [previous downstream lock](../../../verification/integer-output/downstream-Cargo-before.lock)
make that change reviewable.

## Adversarial and finite coverage

The original finite integer test still runs every pair of subsets of five
boundary values for both unsigned widths and all nine input representation pairs:
**18,432 input/representation combinations**. It compares actual public operations,
counts, bounds, order and predicates with independent standard-set answers. Three
property tests run 128 generated cases each. These are finite and generated
checks, not proofs over every integer set or arbitrary downstream codec.

The new output suite tests compact tiny updates, distant Dense windows, exact
payload diagnostics, independently composed limits, explicit result policies,
empty/self shortcuts and Roaring partition caps. Four boundary windows include
u32-prefix transitions, large 64-bit bases and u64::MAX. All nine input pairs,
three forced outputs and four operations give **432 forced materializations** in
that test, in addition to default operations and scalar checks. Signed and
restricted domains remain covered.

Hash tests compare complete Hasher event sequences, not merely one final digest.
They cross 64-bit words, 16-bit containers, upper-32-bit prefixes, signed zero and
maximum integer endpoints. The 64/65-member and 16/17-run selection boundaries
are checked with at most 85 members per fixture. Separate counter tests check
that skewed intersection avoids scanning the large operand and that discovering
a seventeenth run stops the scout before traversing its tail. Neither test uses
a wall-clock assertion.

Native snapshot tests retain old versions while applying updates and check that
repeated tiny updates keep Dense storage. The existing concurrent integer test
uses four writers publishing 64 distinct IDs each through the actual SnapshotStore
and checks that complete versions retain every committed ID. This exercises the
real wrapper and dependency; it does not exhaust operating-system schedules.

## Miri

The final full Miri script passed with **exit 0** on current production code and
tests. `FROST_MIRI_TOOLCHAIN=nightly` resolved to Rust
**1.102.0-nightly**, commit `f5cedb83a`, dated 2026-11-07. The full commands,
flags and output are in [miri.log](../../../verification/integer-output/miri.log).

| Executed lane | Passed | Explicitly filtered |
| --- | --- | --- |
| Native nightly doctests | 58 (33 ordinary + 25 compile-fail) | 0 |
| Strict provenance: core review | 14 | 0 |
| Strict provenance: families | 11 | 2 random-operation tests |
| Strict provenance: inline ownership | 12 | 0 |
| Explicit Tree Borrows + strict provenance: inline ownership | 12 | 0 |
| Tree Borrows: named three-operation ownership model | 1 | 2 other test functions |
| i686 strict provenance: original Dense boundary cases | 4 | 17 other integer tests |
| i686 strict provenance: private byte-promotion cases | 2 | 6 other unit tests |
| i686 strict provenance: private hash/skew counter cases | 3 | 5 other unit tests |
| i686 strict provenance: five exact public output/hash cases | 5 | 15 other functions per invocation |

This is **64 successful interpreter test executions**, plus **58 native doctest
cases**. The repeated inline tests exercise the scripted flag configurations.
Doctests are compiler executions, not Miri executions. Selected integer cases
cover canonical hashing, both scout thresholds, construction history, empty/self
policies, explicit result policies, byte promotion and maximum-width arithmetic.
The interpreter does not execute every native integer/property test. The
native greater-than-8-MiB budget test remains native-only and is not counted as
an interpreter pass.

The complete earlier pass is preserved as
[miri-first-pass.log](../../../verification/integer-output/miri-first-pass.log). A later
attempt was deliberately interrupted while an unnecessarily large counter-test
fixture was interpreted; its
[incomplete log](../../../verification/integer-output/miri-long-fixture-incomplete.log)
is not a passing result. The fixture was changed globally, for native and Miri
execution alike, from a 7,168-member final run to a 128-member final run. The
remaining two-word tail still exceeds the asserted first-word endpoint and
checks the same early-stop property. No model domain, validation flag or native
large-budget case was selectively reduced for Miri.

## Bounded concurrency and ownership models

The [bounded-model log](../../../verification/integer-output/bounded-models.log) records
`./scripts/verify-formal.sh bounded` completing with **exit 0**. Seven native test
functions passed: four exhaustive collection/ownership checks, two publication
models and one ordinary Kani-helper driver. The domains include:

- **4,096** ordered-set input pairs, including iterator prefixes and counts.
- **66,564** bitmap input/padding combinations.
- **33,614** five-operation ownership traces and **686** three-operation traces.
- **198** three-writer publication schedules and **6** schedules covering an
  equal-content replacement with a different version identity.

The publication models execute the exact production retry helper. They assume
an atomic identity-checked store, coherent immutable handles, finite one-shot
writers and terminating derivations. They verify the stated finite protocol;
they do not prove ArcSwap's atomics/reclamation, weak-memory behavior inside that
dependency, fairness, or unbounded writer progress. See
[verification scope](../../../docs/verification.md) for exact assumptions and bounds.

The Kani-helper driver is an ordinary Rust test. It is not a symbolic proof.

## Final source recheck

After the bounded Miri fixture and the research documentation were finalized,
`./scripts/verify.sh check` passed again with **exit 0**, including strict
Clippy/rustdoc, formatting, document links, the primary all-feature suite and
the explicit example-test lane. The primary suite remains **312** cases; the
example lane repeats 11 cases and is not added to that primary count. See
[final-check.log](../../../verification/integer-output/final-check.log) and its
[status](../../../verification/integer-output/final-check-status.json).

`./scripts/verify.sh package` also passed again with **exit 0**, verifying and
testing the extracted Cargo source package with all features and examples.
Its [log](../../../verification/integer-output/final-package.log) and
[status](../../../verification/integer-output/final-package-status.json) record the
actual command and completion. The full ZIP additionally includes supplied
research and verification evidence intentionally excluded from Cargo packaging.

## Open release and proof gates

Current availability probes are preserved separately:

| Gate | Actual status |
| --- | --- |
| Symbolic Kani execution | Unexecuted: `cargo kani` is not installed; [probe](../../../verification/integer-output/kani-availability.log) |
| Loom exploration | Unexecuted: the pinned Loom package was unavailable in offline resolution; [probe](../../../verification/integer-output/loom-availability.log) |
| Current RustSec advisory audit | Unexecuted: `cargo audit` is not installed; [probe](../../../verification/integer-output/audit-availability.log) |
| Native macOS/Windows validation | CI jobs are provided; this workspace's recorded runs are Linux |
| Publication and release identity | Owner must supply repository, support policy and security contact, accept the gates, and deliberately enable publishing |

No unavailable tool is counted as passing. There is no unbounded formal-correctness
or universal performance claim. The integer implementation introduces no unsafe
blocks; the crate's existing reviewed unsafe surface remains in inline storage.
Dependency internals, allocation failure, custom codec law violations and
platform-specific behavior retain the limits described in the design.

## Corrected intermediate failures

Evidence preserves unsuccessful attempts as well as final results. An initial
all-target Clippy run rejected an empty-collection assertion style in a new unit
test; the assertion was corrected without changing the production algorithm.
A later release run stopped at the stale downstream lockfile, which was refreshed
as described above. An early embedded driver returned a shell parse error while
the verification script was being edited; the stable rerun and final full gate
passed all three builds. The interrupted large Miri fixture is explicitly labeled
incomplete. These are distinct from the first measured performance regressions,
which are preserved in the [review](../../../docs/reviews/integer-output-review.md) and
[benchmark evidence](../../../verification/integer-output/first-pass/README.md).

The final ZIP contains the complete source, tests, documentation, CI and scripts,
the prior full archive, comparison harnesses and recorded evidence. Generated
build caches are not part of the source distribution.

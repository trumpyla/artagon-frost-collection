# Integer output review: reproducible comparison

This harness independently reproduces the supplied review summary. The reviewer's
full new harness was not available. The bundled `baseline.zip` is the previously
delivered complete source archive; its SHA-256 is
`516a019e46cf7755bb495fbb92356ea3044a0560863d075ef4f2b4dea9195886`.

The benchmark source is `benches/integer_output.rs`, with the common timing
support in `benches/support/mod.rs`. The same files are compiled against both
versions, using only their shared public API. The comparison script extracts the
baseline into an isolated directory and creates standalone manifests; neither
version's source manifest is edited. The script seeds both comparison lockfiles
from their source lockfiles and rejects newly selected dependency versions.

The recorded measured-source hashes remain unchanged. A final rustdoc-only
correction in `src/integer/mod.rs` is documented in
[post-measurement-changes.json](post-measurement-changes.json), together with a
retained copy of that measured module. Every other measured input still matches;
no runtime logic or benchmark setting changed after the recorded comparison.

## Reproduce

Run these from the crate root with Python 3.11 or later and Rust 1.99.1 installed:

```sh
python3 verification/integer-output/compare.py --work-dir /tmp/frost-integer-output --phase build
python3 verification/integer-output/compare.py --work-dir /tmp/frost-integer-output --phase verify
python3 verification/integer-output/compare.py --work-dir /tmp/frost-integer-output --phase bench
python3 verification/integer-output/summarize.py /tmp/frost-integer-output
```

Build is the default phase. `verify` runs all complete-answer checks and one
untimed invocation of each measured operation. Only `bench` collects timing
samples. All compilation finishes before timed binaries run sequentially. Run
that phase without competing compilation, tests, or other benchmarks. Cargo
uses `--offline --locked` for builds; `--online` permits fetching missing cached
dependencies during reproduction.

The default builds both explicit x86-64 profiles: `-C target-cpu=x86-64` and
`-C target-cpu=x86-64-v3`. Native controls share the same flags as Frost in each
binary. The v3 binary requires a compatible CPU. `--profile baseline` or
`--profile v3` selects one profile. The script records the compiler, host, harness
and source hashes, profile flags, matched lockfiles, and output logs. Timed runs
are pinned to CPU 0 (`--cpu` selects another available CPU) and execute
before/after/after/before in each profile, retaining both independent seven-sample
runs per version. This counterbalanced order exposes drift; it does not remove
environmental noise. Untimed verification uses one run per version/profile.

`--case` selects `mixed`, `distant`, `sorted_roaring`, `equality_hash`, `construction`,
`intersection`, or `contains`. Each saved CSV preserves all seven samples in
execution order; its median is a warm-cache wall-clock observation, not a
confidence interval. Input creation and full-answer checks are outside timing.
Output allocation and destruction are inside. The 32-update case also includes
cloning the initial snapshot value; it measures value derivation, without atomic
publication. `contains_1024_queries` rows report the whole 1,024-query batch.

For a candidate-only run using Cargo's benchmark target:

```sh
cargo bench --locked --features int-set --bench integer_output
```

## Workload and control scope

| Group | Reproduction and controls |
|---|---|
| Mixed output | One million alternating integers in a 250,000-byte dense window, union/XOR with a Roaring singleton, difference of one member, and 32 repeated updates. Raw bitmap clone-and-set and native Roaring clone-and-insert controls. |
| Distant runs | Two 4,194,304-member dense runs, each a 524,288-byte payload, separated by `2^27`. Native Roaring union control and a separately labeled serialized-size observation. |
| Sorted/compressed | Two balanced million-member inputs for union and both difference directions. Native prepared-Roaring controls exclude conversion; separately labeled build-and-operation controls include the sorted builder and optimization. Million-member Sorted intersections with two-member Dense/Roaring sets exercise both operand orders and include raw binary-search/native controls. |
| Equality and hashing | Equal independently cloned storage for a million-member run, a million alternating values, eight runs totaling 32,768 members, seventeen runs totaling 69,632 members, 2,048 sparse local values, and 2,048 upper-prefix partitions. Native Roaring Eq, sorted-payload Eq, and dense-word Eq controls. |
| Construction | Validated Frost Roaring construction compared with native `collect` plus `optimize` and `from_sorted_iter` plus `optimize` on identical ascending inputs. Frost's codec validation and normalization remain included. |
| Intersection | Dense/Dense and Dense/Roaring count/materialization, native promotion through `from_lsb0_bytes` from bytes prepared outside timing, cached promotion count, and raw word AND/popcount. |
| Membership | `0..1000` with Auto, Sorted, Dense, Roaring, and native Roaring, using the same 1,024 mixed hit/miss queries. |

The native `promotion_and_count` and `promotion_and_materialize` controls include
`from_lsb0_bytes` and the native operation, but the input byte vector is built
before timing. Frost's timed Dense path also constructs byte buffers from its
stored u64 words, then applies result metadata and policy handling. Consequently
the residual ratio against these controls is not pure wrapper overhead. The
separate `cached_promotion_count` control excludes promotion entirely. This
qualification also applies to the same controls preserved under `first-pass/`.

Roaring 0.12.0 has a native equality implementation, but no native `Hash`
implementation. `native_roaring.custom_member_hash` explicitly hashes cardinality
and each iterated member with `DefaultHasher`. `raw_sorted.payload_hash` and
`raw_dense.payload_hash` hash their buffer values. These controls perform
different logical work from Frost's canonical hashing with a bounded run scan;
their timings are not interchangeable semantic baselines. Before/after
`.logical_hash` rows do measure the same public Hash operation and logical sets.
The production contract does not promise stable hash bytes across crate releases.

## Size interpretation

Sorted payload bytes are cardinality times encoded word width, excluding the
wrapper and allocator. The shared harness can calculate only the minimum Dense
payload from aligned bounds because the historical API has no payload accessor.
Candidate regression tests additionally use `contiguous_payload_bytes()` to
assert the actual Sorted or Dense buffer length. Roaring's serialized size is a
wire-format measurement and is never reported as allocated resident memory.
The payload budget does not limit staging buffers, allocator overhead, total
peak memory, or Sorted fallback size.

The standalone candidate-only `payload.rs` checks the original large workloads
through the public payload accessor, without timing or adding a package to the
library dependency graph:

```sh
python3 verification/integer-output/check_payload.py --work-dir /tmp/frost-integer-payload
```

`payload-check.csv` records the exact 250,000-byte Dense input, singleton-union
result, and result after 32 updates; the 16-byte Sorted fallback retaining an
8-byte dense budget; both 524,288-byte distant Dense inputs; and `None` for the
Roaring union. `None` means that this accessor does not report a discontiguous
allocation size. It is not a zero-byte or resident-memory assertion. All
complete member sequences, cardinalities, expected representations, and payload
values are asserted by the reproducer.

## Correctness evidence

`tests/integer_output.rs` compares complete materialized answers and count queries
with `BTreeSet`, covers all representation pairs and forced output policies, and
checks coordinate boundaries, retained budgets, empty/identity shortcuts,
partition caps, and explicit budget overrides. Its recording hasher compares
the exact sequence of typed write calls and byte boundaries across equal sets
with different representations, limits, and construction histories. Boundary
datasets cover 64-bit words, 16-bit containers, 32-bit prefixes, signed zero,
maximum integer endpoints, and the 64/65-member and 16/17-run selection
boundaries. The `arc-swap-1` case verifies that repeated
snapshot updates keep compact storage and that retained versions stay unchanged.

These finite tests and measured workloads do not constitute proof of arbitrary
codec implementations or dependency internals. Their role is to make each
specific reviewed failure and each new output-policy contract reproducible.

`first-pass/` retains the complete first measured implementation's evidence and
source snapshot. Its additional materialization and sparse-hashing costs drove
the final refinement; those observations are preserved rather than overwritten.

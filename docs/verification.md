# Verification scope and reproducible checks

Frost uses executable finite models, property tests, native concurrency tests,
and Miri checks. Optional Loom and Kani harnesses add independent verification
tools. A green unit-test run does not establish memory safety for every safe
caller, and a model of the retry protocol does not establish the correctness of
arc-swap's implementation.

Use the root [Makefile](../Makefile) for current execution: `make check` is the
normal gate, `make verify` runs the broader host matrix, and `make embedded`,
`make miri`, `make bounded`, `make loom`, and `make kani` are explicit additional
scopes. `make help` lists the smaller targets. The contributor guide describes
[tool prerequisites and shell checks](../CONTRIBUTING.md).
Older reports retain the exact historical `scripts/verify*.sh` commands; those
dispatchers have been replaced by Make recipes. The current documentation-link
check uses Bash and the metadata recipes use `jq`, with no Python dependency.

The release report records which commands actually ran, their toolchain, and
their results. The presence of a harness, workflow, or proof annotation is not a
successful verification result. Do not describe an unavailable, unsupported,
timed-out, or partially explored check as passing.

The current integer output corrections have a separate
[execution record](integer-output-validation.md) under `verification/integer-output/`.
Earlier logs under `verification/results/` describe the prior production candidate;
the initial integer addition is recorded under `verification/int-set/` and its
[historical report](integer-validation.md).

### Optional-tool status recorded in this review

The Kani package's ordinary helper test passed with Rust 1.99.1 on
`x86_64-unknown-linux-gnu`; see the
[native helper log](../verification/results/kani-native-smoke.log). Its isolated
lockfile records the tested dependency resolution. This is a native test result,
not a Kani proof result. The verifier command was unavailable
([log](../verification/results/kani-availability.log)), and offline resolution
could not find Loom ([log](../verification/results/loom-availability.log)).
Therefore the six symbolic Kani proofs and the two Loom explorations remain
unexecuted gates in this review.

## Coverage map

| Target | What is executed | Domain or bound | What the result does not establish |
| --- | --- | --- | --- |
| `tests/exhaustive_collections.rs`: ordered algebra | Actual public frozen-set constructors, iterators, folds, counts, and predicates | All 4,096 pairs of subsets of six ordered values; every consumed iterator prefix | Correctness for arbitrary sizes, arbitrary `Ord` implementations, or every galloping path |
| Same file: dense algebra | Actual public bitmap operations and full-word kernels | Six membership positions at word boundaries; eight logical lengths; all valid masks; independent clean/all-one input padding | Every possible multiword bitmap, every padding pattern, or every architecture |
| Same file: inline ownership | Actual `InlineVec` mutation, retention, spill, shrink, freeze, and thaw | All 7^5 operation traces at capacities 0 and 2; 33,614 traces total | Arbitrary capacities, arbitrary destructors, or panic behavior |
| `tests/publication_model.rs` | The exact production `publication::update` loop | All modeled load/CAS orders for three one-shot writers, and a two-writer equal-value publication case | arc-swap internals, weak-memory behaviors inside a primitive, fairness, or unbounded writer progress |
| `tests/integer_sets.rs` | Actual typed constructors, all nine storage pairs, all four materialized operations, counts, predicates, and cached bounds | All pairs of subsets of five boundary values for both unsigned widths: 18,432 input/representation combinations, plus generated and adversarial cases | Arbitrary codecs, every integer input, all allocations, or internals of the Roaring dependency |
| `tests/integer_output.rs` and integer storage unit tests | Actual result planning, byte promotion, limit inheritance, native equality, canonical Hash, and snapshot updates | All nine input representation pairs and three forced outputs across four boundary windows; explicit budget/identity cases; endian/chunk regressions; exact Hasher event sequences | Minimal memory for every workload, bounded process memory, arbitrary codecs, or the complete dependency implementation |
| `tests/concurrent.rs` | Actual snapshot wrapper and dependency | Native regression histories, forced conflict/retry, and bounded concurrent writer workloads | Exhaustive operating-system schedules or a proof of the dependency |
| `tests/inline_safety.rs` under Miri | Actual unsafe inline storage through public APIs | Concrete ownership, alignment, zero-sized-value, partial-iterator, and panic paths | Soundness for every safe caller or a complete model of Rust's semantics |
| Three-operation inline test under Miri | Actual inline transitions and per-element ownership counters | All 7^3 traces at capacities 0 and 2; 686 traces total | The larger native five-operation domain, arbitrary capacities, or every unwind path |
| `verification/loom` | The same production retry loop with Loom-instrumented synchronization | Two and three one-shot incrementing writers; no preemption, permutation, or duration truncation | arc-swap's pointer publication, reference counting, reclamation, or lock-free progress |
| `verification/kani` | Actual public collection APIs and the exact private `InlineBuf` source | Six bounded harnesses, described below | Unbounded collection correctness, arbitrary callbacks/destructors, or concurrent memory ordering |

The ordinary tests and the standalone Kani property helpers are executable
without installing Loom or Kani. The two optional packages are isolated from
the main dependency graph. Installing a verifier must not change the crate's
runtime dependencies or supported Rust version.

## Finite exhaustive collection checks

Run from the repository root:

```sh
cargo +1.99.1 test --locked --test exhaustive_collections --features fixedbitset-0_5
```

The ordered-set oracle encodes membership in a six-bit integer. It independently
specifies union, intersection, difference, symmetric difference, subset,
superset, and disjointness with Boolean operations. For each resulting iterator,
the test checks every possible already-consumed prefix, the remaining sequence,
`size_hint` bounds, specialized `count` and `fold`, and repeated exhaustion.
This gives complete input coverage for that finite domain; the larger generated
and adversarial tests cover shapes that six-element sets cannot reach.

The bitmap domain places independent membership bits at positions
`0`, `W-1`, `W`, `W+1`, `2W-1`, and `2W`, where `W = usize::BITS`. Logical lengths
are `0`, `1`, `W-1`, `W`, `W+1`, `2W-1`, `2W`, and `2W+1`. There are 129 valid
length/membership configurations. Every pair is checked with four independent
choices of clean or all-one trailing padding: 66,564 input cases. These cases
exercise empty, one-word, two-word, and multiword paths. Result enumeration,
cardinality, logical universe, and the underlying backend's sanitized padding
must agree with the independent membership oracle.

The inline test uses non-`Clone` values with individually tracked ownership.
Its seven operations are push-zero, push-one, pop, remove-first, retain-zero,
freeze/thaw, and shrink. After every transition, a plain sequence model checks
the contents and a per-element counter checks that every removed element was
dropped once and every retained element remains alive. Final destruction checks
all remaining owners. The separate panic regressions cover unwinding behavior.

## Snapshot retry model

Run:

```sh
cargo +1.99.1 test --locked --test publication_model -- --nocapture
```

Both the real store and the model call
[`src/concurrent/publication.rs`](../src/concurrent/publication.rs). The test
includes that file directly; it does not maintain a second copy of the retry
algorithm. Only the atomic store is replaced.

Each worker runs the production loop on a real thread. Channels stop it before
each modeled load or CAS. A deterministic scheduler chooses the next enabled
atomic operation, executes it, and lets that worker run its pure local work
until the next atomic boundary. The explorer replays every alternative enabled
choice at each prefix. Local derivation and return steps do not access shared
model state, so collapsing those steps preserves the modeled outcomes.

The three writers use noncommuting transformations `4*x + 1`, `4*x + 2`, and
`4*x + 3`. The successful commit order therefore determines a distinct serial
result. Every run checks that each writer commits exactly once, derives once
per attempt, returns its own displaced version, and produces that serial
result. Another case includes a publication that preserves the value while
changing version identity. Native wrapper tests independently check the real
pointer-identity contract. The tests also require the exact number of terminal
schedules: 198 for three writers and six for two writers. These independent
expected counts detect accidental pruning in the explorer.

### Trusted primitive and finite bounds

The model assumes all of the following:

1. A load returns a coherent immutable version handle.
2. CAS compares handle identity in one atomic step. On success it publishes the
   candidate and returns the displaced handle. On failure it leaves the store
   unchanged and returns the candidate plus the version observed by that CAS.
3. A retained handle stays valid and its identity is not reused while live.
4. Derivation terminates, has no shared side effects, and does not panic.
5. Each of the two or three writers performs exactly one successful update;
   no other writer or unconditional publication exists in the model.

Under these assumptions, a failed attempt observes another successful commit.
For `n` one-shot writers there are at most `n(n-1)/2` failed attempts, `n`
successful attempts, and `n` initial loads. The scheduler asserts the resulting
bound of `n + n(n+1)/2` atomic steps: at most nine for three writers. An exceeded
bound fails the test; it is not a cutoff that silently discards paths.

The expected handle may become stale immediately after any load or failed CAS.
The scheduler allows another writer between that observation and the next CAS.
The model proves no fairness or starvation property for an unbounded workload.
Allocation, callback execution, destruction, and the dependency's internal
progress properties remain outside this model.

### Optional Loom lane

```sh
cargo +1.99.1 test --manifest-path verification/loom/Cargo.toml
```

This isolated package pins Loom 0.7.3. Its backend implements the trusted store
with `loom::sync::Mutex` and uses Loom threads and shared ownership. It explores
the same production retry loop with two and three writers and checks the final
value and the complete set of displaced versions.

The builder explicitly clears permutation, duration, preemption, and checkpoint
limits inherited from the environment. Its 256-branch ceiling is a failure
condition if exceeded. Do not lower the exploration coverage to obtain a green
release check. Obtain dependencies, generate and retain this package's lockfile,
and rerun with `--locked` before recording a reproducible release result.

Loom can only observe operations implemented using its instrumented types.
Merely running ordinary `std` atomics or arc-swap under `loom::model` would not
verify those operations. This harness deliberately claims protocol coverage
under an atomic-store assumption. See the [Loom documentation][loom] and
[builder controls][loom-builder].

## Unsafe storage under Miri

The only runtime module allowed to use `unsafe` is
[`src/adaptive/seq/buf.rs`](../src/adaptive/seq/buf.rs). Its invariant is that an
inline buffer owns exactly the initialized prefix, and an owning iterator owns
exactly its remaining contiguous range. Moving storage must transfer that
ownership before any destructor can run.

The reproducible nightly selected for this review is `nightly-2026-11-08`:

```sh
rustup toolchain install nightly-2026-11-08 --component miri --component rust-src
cargo +nightly-2026-11-08 miri setup
cargo +nightly-2026-11-08 miri test --locked --test inline_safety
cargo +nightly-2026-11-08 miri test --locked --test exhaustive_collections inline_ownership_exhausts_three_operation_traces
```

The three-operation test has its own name and is also an ordinary native test.
It uses the same ownership-checking helper as the five-operation test, crosses
the capacity-two spill boundary, and keeps interpreter runtime manageable. The
five-operation test remains unchanged and is not silently reduced under Miri.

The integer profile adds a separate 32-bit arithmetic and alignment check:

```sh
MIRIFLAGS=-Zmiri-strict-provenance cargo +nightly-2026-11-08 miri test --locked \
  --no-default-features --features int-set --target i686-unknown-linux-gnu \
  --test integer_sets dense_ -- --test-threads=1
```

These four named tests execute the production constructors and operations near
signed zero, `u64::MAX`, and distant dense windows. They verify checked rejection
of a full-width dense allocation before narrowing to the 32-bit address size.
They are not exhaustive over all 64-bit inputs. The command is included in the
Miri script for future CI runs; its current result is recorded in
[integer output validation](integer-output-validation.md).

The earlier integration review's complete `scripts/verify.sh miri` gate passed using the
installed `nightly` alias for that dated toolchain; see the
[current Miri log](../verification/results/miri-current.log). Strict-provenance
Stacked Borrows passed 37 cases across `core_review`, `families`, and
`inline_safety`, excluding two named random-operation cases. Strict-provenance
Tree Borrows passed all 12 inline-safety cases again and the separately named
686-trace ownership test. The same gate also passed 51 native nightly doctests;
those doctests are compiler checks, not Miri executions.

The output revision also runs two private conversion unit tests and selected
public output tests on the 32-bit Miri target. The conversion test instantiates
the same private chunking function with small chunks to exercise splitting
without allocating an 8 MiB scratch buffer in the interpreter. Native tests keep
the production-sized path and the explicit greater-than-8-MiB result-budget case.
Selected Miri names and executed counts are recorded in the current report;
large native-only or unselected cases are not counted as Miri passes.

Keep alignment, validity, aliasing, race, and leak checks enabled. If the target
or toolchain rejects an operation, record the unsupported check and investigate;
do not disable validation and report an equivalent result. Additional Miri seeds
or Tree Borrows runs can find different issues, but must be identified separately
in the results. A normal unit-test pass is not evidence of a Miri pass.

Miri detects violations on executed paths. Its own documentation explicitly
distinguishes that from a proof of soundness and describes incomplete concurrent
schedule and weak-memory exploration. See the [official Miri guide][miri].

## Optional Kani proofs

The proof package calls Frost through a path dependency. For private storage it
includes the exact `InlineBuf` source by path, so an implementation change changes
the program submitted to the verifier. It does not stub the algorithms under
test or substitute copied collection implementations.

| Harness | Symbolic domain | Assertions and checks | Unwind limit |
| --- | --- | --- | --- |
| `ordered_set_algebra` | Every pair of subsets of four ordered values | Four set operations, counts, and three relations agree with bit membership | 16 |
| `sorted_constructor_preserves_or_rejects_input` | Four arbitrary bytes, length 0 through 4 | Accept exactly strict ascending inputs; preserve accepted or rejected input; report first invalid adjacent pair | 16 |
| `inline_prefix_preserved` | Five arbitrary operation bytes and five arbitrary value bytes, capacity 3 | Push rejection, pop, truncate, mutable access, initialized prefix, length, and full-state invariants | 16 |
| `inline_transfer_drops_each_owner_once` | Length and consumed count 0 through 3, arbitrary front/back choices, heap/iterator transfer | Every original owner is destroyed exactly once; source ownership becomes empty | 16 |
| `inline_zero_capacity` | Capacity zero | Rejected push, empty pop/slice/iterator, and empty heap transfer | 4 |
| `dense_padding_does_not_create_members` | Lengths 0 through 6; two arbitrary complete machine words | Scalar membership, result counts/universes, subset/disjointness, and sanitized result padding | 32 |

Kani also checks the supported classes of memory errors, panics, and arithmetic
errors in the explored program. The inline destruction harness uses a bounded
nonpanicking destructor; panic/unwind ownership is covered separately by native
and Miri tests. The dense proof domain is intentionally small; word-boundary and
longer kernels remain covered by the executable exhaustive and property suites.

The verifier is installed separately according to the [Kani installation
guide][kani-install]. Run the native property helpers first, then the verifier:

```sh
cargo +1.99.1 test --manifest-path verification/kani/Cargo.toml
cd verification/kani
cargo kani --lib
```

The first command checks that the property helpers compile and execute against
the real implementations. It does not exercise `kani::any`, the proof attributes,
or symbolic verification. A release must report the Kani command separately and
require all six expected harnesses to be discovered and pass.

As reviewed on 2026-11-09, Kani 0.68.0 is the published release and uses its own
nightly compiler dated 2026-08-21. This crate requires Rust 1.99.1. Compatibility
of that verifier/compiler with this package must be established by running it;
the ordinary Rust toolchain's success does not establish compatibility. The
review environment did not initially have Kani or its solver installed. Do not
lower `rust-version`, remove checks, or describe a blocked verifier run as a
successful proof. See the [Kani release notes][kani-releases].

Every harness states an unwind limit, and the package defaults otherwise to one
iteration so newly added loops cannot silently receive an arbitrary generous
bound. Keep unwinding assertions enabled. If a bound fails, inspect why, increase
it only to cover the intended finite execution, and rerun. Never disable
unwinding assertions to turn an incomplete analysis into a green result. See
[Kani's bounded-proof and loop-unwinding explanation][kani-unwind].

## Recording and maintaining evidence

For a verification result, record the exact revision or source digest, command,
toolchain/verifier version, enabled features, target, input or schedule bounds,
and full success/failure output. Record unavailable dependencies and unsupported
toolchain combinations as open gates. Keep timing benchmarks separate from
sanitizers, interpreters, and model-checking processes so they do not compete for
CPU or memory.

Any change to unsafe ownership, iterator state, bitmap padding, or snapshot
publication must preserve the relevant invariants and run the corresponding
checks. Expand a finite domain when it exposes a new representation boundary;
do not relabel a finite result as a proof over arbitrary sizes. A change to the
atomic publication dependency also needs upstream evidence and native wrapper
regressions, because the protocol models intentionally trust that primitive.

[loom]: https://docs.rs/loom/0.7.3/loom/
[loom-builder]: https://docs.rs/loom/0.7.3/loom/model/struct.Builder.html
[miri]: https://github.com/rust-lang/miri/
[kani-install]: https://model-checking.github.io/kani/install-guide.html
[kani-releases]: https://github.com/model-checking/kani/releases/tag/kani-0.68.0
[kani-unwind]: https://model-checking.github.io/kani/tutorial-loop-unwinding.html

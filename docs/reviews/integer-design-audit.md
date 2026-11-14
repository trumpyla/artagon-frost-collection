# Adversarial review of the integer-profile proposal

> Historical audit of the initial integer-profile delivery. Its std requirement
> and construction-only output policies were superseded by the
> [integer output review](integer-output-review.md). See the current
> [integer design](../design/integer-sets.md) for maintained contracts.

## Decision

The core split is sound: a logical K, one associated primitive Word, and private
data-dependent storage are separate decisions. Implement the explicit profile
without changing ordinary mutable/frozen pairings. Harden the proposed design
around codec limits, arithmetic, mixed coordinates, planner accounting, and
feature unification before treating it as a production collection.

The supplied text's isolated-prototype results are a claim about that prototype.
This integration relies on the actual crate tests, downstream compilation,
compiler probe, and benchmarks recorded for this source. A successful prototype
alone says nothing about mixed dense/Roaring operations or a storage planner.

## Findings and disposition

| Severity | Finding | Resolution |
| --- | --- | --- |
| High | Safe downstream codecs can discard data, collide, reverse order, or change over time. | State full-value/canonical/order/stability laws; check observable violations; retain checked decoding; never derive unsafe access from those laws. |
| High | Testing `decode(encode(k)) == k` cannot prove full-value reconstruction when equality ignores metadata. | Explicitly document the limitation and require ordinary frozen sets for representative metadata. |
| High | Inclusive u64 span overflows u64 for the full domain; allocation lengths can truncate on smaller targets. | Compute spans/budgets in u128, check before narrowing, and use u64 cardinality independently of usize. |
| High | Dense windows at different bases cannot combine corresponding buffer indices directly. | Align bases to absolute 64-bit word coordinates and use overlap/gap-aware kernels. |
| High | A dense union across a huge gap can allocate a catastrophic window despite small operands. | Bound result window allocation and fall back to encoded storage instead of blindly extending the bitmap. |
| High | Cargo feature unification can change representation variants behind an existing public type. | The single `int-set` feature enables the complete fixed profile and explicitly requires std/Roaring. |
| Medium | Cardinality alone predicts RoaringTreemap poorly for scattered high prefixes. | Add a hard upper-32-bit partition cap and a conservative planner; benchmark scattered prefixes directly. |
| Medium | Serialization size is not actual heap or resident size. | Label Roaring accounting as a planning estimate; make dense payload budgeting exact and explicitly limited in scope. |
| Medium | Replanning every native algebra result would add full enumeration and transient allocations. | Pair kernels select output directly; `with_policy` is an explicit costly rebuild. |
| Medium | The smaller mixed operand can require many expensive binary searches while a slightly larger sorted scan could use constant-time bitmap membership. | Compare scan and probing work before selecting direction; retain large-skew controls and measured before/after evidence. |
| Medium | A phantom marker may accidentally erase lifetimes or widen auto traits. | Use `PhantomData<K>` and ordinary auto-trait derivation; no unsafe widening. |
| Medium | Compressed values do not exist at stable K addresses. | Yield K by value, expose `ValueElements`/`Cardinality`, and omit borrowed-element and ExactSizeIterator promises. |
| Medium | Range filling/complement/deserialization can synthesize invalid restricted-domain codes. | Only checked logical-word import is added; complement is relative to an explicit valid universe. |
| Medium | SIMD is neither universally stable nor automatically beneficial for every pair. | Reuse the pinned Roaring kernels; prove stable compiler constraints with a real probe; defer fresh intrinsics and C dependencies. |

## Codec validation cannot be universal

The checked constructor validates the keys it observes. It does not run all
possible inputs to a downstream function or prove an arbitrary Ord implementation.
Adjacent order checks after primitive-word sorting detect a reversed codec over
the provided domain, but cannot prove transitivity for every possible custom
ordering or detect future global-state changes.

A useful counterexample is a key containing `(id, metadata)` whose Eq and Ord
compare only id while its codec emits only id. A value reconstructed with empty
metadata can compare equal to the input. Accepting it is not evidence that the
full key round-tripped. The contract must require full reconstruction, and must
explain why ordinary frozen sets remain appropriate for such representatives.

An invalid codec is a logical-contract violation, not permission for undefined
behavior. The collection sorts primitive words, uses checked array/index
operations, and checks decode failure. A changing decoder can cause a checked
panic during iteration; it cannot turn a reserved word into an invalid enum via
an unchecked transmute. Backend algebra itself never decodes K.

## Arithmetic and allocation boundaries

For `[0, u64::MAX]`, the inclusive span is 2^64 even though cardinality is two.
These quantities must not share an integer-size assumption. Dense planning
checks its u128 payload before conversion or allocation. The `Cardinality`
capability returns u64; conversion to usize is explicit through `try_len`.
Iterator upper size hints must become None when a remaining count cannot fit.

Union cardinality must subtract overlap before addition or check equivalent
wide arithmetic. `a + b - common` can overflow even when the final union fits.
Full u64-universe cardinality itself is outside the public u64 count domain;
that mathematical overflow is checked rather than silently wrapped.

The dense limit bounds bitmap payload, not all memory used by construction.
Input collection, normalization, a temporary candidate, allocation headers,
and rebuilding are separate costs. Explicit limits do not turn ordinary Rust
allocator failure into a recoverable error. A plan that advertised a complete
peak-memory cap would need a different streaming and allocation design.

## Type abstraction and feature behavior

The public associated type is u32/u64. Backend and iterator mappings stay inside
the crate. The private supertrait is deliberately restricted to the two words,
and downstream fixture compilation is required because an internal test alone
does not establish that applications can implement the public key trait.

The complete profile requires std through the existing Roaring integration.
Core-only and alloc-only Frost users retain their existing dependency graphs.
There is no feature-dependent extra variant added to a type already exposed
under a weaker feature. C-backed alternatives would be a separately named
profile, and a future offset-u32 representation would require its own reviewed
compatibility decision rather than silently changing a promised candidate set.

## Performance work and current limits

The accompanying `integer_sets` benchmark supplies independent BTreeSet answers
and native BTreeSet, FrozenOrderedSet and width-specific Roaring controls. It
includes auto/forced choices, large-base local windows, skewed u32 inputs,
consecutive run clusters, scattered u64 prefixes, signed zero crossings,
application IDs, and all nine nonempty storage-pair combinations on one common
large-base dataset. Dataset generation is excluded. Explicit construction
cases include validation, sorting, allocation and destruction. Materialization
includes allocating and dropping the output.

Dense cases rejected by the configured budget are reported as ineligible; they
are not timed through an unbounded allocation. The benchmark validates full set
answers before timing and uses seven chronological raw samples. A correct
cardinality alone would be insufficient to validate a materialized answer.
The [integer benchmark report](../integer-benchmarks.md) preserves the recorded
results, the initial mixed-direction problem, and the scope of each measurement.

These are warm-cache synthetic workloads, with chronological samples rather than
statistical confidence intervals. Construction inputs are already ascending, so
those cases measure codec validation and representation construction with an
easy sorting input; they do not establish shuffled or duplicate-heavy ingestion
throughput. No allocator instrument measures total live bytes or allocation
counts. Native Roaring controls are optimized before operation timing, matching
the profile's construction-time run optimization.

The planner targets an understandable storage-cost proxy, not universal fastest
operations. Sorted membership, scalar intersection, materialized intersection,
and repeated construction have different preferences. A Roaring set can win on
long runs yet lose on scattered high prefixes. A dense representation can save
space but scan many empty words for sparse operands. Mixed pairs can require
linear traversal or allocate a sorted result. These are measured tradeoffs to
expose, not reasons to hide the representation diagnostic.

## SIMD evidence and deferred work

The inspected, locked Roaring 0.12.0 package has dependencies on fearless_simd
and fearless_simd_macros; its std feature forwards to fearless_simd/std. Its
README and the [current upstream README](https://github.com/RoaringBitmap/roaring-rs)
describe runtime x86/x86-64 instruction selection for supported kernels. Its
treemap source stores `BTreeMap<u32, RoaringBitmap>`, explaining the distinct
upper-prefix cost. These dependency-source facts were checked against the
installed package rather than inferred from a stale search excerpt.

The actual [Rust 1.99.1 probe](../../verification/int-set/portable-simd-probe.log)
reports E0658 for `std::simd::u32x4::from_array`. The
[standard-library documentation](https://doc.rust-lang.org/stable/std/simd/)
also identifies portable SIMD as experimental. Adding `#![feature(portable_simd)]`
would violate the pinned stable compiler requirement. Stable intrinsics or an
audited abstraction remain possible future kernels, with runtime detection and
portable fallback tests as separate acceptance criteria.

The initial implementation does not add a new CRoaring backend, Lemire kernel
binding, offset-u32 narrowing, alternate codec parameter, arbitrary BLOCK_SIZE,
mutable integer family, or compressed-format deserializer. Those would expand
the verification surface without being necessary to test the proposed generic
interface and fixed representation family. Rank/select and range APIs can be
considered after their domain, cardinality, and performance contracts are set.

## Review anchors

- [Implemented profile design](../design/integer-sets.md)
- [Public key laws](../../crates/frost-collections-integer/src/integer/key.rs)
- [Public construction and set API](../../crates/frost-collections-integer/src/integer/set.rs)
- [Policy and checked errors](../../crates/frost-collections-integer/src/integer/policy.rs)
- [Benchmark source](../../crates/frost-collections-integer/benches/integer_sets.rs)
- The current integer validation report records executed results; historical
  snapshot/concurrency results remain separate evidence for the prior revision.

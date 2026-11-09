# Design philosophy

Frost makes collection semantics, ownership transitions, and access costs
explicit. A family name should be dependable in an application signature even
when a private kernel is improved. An optimization earns its place by retaining
the semantic contract and removing measured work.

## Stable semantics before clever storage

Ordinary families own duplicate handling, traversal order, mutation access, and
a fixed frozen partner. They are not a universal enum over every backend. An
ordered map always has ordered semantics; enabling a hash-table feature must
not alter its bounds or representation. Native backend wrappers remain useful
when native behavior and specialized APIs are the desired contract.

The first equivalent key or set element remains the stored representative; the
last map value wins. Left representatives survive union and intersection.
These rules matter for keys whose comparison ignores metadata. They are tested
with distinct representatives rather than only small integers.

The explicit integer profile has a stronger value contract: its codec must
reconstruct the complete key. It stores canonical unsigned codes and decodes
values on demand. Metadata discarded by a codec cannot be recovered, even if
equality ignores it; use an ordinary family when stored representatives matter.
Codec laws remain safe-trait requirements, never permission for unchecked memory
access. See [the integer profile](integer-sets.md).

## Freezing is an ownership transition

A frozen value provides no structural mutation or mutable element access.
Freezing does not promise deep immutability, pinning, zero-copy conversion, or
cheap clone. A boxed sorted array, a retained native table, and a published Arc
have different costs. Name those costs rather than redefining “frozen” to imply
one preferred layout. Recover ownership by consuming the frozen value through
its documented thaw or extraction API.

## Capability traits express what an algorithm needs

Borrowed entries, reconstructed values, ordered traversal, indexed access, and
cardinality are distinct capabilities. Compressed storage must not fabricate
references to values it reconstructs. A wide bitmap count must not silently
narrow to usize. Specialized APIs remain available without inflating every
generic bound or pretending all backends have the same iterator properties.

## Policy is explicit and features are additive

Adaptive profiles are separately named types with caller-visible thresholds.
Default thresholds are conservative policy values, not universal crossover
claims. Removal does not silently demote; explicit shrinking can rebuild.
Compatibility-line feature names make incompatible third-party APIs coexist
deliberately. Core-only and alloc-only users must not inherit an optional
integration's std requirement or a verification tool dependency.

## Optimize work, then verify the tradeoff

Use a linear merge for balanced sorted input, and seek when the shape justifies
it. Preserve contiguous buffers when consuming ownership. Avoid repeated key
comparisons and per-word machinery when invariants allow a simpler kernel.
Measure clustered as well as distributed data, tiny as well as large inputs,
partial iterators as well as bulk counts, and native operations as controls.

An extra branch, iterator field, allocation, or lookup can hurt a common path
while helping a benchmark. Record those costs. A large improvement against a
scan-the-large-set baseline is not evidence of superiority over the native
small-side intersection. Keep correctness checks outside timing and preserve
raw measurements, including slower cases.

## Concurrency guarantees belong to a precise operation

Live guarded access and whole-snapshot publication are different contracts.
One loaded snapshot gives a stable structural version. Unconditional publish
can overwrite another writer's work; conditional publish compares identity;
update retries derivation after conflict. A callback can run more than once.
The successful atomic replacement is the update's linearization point.

Avoid unconditional “wait-free,” “lock-free,” or “reference-count-free” promises
for a workflow that includes allocation, callbacks, user drops, finite fast-read
slots, and retries. Prefer explicit lifetime and progress statements that an
application can use when setting its latency or memory budget.

## Evidence has a scope

Differential tests compare observable behavior with independent reference
collections. Exhaustive finite tests quantify small input domains. A scheduler
model explores stated schedules using the actual retry helper. Miri diagnoses
undefined behavior on executed paths. Symbolic harnesses need executed solver
results with loop bounds and assumptions. None of these alone proves all
possible programs or the atomics inside every dependency.

Keep the unsafe surface small, explain initialized-prefix and transfer
invariants locally, and use non-Clone, zero-sized, panic, and drop-accounting
cases. Correctness and explainability come before a speculative layout. Record
deferred experiments with their decision criteria so a future maintainer can
revisit them without repeating unsupported claims.

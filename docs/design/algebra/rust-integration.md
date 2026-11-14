---
id: FROST-ALG-RUST
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Rust capabilities, type names and implementation placement

This document maps the specification to inspected source, without implementing
or renaming its public API. The project remains one root package until epic
frost-wlt performs the migration under [CRATES.md](../../../CRATES.md).

## Preserve existing capability boundaries

| Existing capability | Formal role | Do not infer |
| --- | --- | --- |
| ElementType / MapTypes | Rust carrier types | Validity, identity laws or source completeness |
| SetContains<Q> | Local infallible membership | Durable error handling or stored references |
| SetGet<Q> | Stored representative recovery | Enumeration or sorted order |
| Elements / Entries | Borrowed stored traversal, GAT iterators | Clone, ExactSizeIterator, fusion or arbitrary dyn compatibility |
| ValueElements / LookupValue<Q> | Reconstructed/copied values | A stored address or exact owned-payload reconstruction beyond the codec contract |
| ValueRef<Q> | Borrow a payload without promising stored key | LookupRef's key provenance |
| OrderedEntries plus Entries | Ascending map traversal and endpoints | Insertion order as an equivalent merge guarantee |
| Len / Cardinality | Declared usize / u64 count | Total bag multiplicity, key-universe size or cheaply computed count |
| FreezeWith / TryFreezeWith | Source/plan-specific typed transitions | Universal thaw or zero allocation |
| Eligible / SnapshotValue | Sealed admission under an owner | User-extensible safety proof from a domain marker |

See inspected [read capabilities](../../../crates/frost-collections-core/src/read.rs),
[plans](../../../crates/frost-collections-core/src/plan.rs), [wrapper](../../../crates/frost-collections-core/src/frozen.rs), and
[integer codec](../../../crates/frost-collections-integer/src/integer/key.rs). Query compatibility belongs on
implementations: Borrow, Equivalent-style queries, ordered comparison or a
specific explicit relation. Do not impose Borrow globally and exclude lawful
backends that supply a different equivalent-query protocol.

## Optional domain interfaces, not universal generics

Use refined key/payload types with checked constructors first. A generic
ValueDomain/ValidateValue interface is optional when a real consumer needs
validation dispatch. Keep no blanket Eq+Ord+Hash+Clone+Send+Sync+'static bound.
A domain-bound computed source additionally needs immutable semantic and
representation bindings plus a compatible evaluation snapshot handle.

Do not retrofit Set<T,D>, Map<K,V,DK,DV> or a generic FrozenValue into current
families by default. API compatibility no longer forbids it (see the
[decision of 2026-11-12](../decisions.md#api-compatibility-is-not-a-constraint-2026-11-12)),
but the design costs remain: every caller pays the extra bounds, feature matrix,
impls and borrowed-query inference, including callers with no runtime-scoped
domain. A new view can wrap a source and a checked binding
without changing the source's published type. A compatibility witness checks
that its source handles still refer to the admitted versions; it is not a free
pass for a mutable registry to change meaning after construction.

## Access and async profiles

Keep three access models: borrowed stored references, reconstructed values, and
lending records whose lifetime is tied to cursor advancement. Dynamic queries
need a deliberate object-safe/erased interface; a GAT-based Elements interface
cannot simply be boxed as a trait object with every associated lifetime intact.

A durable source should use a separate fallible sync/async adapter, with error,
end, pending, cancellation and budget states. Do not make local no_std traits
runtime-dependent. Pinning page buffers, backpressure and dropping a cursor are
execution obligations. Return owned keys or lender-scoped references honestly;
never let a result reference outlive the next advance of its scratch buffer.

## Ownership and output types

A borrowed binary join can keep two source lifetimes. A simpler API may require
a shared shorter borrow, but it must not impose 'static or Clone to conceal the
choice. Consuming variants can move payloads or reuse buffers with documented
panic/drop behavior. Error types own the specific recovery guarantee. A map of
references is a legitimate materialized output, not an independently owned copy.

Open semantic traits must remain safe. Incorrect Eq/Hash/domain law assertions
must not justify unchecked indexing, invalid references or unsafe conversion.
Sealed admission still belongs to the crate defining the seal. Logical validity
and memory safety are separate proof obligations.

## Proposed ownership after extraction

| Responsibility | Current owner / suggested future owner |
| --- | --- |
| Existing traits, exact wrappers, ordinary/adaptive families and their native/framework impls | Root package; cohesive frost-core candidate |
| Existing binary family methods | The crate defining each family |
| Generic sorted-slice kernels | `crates/frost-collections-core/src/family/ordered/set_ops.rs`; frost-set-kernels candidate |
| Typed integer storage and full-value codecs | src/integer; frost-integer-sets candidate |
| New composed set expressions | New owner such as frost-set-algebra, depending downward on capabilities/kernels |
| New map expressions and joins | Candidate frost-map-algebra |
| Sequence expression composition | Candidate frost-sequence-algebra; do not move Vec storage merely for the name |
| Bags / multimaps / BiMaps | Future family owners, with their own domain operations |
| Live access and sealed snapshot publication | src/concurrent; frost-concurrent candidate |
| Curated public entry point | Root frost today; target frost-api package with library name frost |
| Durable indexing, credential verification, policy or symbolic semantics | Separate engine/adapter layer |

A separate expression crate does NOT require moving current inherent methods.
Keep those methods with their types and put new view types in the expression
owner. Framework impls must obey Rust coherence: either trait ownership or a
qualifying local type in the impl is needed; a local associated output type is
not itself such a qualifying trait/self type. Existing foreign backend impls,
freeze plans and private transfers are reasons not to over-split core.

Package names and boundaries remain design inputs to frost-wlt.1; this document
creates no crates and approves no earlier package catalog. No naming rule is
adopted: [design decisions](../decisions.md#research-corpus-and-rationale-2026-11-09)
record plural and singular spellings as alternatives, and the research synthesis
proposes singular names.
No frost-domains microcrate is needed before a concrete consumer justifies it.

## Implementation sequence

1. Accept domain definitions and naming dispositions, with explicit exceptions.
2. Implement a simple borrowed semantic baseline without blanket domain params.
3. Map each public obligation to actual Rust tests/catalog records under CRATES.
4. Add terminal-specific sorted/hash/encoded execution, checked against the baseline.
5. Add bound runtime-source descriptors and fallible durable adapters separately.
6. Measure construction, result enumeration, counts, allocations, retained versions,
   cleanup, and cold/warm I/O before choosing materialization thresholds.

Current make check/feature/Miri/model gates are not replaced by this research
model.

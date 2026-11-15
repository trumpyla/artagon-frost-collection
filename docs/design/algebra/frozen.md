---
id: FROST-ALG-FROZEN
status: proposed-contract
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Frozen values as a lifecycle refinement

Frozen is not a new algebraic carrier. It constrains the structural access and
ownership of a representation of a set, map, sequence, bag, relation or BiMap.
Its denotation still comes from that underlying domain and invariant.

## 1. Typed transitions, not a universal unary operator

For a particular family and plan, specify

    freeze_p : Mutable_R ⇀ Frozen_S
    recover_q : Frozen_S ⇀ Mutable_R'   (when supported).

[FROZEN-DENOTE] On successful freezing of an already well-formed family,
α_S(freeze_p(x))=α_R(x), with representative/payload guarantees stated separately.
Raw input construction may first normalize duplicate/order constraints according
to a named policy. It is not the same operation as freezing a lawful family.

freeze(freeze(x))=freeze(x) can be ill-typed because freeze takes a mutable
family and returns a different type. Do not advertise generic idempotence on
Rust APIs. A normalization may be idempotent, or an exact retaining plan may
preserve denotation; neither implies a universal Frozen endofunction.

Where a thaw/recovery exists, semantic round trips do not establish a full
representation isomorphism. Capacity, adaptive thresholds, allocation identity,
encoding or cost may change. The current adaptive ordered map freezes to
FrozenOrderedMap, whose ordinary thaw returns MutableOrderedMap; returning to
an adaptive profile is explicitly different. Do not promise a universal inverse.

## 2. Exact wrappers, families, encoded types and queries

| Kind | Current/proposed meaning |
| --- | --- |
| Frozen<B> | Implemented exact-backend wrapper with sealed Eligible admission and consuming into_inner |
| Named frozen family | Implemented ordinary mutable/frozen partner contracts |
| Adaptive frozen profile | Implemented explicit representation policy; partner behavior documented individually |
| FrozenIntSet | Implemented encoded eager set, full-value IntKey codec, sharing and checked result policies |
| IntSetExpr / generic SetView | Proposed computation; not an already materialized set |
| FrozenIntSetView | Separate proposed validated storage view; not a synonym for IntSetExpr |
| PersistentIntSet | Separate proposed persistent family; not all frozen values have its intended update costs |

Consult the [Frozen API guide](../../../crates/frost-collections-core/docs/frozen-api.md) for actual method and recovery
contracts. No universal FrozenValue, FrozenCollection or FrozenValueDomain trait
is introduced here. New semantic interfaces belong only where an executable
consumer needs them.

## 3. What freezing does not prove

[FROZEN-STABLE] No structural mutation through the frozen API does not imply
stable payload state or stable identity fields reachable through interior
mutability. Hash/order-relevant fields must remain stable while indexed.
A foreign validator returning Ok once does not establish lifetime-long validity.

Frozen does not imply deep immutability, pinning, zero allocations, cheap clones,
portable serialization, thread safety, or current revocation data. Existing
Send/Sync auto traits follow contained types and the target's actual Arc/Rc
choice; do not add blanket unsafe implementations from algebraic arguments.

## 4. Materialization and persistence

Materialization constructs a result container. Its elements can be borrowed,
shared-owned, decoded, moved or cloned. FrozenVec<&T> is materialized but still
borrows T; an Arc<T> removes one container-lifetime dependency while possibly
sharing mutable interior state. Ownership independence and semantic stability
are different observations.

Persistent representation retains old logical versions through sharing. An Arc
around an entire collection alone is not a path-copy tree. Current changed
Chunked integer results share untouched payload containers while copying a flat
directory. That implementation must not be described as logarithmic tree-path
updating. Retained versions retain storage and eventual destruction costs.

## 5. Frozen algebra and result policy

A frozen result may be returned by a pure operation without modifying its
inputs. Output policy can reject a representation even when a mathematical
identity would return an operand. Every identity shortcut must satisfy that
policy before reusing storage. Logical equality does not include a storage
variant, but representation diagnostics remain observable where exposed.

Auto construction and Auto output planning differ: the latter may reuse an
eligible empty Chunked operand. Do not strengthen empty-set algebra into an
unsupported Sorted-normalization requirement. Forced policies must still be
honored. The formal model does not change production behavior to make every empty
result Sorted.

SnapshotStore has sealed payload admission. An arbitrary aggregate, query
expression or new FrozenBag is not automatically eligible. Future new payload
admission needs its own ownership and cross-version review. A retained in-memory
snapshot does not establish a transaction across separately published stores.

## 6. Acceptance

Test semantic and representative preservation; fixed partners; exact-wrapper
native semantics; rejected-input recovery; non-Clone/non-Default payloads;
interior-mutability boundaries; partial iterators and destructor behavior;
old-version retention; explicit result policies; and target-specific auto traits.
Miri/Loom/Kani evidence remains separate from the finite mathematical model of
phase transitions. No such Rust verification is claimed by this document.

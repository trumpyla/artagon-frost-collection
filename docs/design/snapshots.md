# Snapshot publication contract

`SnapshotStore<F>` is available with `arc-swap-1`. `F: SnapshotValue` is sealed
to structurally frozen collection types. `Send` and `Sync` derive from the
actual payload and backend; Frost does not assert them with unsafe impls.

| Operation | Ownership and result | Concurrency contract |
| --- | --- | --- |
| `load()` | Owned `Arc<F>` | Retains one complete version for reuse |
| `load_guard()` | Crate-owned `SnapshotGuard<F>` | Protects one version; dereferences to `F` |
| `peek(f)` | Callback result | Borrows one guarded version for a short synchronous read |
| `publish(value)` | Displaced `Arc<F>` | Unconditional atomic replacement |
| `publish_if_current(&expected, value)` | Displaced `Arc<F>`, or `(rejected F, observed Arc<F>)` | Replacement succeeds only if pointer identity matches |
| `update(f)` | Displaced `Arc<F>` | Re-derives and retries conditional publication after conflict |
| `SnapshotGuard::into_owned()` | Owned `Arc<F>` | Retains exactly the protected version and releases its guard |

## Identity and linearization

Conditional publication compares the expected allocation's identity, not
collection equality. Two equal collections in different Arcs are different
versions. Holding the expected Arc keeps its allocation alive, preventing
address reuse while that expectation is retained. A successful compare and
replace is the linearization point. The returned previous Arc is exactly the
version displaced by that operation, even if another writer has published by
the time the call returns.

On failure the candidate was never published. Ownership returns to the caller
without requiring `F: Clone`. The observed version was current at the failed
comparison; another writer may already have replaced it. The retry helper uses
that observation for another attempt rather than promising a fresh value at
return. `publish` remains unconditional and is suitable for replacing a full
configuration from an authoritative source.

## Callback, panic, and drop behavior

An update callback may execute repeatedly. It should derive a replacement from
the supplied snapshot without performing effects that require exactly-once
execution. Success returns the displaced snapshot, not the newly published
snapshot. Use a subsequent load if only the current value matters; concurrent
publication means that load may observe an even newer version.

A panic in derivation does not publish that attempt's candidate. Other writers
can still have published concurrently. Failed candidates and displaced values
are dropped by their owners; user destructors may execute on these paths and
can panic or take time. No rollback across independent writers is promised.

## Reader lifetime, memory, and progress

Both guards and owned Arcs keep old versions alive across publication and even
after the store is dropped. Guarded loads can use a fast protection path but
can fall back to owned reference counting; publishing can promote protected
versions to ownership. Prefer short guard lifetimes and owned Arcs for storage
or retention across suspension. The wrapper deliberately does not expose the
upstream guard type as its public contract.

Slow readers can retain multiple large old versions. Contending update writers
can rerun expensive derivations and starve an individual writer. Publication
also allocates, and callbacks/destructors have application-defined behavior.
There is no unconditional bounded-latency, wait-free, or lock-free guarantee for
the complete workflow. Use external writer coordination when fairness, bounded
retry cost, or effects with exactly-once semantics are required.

Freezing protects structure, not interior state shared by elements. Load once
and query that version when the application needs a consistent structural view.
The model boundary and actual executed checks are described in
[verification](../verification.md).

## Implementation and review anchors

- [Public wrapper and guard](../../crates/frost-collections/src/concurrent/snapshot.rs)
- [Retry helper shared with the scheduler model](../../crates/frost-collections/src/concurrent/publication.rs)
- [Real backend concurrency tests](../../crates/frost-collections/tests/concurrent/snapshot.rs)
- [ArcSwap's upstream guarantees](https://docs.rs/arc-swap/1.9.4/arc_swap/)

The model assumes a linearizable publication primitive. It does not replace a
review of ArcSwap's memory ordering, reference counting, or reclamation.

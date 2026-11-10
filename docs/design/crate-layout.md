# Crate layout: current package and reorganization target

Status: **current layout plus a proposed target.** The repository has one
library package. The flat `crates/frost-*` layout is the target of the
reorganization epic `frost-wlt`, which depends on the standards epic
`frost-wou`. No package boundary, crate name, or module mapping in this document
is approved. [CRATES.md](../../CRATES.md) holds the rules that apply to every
package; this document holds the design inputs for the split.

## Current layout

One package, `frost-collections` 0.4.0, library name `frost`, Rust 1.99.1,
edition 2024, at the repository root. [Architecture](architecture.md) describes
its module boundaries:

| Module | Responsibility |
| --- | --- |
| `src/lib.rs`, `read.rs`, `write.rs`, `plan.rs`, `frozen.rs`, `iter.rs`, `backend.rs`, `mutable.rs` | Capability traits, `Frozen<B>`, sealed eligibility, freeze plans, re-exports |
| `src/family/` | Five mutable/frozen family pairs and their set algebra |
| `src/adaptive/` | Explicit adaptive profiles; the only unsafe module, `seq/buf.rs` |
| `src/backends/` | Native backend capabilities, one optional feature each |
| `src/adapters/` | Serde, Borsh, Rayon, Arbitrary, and Schemars integrations |
| `src/concurrent/` | Live guarded access and whole-snapshot publication |
| `src/integer/` | The typed `int-set` profile, its codec, policies, and private storage |

Tests live in `tests/`, benchmarks in `benches/`, examples in `examples/`, and
isolated Loom, Kani, and downstream-codec packages under `verification/`.

## Target shape

```text
Cargo.toml                 virtual workspace: members = ["crates/*"], resolver = "3"
crates/
  frost-api/               facade package; [lib] name = "frost"; re-exports and feature forwarding
  frost-<domain>/          one package per approved responsibility, laid out per CRATES.md
verification/              historical evidence and repository-level tooling
Makefile                   orchestration over Cargo metadata
```

Each package follows the [standard crate-local layout](../../CRATES.md#1-standard-crate-local-layout).
Consumers keep the `frost::` import spelling through the facade.

## Constraints a split must satisfy

These come from Rust's coherence and privacy rules and from current Frost
contracts. A proposed boundary that violates one needs an explicit
compatibility decision.

1. **Inherent methods stay with their type.** Methods such as
   `FrozenOrderedSet::union` must be defined in the package that defines the
   type. Set algebra for a family cannot move to a separate package without
   becoming a trait or an extension API.
2. **Trait implementations follow the orphan rule.** An implementation of a
   foreign trait (for example a Serde or Rayon trait) for a Frost type must live
   in the package that defines the type or the trait. Framework adapters
   therefore stay with the types they implement, behind the same features.
3. **Seals are crate-private.** `Eligible` requires `frozen::sealed::Sealed`,
   a `pub(crate)` module in `src/frozen.rs`. `SnapshotValue` requires a seal in
   a module private to `src/concurrent/snapshot.rs`. A sealed trait's
   definition and every seal `impl` must therefore be in the same crate. The
   implementing *types* need not be: the trait's crate can implement its own
   sealed trait for types from crates it depends on. That is how
   `src/backends/*` implements `Eligible` for foreign native types such as
   `BTreeMap` and the Roaring bitmaps today. Two consequences follow:
   - `Eligible` and its implementations for native backends stay together, in
     whichever crate defines `Eligible`, behind the same backend features.
   - A `frost-concurrent` crate can define `SnapshotValue` and its seal and
     implement them for frozen family, adaptive, and integer types by depending
     on those crates. The payload types stay in their own crates. Those crates
     must then not depend on `frost-concurrent`.
4. **Feature forwarding preserves additivity.** Each facade feature forwards to
   the owning package's feature of the same name. Enabling a feature still never
   changes a type's representation, hasher, bounds, or freeze/thaw partner.
5. **Public paths are API.** Every current `frost::` path, prelude item, and
   `compile_fail` doctest path must resolve through the facade.
6. **Unsafe stays confined.** `src/adaptive/seq/buf.rs` moves as a unit with its
   safety comments, Miri lanes, and the Kani harness that includes it by path.
7. **Path-included harnesses keep linking production code.** The publication
   model, Loom package, and Kani package include production files by relative
   path. Their paths change with the move; they must not switch to copies.
8. **Integer storage keeps its private `Shared` alias** and the atomic and
   non-atomic embedded builds.
9. **The dependency graph is acyclic and layered**: capability traits at the
   bottom, the facade at the top, and no package depending on a verification or
   support package except as a dev-dependency.

## Candidate decomposition (not approved)

The supplied research lists `frost-api`, `frost-core`, `frost-set-algebra`,
`frost-set-kernels`, `frost-integer-sets`, and `frost-concurrent`, plus
alternative plural and singular names for future algebra packages; see the
[integration audit](../research/integration-audit.md). Those names predate the
constraints above. For example, a separate `frost-set-algebra` conflicts with
constraint 1 for today's inherent set operations. A `frost-concurrent` crate is
compatible with constraint 3 if it depends on the crates that define its
snapshot payload types and none of them depends back on it.

### Dependency-cycle analysis

Observed module references in `src/` (`crate::` paths found with `rtk rg` on
2026-11-09). This is `PARTIAL` evidence: `super::` paths, re-exports, macros,
and `cfg` combinations were not traced.

| Edge | Source evidence | Effect on a split |
| --- | --- | --- |
| family → adaptive | `src/family/vec/mutable.rs` uses `InlineFrozenVec`; `src/family/hash_map/mutable.rs` uses `AdaptiveFrozenHashMap` (explicit freeze plans) | With adaptive → family below, this is a cycle. Keep both in one crate, or move the explicit plans to the adaptive side. |
| adaptive → family | `src/adaptive/ordered_map/*`, `seq/mutable.rs`, `hash_map/frozen.rs` use family types and `FrozenOrderedMap` | Adaptive profiles freeze to family types. |
| backends → family | `src/backends/{std_btree,std_hash,std_seq,smallvec,arrayvec}.rs` produce `FrozenOrderedMap`, `FrozenHashMap`, `FrozenVec`, and related `Compact` outputs | If backends stay with `Eligible` (constraint 3) in a lower crate, that crate would depend on family while family depends on it for traits. Either `Eligible`, the backends, and the families share a crate, or the `Compact` conversions move to the family side after an orphan-rule check. |
| integer → family | `src/integer/{set,storage,chunked}.rs` | One-way; no reference back was found. |
| concurrent → family, adaptive, integer | `src/concurrent/snapshot.rs` seal implementations | One-way. No other module references `crate::concurrent`, so a top-level concurrent crate adds no cycle. |
| adapters → family, adaptive | `src/adapters/*` | Orphan rule (constraint 2): Serde, Borsh, Rayon, Arbitrary, and Schemars implementations move with the types they implement, not into a separate adapters crate. |

Task `frost-wlt.1` must resolve the first three rows before any move. The
analysis does not cover public re-exports in `src/lib.rs`, `src/mutable.rs`,
or the prelude, which the facade must preserve (constraint 5).

The decision in `frost-wlt.1` must produce:

| Output | Content |
| --- | --- |
| Package list | Name and single responsibility of each package, using one naming rule |
| Module mapping | Every current `src/` module, test, benchmark, example, and harness assigned to one owner |
| Dependency graph | Normal edges only; reviewed against the nine constraints |
| Seal and coherence plan | Where each sealed trait and foreign-trait implementation lives |
| Feature map | Facade feature to owning-package feature, with environment requirements |
| Compatibility notes | Any public path, bound, or auto-trait change and why it is unavoidable |

A package exists only if it owns code. No empty placeholder packages.

## Migration sequence

The tracker tasks under `frost-wlt` are authoritative. In summary:

1. `frost-wlt.1`: approve the outputs above.
2. `frost-wlt.2`: move code, tests, examples, benchmarks, factories, and
   catalog records to their owners, and update the one-package policy in
   [AGENTS.md](../../AGENTS.md), [decisions](decisions.md), and
   [CRATES.md](../../CRATES.md) in the same change. Do not weaken tests or
   duplicate algorithms. Catalog IDs survive the move unchanged, even though
   their middle segment names the target where each case was first registered
   (CRATES.md CR-META-1). Moved selectors and paths are updated so
   `make catalog` passes.
3. `frost-wlt.3`: colocate fuzz, Loom, Kani, and downstream harnesses with
   their owning package and keep them isolated (`workspace.exclude` or their own
   `[workspace]`).
4. `frost-wlt.4`: verify facade API and feature parity.
5. `frost-wlt.5`: update Make recipes, CI, package tests, feature and embedded
   matrices, and documentation links.
6. `frost-wlt.6`, after tasks 3–5: rerun the full matrix, Miri, and models
   against the migrated source and record new evidence. Earlier results do not
   validate the moved code.

## What this document does not decide

It does not choose package names, approve the split, change the current
installation instructions, or authorize publishing. Until `frost-wlt` lands,
[README.md](../../README.md) installation and the one-package policy remain
current.

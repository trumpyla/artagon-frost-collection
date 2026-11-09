# Changelog

## Unreleased — Make-based development tasks

- Replace verification dispatchers with GNU Make targets for builds, checks,
  profiles, feature matrices, packaging, ownership diagnostics, and formal tools.
- Replace the Python documentation-link checker with a Bash 3.2+ helper and
  use `jq` for Cargo metadata. Add shell/Make failure-path tests.
- Update CI and current contributor guidance; preserve historical evidence.

## Unreleased — general set-algebra design

- Review generic frozen algebra and coverage gaps across mutable families,
  exact wrappers, persistent sets, bitmaps, and FST streams.
- Propose typed borrowed views with operation-specific bounds, explicit
  materialization, and separate value/lending/runtime execution profiles.
- Specify stored-representative provenance independently of membership and
  order, including XOR reassociation and nonlocal cancellation counterexamples.
- Add an isolated finite semantics oracle and retain baseline verification
  evidence. This is design work; no new production API or backend is implemented.

## Unreleased — views research and proposed ownership profiles

- Preserve the supplied lazy-views research and add adversarial, primary-source,
  and performance audits with versioned issue/PR status and evidence limits.
- Propose explicit borrowed `IntSetExpr` queries and a separate
  `PersistentIntSet` profile. These are documentation designs, not implemented
  APIs; current integer algebra remains eager.
- Add an isolated executable model for cardinality-bound formulas, with finite
  domains and recorded results. It does not execute a production view engine.
- Correct the integer module's stale std-requirement documentation to match
  the tested alloc-only feature boundary.

## Unreleased — integer output policies and compact algebra

- Keep Dense results for small encoded updates and promote wide Dense unions to
  Roaring instead of unconditionally enumerating sorted output. Add safe bounded
  byte conversion and representation-aware mixed operations.
- Retain `IntSetLimits` on every set. Union/intersection/XOR compose the smaller
  limits; difference retains the left limits. Add four fallible `*_with_policy`
  operations for explicit result selection, including identity and empty cases.
  These limits constrain representations, not total allocation or process memory.
- Use native equality and sorted Roaring builders. Hash canonical members or maximal runs
  consistently across representations; existing hash outputs change and are not
  a persistent format. Keep ordered comparison over canonical ascending codes.
- Prefer Dense when the estimated Roaring saving is at most 128 bytes. Preserve
  competitive legal Dense and Roaring outputs without unnecessary conversion;
  tiny Dense updates do not rescan all members. The allowance is a transparent policy value, not a speed guarantee.
- Support the complete integer profile with `no_std + alloc`, weakly forward
  `std` to the optional Roaring dependency, and preserve native adapter APIs.
  Consumers that disable default features and previously relied on `int-set`
  to enable standard-library APIs must now also request `std` explicitly.
- Add output-size, limit-composition, boundary, hashing, and repeated snapshot
  regressions; include a complete prior source archive and a common comparative
  harness. See `docs/integer-output-validation.md` and
  `docs/integer-output-benchmarks.md` for executed evidence and remaining gates.

## Unreleased — typed integer profile

- Add the explicit `integer::FrozenIntSet<K>` profile and complete `int-set`
  feature, preserving all ordinary mutable/frozen pairings and existing defaults.
- Add safe extensible `IntKey`, sealed `IntWord` widths, signed order-preserving
  encodings, and checked nonzero integer codecs.
- Add validated key/code construction, source-recovering integer freeze plans,
  explicit and automatic storage policies, value iteration, wide cardinality,
  and distribution diagnostics.
- Provide fixed sorted/dense/Roaring candidates, aligned local dense windows,
  encoded materialized algebra, counts, predicates, and mixed-representation
  operations. Reuse the existing supported Roaring implementation and SIMD paths.
- Add adversarial codec, boundary, mixed-storage, downstream-extension, and
  exhaustive tests with comparative benchmarks. See `docs/integer-validation.md`
  for executed results and precise limits; harness presence is not a proof result.

## Unreleased — integration and production hardening

- Integrate `SnapshotStore::load_guard`, `peek`, `publish_if_current`, and `update`.
  The public guard is crate-owned and can retain its exact version as an Arc.
- Specify identity comparison, rejected-value recovery, displaced-version return,
  repeated callbacks, panic behavior, retention costs, and progress limits.
- Split concurrency traits, backend adapters, snapshot API, and retry policy into
  cohesive modules without changing existing public paths.
- Add deterministic snapshot regressions, actual retry-helper schedule exploration,
  exhaustive finite collection/ownership tests, and isolated Loom/Kani harnesses.
  Tooling presence is not a proof result; see `docs/production-readiness.md`.
- Keep the prior ordered kernel after direct comparison with the attached gallop
  replacement; incorporate stronger skew and predicate regressions instead.
- Correct benchmark setup/operation accounting and native baselines, add focused
  ordered/adaptive/snapshot targets, and preserve raw comparison evidence.
- Organize design/review/maintainer documentation; add AGENTS.md, contributor and
  security policies, composable verification commands, and pinned CI actions.
- Preserve the library's dependencies, Rust 1.99.1/edition 2024, no_std boundaries,
  feature additivity, family pairings, and default hasher policy.

### Prior performance pass retained

- Keep the 0.4.0 public API, Rust 1.99.1 target, edition 2024, dependencies,
  feature model, and mutable/frozen family pairings.
- Add monotonic seeking for skewed ordered set operations, same-storage and
  endpoint shortcuts, and bulk intersection dispatch. Keep the balanced merge.
- Split dense bitmap word loops into shared spans, masked boundaries, and
  suffixes; handle one/two-word results directly and remove provably redundant
  per-word overflow checks. Preserve dirty-padding and universe semantics.
- Reuse small adaptive-map entry lookup results instead of searching twice.
- Transfer existing ordered-set and adaptive sorted-map buffers to owning
  Rayon iteration; forward native bulk operations from set iterator wrappers.
- Add 16 regression cases and a 140-case before/after benchmark corpus with
  native controls. Record both gains and tradeoffs in `docs/reviews/performance-review.md`.

## 0.4.0 — 2026-11-08

### Fast frozen set operations

- Add borrowed lazy union, intersection, difference and symmetric difference
  for `FrozenOrderedSet` and `FrozenHashSet`.
- Preserve the left stored representative on union/intersection overlaps,
  including hash intersections that scan the smaller right operand.
- Add result-free intersection, union, difference and symmetric-difference
  cardinality queries, plus complete subset/superset/disjoint relations.
- Use monotonic merge scans for ordered sets and smaller-side probing for hash
  intersection and disjointness. Different hashers are supported without
  requiring `Clone` or `Default`.
- Add corresponding frozen-result operations and scalar kernels for
  `Frozen<RoaringBitmap>`, `Frozen<RoaringTreemap>` and `Frozen<FixedBitSet>` under
  their existing feature flags.
- Bound FixedBitSet operations by each operand's logical universe and ignore
  dirty native padding without rewriting either source.
- Add the tested `set_operations` example, adversarial and differential tests,
  and a dependency-free release-mode comparison benchmark.
- Retain Rust **1.99.1**, edition **2024**, and the existing additive feature
  model. See `docs/set-operations.md` and `docs/reviews/historical-verification.md` for exact contracts and
  results.

## 0.3.0 — 2026-11-08

This revision refactors the attached `0.1.0-proto` project into the previously
reviewed family architecture, while retaining and correcting its useful adaptive
profiles, entry APIs, and framework integrations. `docs/design/decisions.md` compares
the two inputs and records the selected design. This is a source distribution;
the manifest remains `publish = false`.

### Required Rust version

- Pin `rust-toolchain.toml` and required CI jobs to **Rust 1.99.1**.
- Set Cargo `rust-version = "1.99.1"` and retain **edition 2024**.
- Keep nightly limited to Miri and compile-fail diagnostic-code checks. Normal
  builds, tests, Clippy, documentation and embedded builds use the pinned release.
- Refresh `Cargo.lock` and verify using `--locked`.

### Selected public architecture

- Keep the canonical `family` modules and their private storage. `frost::mutable`
  and the prelude reexport the same types; they are not duplicate implementations.
- Provide five ordinary mutable/frozen pairs: vector, hash map, ordered map,
  hash set and ordered set. Add `MutableOrderedSet`, `MutableHashSet` and
  `FrozenHashSet` to complete the attached project's families.
- Retain crate-owned occupied/vacant entry handles and map iterator wrappers.
  Mutable keys and a mutable backend escape hatch are not exposed.
- Restore explicit native interoperability aliases: mutable `from_vec` and
  `from_map`, `into_vec`, map `into_map`, and set `into_set`. Keep the attachment's
  existing `into_hash_map`, `into_btree_map`, `into_hash_set` and `into_btree_set`
  names where applicable.
- Restore mutable map `IndexMut`, strict ordinary-family builders, and explicit
  hash builders that accept a hasher without requiring `Clone` or `Default`.
- Preserve the attachment's `MapTypes` supertrait for map mutation. `SetInsert`
  likewise shares its element type through `ElementType`.
- Keep `InlineVec`, `AdaptiveHashMap` and `AdaptiveOrderedMap` as explicitly
  selected profiles. Promotion and demotion policies remain separate from Cargo
  feature selection.
- Keep exact `Frozen<B>` for audited native backends. There is no `Mutable<B>`
  facade, universal map enum, or public backend-marker parameter on ordinary
  families.

### Advanced and concurrent collections

- Retain the attachment's `hashbrown-0_17` and `indexmap-2` integrations.
- Add `smallvec-1`, `arrayvec-0_7`, `imbl-7`, `roaring-0_12`, `fixedbitset-0_5`,
  `slotmap-1`, `phf-0_14` and `fst-0_4`.
- Add `dashmap-6`, `scc-3`, `crossbeam-skiplist-0_1` and `arc-swap-1`.
- Separate borrowed entries, borrowed values, decoded values and reconstructed
  elements through capability traits. Preserve natural `u64` bitmap cardinality.
- Keep live concurrent access guard-scoped and synchronous. `SnapshotStore`
  publishes whole approved frozen values and rejects live mutable containers.
- Reexport enabled dependencies from `frost::backend`; native specialized
  operations remain available without forwarding their entire APIs.
- Provide additive `advanced`, `concurrent` and `frameworks` feature groups.
  Enabling a feature does not change an existing ordinary family's representation,
  hasher or trait bounds.

### Framework integrations

- Retain Serde, Rayon, Borsh, Arbitrary and Schemars, and extend all five to the
  added set families.
- Encode logical members and entries, without private representation tags,
  capacities, hashers or runtime growth policies.
- Make framework tests run when the corresponding feature is enabled alone.
- Correct Rayon bounds so traversal does not require unused hashing capabilities
  and a hasher need not be `Send` when it stays on the calling thread.
- Apply family duplicate rules to ordered parallel input. Document that an
  unordered source has no deterministic duplicate winner.
- Remove redundant sorting and an output allocation from ordered parallel
  collection. Document temporary staging for native hash/tree parallel traversal.
- Reject hash-key cardinality collapse in strict Borsh family decoding, including
  defensive handling of inconsistent user `Ord` and `Eq` implementations.
- Preserve native wire semantics for exact `Frozen<B>`. Document that strict
  decoding does not impose a total input-size or allocation budget.

### Correctness and API fixes

- Reject overflowing `InlineVec::reserve` requests before changing storage,
  including nonempty zero-sized element collections.
- Construct fitting inline collections directly in their inline storage.
- Deduplicate ordered family construction in place after stable sorting.
- Enforce first stored key representative / last map value through explicit
  insertion in hash-family bulk builders. Set builders keep the first element.
- Remove unnecessary `Clone`, `Default` and ordering bounds from operations
  that only move already-valid owned data.
- Clarify strict-builder ownership: the error returns the rejected duplicate,
  while other consumed entries are dropped. It is not an input-recovery API.
- Document adaptive map panic guarantees: the collection remains valid, but a
  user hashing/comparison panic during rebuilding does not promise rollback.
- Bound FixedBitSet member traversal by its logical universe while preserving
  native raw padding; make SecondaryMap membership independent of the source map.

### Verification and examples

- Add nine executable examples with tests that call the actual example scenario.
- Include README Rust examples in rustdoc.
- Add regression tests for ownership, destructor panics, iterator panics,
  duplicate representatives, custom hashers, wire strictness, schemas, concurrent
  access and snapshot publication.
- Provide `scripts/verify.sh` for core-only, alloc-only, default, all-feature and
  each of the nineteen optional integrations independently, plus actual example
  execution, warnings-denied Clippy/rustdoc and optional embedded/Miri checks.
- Add required Rust 1.99.1 CI jobs, bare-metal builds and separate nightly
  diagnostics. See `docs/reviews/historical-verification.md` for checks actually executed locally and
  their limits; CI configuration is not a claim that remote CI has run.

### Migration notes

| Earlier usage or assumption | Selected API or contract |
| --- | --- |
| Rust 1.88 from the attached prototype | Install Rust 1.99.1; edition remains 2024 |
| Import mutable types through `family` | Existing imports remain; `mutable` and prelude are convenience reexports |
| Standard HashSet compacted to an exact frozen wrapper | `compact()` returns the named `FrozenHashSet<T, S>` family; use `seal()` to retain an exact backend |
| Assume a strict-builder error returns all input | Recover only the duplicate carried by the error; other consumed entries are dropped |
| Expect `AdaptiveFrozenHashMap::thaw()` to recover a custom threshold | Pass the chosen policy to `thaw_with(policy)` |
| Expect compact ordered data to remember its adaptive origin | Use `AdaptiveOrderedMap::from_frozen(frozen, policy)`; ordinary `thaw()` always returns `MutableOrderedMap` |
| Treat every set as borrowed elements or every map as borrowed entries | Select the capability matching the storage: borrowed, decoded or reconstructed |
| Freeze a live concurrent collection | Use scoped concurrent access or publish an owned approved frozen snapshot |
| Expect feature flags to replace an ordinary family's backend | Select an explicit native backend or adaptive profile; feature flags only add integrations |
| Rely on deterministic duplicates from unordered parallel work | Supply an ordered source or resolve duplicates explicitly before collection |

The new types, capabilities and corrected contracts expand the crate, but this is
an intentional prototype-to-0.3 refactor, not a promise of blanket source or binary
compatibility with every earlier prototype. `docs/design/architecture.md` describes the maintained
contracts in detail.

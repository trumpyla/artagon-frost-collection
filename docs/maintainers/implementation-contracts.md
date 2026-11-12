---
id: FROST-RULES-IMPLEMENTATION
status: maintainer-guidance
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Implementation contracts

This is the canonical location of the implementation, tooling, and verification
rules. The text below the next paragraph was moved unchanged from the regular
`AGENTS.md` at `27b1439`, which is now a symlink to the short
[entry point](agent-rules.md). Repository-root paths in code spans remain
root-relative.

These are semantic, safety, and process contracts. They do not freeze the public
API shape: by the maintainer decision of 2026-11-12 in
[design decisions](../design/decisions.md#api-compatibility-is-not-a-constraint-2026-11-12),
names, paths, signatures, bounds, and type parameters may change. Changing a
contract listed here needs its own recorded decision.
[CRATES.md](../../CRATES.md) stays authoritative for its scope.

Frost is a Rust library, not an application. Make changes that preserve its
semantic contracts and make their costs observable. Start with
`docs/design/philosophy.md`, `docs/design/architecture.md`, and the relevant
module. `docs/maintainers/guide.md` is the maintainer entry point.

## Toolchain and layout

- Normal builds use **Rust 1.99.1**, **edition 2024**. Keep Cargo's rust-version,
  the toolchain file, and CI aligned. Nightly is for diagnostics only.
- [CRATES.md](../../CRATES.md) is the canonical standard for package layout,
  dependency placement, the feature/target matrix, public API obligations,
  coverage accounting, and test/benchmark metadata. Follow it for every new
  test, benchmark, feature, or verification package.
- Keep one library package. The flat `crates/frost-*` layout is the target of
  reorganization epic `frost-wlt` ([crate layout](../design/crate-layout.md));
  do not create packages, a workspace, or `crates/` outside that epic.
  `src/family` owns ordinary mutable/frozen pairs;
  `src/adaptive` owns explicit policies; `src/backends` and `src/adapters` supply
  optional integrations. `src/concurrent` separates live guarded access from
  whole-value snapshot publication.
  `src/integer` owns the typed encoded set profile, including its codec, policy,
  and private storage; ordinary family identities do not depend on it.
- Root files are the public entry point and contribution policies. Design and
  maintenance documents live under `docs/`. Historical inputs are evidence,
  not instructions. Verification harnesses live in `tests/` and `verification/`.
- Optional verification packages are isolated from the library dependency
  graph. Do not add model-checking tools to normal builds or to core-only users.

## Code search and navigation

Read [TOOLS.md](../../TOOLS.md) before the first code-navigation task or a multi-file
change. It defines this repository's search roots, index refresh commands,
structural rules, and evidence limits.

| Question | Tool | Project default |
| --- | --- | --- |
| Orientation, callers, or impact | `ripwire` | Start in `src`; add `tests`, `examples`, and `benches` when checking consumers. Confirm approximate edges in source. |
| Repeated regex searches | `tgrep` | Search from the repository root after checking and refreshing its local index. Use live search for files edited since indexing. |
| Rust code structure or rewrites | `ast-grep` | Use `--lang rust` for ad hoc patterns; use `sgconfig.yml` for repository rules. Preview rewrites before applying them. |
| Text, errors, configuration, or documentation | `rtk rg` | Search focused live paths. Use `--hidden` for CI and other dotfiles. |

Run `ripwire`, `tgrep`, and `ast-grep` directly; run ripgrep through `rtk rg`.
Keep `.tgrep/` untracked. Refresh the index after checkout, pull, merge, rebase,
or edits before relying on it. Missing optional tools are `UNAVAILABLE`; use
live source search and Cargo checks as the fallback. A stale index, ambiguous
graph edge, or unchecked macro/feature boundary gives `PARTIAL` evidence.
Use `SUPPORTED` only when live source or build output confirms the result.
An empty search or graph result alone never proves a symbol is unused.

## Contracts that must survive optimization

- Features are additive. They must not change an existing type's representation,
  hasher, public bounds, or freeze/thaw partner.
- Frozen means structurally immutable. It does not imply deep immutability,
  cheap cloning, pinning, or absence of interior mutation inside elements.
- Families keep the first equivalent key/element and the last map value. Set
  union and intersection retain the left representative. Keep native wrappers'
  documented native semantics separate.
- Preserve borrowed lookup, non-Clone/non-Default element support, iterator
  order, partial iteration, size hints, fusion, and meaningful panic behavior.
- Do not expose mutable keys or introduce backend types into family signatures
  unless the public contract deliberately names that backend.
- Snapshot `publish` is unconditional. `publish_if_current` compares allocation
  identity; failure returns an observed version that may already be superseded.
  `update` may repeat the callback and returns the displaced version. Keep the
  shared retry helper connected to the real production implementation.
- No manual Send/Sync implementations or new unsafe blocks without a written
  invariant and a focused ownership review. Unsafe is currently confined to
  `src/adaptive/seq/buf.rs`; retain its safety comments and Miri coverage.
- The `integer` profile uses a safe extensible `IntKey` codec and sealed unsigned
  widths. Codec laws never justify unsafe access. Preserve canonical codes,
  checked imports, logical-domain type separation, and by-value traversal.
  Dense offsets use absolute aligned coordinates; span arithmetic uses `u128`.
  Count queries and materialized algebra must not decode every member. The
  complete `int-set` feature has fixed Sorted, Dense, Roaring, and Chunked
  candidates and requires alloc; std enables the dependency's optional runtime
  CPU dispatch and std APIs. Chunked is explicitly selected, never chosen by
  automatic construction. Retain the memory score and 128-byte Dense preference.
  Preserve IntSetLimits and all *_with_policy APIs: commutative operations take
  the smaller limits; difference keeps the left limits. Every identity shortcut
  must check result eligibility. Keep Hash event streams identical across all
  representations and sharing histories.
- Integer storage uses the private Shared alias: Arc with pointer atomics, Rc
  otherwise. Clones share immutable storage; changed Chunked results share
  untouched containers but copy the flat directory. No logarithmic path-copy
  claim is valid for that index. Keep published containers immutable and retain
  old-version tests, partial-iterator tests, and both atomic/non-atomic embedded
  builds. FrozenIntSet::new allocates its shared header. Reference counts and
  final-owner release are observable costs, not free operations.

## Verification and review

Run `make check` for changes before review. Use the smallest
relevant target while developing; run `make verify` for release
candidates. Feature or dependency changes also require the isolated feature
matrix and embedded build. Ownership changes require `make miri`.
The exact commands and evidence limits are in `docs/verification.md`; gate
tiers are in `docs/maintainers/testing.md`. `make check` includes `make catalog`
and `make catalog-inventory`, so a new test or benchmark needs its catalog or
inventory record in the same change.

Use the root Makefile for build and verification tasks; `make help` lists
targets. Do not restore verification dispatcher scripts or add Python to the
build tooling. Shell helpers target Bash 3.2+ on macOS/Linux and follow the
Artagon shell-authoring guidance when available. Run `make shell-check` for
shell or Makefile changes, including the minimum Bash version available on the
host. Keep Cargo metadata parsing in recipes with `jq`, not Make expansion.

Add regression tests for changed semantics or a demonstrated risk. Prefer
standard-library differential tests, exhaustive small domains, comparison/drop
counts, deterministic barriers, and recorded failing property-test seeds.
Do not add timing assertions or sleeps to correctness tests. Map each changed
public obligation to asserting tests, as CRATES.md requires. Scaffolded catalog
records are `unreviewed`; mark a record `curated` only after reviewing its
contract, oracle, and scenario. Keep existing catalog IDs. Report coverage only
with its scope; never present a target, waiver, or catalog count as measured
coverage. A model must state
its domain, scheduler bound, dependency assumptions, and the production code it
executes; a passing model is not proof of unmodeled dependency internals.

Use the common benchmark support and native controls. Validate answers before
timing, run competing versions serially with identical compiler/profile flags,
and preserve raw samples and slower observations. Report measured workloads,
not universal speedups. Do not change adaptive thresholds on one machine's
timings alone. Keep allocation, construction, destruction, and thread startup
inside or outside timing consistently and document that choice.

Update public rustdoc, changelog, and design decisions when contracts change.
Keep historical evidence labeled as historical. Report commands actually run,
failures, unavailable tools, and remaining release gates accurately. Do not
publish a crate or invent maintainer/repository metadata as part of a code edit.

# Working on Frost

Frost is a Rust library, not an application. Make changes that preserve its
semantic contracts and make their costs observable. Start with
`docs/design/philosophy.md`, `docs/design/architecture.md`, and the relevant
module. `docs/maintainers/guide.md` is the maintainer entry point.

## Toolchain and layout

- Normal builds use **Rust 1.99.1**, **edition 2024**. Keep Cargo's rust-version,
  the toolchain file, and CI aligned. Nightly is for diagnostics only.
- Keep one library package. `src/family` owns ordinary mutable/frozen pairs;
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
  complete `int-set` feature has fixed candidates and requires alloc; std enables
  the dependency's optional runtime CPU dispatch and std APIs. Retain result
  limits, preserve compact algebra outputs, and keep Hash event streams identical across representations.

## Verification and review

Run `./scripts/verify.sh check` for changes before review. Use the smallest
relevant target while developing; run `./scripts/verify.sh all` for release
candidates. Feature or dependency changes also require the isolated feature
matrix and embedded build. Ownership changes require `./scripts/verify.sh miri`.
The exact commands and evidence limits are in `docs/verification.md`.

Add regression tests for changed semantics or a demonstrated risk. Prefer
standard-library differential tests, exhaustive small domains, comparison/drop
counts, deterministic barriers, and recorded failing property-test seeds.
Do not add timing assertions or sleeps to correctness tests. A model must state
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

# Crate layout and verification standard

This file is the canonical standard for how a Frost crate is laid out, what
each directory may contain, which dependencies may reach which build, and what
evidence a public API needs. It applies now to the single root package and,
after the reorganization, to every package under `crates/`.

The [implementation contracts](docs/maintainers/implementation-contracts.md)
(reached through the [AGENTS.md](AGENTS.md) entry point) and
[CONTRIBUTING.md](CONTRIBUTING.md) state the semantic contracts and the normal
workflow. The
[testing standard](docs/maintainers/testing.md) defines gate tiers and
obligation categories in detail; [verification scope](docs/verification.md)
states what each tool proves; the [crate layout design](docs/design/crate-layout.md)
records the target package graph and its open decisions. When this file and
another document disagree on layout, dependency placement, coverage accounting,
or test metadata, this file wins; fix the other document in the same change.

## Current state and target

| Scope | State | Where it is decided |
| --- | --- | --- |
| A Cargo workspace whose members live under `crates/`: `frost-collections`, imported as `frost`, the internal `frost-collections-core`, `frost-collections-integer` and `frost-collections-concurrent`, and the dev-only `frost-collections-test-support` | **Current.** All rules below apply to it now. | [Cargo.toml](Cargo.toml), [decisions](docs/design/decisions.md) |
| This standard, the verification catalog, workload factories, coverage and mutation policy, fuzz harness, and issue/PR templates | **Implemented** by standards tasks `frost-wou.1`–`.5` | Beads epic `frost-wou` |
| Full API obligation curation, branch/profile/Linux baselines, mutation baseline, high-target tests, dependency-isolation guard | **Open adoption work**: `frost-wou.6`–`.10` | Beads epic `frost-wou` |
| `frost-collections-core`, `-integer` and `-concurrent` behind the facade `frost-collections`, plus the dev-only `frost-collections-test-support` | **Implemented** by `frost-wlt.8`–`.14`. The maintainer approved the names and boundaries on 2026-11-14; they may be revisited in the API refactor. An adaptive package is an open decision (`frost-adp`). | Reorganization epic `frost-wlt`; [crate split plan](docs/design/crate-split-plan.md) |

`frost-wlt` uses these rules; it does not define them. Since 2026-11-13 it no
longer waits for the open `frost-wou` tasks
([decision](docs/design/decisions.md#crate-split-execution-2026-11-13)). Do
not add a package, or a placeholder for one, outside `frost-wlt`. Where a rule
below names `src/`, `tests/`, `benches/` or `crates/frost-collections/examples/`, it means that
directory of a package.

Rules carry stable IDs (`CR-<AREA>-<n>`) so reviews, issues, and catalog records
can cite them. Do not renumber a rule; retire it and add a new ID.

## 1. Standard crate-local layout

Every library package uses the same tree and is self-contained (CR-LAYOUT-9).
Directories that a package does not need are omitted, never created empty.

```text
<package>/
  Cargo.toml          manifest; features, targets, lints, package include/exclude
  README.md           package purpose, contracts, and links (root README serves today)
  CHANGELOG.md        changes to this package (root changelog serves today)
  docs/               design and contract documents for this package's types
  src/                production code; private unit tests in #[cfg(test)] modules
  tests/              integration and contract tests; one file per contract area
    <area>.rs           one Cargo test target
    <area>/main.rs      a multi-file test target; its modules live beside it
    formal_models/      finite formal models of this package's laws (CR-LAYOUT-10)
    support/mod.rs      helpers private to this package's tests; never a target
  benches/            harness = false benchmark targets
    support/mod.rs      helpers private to this package's benches; shared ones are in the support package
  examples/           executable examples; each declares test = true
  fuzz/               isolated cargo-fuzz package (target layout; see CR-LAYOUT-6)
  verification/       isolated verification packages and package-owned metadata
    loom/               Loom model package, own manifest and lockfile
    kani/               Kani harness package, own manifest and lockfile
    verus/              spec-level Verus proofs of this package's laws; not a Cargo package
    <downstream>/       downstream-consumer checks, for example int-key-downstream
    metadata/           catalog and benchmark-inventory records for this package
```

Repository-level files stay at the root: `Makefile`, `rust-toolchain.toml`,
`Cargo.lock`, the workspace manifest, licenses, policy files, `.config/` tool
configuration, repository-wide documents under `docs/` (philosophy,
architecture, decisions, maintainer guides, the research corpus), historical
evidence under `verification/`, and repository tooling such as the catalog
validator in `verification/catalog/`. The root
[verification/metadata/](verification/metadata/catalog.json) catalog records
repository tooling only. The root `docs/` holds repository-wide documents;
each package has its own `README.md`, `CHANGELOG.md` and `docs/`. The root
`CHANGELOG.md` records repository-wide changes and everything from before the
packages existed.

- **CR-LAYOUT-1.** `src/` contains code that ships. Test helpers, factories,
  oracles, and benchmark drivers live in `crates/frost-collections-test-support/src/`, `crates/frost-collections-test-support/src/`,
  or an isolated package, never in `src/` behind `pub`, `#[doc(hidden)]`, or a
  feature.
- **CR-LAYOUT-2.** Private invariants are tested in `#[cfg(test)] mod tests`
  next to the code. Public behavior is tested from `tests/` through the public
  API, so a test can fail only on an observable contract.
- **CR-LAYOUT-3.** Each file directly under `tests/` is a separate test crate.
  Name it after a contract area, not after a review round. A multi-file target
  uses `tests/<area>/main.rs` or a root file with `#[path]` modules, as
  `crates/frost-collections-concurrent/tests/concurrent.rs` does; modules in a subdirectory do not become targets.
- **CR-LAYOUT-4.** Every example is an executable scenario with assertions,
  `test = true`, and `required-features` when it needs any. Cargo skips a target
  whose required features are disabled, so a skipped example is never counted
  as passing.
- **CR-LAYOUT-5.** Benchmarks use `harness = false`, declare
  `required-features`, share measurement code through the `bench` module of
  the support package (CR-BOUND-1), and support listing with
  `FROST_BENCH_LIST=1`. They follow
  [benchmark guidance](docs/maintainers/benchmarks.md).
- **CR-LAYOUT-6.** Fuzz, Loom, Kani, catalog, and downstream-consumer harnesses
  are separate packages with their own `Cargo.toml`, `[workspace]` table and
  committed lockfile, and each lives with the package whose code it exercises:
  the Kani package and the Verus proofs under
  `crates/frost-collections-core/verification/`, the fuzz package at
  [`crates/frost-collections-integer/fuzz`](crates/frost-collections-integer/fuzz/README.md)
  (the cargo-fuzz default directory; it uses core and integer code, and the
  integer package is the lowest that depends on both), Loom under
  `crates/frost-collections-concurrent/verification/`, and the downstream
  codec check under `crates/frost-collections/verification/`. The catalog tool
  is repository tooling and stays in `verification/catalog`. A harness exists
  in one location only.
- **CR-LAYOUT-7.** A verification harness that must execute private production
  code includes the exact production file (`#[path]` or `include!`) instead of
  copying the algorithm or widening its visibility. The publication model test
  and the Loom package include `crates/frost-collections-concurrent/src/concurrent/publication.rs` of the
  concurrent package; the Kani package includes `crates/frost-collections-core/src/adaptive/seq/buf.rs` of
  the core package. The included file stays self-contained so the harness and
  the library compile the same text.
- **CR-LAYOUT-8.** Historical logs, raw samples, and supplied inputs are
  evidence. They stay where they were recorded and are not moved into a
  package during the reorganization.
- **CR-LAYOUT-9.** A package is self-contained. Everything that belongs to it
  lives under its directory: code, tests, benchmarks, examples, fuzz targets,
  Loom, Kani, and downstream harnesses, formal models, catalog and inventory
  records, README, changelog, and the design documents for its contracts.
  These refer only to files under the package, workspace-root files, and the
  packages it depends on. A test that needs several packages lives in the
  lowest package that depends on all of them. Examples are the exception: they
  document the supported entry point, so every example lives in the facade.
  Moving a package directory moves everything it owns. Repository-wide
  documents and historical evidence (CR-LAYOUT-8) are not package artifacts.
- **CR-LAYOUT-10.** A package owns the formal artifacts for its laws.
  Executable models are Rust test targets in `crates/frost-collections-core/tests/formal_models/`, one module
  per design chapter, run by `make check`. Proof harnesses for those laws live
  in `<package>/verification/<tool>/` as isolated packages (CR-LAYOUT-6). Model
  reference definitions are never imported from production code. Where the
  package implements an operation, the same inputs also run through the real
  types. Each law check also runs on deliberately wrong definitions, which
  must fail. A model of a proposed operation lives in the package its design
  assigns as owner. Models state their finite domain and are not proofs.
  Formal artifacts are written in Rust or in a tool adopted in section 7, and
  run through the Make gates. Spec-level proofs of the same laws live in
  `<package>/verification/verus/` and are not linked to production code.

## 2. Responsibility boundaries

| Concern | Owner location | Must not |
| --- | --- | --- |
| Semantic contracts and their rustdoc | `src/` of the package that defines the type | Depend on test support or a verification package |
| Contract and differential tests | `tests/` of the defining package; cross-package contracts in the facade package | Mirror the implementation as its own oracle |
| Workload factories and oracles | `crates/frost-collections-test-support/src/`, `crates/frost-collections-test-support/src/`, or a `publish = false` support package | Produce Frost types when a plain `Vec`/`BTreeSet` input suffices |
| Benchmark timing | `benches/` with common support | Validate answers inside the timed region |
| Model checking, proofs, fuzzing, catalog validation | Isolated packages | Become normal or dev-dependencies of the library |
| Test and benchmark metadata | `<package>/verification/metadata/` (section 6) | Live only in comments or test names |
| Formal models | `<package>/tests/formal_models/` (CR-LAYOUT-10) | Import production code into a reference definition, or run outside the Make gates |
| Package design documents | `<package>/docs/` | Duplicate a repository-wide decision instead of linking it |
| Orchestration | Root `Makefile` | Hide failures, skip missing tools, or require Python |

- **CR-BOUND-1.** The shared support package, `frost-collections-test-support`,
  is `publish = false`, is used only as a dev-dependency, and depends on no
  Frost package. It generates plain data (seeds, vectors, sorted keys,
  operation scripts) and holds the dependency-free benchmark timing helpers.
  If it depended on a crate under test, that crate's unit tests could link two
  distinct copies of its types.
- **CR-BOUND-2.** The facade package re-exports by name and forwards
  features; it holds no algorithms and does not re-export a hidden module.
  Public paths under `frost::` are part of the API contract. The integer and
  concurrent packages mirror those paths at their own roots, by re-exporting
  the core package and adding their module, so their code, tests and doctests
  use the paths users write. Those roots, and `#[doc(hidden)]` items, are not
  API.
- **CR-BOUND-3.** Unsafe code stays in
  `crates/frost-collections-core/src/adaptive/seq/buf.rs`, with its safety
  comments, Miri coverage, and Kani harness. The workspace denies
  `unsafe_code`; that file alone allows it, and every other package forbids
  it. A new unsafe location needs a written invariant and an ownership review
  first.

## 3. Dependency placement and leakage

- **CR-DEP-1.** `[dependencies]` contains only runtime dependencies. Each is
  optional and activated only by features declared in `Cargo.toml`. A public
  integration that exposes or implements a dependency's API uses a
  compatibility-line feature name (`serde-1`, `roaring-0_12`). A semantic
  profile may also use that dependency privately: `int-set` activates `roaring`
  directly for its private storage, needs only `alloc`, and exposes no `roaring`
  type, while `roaring-0_12` is the separate `std` native adapter. Features that
  share a dependency share its one resolved version; enabling one must not
  change the other's types, bounds, or environment requirement. Features are
  additive and never change an existing type's representation, hasher, bounds,
  or freeze/thaw partner.
- **CR-DEP-2.** Test and benchmark libraries (`proptest`, format crates,
  default-feature backends) are `[dev-dependencies]`. Libraries that generate
  inputs or drive reference models inside a package's own test targets
  (`exhaust`, `itertools`, `proptest-state-machine`) are test libraries in this
  sense, because CR-LAYOUT-10 makes formal models test targets. Cargo does not propagate
  dev-dependencies to dependents ([Cargo: development dependencies][cargo-dev-deps]).
- **CR-DEP-3.** Proof, model-checker, fuzz, mutation, coverage, and catalog
  tools are never normal or dev-dependencies of a library package. They live in isolated
  packages or are external commands invoked by Make.
- **CR-DEP-4.** Host test builds unify dev-dependency features, which can hide
  a `std` requirement. Bare-metal library builds (`make embedded`) are the leak
  check for `core`/`alloc` profiles; a host `--no-default-features` test is not.
- **CR-DEP-5.** The packaged source must build and test on its own.
  `package.exclude` keeps verification and supplied evidence out of the
  packaged file set. Nothing is published, and `cargo package` cannot resolve
  unpublished path siblings, so `make package` copies the file set that
  `cargo package --list` reports for each member into a fresh scratch workspace
  and builds and tests it there with all features.
- **CR-DEP-6.** In the target workspace, set `resolver = "3"` explicitly in the
  root manifest; the resolver is a workspace-wide setting ([Cargo resolver
  versions][cargo-resolver]). Isolated verification and fuzz packages are listed
  in `workspace.exclude` or carry their own `[workspace]` table, so they keep
  their own lockfiles. `cargo fuzz init` does not add that table by default.
- **CR-DEP-7.** Shared versions, lints, and package metadata use
  `[workspace.dependencies]`, `[workspace.lints]`, and `[workspace.package]`
  inheritance ([Cargo workspaces][cargo-workspaces]). A member must not loosen an
  inherited lint without a recorded reason.

Evidence on this checkout: `cargo tree --locked --no-default-features -e normal`
lists only `frost-collections`; with `--features alloc,int-set` it adds only
the existing `roaring` chain. The isolated catalog and fuzz packages enter
neither tree ([standards evidence](verification/crate-standards-validation/README.md)).
This is a manual observation; an automated guard is open task `frost-wou.10`.

## 4. Feature and target matrix

Every public item states the profile that provides it. Every test either runs
or is compiled out in each profile; it never fails because a feature is absent.

| Profile or target | Make entry | What it checks |
| --- | --- | --- |
| `core` only (`--no-default-features`) | `make profiles` | Clippy, tests, rustdoc without `alloc` |
| `alloc` only | `make profiles` | Allocating families without `std` |
| default (`std`) | `make profiles`, `make check` | Standard hash families |
| Each optional feature alone | `make features` | Discovered from Cargo metadata; aggregates excluded |
| All features | `make check` | Clippy, rustdoc, tests, examples, catalog |
| Integer profile, debug/release, downstream codec | `make integers` | Codec laws from an external crate |
| `thumbv7em-none-eabihf`, `thumbv6m-none-eabi` | `make embedded` | `core`/`alloc` builds with and without pointer atomics |
| Miri, including `i686-unknown-linux-gnu` | `make miri` | Ownership and aliasing on executed paths |
| Extracted package | `make package` | Missing files, coupled verification dependencies |

- **CR-FEAT-1.** A new feature is added to `Cargo.toml` with its environment
  requirement. `make features` discovers it automatically; an aggregate feature
  is added to that recipe's exclusion list. An `alloc`-compatible feature is
  added to the embedded build list.
- **CR-FEAT-2.** A test file that needs features declares them with
  `#![cfg(feature = "...")]` or a `[[test]]` entry with `required-features`. Its
  catalog target record lists the required features, and the case records the
  structural `cfg` that the catalog discovers (section 6).
- **CR-FEAT-3.** The matrix covers each feature alone and all together, not the
  powerset. A claim about a specific combination needs that combination run.
- **CR-FEAT-4.** The facade forwards each feature to the package that owns its
  code, under the same name, and every library package declares the features
  of the workspace packages it depends on (`make feature-forwarding`).
  `make profiles` and `make features` run once per library package, each in
  its own Cargo invocation, so one package's features never reach another's
  build; `make feature-isolation` asserts that the facade's no-`std` profiles
  resolve the core package without `std`.

## 5. Public API coverage

The unit of coverage is a **public API obligation**: one observable promise of
one public item, under the features and targets that provide it. Line and region
percentages show what code executed; they do not show that a promise was
checked.

### Obligation categories

A public item has an obligation in each category that applies to it. The
[testing standard](docs/maintainers/testing.md#2-public-api-semantic-obligations-beyond-percentages)
gives the detailed test requirements for each.

| Category | Required evidence |
| --- | --- |
| Contract mapping | Each obligation names its documented contract (rustdoc, design document, or implementation-contracts rule) and the tests that check it |
| Positive behavior | Results compared with an independent oracle: a standard-library collection, a finite model, or fixed expected values |
| Negative behavior | Rejection, error, and returned-input paths; strict constructors return the rejected item |
| Compile-fail | Forbidden mutation, capability, lifetime, and payload uses, as `compile_fail` doctests with error codes where stable |
| Representative identity | Keys whose equality ignores metadata, showing first key, last map value, and left representative |
| Ownership and panic | Non-`Clone` and non-`Default` elements, drop counts, panicking user callbacks, partial iteration, `size_hint`, fusion |
| Feature and target | The obligation is exercised in every profile where the item exists, plus the relevant embedded build |
| Concurrency | Native tests plus the stated model bound when the item publishes or shares versions |

- **CR-API-1.** A new or changed public item adds its obligations and at least
  one test per applicable category in the same change.
- **CR-API-2.** An obligation counts as covered only when a test that a person
  has reviewed (a `curated` catalog case) asserts it. Executing the code, or a
  structural catalog record, is not enough.
- **CR-API-3.** Each obligation has one status: `covered` (with case IDs),
  `partial` (what is missing), `uncovered`, `waived`, or `not-applicable` (with
  the reason). Reports give counts per status; they never fold `waived` or
  `not-applicable` into `covered`. The complete inventory is open task
  `frost-wou.6`; until it lands, obligation coverage is **unmeasured**.
- **CR-API-4.** Exhaustive finite domains and differential oracles are
  preferred. Timing assertions and sleeps are not used in correctness tests.

### Measured baseline and targets

Both observations use the unchanged production source at revision `6ab0918`,
Rust 1.99.1, `aarch64-apple-darwin`, `cargo-llvm-cov` 0.9.1, all features, host
test and example targets, and the exclusion regex
`(^|/)(tests|benches|examples|verification)/`. Benchmarks do not run and
doctests are not instrumented. The [baseline](verification/crate-standards-validation/baseline-coverage.json)
was recorded before the standards work; the
[final observation](verification/crate-standards-validation/after-coverage.json)
was recorded by `make coverage-gate` after
[the six API contract tests](verification/crate-standards-validation/api-contracts.md)
were added. The [standards evidence](verification/crate-standards-validation/README.md)
records both.

The denominator covers instrumented code in `src/` files in the stated build
scope, **including inline `#[cfg(test)]` modules inside those files**.
It is file-scoped library-source coverage, not a test-free production-code
denominator. Code excluded by conditional compilation is not counted.

| Metric | Baseline (`6ab0918`) | Final same-scope observation | Target | Status |
| --- | --- | --- | --- | --- |
| Lines | 77.197% (7,089/9,183) | 79.157% (7,269/9,183) | ≥ 90% | Not attained |
| Regions | 77.772% (11,067/14,230) | 79.431% (11,303/14,230) | ≥ 90% | Not attained |
| Functions | 70.908% (1,343/1,894) | 73.759% (1,397/1,894) | ≥ 85% | Not attained |
| Generic instantiations | 63.171% (5,434/8,602) | Recorded in the JSON | Report only | Not gated |
| Branch and MC/DC | Unmeasured (a zero denominator, not 0%) | Unmeasured | Set after the first measurement | `frost-wou.7` |
| Instrumented doctests, isolated profiles, non-host targets | Unmeasured | Unmeasured | Report per scope | `frost-wou.7` |
| Public API obligations | Unmeasured | Unmeasured | 100% dispositioned, zero `uncovered` | `frost-wou.6` |
| Mutation survivors | Unmeasured; `cargo-mutants` not installed on the host | Unmeasured | Every survivor triaged; tier scores in the [mutation policy](verification/mutation/README.md) | `frost-wou.8` |

Raising the three source metrics to their targets with meaningful API tests is
open task `frost-wou.9`.

- **CR-COV-1.** Report a percentage only with its scope: revision, compiler,
  target, features, tool version, instrumented paths, and exclusions. A number
  from one scope is never quoted for another; a macOS result does not establish
  the Linux CI value.
- **CR-COV-2.** `make coverage-gate` applies two separate checks and records
  which one ran in `scope-metadata.json` beside the JSON summary:
  - **Portable adoption floor**: `--fail-under-lines 77`,
    `--fail-under-functions 70`, `--fail-under-regions 77` (`COVERAGE_MIN_*`).
    The floor sits below the baseline and applies on every host. It is an
    adoption floor, not a non-regression check.
  - **Exact ratchet**: only when the effective target is
    `aarch64-apple-darwin`, `${RUSTC:-rustc} -vV` reports release 1.99.1, any
    `RUSTUP_TOOLCHAIN` is 1.99.1, `RUSTFLAGS` and `CARGO_ENCODED_RUSTFLAGS` are
    empty, the tool is `cargo-llvm-cov 0.9.1`, and the exclusion regex is
    unchanged. Then the gate also fails if any of the three percentages falls
    below the baseline JSON, and the mode is `scoped_exact_ratchet_enforced`.
  - **On any scope mismatch** the ratchet is not applied, the floor alone
    decides, and the mode is `portable_adoption_floor_only_cross_target`. A
    mismatched run is never reported as passing the ratchet.

  Raising the floor or replacing the baseline is a reviewed change that reports
  before and after values in the same scope.
- **CR-COV-3.** Do not write "high coverage" or "fully covered" for a metric
  that is below its target or unmeasured.

### No waiver laundering

- **CR-WAIVE-1.** A waiver names the obligation or file, the reason, an owner,
  a tracker issue, and a review date. Expired waivers count as `uncovered`.
- **CR-WAIVE-2.** Changing the coverage denominator (ignore patterns, excluded
  files, `cfg` exclusions, test filters) is a reviewed policy change, recorded in
  [coverage-policy.json](verification/coverage/coverage-policy.json). Report the
  metric before and after it.
- **CR-WAIVE-3.** Moving code into an excluded path, renaming a test to match an
  obligation, marking a record `curated` without review, or splitting a
  function to raise a ratio does not cover an obligation.
- **CR-WAIVE-4.** Quarantined, ignored, filtered, or skipped tests are reported
  separately and never count as passing.

## 6. Test and benchmark metadata

The isolated [catalog tool](verification/catalog/README.md) validates two
sidecar files. Neither adds a runtime or dev-dependency to the library.
`make catalog` runs the tool's own tests and `check`; `make catalog-inventory`
lists every benchmark target and compares the exact case names. Both run in
`make check`.

### Catalog: `frost-verification-catalog/1`

A catalog file holds `schema`, `contracts[]`, `targets[]`, `cases[]`, and
optionally `isolated[]`. There is one per base directory, at
`<base>/verification/metadata/catalog.json`:

- the [root catalog](verification/metadata/catalog.json) records repository
  tooling and the isolated packages under `verification/`;
- each workspace member may have its own, for example
  [the library's](crates/frost-collections/verification/metadata/catalog.json).

`targets[].path`, `cases[].file`, `cases[].target` and `isolated[]` are relative
to the file's base, so a package's records move with it. One `check` reads all
of them. Contract and case IDs share one namespace across files, and a case may
cite a contract defined in another file. `isolated[]` lists the manifests of
isolated packages whose targets the tool also discovers.

| Record | Fields |
| --- | --- |
| Contract | `id` (for example `C-ORDERED-SET-ALGEBRA`), `title`, `source` documents, `obligations[]` (public paths as traceability references, or `law:<ID>` for a law in `docs/design/algebra/catalog.json` that has no public item yet) |
| Target | `path`, Cargo `kind`, `features[]`; must include the target's Cargo `required-features` |
| Case | `id`, `target`, `file`, `selector`, `status`, `kind`, `tags[]`, `features`, `cfg`, `area`, `provenance`, `contracts[]`, `oracle`, `scenario`, `seed`, `cost`; benchmark cases add `timing` and `control` |

| Field | Implemented values and rule |
| --- | --- |
| `id` | Matches `[A-Z]+-[A-Z0-9_]+-[0-9]+` and is unique. Existing IDs use `T-<TARGET>-<NNN>` for tests and `B-<TARGET>-<NNN>` for benchmarks, assigned at registration |
| `selector` | `test:<qualified-name>`, `macro:<qualified-template-name>`, `fuzz:fuzz_target`, `target`, `bench`, `fn:<name>`, or `literal:<text>`; the last two name benchmark source text, not executable names |
| `status` | `curated` (semantics reviewed) or `unreviewed` (structural only) |
| `kind` | `differential`, `exhaustive`, `property`, `regression`, `model`, `ownership`, `contract`, `benchmark`, `example`, `unreviewed` |
| `tags` | Nonempty; each from the controlled vocabulary in the [catalog README](verification/catalog/README.md#catalog-schema-frost-verification-catalog1): area terms (`ordered`, `integer`, `snapshot`, ...), contract topics (`freeze-thaw`, `set-algebra`, ...), machinery (`factory`, `catalog`, `native-control`), work kind (`integration`, `benchmark`, `root-contract`), and `unreviewed` |
| `features` | An explicit array of the package's features, or `"inherit"` to take the target record's features |
| `cfg` | The structural enclosing and item conditions the tool discovered, or `none`; must equal the discovered value |
| `area` | `family`, `adaptive`, `integer`, `backends`, `adapters`, `concurrent`, `verification`, `support` |
| `provenance` | An existing path (first token), `review <note>`, or `discovery:scaffold`. The path names a file under the catalog's base if one exists there, and otherwise a file relative to the repository root. A contract's `source` paths resolve the same way |
| `contracts` | Known contract IDs; a curated case must name at least one |
| `oracle`, `scenario` | Nonempty; a curated case names a real oracle and scenario |
| `seed` | `none`, `fixed`, `proptest`, `factory`, `unreviewed` |
| `cost` | `fast`, `bounded`, `slow`, `timed`, `unreviewed` |
| `timing` | Benchmarks: `operation`, `destructive-with-setup`, `aggregate-readers` |
| `control` | Benchmarks: the native control, or `none` |

### Benchmark inventory: `frost-bench-inventory/1`

[bench-inventory.json](crates/frost-collections-core/verification/metadata/bench-inventory.json) maps each
benchmark target to every exact case name that `FROST_BENCH_LIST=1` prints.
Each entry has `name`, `timing`, `status`, `tags`, `features`, `scenario`,
`control`, `area`, `provenance`, `seed`, `cost` (always `timed`), and
`contracts`. The enclosing target supplies target scope; `"inherit"` refers to
that target's catalog record. `control` names an equivalent sibling case,
`none` for a reviewed native control, or `unreviewed: <reason>`. Factory case
names carry the full canonical specification and seed, so a name replays its
input.

### Rules

- **CR-META-1.** Existing IDs are stable. They are not renamed in bulk, and a
  moved or renamed test keeps its ID. The target stem inside an ID records where
  the case was registered; it is not recomputed from the current path. A
  retired ID is never reused.
- **CR-META-2.** Every discovered test, proof, fuzz entry, and benchmark target
  needs a case record, and every benchmark name needs an inventory entry. Add
  missing records with `scaffold` or `inventory-scaffold`; both add
  `unreviewed` records only.
- **CR-META-3.** `check` and `inventory` fail on malformed JSON or field types,
  unknown fields or vocabulary terms, duplicate IDs or selectors, stale or
  missing selectors, unregistered targets or cases, unknown contracts, `cfg`
  that differs from discovery, and missing or extra benchmark names. A curated
  case without a contract, oracle, scenario, or reviewed kind and seed fails.
- **CR-META-4.** Structural discovery is not semantic review. The tool parses
  source with `syn` and Cargo metadata. It does not expand macros, enumerate
  doctests (including `compile_fail` examples), evaluate `cfg` for a profile, or
  run anything. A `macro:` record names a template, not an executed expansion.
- **CR-META-5.** Counts are not coverage. On this checkout `check` reports 386
  structural test and proof declarations and 403 case records (78 `curated`,
  325 `unreviewed`): core 246, integer 78, concurrent 28, the facade 12, the
  support package 1, and 38 in the root catalog. There are 12 contracts, and
  922 benchmark names in seven targets, with every inventory entry
  `unreviewed`. Every contract has at least one curated case. Curating the
  unreviewed legacy records is part of `frost-wou.6`.
- **CR-META-6.** Benchmark records keep raw samples and slower observations,
  and identify the compiler, profile, target CPU, and source revision. Listing
  validates answers and names without timing; it supports no performance claim.
- **CR-META-7.** A new tag, kind, area, seed, or cost value is added to the
  catalog tool's vocabulary, its README, and this section in the same change.

## 7. Verification tools and proof boundaries

| Tool | Make entry | Claim it supports |
| --- | --- | --- |
| Standard tests, `proptest`, `proptest-state-machine`, `exhaust` and `itertools` domains | `make check`, `make test` | Behavior on executed and generated inputs; committed failing seeds |
| cargo-nextest with [nextest.toml](.config/nextest.toml) | `make nextest` | The same tests with explicit profiles and zero retries |
| Miri | `make miri` on the pinned diagnostic nightly | Undefined behavior on executed paths, not soundness for all callers |
| Loom | `make loom` | The production retry loop under the stated writer bound and store assumption |
| Kani | `make kani` | Bounded symbolic results with stated unwind limits, only when the verifier ran |
| Verus, pinned by `VERUS_VERSION` | `make verus` | A law holds for its reference definition over vstd types; says nothing about Frost code |
| Fuzzing ([fuzz harness](crates/frost-collections-integer/fuzz/README.md)) | `make fuzz-check` replays the seed corpus on stable; `make fuzz-smoke` runs bounded libFuzzer on `FUZZ_TOOLCHAIN` | Oracle agreement on inputs reached in a recorded run; no absence-of-bugs claim |
| Source coverage ([policy](verification/coverage/README.md)) | `make coverage`, `make coverage-gate`; `make coverage-branch` on nightly | Executed code in a stated scope (CR-COV-1, CR-COV-2) |
| Mutation testing ([policy](verification/mutation/README.md)) | `make mutants` with `.config/mutants.toml` | Survivors show unasserted behavior; a low survivor count is not proof |
| Catalog | `make catalog`, `make catalog-inventory` | Structural registration and metadata validity (section 6) |

- **CR-TOOL-1.** A harness, workflow, or annotation is not a result. Record the
  command, versions, target, features, bounds, and output as described in
  [verification scope](docs/verification.md).
- **CR-TOOL-2.** A model states its domain, scheduler bound, dependency
  assumptions, and the production code it executes.
- **CR-TOOL-3.** Do not lower bounds, disable unwind or validity checks, or
  filter cases to turn a gate green.
- **CR-TOOL-4.** Every optional-tool target checks the tool's version first and
  fails when it is absent. On the reference host `cargo-mutants`, Kani and Verus
  are not installed, so `make mutants`, `make kani` and `make verus` fail there
  by design. With Kani 0.68.0 installed, `make kani` still does not pass: two
  harnesses do not finish within the host's memory
  ([recorded run](docs/verification.md#recorded-run-2026-11-13)).
- **CR-TOOL-5.** Proof layers are named by what they cover. Finite models and
  sampled tests cover their stated domains. Kani refinement harnesses tie a real
  operation to its reference definition within their bounds. Verus proves the
  reference definition without bounds and touches no production code. No
  combination is an unbounded proof of Frost code.

## 8. Make orchestration

- **CR-MAKE-1.** The root `Makefile` is the only build and verification entry
  point. `make help` lists targets and runs no Cargo command. No per-package
  Makefiles, dispatcher scripts, or Python.
- **CR-MAKE-2.** Recipes target GNU Make 3.81+ and Bash 3.2+, discover packages,
  features, and targets from `cargo metadata` with `jq` inside recipes, and stop
  on the first failure.
- **CR-MAKE-3.** Optional tools are explicit targets that fail with a clear
  message when the tool is absent. They are never silently skipped or reported
  as passed.
- **CR-MAKE-4.** Recipes iterate packages from Cargo metadata; adding a package
  must not require a hand-edited list unless it is an intentional exclusion.
  A recipe that sets features names its package with `-p` and never relies on
  feature unification across workspace members: at the workspace root,
  `--no-default-features` does not turn `std` off in a package that another
  member builds with it.
- **CR-MAKE-5.** Scripts kept with recorded evidence, such as those under
  `verification/results/crate-split/`, are not build tooling. No recipe calls
  them, and they may use Python.

`make check` runs formatting, links, Clippy, rustdoc, tests, examples,
`catalog`, and `catalog-inventory`. `make verify` adds profiles, isolated
features, integers, examples, and the package test. The
[testing standard](docs/maintainers/testing.md#6-gate-tiers-and-make-commands)
assigns every target to the pull-request, nightly, or release tier, and
[CONTRIBUTING.md](CONTRIBUTING.md#verification-scopes) lists each command.

[.github/workflows/verification.yml](.github/workflows/verification.yml) runs
these targets in CI. It has been validated locally with `actionlint`; this
change does not claim that it has run on GitHub.

## 9. Change checklist

| Change | Required in the same change |
| --- | --- |
| New or changed public item | Rustdoc with ownership, panic, complexity, feature, and concurrency notes; obligations; tests per category; curated catalog cases linked to a contract; changelog |
| New or changed formal law | A model module in the owning package's `crates/frost-collections-core/tests/formal_models/` with wrong-definition checks; the law in `docs/design/algebra/catalog.json` and the proof ledger; curated catalog cases |
| New test or benchmark | Catalog case or inventory entry (scaffold, then curate when reviewed); features and `cfg`; benchmark control, timing, and seed; `make catalog` and `make catalog-inventory` pass |
| New feature | `Cargo.toml` entry, feature catalog in [README.md](crates/frost-collections/README.md), isolated and embedded matrix entries, tests compiled out when disabled |
| New isolated verification package | Own manifest and lockfile; Make target with a tool-version check; exclusion from packaging; proof-scope statement |
| Coverage denominator, floor, or waiver change | Before/after metric in the same scope; policy file and waiver fields |
| Layout or package move (`frost-wlt` only) | Module mapping, preserved IDs and import paths, updated links, full matrix rerun |

[cargo-dev-deps]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#development-dependencies
[cargo-resolver]: https://doc.rust-lang.org/cargo/reference/resolver.html#resolver-versions
[cargo-workspaces]: https://doc.rust-lang.org/cargo/reference/workspaces.html

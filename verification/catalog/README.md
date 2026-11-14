# Frost verification catalog

This isolated Rust tool uses `syn` and Cargo target metadata; its dependencies
do not enter the library graph. It needs the package lockfiles and an offline
Cargo cache sufficient for `cargo metadata --locked --offline --no-deps`.

```sh
cargo test --locked --manifest-path verification/catalog/Cargo.toml
cargo run --locked --manifest-path verification/catalog/Cargo.toml -- check --root .
cargo run --locked --manifest-path verification/catalog/Cargo.toml -- scaffold --root .
FROST_BENCH_LIST=1 cargo bench --locked --all-features --bench ordered_sets > /tmp/ordered_sets.inventory
cargo run --locked --manifest-path verification/catalog/Cargo.toml -- inventory benches/ordered_sets.rs /tmp/ordered_sets.inventory --root .
```

Repeat the listing and `inventory` for all seven benchmark targets. Listing
validates answers and emits names without timing; snapshots enumerate reader
counts 1 through 4 without worker threads. Timed snapshots retain their normal
host-dependent reader selection. `inventory` checks exact two-way name and
timing agreement. `inventory-scaffold <target> <listing>` adds missing entries
as unreviewed; it does not remove stale names.

## Structural discovery

Cargo metadata supplies roots for every package of the root manifest (library,
integration tests, examples, benches) and for each isolated package a catalog
lists under `isolated[]`: today the catalog, fuzz, Kani, Loom and downstream
packages.
The parser follows inline and external modules, including `#[path]`, and uses
fully qualified module names. It records `#[test]`, `#[kani::proof]`, proptest
declarations, fuzz entry macros, and benchmark targets. Target roots without a
discovered case get a `target` marker. `macro:` records are test declarations
inside a macro definition; they do not claim its expansion was executed.
Comments and string contents do not create tests. Macro expansion, conditional
compilation evaluation, doctests, dynamic code, and actual execution are not
proved. The `cfg` field records structural enclosing and item conditions;
conditions inside macro expansions are not resolved. It is not a full
evaluation of feature or target eligibility.

## Catalog schema `frost-verification-catalog/1`

A catalog file is `<base>/verification/metadata/catalog.json`. The repository
root has one; each package of the root manifest may have its own. `check` and
`scaffold` read them all from `--root <repository root>`. Target paths, case
`file` and `target`, and `isolated[]` manifests are relative to the file's
base. A `provenance` or contract `source` path names a file under the base if
one exists there, and otherwise a file relative to the repository root.
Contract and case IDs share one namespace across files. `scaffold` writes a
new record to the file whose base owns the target. The benchmark inventory
follows the same rule; `inventory` takes `--root <package directory>`.

A file contains `schema`, `contracts[]`, `targets[]`, `cases[]`, and
optionally `isolated[]`. Contracts have `id`, `title`, `source` paths, and
`obligations[]`. Names are traceability references: the tool does not resolve
`Type::method` or measure public API obligation coverage.

Targets identify `path`, Cargo `kind`, and `features[]`; required Cargo
features must be included. A case has `id`, `target`, `file`, `selector`,
`status`, `kind`, `tags[]`, `features` (a feature array or `"inherit"`),
`cfg`, `area`, `provenance`, `contracts[]`, `oracle`, `scenario`, `seed`,
and `cost`. A benchmark case also has `timing` and `control`. Case selectors
are `test:<qualified-name>`, `macro:<qualified-template-name>`,
`fuzz:fuzz_target`, `target`, `bench`, `fn:<name>`, or `literal:<text>`.
The latter two describe selected benchmark source text, not executable names.
The `cfg` value `none` means no recorded condition on that structural path;
target feature inheritance and `cfg` must both be considered.

Statuses are `curated` or `unreviewed`; scaffolding never establishes semantic
coverage. Kinds are differential, exhaustive, property, regression, model,
ownership, contract, benchmark, example, and unreviewed. Areas are family,
adaptive, integer, backends, adapters, concurrent, verification, and support.
Seeds are none, fixed, proptest, factory, or unreviewed. Costs are fast, bounded,
slow, timed, or unreviewed. Timing is operation, destructive-with-setup, or
aggregate-readers. Curated cases require an oracle, scenario, and contract.

Controlled tags: `ordered`, `hash`, `vec`, `adaptive`, `inline`, `integer`,
`bitmap`, `roaring`, `concurrent`, `snapshot`, `adapters`, and `backends`
identify the affected family, profile, storage, or access area; `freeze-thaw`,
`set-algebra`, `map-entry`, `iterator`, and `ownership` name contract topics;
`factory` and `catalog` identify verification machinery; `native-control`
identifies a native comparison; `integration`, `benchmark`, and
`root-contract` identify the kind of work; `unreviewed` signals structural
metadata awaiting semantic review. These labels do not assert execution or
coverage.

## Benchmark inventory `frost-bench-inventory/1`

`verification/metadata/bench-inventory.json` maps each benchmark target to
exact named cases. Each record has `name`, `timing`, `status`, `tags`,
`features`, `scenario`, `control`, `area`, `provenance`, `seed`, `cost`, and
`contracts`. Target scope is the enclosing target key; feature inheritance refers
to that target's catalog record. Factory benchmark names carry their full
canonical spec and seed. Controls name a matching sibling, `none` for a reviewed
native control, or `unreviewed: <reason>` when equivalence is unresolved.
Name-derived entries are structural, not semantic reviews. No measurement or
performance claim follows from listing.

The checker rejects malformed JSON or field types, invalid vocabularies,
missing paths or target/contract links, stale or missing discovered selectors,
and missing or invalid benchmark records. It does not prove test quality,
obligation coverage, performance, or unexpanded macro behavior.

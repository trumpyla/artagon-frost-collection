# Code search in Frost

Run these commands from the repository root. The optional development tools
stay outside Cargo's dependency graph. The installed `artagon-ripwire`,
`artagon-tgrep`, and `artagon-ast-grep` plugin skills provide further help when
available; this checkout's commands also work without those plugins.

## Choose the tool and scope

| Need | Tool | Scope and limits |
| --- | --- | --- |
| Find an entry point, callers, callees, or likely impact | `ripwire` | Syntax-derived graph; ambiguous edges are candidates and counts are lower bounds. |
| Repeat regex queries across a stable tree | `tgrep` | Local index is a snapshot; refresh before trusting results. |
| Find Rust syntax shapes or preview a structural rewrite | `ast-grep` | Parses syntax, not receiver types, macro expansions, or feature combinations. |
| Search text, current edits, feature declarations, CI, or prose | `rtk rg` | Reads current files; respect exit 1 for no match and exit 2 for errors. |

Use `src/` for production code, `tests/` for contracts, and `examples/` and
`benches/` for consumers. The isolated `verification/loom`, `verification/kani`,
and `verification/int-key-downstream` packages need separate attention when
their included production sources or dependency contracts change.

[.ignore](.ignore) keeps historical runs and supplied source copies out of
default ripgrep, tgrep, and ast-grep walks. It does not remove Git-tracked files.
For historical evidence, name the path and bypass search ignores explicitly:

```sh
rtk rg --no-ignore -n 'rustc' verification/results/environment.json
```

Do not use `target/`, archived source, or benchmark snapshots as evidence of
the current implementation. Ripwire commands below use explicit live roots.

## Ripwire

```sh
ripwire --version
ripwire . --doctor
ripwire src --for="ordered set intersection" --legend=compact
ripwire src tests examples benches --callers=intersection_len --legend=compact
```

Replace the selector with the actual symbol; use `file:name` when names
collide. Use `--callees=SYMBOL`, `--uses=SYMBOL`, or `--impact=SYMBOL` for the
corresponding graph query. Ripwire uses its own temporary cache and needs no
project daemon or checked-in cache. Confirm results in source. An `amb=` edge,
missing known symbol, or unresolved trait dispatch makes a claim `PARTIAL`.

## Tgrep

[.gitignore](.gitignore) excludes the local `.tgrep/` index. Build or refresh it
with hidden CI/configuration coverage while excluding Git internals and IDE
state:

```sh
tgrep --version
tgrep index --hidden --exclude .git --exclude .idea .
tgrep status .
tgrep --hidden -n -g 'src/**/*.rs' 'publish_if_current' .
```

Use `.` as the query root so tgrep finds this checkout's index; select a
subtree with `-g`. Check the root, update time, and hidden coverage in `status`.
Refresh with the same index command after Git transitions or edits. For a
file changed since indexing, use `rtk rg` immediately. A query can fall back
to a live scan if the index is absent or its coverage is incompatible; inspect
`--stats` before claiming indexed execution. No watcher or refresh hooks are
required by this setup. Start a long-running server only when requested.

## Ast-grep

[sgconfig.yml](sgconfig.yml) loads the rule and tests under
[verification/ast-grep](verification/ast-grep/). Ad hoc Rust queries name the
language explicitly; configured rules declare `language: Rust` in YAML.

```sh
ast-grep --version
ast-grep run --lang rust --pattern '$VALUE.freeze()' crates
ast-grep test --config sgconfig.yml
ast-grep scan --config sgconfig.yml --inspect summary crates
```

The `rust-unsafe-block-scope` rule flags unsafe blocks in production source
outside `src/adaptive/seq/buf.rs`, matching the existing project boundary.
Its fixtures cover real blocks and near misses in strings/comments. Path
scoping is checked by scanning the production tree; fixture tests exercise
the matcher. The rule does not audit unsafe functions, trait implementations,
macro expansions, or the safety of the allowed buffer module. Cargo lints,
ownership review, and Miri remain required for the relevant changes.

Test a new structural pattern against a known match before relying on an empty
result. Preview a rewrite without `--update-all`, inspect every receiver, and
apply only within the authorized edit scope. Rule snapshots are reviewable
fixtures; update them only after inspecting changed diagnostic ranges.

Run the test and scan commands when changing rules or production Rust. These
are optional development checks, separate from the required library gate.
Configuration follows the upstream [project configuration reference](https://ast-grep.github.io/guide/project/project-config.html)
and [rule testing guide](https://ast-grep.github.io/guide/test-rule.html).

## Ripgrep and verification

```sh
rtk rg -n 'publish_if_current' src tests
rtk rg --hidden -n 'make check' .github
rtk rg -n 'int-set|no_std|cfg\(' Cargo.toml src
rtk make check
```

Run ripgrep through `rtk rg`, and the other three CLIs directly. Read files
with a native reader or focused `rtk proxy sed` ranges. Do not hide errors or
pipe search results through bare grep/sed/awk. No global ripgrep environment
variable is needed: Git ignores and `.ignore` supply the project exclusions.

Rust macros/derives, `cfg` features, trait objects, generic dispatch, public
reexports, serde/borsh adapters, and `#[path]`/`include!` verification harnesses
cross syntax-graph boundaries. Check their source, feature declarations, and
applicable builds before making an impact or absence claim.

Use `SUPPORTED` for tool results confirmed in live source or build output,
`PARTIAL` for known coverage gaps, and `UNAVAILABLE` for a missing or failed
tool. Fall back to `rtk rg` plus Cargo; use a healthy Rust semantic server when
available for exact references. An unused/no-callers claim requires independent
semantic and live-text evidence, including configuration. Search and AST
checks alone do not prove compilation, ownership safety, or behavior.

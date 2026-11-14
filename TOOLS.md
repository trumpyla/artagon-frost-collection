# Code search and editing tools in Frost

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
| Edit a file by line anchors instead of repeating its old text | `hashline` | Rejects an edit whose anchor no longer matches the file. Optional; use the CLI's own edit tool when it is absent. |

Use `crates/*/src/` for production code, `crates/*/tests/` for contracts, and
`crates/frost-collections/examples/` and `crates/*/benches/` for consumers. The isolated `crates/frost-collections-concurrent/verification/loom`, `crates/frost-collections-core/verification/kani`,
and `crates/frost-collections/verification/int-key-downstream` packages need separate attention when
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
ripwire crates --for="ordered set intersection" --legend=compact
ripwire crates --callers=intersection_len --legend=compact
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
tgrep --hidden -n -g 'crates/*/src/**/*.rs' 'publish_if_current' .
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
outside `crates/frost-collections-core/src/adaptive/seq/buf.rs`, matching the existing project boundary.
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

## Hashline

`hashline` shows each line of a file with a two-character hash of its content.
An edit names the lines by `LINE:HASH` and gives only the new text. If the
file changed after it was read, the hash no longer matches and the edit is
refused, so a stale edit cannot land.

```sh
hashline --version
hashline read crates/frost-collections-core/src/plan.rs
hashline patch --dry-run crates/frost-collections-core/src/plan.rs 'SWAP 12:9f:
+    let limit = 40;'
hashline guide
```

`read` prints `[path#HASH]` and then `LINE:HASH|text` for every line. In a
patch, `SWAP 12:9f:` replaces a line or a range such as `12:9f..14:a1`, `DEL`
deletes, and `INS.POST` inserts after the anchor; each new line starts with
`+`. `hashline guide` documents the whole patch language for the installed
version. Always give the hash: a bare line number applies by position and
detects nothing.

On `ERR STALE_ANCHOR`, read the file again and retry with the new anchors. Do
not retry with a bare line number.

Agent CLIs reach the same commands as tools named `read`, `patch`, `write`,
`find_block`, `remove_file` and `rename_file` through the `hashline mcp`
server. The repository registers that server for Claude Code and Copilot CLI
(`.mcp.json`), Codex (`.codex/config.toml`) and the Antigravity CLI
(`.agents/mcp_config.json`). Each CLI asks before it starts a server from a
repository file. When the binary is not installed the server does not start,
and the CLI's own edit tool is the fallback.

Install the version these files were checked with, 0.9.24:

```sh
cargo install --git https://github.com/quangdang46/hashline \
  --rev 4b7f8debdd7864480dbea0a58562e1af11932319 --locked hashline
```

`hashline` is a third-party tool under the MIT license. It is not a
dependency of any package here. Its tools read and write the paths they are
given, with no restriction to this repository, so the CLI's own permission
prompts are the control. The registered server sets
`HASHLINE_NO_UPDATE_CHECK=1`, which turns off its daily check for a new
release. The project's own installer script also writes a server entry into
each agent CLI's user configuration; the `cargo install` command above does
not.

## Ripgrep and verification

```sh
rtk rg -n 'publish_if_current' crates
rtk rg --hidden -n 'make check' .github
rtk rg -n 'int-set|no_std|cfg\(' Cargo.toml crates
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

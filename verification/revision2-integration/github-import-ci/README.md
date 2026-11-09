# GitHub integration and Make validation

This record covers the reviewed integer-storage integration onto GitHub main
`8807609c62e29f2ef009bfc446625060e4cf37b8`, preserving its newer Make-based
verification tooling and code-search guidance.

## Confirmed upstream CI failure

[Run 42628268660](https://github.com/trumpyla/artagon-frost-collection/actions/runs/42628268660)
completed with ten successful jobs and one failed Windows host job. Within
Windows, quality/tests and core/alloc/default profiles passed; launching the
example programs failed. The native Windows jq output supplied an example name
with a terminal carriage return, so Cargo received `adaptive\r` rather than
`adaptive`. The [failure excerpt](windows-original-excerpt.log) retains the
relevant job output with log record line endings normalized to LF.

The PR normalizes raw metadata records at the Bash consumer boundary, covering
example names, feature names, and the packaged manifest path. It preserves
quoting, command failure propagation, and the existing Bash 3.2/Make 3.81 target.
The new regression tests reproduce CRLF records without requiring Windows.

## Production source provenance

All 67 production Rust source files match the previously reviewed and verified
integration byte for byte. All 117 files across production source, tests,
examples, benches, Cargo manifests/lockfile, and the pinned toolchain match that
integration. Relative to the GitHub base, production changes affect only the
eight integer source files; all 59 noninteger Rust files remain unchanged.
The production dependency declarations and lockfile remain unchanged.

The earlier native, feature, embedded, Miri, and benchmark evidence in the parent
directory retains its actual original commands and source scope. The current
Make targets carry the same ownership regressions and both atomic/non-atomic
embedded integer builds. Current adaptation checks are recorded below after
completion; historical measurements are not rerun or relabeled as new results.

## Current adaptation validation

All five local Make adaptation gates passed on Rust 1.99.1:

| Command | Result | Evidence |
| --- | --- | --- |
| `make check` | Formatting, links, Clippy, rustdoc, native tests, and examples passed | [check.log](check.log) |
| `make examples` | All 11 example programs ran successfully | [examples.log](examples.log) |
| `make package` | Package creation and extracted-source tests passed | [package.log](package.log) |
| `make integers` | Debug/release integer suites and downstream codec fixture passed | [integers.log](integers.log) |
| `make embedded` | Both integer targets, core, and alloc profiles built | [embedded.log](embedded.log) |
| `make shell-check` | ShellCheck, shfmt, and all 24 Bats cases passed | [shell-check.log](shell-check.log) |

[Machine-readable Make status](local-make-status.json) records each command's
exit status, elapsed time, and production hashes. No production source changed
during these runs. The native and packaged-source suites each passed 271 native
tests and 59 doctests, followed by 11 repeated example test executions.

All three new Windows-CRLF cases failed with the original Makefile and passed
with the fix: [before](crlf-before.log), [after](crlf-after.log). The tests check
Cargo's actual received arguments, including a package path containing spaces.
[Dry runs](make-dry-runs.log) cover features, examples, packaging, embedded, and
Miri command construction.

The shell lane used Bash 5.2.37, GNU Make 4.4.1, jq 1.7, Bats 1.14.0,
ShellCheck 0.11.0, and shfmt 3.14.1. Bash 3.2/Make 3.81 binaries and a Windows
runtime were unavailable locally. The fix uses existing-compatible shell suffix
removal, and the hosted Windows job supplies the platform execution check.

The prior full Miri and benchmark runs certify matching production bytes and
remain in the parent evidence directory. Those expensive runs were not repeated
locally merely to change their command entry point. GitHub Actions will execute
the current Make-based ownership and cross-platform verification on this PR.

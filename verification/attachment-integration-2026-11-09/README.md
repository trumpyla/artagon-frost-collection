# Attachment integration on 2026-11-09

Baseline: `1ef810fe0` on `main`; the checkout was clean before integration.
[Input digests](inputs.json) identify all five supplied attachments. This record
covers the current checkout, separately from evidence imported in the patch.

## Input disposition

| Input | Integration |
| --- | --- |
| `frost-revision2-github-pr.patch` | Applied the reconciled code, tests, docs, CI, and historical evidence; manually reconciled the existing Windows fix in the three conflicting files. |
| `integer-revision2.diff` | Earlier 29-file revision, retained exactly under [input/](../revision2-integration/input/integer-revision2.diff). Its applicable changes are reconciled in the larger patch. |
| `integer-revision2-code.diff` | Exact 12-file subset of the earlier diff, retained under [input/](../revision2-integration/input/integer-revision2-code.diff); no second application. |
| `Frost1.md.md` | Preserved byte for byte as [research 1](../../docs/reviews/supplied/frost-collection-algebra-research-1.md). |
| `Frost2.md.md` | Preserved byte for byte as [research 2](../../docs/reviews/supplied/frost-collection-algebra-research-2.md). |

The [collection algebra synthesis](../../docs/design/collection-algebra.md)
integrates both reports' proposed contracts with the current architecture.
Research citation tokens and external claims were not independently verified.
The separately referenced research-template attachment was not supplied.

## Conflict resolution

The large patch passed `git apply --check` except for `CHANGELOG.md`, `Makefile`,
and `verification/shell/makefile.bats`. All other 282 file changes applied.
The checkout already had equivalent Windows CRLF handling and regression tests
from `1ef810fe0`; those were preserved. The Makefile gained the non-atomic
embedded target and bounded Chunked/Dense Miri cases. The changelog retains both
the existing Windows-fix entry and the new revision entries.

The earlier diff's sole path absent from the large patch,
`verification/int-key-downstream/Cargo.lock`, already matched its result blob
`7f7442907aa0630801bff23a121e94b139d757d2`. No lockfile replacement was needed.

The stricter retained result limits, canonical Hash events, automatic planner,
and bounded conversions follow the reconciled patch. The earlier diffs'
larger-limit inheritance and unconditional small-window Dense preference are
not applied. See the supplied [integration review](../../docs/reviews/revision2-integration-review.md).

Project-local `.gitignore` exceptions retain the two imported evidence trees'
logs and this run's logs even when global Git excludes contain `*.log`. No
global configuration was changed. The [patch audit](patch-audit.json) confirms
279 of 285 resulting files match the supplied large patch exactly. The six
adaptations are the three conflict paths and the documentation index,
architecture overview, and supplied-research index. Both research documents and
both original diffs are preserved byte for byte.

At staging, the full whitespace check reports context-line spaces in supplied
diffs, retained historical log whitespace, and final blank lines in two fresh
test logs. [The staged diagnostics](staged-whitespace.json) record the exact
file counts. These evidence bytes are preserved. The staged check excluding
patch/log evidence passes; this differs from the earlier unstaged tracked-file
check, which did not inspect newly added evidence files.

## Fresh verification

The host is Apple Silicon macOS with Rust 1.99.1 and Cargo 1.99.1. The
[source manifest](source-manifest.json) pins 123 production, test, example,
benchmark, build, script, and CI files. Those files remained unchanged during
verification. [Command status](status.json) records exit codes and attempts.

| Check | Result | Fresh evidence |
| --- | --- | --- |
| `make check` | Passed formatting, links, all-feature Clippy/rustdoc, 271 native tests, 59 doctests, and the repeated 11 example tests | [check.log](check.log) |
| `make shell-check SHELL=/bin/bash` | Passed ShellCheck, shfmt, and all 24 Bats tests using Bash 3.2.57 and GNU Make 3.81 | [shell-check.log](shell-check.log) |
| `make doc-links` after the research integration | Passed, 485 local links checked | [doc-links.log](doc-links.log) |
| `CARGO_TARGET_DIR=target/attachment-miri make miri` | Passed on retry using nightly-2026-11-08: native ownership and both aliasing configurations, the bounded trace model, and the prescribed i686 integer/Chunked/policy/Hash regressions | [miri-retry.log](miri-retry.log) |
| `make features integers embedded tools-check` | Passed all 20 isolated features (Clippy/tests/rustdoc), debug/release integer tests, the downstream codec package, all four embedded builds across both targets, and the ast-grep rule test/67-file scan | [feature-integer-embedded.log](feature-integer-embedded.log) |
| `git diff --check` and source manifest comparison | Passed; verification did not change the pinned source files | [status.json](status.json) |

The first Miri attempt passed its native scopes but exited 2 when a crates.io
download timed out before the i686 stage. Its [failed log](miri.log) is retained.
The complete retry passed after downloading the missing packages. The pinned
nightly, its Miri/rust-src components, and both embedded standard-library targets
were installed for these checks. No lockfile or compiler requirement changed.

Logs in this directory are from this integration run. Local absolute checkout
and home paths are replaced with `$REPO` and `$HOME`; other output is retained.
Imported logs under `verification/revision2-integration/` and
`verification/int-set/revision2/` remain historical, including their original
source snapshots, commands, and platform limitations. No new benchmark claim
is inferred from those results.

Verification above ran on the integration working tree before its delivery
commit. The full release
aggregate, extracted-package gate, optional Loom/Kani runs, and benchmarks were
not repeated. Embedded verification is compilation, not execution on hardware.
The imported research proposes future collection APIs; only the supplied
integer implementation is added to the production library here.

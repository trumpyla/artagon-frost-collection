---
trigger: always_on
---

# Repository rules

The rules for this repository are in `docs/maintainers/agent-rules.md`, which
is also the root `AGENTS.md`. Read that file before you change anything. The
points below repeat the ones that must not be missed.

- Build and verify only through the root `Makefile`. Run `make check` before
  review. Do not run `make miri`, `make kani`, `make mutants` or
  `make fuzz-smoke` unless the maintainer asks.
- All unsafe code is in
  `crates/frost-collections-core/src/adaptive/seq/buf.rs`. Add none elsewhere.
- Do not push, force-push or publish without the maintainer's explicit
  approval. Never push to `main`.
- Add no agent or tool attribution to commits, pull requests, comments, code
  or documents.
- Report a check you could not run, or that failed, as exactly that.

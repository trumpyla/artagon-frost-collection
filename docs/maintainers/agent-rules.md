---
id: FROST-RULES-ENTRY
scope: repository
path_base: repository-root
canonical_path: docs/maintainers/agent-rules.md
entry_symlink: AGENTS.md
rules_registry: docs/maintainers/rules.json
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
---

# Working on Frost

Paths below are repository-root-relative, including when this file is read
through the root `AGENTS.md` symlink.

Read `CRATES.md`, `TOOLS.md`, and `docs/maintainers/implementation-contracts.md`
before code or multi-file edits. The latter retains the complete current-main
semantic, safety, feature, integer-storage, snapshot, testing and benchmark rules.
Use `docs/maintainers/repository-rules.md` as the short checklist.

For documentation and research, also read `docs/maintainers/documentation-rules.md`.
Specify domains and observations before laws; distinguish source-backed behavior,
selected names, proposed contracts, and historical research. Start formal work at
`docs/design/algebra/README.md`; use the existing research templates.

Preserve current APIs and selected names. No production `ValueDomain` parameter
or placeholder crate is authorized by these documents. Workspace extraction is
tracked under `frost-wlt`, following `frost-wou` and `CRATES.md`.

Use the root Makefile and pinned toolchain. Run required gates for the changed
scope; report unavailable or failed checks honestly. Research mathematics is not
Rust test evidence, Miri, formal proof, coverage, or a performance measurement.
Do not mutate remote branches, publish packages, or claim a push without explicit
authorization and verification. Preserve unrelated work and raw evidence.

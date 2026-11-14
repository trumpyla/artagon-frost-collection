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

Public API compatibility is not a constraint (maintainer decision of 2026-11-12,
recorded in `docs/design/decisions.md`): names, paths, signatures, bounds and
type parameters may change in a refactor. That decision relaxes no semantic or
safety contract, and approves no specific redesign. A breaking change needs its
design record, changelog entry, rustdoc and catalog updates in the same change.
The repository is a Cargo workspace; packages live under `crates/`. Package
extraction is tracked under `frost-wlt` and follows `CRATES.md` and
`docs/design/crate-split-plan.md`. No placeholder crate is authorized.

These rules hold for every change; the implementation contracts state them in
full:

- The packages are those of `docs/design/crate-split-plan.md`; add or split one
  only under `frost-wlt`. Until its extractions land, `frost-collections` under
  `crates/` holds all the code, and `src/` below means its `src/`.
- Features are additive: enabling one never changes an existing type's
  representation, hasher, bounds or freeze/thaw partner.
- No new `unsafe` block or manual `Send`/`Sync` implementation without a written
  invariant and an ownership review. Unsafe stays in
  `crates/frost-collections-core/src/adaptive/seq/buf.rs`.
- Run `make check` before review. A new test or benchmark needs its catalog or
  inventory record in the same change. Keep existing catalog IDs.

Use the root Makefile and pinned toolchain. Run required gates for the changed
scope; report unavailable or failed checks honestly. Research mathematics is not
Rust test evidence, Miri, formal proof, coverage, or a performance measurement.
Do not mutate remote branches, publish packages, or claim a push without explicit
authorization and verification. Preserve unrelated work and raw evidence.

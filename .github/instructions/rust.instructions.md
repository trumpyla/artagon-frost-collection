---
applyTo: "**/*.rs,**/Cargo.toml"
---

- Build and verify through the root `Makefile` (`make check`), not raw `cargo`
  commands. Do not run `make miri`, `make kani`, `make mutants` or
  `make fuzz-smoke` unless asked.
- All unsafe code is in
  `crates/frost-collections-core/src/adaptive/seq/buf.rs`. Flag any new
  `unsafe` block, or any manual `Send` or `Sync` implementation, anywhere else.
- A Cargo feature is additive: enabling one must not change an existing type's
  representation, hasher, bounds or freeze/thaw partner. Each package declares
  and forwards the features of the packages below it.
- A new test or benchmark needs its record in that package's
  `verification/metadata/` in the same change. Existing catalog IDs are kept.
- `frost-collections` is the only supported entry point. The `-core`,
  `-integer` and `-concurrent` packages are internal, and every package sets
  `publish = false`.

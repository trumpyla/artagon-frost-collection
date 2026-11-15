---
paths:
  - "crates/frost-collections-core/src/adaptive/seq/**/*.rs"
  - "crates/frost-collections-core/tests/inline_safety.rs"
---

All unsafe code in this repository is in
`crates/frost-collections-core/src/adaptive/seq/buf.rs`. A new `unsafe` block
or a manual `Send` or `Sync` implementation needs a written invariant and an
ownership review (`docs/maintainers/implementation-contracts.md`). Move
`buf.rs` only as a unit. A change here is checked by the Miri lanes, which run
in CI's nightly and release tiers; do not run `make miri` unless the
maintainer asks.

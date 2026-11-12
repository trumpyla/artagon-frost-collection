---
id: FROST-RESEARCH-FORMAL-SOURCES
status: research-evidence
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Sources and evidence boundaries

Verified date: 2026-11-12. Repository baseline is `27b14399b0245df2a018fd5eb6f24f47ce7c2dbe`.
The collection carriers, equations, proof sketches, counterexamples and execution
profiles are definitions or derived reasoning in this proposal. They are not
attributed to Rust documentation as if the standard library specified them.

## Primary contracts inspected

| ID | Primary source | What it supports | Limit |
| --- | --- | --- | --- |
| SRC-RUST-EQ | [Rust Eq](https://doc.rust-lang.org/std/cmp/trait.Eq.html) | Equivalence laws are semantic obligations; compiler cannot verify reflexivity; safe-law failures must not justify unsafe reliance | Does not implement Frost domain validation |
| SRC-RUST-HASH | [Rust Hash](https://doc.rust-lang.org/std/hash/trait.Hash.html) | Equal keys must hash consistently; hash output is not a portable stable encoding | No exact identity from a hash, no cross-seed hash reuse guarantee |
| SRC-RUST-BORROW | [Rust Borrow](https://doc.rust-lang.org/std/borrow/trait.Borrow.html) | Borrowed key equality/order/hash must match stored form; case-insensitive versus str example | Not the only lawful lookup protocol |
| SRC-RUST-COHERENCE | [Rust implementations](https://doc.rust-lang.org/reference/items/implementations.html) | Inherent implementation ownership and trait coherence/orphan constraints | Does not approve a Frost crate graph |
| SRC-BIMAP-063 | [bimap 0.6.3](https://docs.rs/bimap/0.6.3/bimap/) | Two-sided uniqueness and explicit insertion overwrite/rejection choices | Native crate semantics are not Frost's proposed checked union |

These pages were checked for the claims above, not for the entire evolving
crate ecosystems. Their use does not establish runtime performance, Rust 1.99.1
build success, or a new dependency/version recommendation.

## Pinned project sources

| ID | Project artifact | Use |
| --- | --- | --- |
| SRC-FROST-READ | [read.rs](../../../src/read.rs) | Existing carrier, lookup, borrowed/value traversal, endpoint and cardinality capabilities |
| SRC-FROST-FROZEN | [frozen.rs](../../../src/frozen.rs) | Exact wrapper, sealed eligibility, access and consuming recovery |
| SRC-FROST-KEY | [integer/key.rs](../../../src/integer/key.rs) | Full original-value reconstruction, canonical accepted codes, order and stability |
| SRC-FROST-GUIDE | [Frozen guide](../../design/frozen-api.md) | Existing family partners and documented transitions; source remains authoritative |
| SRC-FROST-SETS | [Set proposal](../../design/set-algebra.md) | Typed view proposal and representative distinctions; not implemented |
| SRC-FROST-NAMES | [Collection algebra](../../design/collection-algebra.md#naming-dispositions) | Names already selected on main, including SeqMultiMap |
| SRC-FROST-STANDARDS | [CRATES.md](../../../CRATES.md) | Layout, dependencies, test/benchmark metadata, and migration decision gates |

Byte matches for every modified baseline file are recorded in
[baseline evidence](evidence/baseline.json). The main branch was read through
GitHub; the patch uses a verified-file baseline, **not a full newly cloned
checkout**. Current source hashes for read.rs, frozen.rs and integer/key.rs were
checked against the available source snapshot before using its contents.

## Session research and limits

The [source map](source-map.md) and [input inventory](input-inventory.json)
record the supplied reports and prior artifacts. Existing
[corpus sources](../SOURCES.md) retain their original evidence/status. This
refinement does not resolve every opaque citation in raw reports, certify every
historical performance claim, or claim recovery of separately linked files.

The Zanzibar/Leopard and symbolic-engine motivations come from the already
integrated corpus. New computed-set equations here are proposed contracts and
proof arguments, not claims of undocumented Zanzibar internals or implemented
Frost compatibility. No new external PDF was analyzed for this refinement.

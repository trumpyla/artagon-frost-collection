---
id: FROST-RULES-REPOSITORY
status: maintainer-guidance
reviewed_base: 27b14399b0245df2a018fd5eb6f24f47ce7c2dbe
updated: 2026-11-12
implementation: unchanged
---

# Repository rules: short checklist

This is a navigation checklist, not a replacement for [CRATES.md](../../CRATES.md)
or the preserved [implementation contracts](implementation-contracts.md).

1. **Scope and layout.** A Cargo workspace with its packages under `crates/`.
   `frost-collections`, Rust import `frost`, is the library and the only
   supported entry point. It re-exports the internal packages
   `frost-collections-core`, `-integer` and `-concurrent`
   ([crate split plan](../design/crate-split-plan.md)). Do not create empty
   packages or adopt a different crate list by copying historical research.
2. **Code.** Keep features additive and preserve the semantic contracts:
   representatives, borrowed queries, iterator contracts, recovery and panic/drop
   behavior. Public API shape (names, paths, signatures, bounds, type parameters,
   freeze partners) may change in a refactor when the change records its design
   decision, changelog entry, tests and catalog updates. Follow the existing
   unsafe boundary and ownership review rules.
3. **Navigation.** Read [TOOLS.md](../../TOOLS.md); confirm approximate search
   evidence in live source. A failed/missing tool or empty search is not proof.
4. **Tests.** Use independent semantic oracles and regression cases; preserve
   catalog IDs, required features and target coverage. No timing/sleep assertions.
   See [testing](testing.md) and [verification](../verification.md).
5. **Benchmarks.** Validate answers outside timing, retain native controls and
   raw/slower observations, record construction/allocation/drop and host/profile
   boundaries. No universal speedups; see [benchmarks](benchmarks.md).
6. **Dependencies and tools.** Keep verifier packages isolated as CRATES requires;
   use Make orchestration, the pinned toolchain, and feature/embedded checks.
   An optional research model is not a build dependency or proof of production.
7. **Documents.** Follow [documentation rules](documentation-rules.md), update
   related active indexes and names, and preserve historical evidence verbatim.
8. **Delivery.** Review the scoped diff, run applicable gates, record actual
   commands/results/limits, preserve unrelated changes, and distinguish local
   artifact, commit, remote push and accepted design.

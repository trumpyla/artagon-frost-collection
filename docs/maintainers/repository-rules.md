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

1. **Scope and layout.** One root `frost-collections` package, Rust import `frost`,
   remains current. The flat `crates/frost-*` target and `frost-api` facade are
   migration work under `frost-wlt`. Do not create empty packages or adopt a
   different proposed crate list by copying historical research.
2. **Code.** Preserve additive features, public bounds, borrowed queries, fixed
   freeze partners, representatives, iterator contracts, recovery and panic/drop
   behavior. Follow the existing unsafe boundary and ownership review rules.
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

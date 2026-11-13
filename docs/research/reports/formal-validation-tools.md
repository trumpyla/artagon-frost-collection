---
id: frost-research-report-formal-validation-tools
kind: report
level: deep-dive
research_status: curated
implementation_status: adopted
source_validation: verified
input_files: [tests/formal_models/main.rs, docs/design/algebra/catalog.json]
input_members: []
input_sha256: []
origin_created_at: 2026-11-13
imported_at: 2026-11-13
repo_baseline: ad931a6
parent_ids: [frost-research-report-rust-verification-strategy]
related_ids: [frost-research-report-collection-algebra-implementation-notes]
supersedes: []
open_questions: [Q-ALG-1]
---

# Established tools for the formal models

The maintainer asked for established Rust tools in place of the hand-written
generators in `tests/formal_models/common.rs`, and for a view on proof tools
for the 27 algebra laws. Two research runs on 2026-11-13 surveyed testing and
proof tools. Testing tools were compiled and run in a scratch package that
depended on Frost by path, on the pinned 1.99.1 toolchain. **No proof tool was
installed or run**: Kani, Verus, Lean, Z3 and Why3 were absent from the host,
so every proof sketch below is uncompiled.

The maintainer approved all three layers on 2026-11-13; see the
[design decision](../../design/decisions.md#formal-validation-tools-2026-11-13).
The testing tools are dev-dependencies, the Kani refinement harnesses are in
`verification/kani/src/set_laws.rs`, and the Verus proofs are in
`verification/verus`.

## Testing tools

| Tool | Version, release | What it gives | Result | Recommendation |
| --- | --- | --- | --- | --- |
| `exhaust` | 0.2.7, 2026-06-17; MIT/Apache-2.0; MSRV 1.80; `no_std` capable | Every value of a type; derive macro; impls for `Option`, arrays, tuples, `BTreeSet<T>` (all subsets), `BTreeMap<K, V>` (all partial functions) | Ran: 8 subsets of 3 keys; 34 partial bijections as `BTreeMap<K, K>` filtered by injectivity; 512 set triples; 8⁴ operation sequences | Adopt for exhaustive domains |
| `itertools` | 0.15.0, 2026-06-16 | `multi_cartesian_product`, `powerset`, `iproduct!` | Ran: the 15 words of length ≤ 3 over 2 letters | Adopt for words and products (already a transitive dependency of `exhaust`) |
| `proptest` | 1.12.0, 2026-10-10; MSRV 1.88 | Sampled strategies, shrinking, regression persistence; no exhaustive mode | `TestRunner::run` returns `Err(TestError::Fail(_, minimal))` for a wrong definition; the right-representative rule shrank to ≤ 2 elements | Keep (already adopted) for sampled domains |
| `proptest-state-machine` | 0.9.0, 2026-10-10 | Reference state machine against a system under test, with sequence shrinking | Ran: `MutableOrderedSet` and its frozen form against a `BTreeSet` model; a broken system that ignores `Remove` was caught and shrunk to two steps | Adopt for model-based tests of real types |
| `bolero` | 0.13.7, 2026-10-03 | One harness for random, exhaustive, fuzz and Kani engines | Exhaustive mode ran on 0.13.4. 0.13.7 does not resolve with Frost: `bolero-generator-derive` pins `indexmap = "=2.11.2"` against Frost's `^2.14` | Reject for now; revisit when the pin is lifted |
| `quickcheck` | 1.1.0 | Type-driven sampling | Works; duplicates proptest; one generator per type | Reject |
| `arbtest` | 0.3.2, 2024-12 | Time-budgeted sampling | Case count depends on time, weak for catalog evidence | Reject |
| `exhaustigen`, `exhaustive` | 0.1.0 (2021), 0.2.2 (2024) | Exhaustive generation | `exhaustigen`'s `gen` is reserved in edition 2024; both idle | Reject; prefer `exhaust` |
| `rstest`, `test-case` | 0.28.0, 3.4.1 | Compile-time test matrices | One generated test per case multiplies catalog records | Reject for law domains |
| `stateright` | 0.31.0, 2025-07 | Explicit-state model checker | Heavy dependencies; CR-DEP-3 would require an isolated package; `exhaust` over operation arrays covers the need | Reject |
| `shuttle`, `lincheck` | 0.9.10, 0.2.1 | Randomized concurrency, linearizability | Not about sequential laws; `lincheck` is stale | Reject; Loom stays |
| `frunk_laws` and similar | 0.6.0 | Associativity and identity predicates for frunk's traits | No representative pinning, partial operations or image laws; no `Eq`/`Ord`/`Hash` consistency crate was found | Reject |
| `cargo-mutants` | 27.1.0, 2026-06-02 | Syntactic mutants | Mutates only library and binary targets, never `tests/` targets (`should_mutate_target` in its `src/package.rs`) | Keep for `src/`; it cannot replace the named wrong definitions |

The strategy report cites a cargo-mutants "25.3.1 CLI contract"; the current
release is 27.1.0.

### What stays hand-written

The domain enums, the conversions from model values to Frost values, the law
predicates, the named wrong definitions, and the state-machine impls. That is
test content. No generator, enumerator, runner or shrinker remains in the
repository. Bit-mask domains such as `0..1 << n` are the model's own carrier,
not a generator, and can stay where the reference model is bit algebra.

A wrong definition is checked through the tool's own failure result:
`assert!(law(reference) && !law(wrong))` for `exhaust`, a matched
`Err(TestError::Fail(..))` for proptest, and a deliberately broken system under
test for state machines.

## Proof tools

| Tool | Version | Links to existing Rust | Fit | Recommendation |
| --- | --- | --- | --- | --- |
| Kani | 0.68.0, 2026-09-16 | In place; harnesses in an isolated package | Bounded, bit-precise; strongest on checked overflow and integer codes; `BoundedArbitrary` for `BTreeMap`/`BTreeSet` in 0.68 | Expand (already the release gate) |
| Verus | rolling 0.2026.11.x; vstd dated 2026-11-08 | Code verification needs `verus!` or `#[verus_spec]` and a vstd dependency in the production crate | Unbounded SMT. vstd has `Set`, `Map`, `Multiset` (no max-union), `Seq`; pointwise laws are mostly one extensionality assertion. Main pins stable 1.99.1 plus rustc-dev and its own driver | Spec-level proofs now in an isolated package; code verification after `frost-wlt` |
| Creusot | 0.14.0 / `creusot-std` 0.15.0 | In place, Pearlite contracts | Nightly, OCaml and Why3; `creusot-std` is LGPL-2.1-or-later; no multiset type | Deferral stands |
| Aeneas + Charon | no release; active | Translates a safe subset of real Rust to Lean | Only route from real Rust to Mathlib; no unsafe, and support for Frost's generic code is unverified | Later: time-boxed trial on `src/family/ordered/set_ops.rs` |
| Lean 4 + Mathlib | Lean v4.36.0-rc2 | None on Rust | Most laws already proved: `Finmap.union_assoc`, `Finset.image_union`, `Multiset.toFinset_add`, `Relation.comp_assoc`, `Matrix.mul_assoc`, `PEquiv.trans_assoc` | Cite in the chapters; adoption would need an amendment to CR-LAYOUT-10 |
| Prusti, Flux, hax, VeriFast, ESBMC, crux-mir, MIRAI | various | — | Weaker libraries, refinement types only, cryptography focus, unsafe memory proofs, or overlap with Kani | Reject for the laws. VeriFast may suit `buf.rs` later; ESBMC can cross-check Kani |

The Verify the Rust Standard Library challenge accepts Kani, ESBMC, VeriFast,
Flux, KMIR and RAPx, not Verus, Creusot, Prusti, Aeneas or hax.

### Correction to the strategy report

The strategy report says Kani 0.68.0's bundled `nightly-2026-08-21` is below
the 1.99.1 MSRV. That toolchain's rustc reports `1.100.0-nightly (2026-08-20)`.
The claim needs re-checking with a local Kani run before the report is changed.

## Recommended layering

| Layer | Tool | Placement | Covers |
| --- | --- | --- | --- |
| C: finite models and differential tests | `exhaust`, `itertools`, proptest, `proptest-state-machine` | `<package>/tests/formal_models/`, dev-dependencies, `make check` | Every law at small scope; real types against models; streaming, effects and traces |
| B: bounded refinement on real code | Kani | `<package>/verification/kani/`, `make kani` | α(impl op) = spec op(α…) at stated bounds; checked counts over full integer widths; IntKey codes |
| A: unbounded mathematics | Verus, spec level only | `<package>/verification/verus/`, isolated, a `make verus` gate that fails when the tool is absent | The laws themselves over vstd types |

A and B together cover the implementation only within Kani's bounds. Unbounded
results on implementation code need Verus-annotated production code or an
Aeneas translation; both are later work.

## Placement questions for the maintainer

- CR-DEP-3 bars model tools from dev-dependencies. `exhaust` and
  `proptest-state-machine` are test libraries in the sense of CR-DEP-2, since
  CR-LAYOUT-10 makes models test targets. That reading should be recorded.
- Layer A adds Verus to `TOOLS.md` with its own pin. Lean instead of Verus would
  need CR-LAYOUT-10 amended.
- Dev-dependencies do not enter library builds, so `make embedded` and the normal
  dependency graph are unchanged. `Cargo.lock` gains `exhaust`,
  `exhaust-macros`, `itertools` 0.15, `mutants` 0.0.4 and
  `proptest-state-machine` 0.9.

## Unverified

- Verus support for edition 2024, and whether every Verus release pins 1.99.1.
- Kani's handling of `RandomState`, and Kani 0.68 on Frost's edition-2024 code.
- Charon and Aeneas on Frost's trait-generic code.
- Repository commit activity (only release dates were checked).

## Sources

Accessed 2026-11-13. Crate facts from `https://crates.io/api/v1/crates/<name>`
and the published crate sources. bolero's pin from the crates.io index entries
for `bolero-generator-derive`, reproduced by a failed resolution. cargo-mutants
target kinds from `src/package.rs` and `src/visit.rs` on its main branch.
Kani from its releases page, `rust-toolchain.toml` at `kani-0.68.0`, and
`library/kani/src/bounded_arbitrary.rs`. Toolchain versions from
`static.rust-lang.org/dist/2026-08-21/channel-rust-nightly.toml`. Verus from its
releases page, `rust-toolchain.toml`, and `source/vstd/{set,set_lib,multiset,map_lib,seq_lib,relations}.rs`.
Creusot from its releases and `creusot-std` sources. Aeneas and Charon from
their READMEs and toolchain files. Mathlib lemma names from Mathlib master
`Data/Finmap.lean`, `Data/Multiset/UnionInter.lean`, `Data/Finset/Image.lean`,
`Logic/Relation.lean`, `Basic/Rel.lean`, `Basic/PEquiv.lean`, and
`Data/Matrix/Mul.lean`. The verify-rust-std tool list from its `doc/src/tools.md`.

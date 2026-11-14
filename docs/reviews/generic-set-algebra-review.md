# Review: generalizing set algebra beyond integers

Review date: **2026-11-08, America/New_York**. Reviewed input: the supplied
`frost-collections-repo(1).zip`, at imported commit `3405657`.

Outcome: recommend the [general set-algebra design](../design/set-algebra.md).
This change adds design documents and an independent finite semantic model.
**It does not implement a generic view API, change production kernels, or
introduce a new collection backend.**

## 1. Review scope

Inspected the set families, exact wrappers, read capabilities, backend adapters,
integer profile, set tests, and project contracts. Read the supplied lazy-views
research and the subsequent source, performance, and adversarial audits, together
with the integer expression and persistent storage proposals.

The current frozen ordinary families already support strings, custom records,
and other lawful noninteger members. No wrong-answer bug in those existing
binary kernels was demonstrated. The findings concern coverage gaps and risks
in broadening the proposed integer expression design.

## 2. Findings and disposition

| Priority | Finding | Evidence | Design disposition |
| --- | --- | --- | --- |
| High | Existing generic algebra is confined to the two frozen families | `family/hash_set.rs`, `family/ordered/frozen.rs`, `family/sets.rs` | Add an explicit mixed-backend view for mutable, frozen, and supported native/wrapped sets |
| High | Integer candidate covers preserve membership but do not select generic stored representatives | Existing left-retention contract; independent labelled-object counterexample | Separate candidate selection and representative provenance |
| High | Flattening generic XOR changes representative metadata | Three equal keys labelled A/B/C; repeated A cancellation case | Preserve original provenance grouping; permit parity rewrites only for compatible terminals |
| High | A universal `SetGet` bound excludes imbl hash sets | `backends/persistent.rs` explicitly omits the capability | Use left traversal plus right membership as the baseline; retrieval enables optional optimizations |
| High | FST streams cannot provide ordinary stable `&T` iteration | `backends/fst.rs`; native lending signature | Keep a stream/value boundary and make owned key copies explicit |
| High | A universal borrowed trait object cannot retain all existing capabilities | `read.rs` uses GAT iterators; Rust dyn-compatibility rules | Typed views first; runtime planner with a deliberately fixed query domain later |
| Medium | Unordered lazy n-ary union may repeatedly query earlier operands | Recursive baseline analysis; disjoint-source probe sum | Separate n-ary strategies and explicit memory/work tradeoffs |
| Medium | `Elements + SetContains` does not promise unique, coherent, stable set contents | Independent capabilities; live concurrent adapters | Add an explicit safe semantic opt-in; exclude uncoordinated live inputs |
| Medium | Blanket `.union` methods conflict with existing inherent methods | Current same-family method signatures | Use `frost::set::view` or explicit module functions |
| Medium | Sorted, insertion-ordered, and unordered traversal are distinct | Existing sorted kernels and IndexSet/PHF tests | New generic view order is unspecified; sorted execution needs a separate capability |
| Medium | Borrowed materialization cannot produce owned non-Clone T automatically | Ownership of the existing borrowed iterators | Explicit collection of references, cloning, or a separate consuming operation |
| Medium | Cheap Arc cloning does not establish cheap updates or non-Clone persistence | Existing persistent proposal and Arc ownership rules | Keep persistence separate and choose an explicit element-storage model |

The second independent review considered both the current API and the supplied
research. The final proposal incorporates provenance-preserving rewrites,
operation-specific bounds, the distinction between static and runtime queries,
and the limits of a generic fallback. Any remaining API spellings are proposals
subject to compile-time prototyping, not claims of tested callable methods.

The second review also tightened the set-source law to require finite, complete,
independently restartable traversal before the first `None`. It added explicit
interval-bound propagation, wide-to-usize hint conversion, predicate derivations,
and release gates for non-fused source iterators and synthetic count overflow.

## 3. How this iterates the supplied research

Retain the separation between deferred queries, materialized results, internal
evaluation laziness, and persistent ownership. Retain existing specialized
integer kernels and result policies. The existing repository has already
corrected the report's earlier global mixed-to-Sorted output behavior; that
historical description is not the starting point for a new performance claim.

Broaden the semantic contract to arbitrary objects before designing another
storage engine. Generic values can have metadata ignored by equality, expensive
comparisons or hashes, borrowed queries, and backend-imposed ownership bounds.
Their algebra is not equivalent to operating solely on canonical integer codes.

The main revised sequence is therefore: generic borrowed contract and coverage;
measured terminal-specific kernels; explicit value/lending/runtime profiles;
persistent storage evaluated independently. A new in-house Roaring implementation
is not a prerequisite for ordinary generic set algebra.

The supplied report is preserved unchanged as historical input. This pass did
not repeat its external paper measurements or re-audit every recent upstream
issue status. Sources actually used for new factual claims are listed in the
[design](../design/set-algebra.md).

## 4. Existing executable contracts

| Contract | Current tests |
| --- | --- |
| Non-Clone objects and left representatives | `ordered_set_ops::overlap_keeps_the_left_representative_without_cloning_elements`; `hash_set_ops::representative_retention_survives_smaller_right_scans_and_collisions` |
| Nonstatic element lifetimes | `ordered_set_ops::keys_may_borrow_nonstatic_data` |
| Lazy iterator construction and inspection do not probe membership | `hash_set_ops::creating_cloning_and_inspecting_iterators_does_not_probe_membership` |
| Traversal-only operand need not have a usable hasher | `hash_set_ops::an_operand_only_traversed_does_not_need_a_hasher` |
| Partial traversal and remaining size hints | `ordered_set_ops`, `hash_set_ops`, and exhaustive six-member subset pairs in `exhaustive_collections` |
| Fixed mutable/frozen pairs retain representatives and hashers | `mutable_sets::ordered_set_keeps_first_rep_and_its_frozen_pair`; `mutable_sets::hash_set_keeps_first_rep_and_the_exact_hasher` |
| Insertion/declaration order is not sorted order | `specialized::indexset_compact_retains_positions`; `specialized::static_maps_and_sets_preserve_their_query_and_order_contracts` |
| FST keys use lending traversal | `specialized::fst_lookup_decodes_values_and_streams_lend_keys` |

The source files are under [tests](../../crates/frost-collections-core/tests/). These tests constrain future
work; they do not exercise a new expression layer.

## 5. Baseline verification actually executed

Toolchain:

```text
rustc 1.99.1 (58cf8f9c3 2026-10-13)
cargo 1.99.1 (5f94df478 2026-08-27)
```

Command from the repository root:

```sh
PATH=/home/agent/.cargo/bin:$PATH bash scripts/verify.sh check
```

The direct `./scripts/verify.sh check` invocation initially could not execute
because extraction left script executable bits unset. Invoking Bash avoided
altering the existing working-tree mode entries. No production source, test,
dependency, or toolchain file was changed to get the check to pass.

| Check | Result |
| --- | --- |
| Formatting | Passed |
| Repository-local documentation links before this design addition | 351 verified |
| All-target/all-feature Clippy with `-D warnings` | Passed |
| All-feature rustdoc with `-D warnings` | Passed |
| Unit and integration tests | 243 passed |
| Doctests | 58 passed: 33 ordinary and 25 compile-fail |
| Example tests | 11 distinct passed; the script executes them twice |
| Distinct successful tests | **312** |
| Successful test executions including repeated examples | **323** |

The [full baseline log](../../verification/set-algebra/baseline-check.log) is
retained. The combined source/test SHA-256 recorded before and after the check
was `4ac9ebd11c36d377d380f207ddcc20ac1608ec4ffe7cde85c8c6605caeedacfd`.

After documentation changes, the local-link checker verified **383** local links
and the copied semantics oracle passed again. All **118** original source,
test, example, benchmark, script, and principal configuration files were also
compared against the input archive and were byte-identical. Scope is recorded in the
[model/evidence README](../../verification/set-algebra/README.md).

This was the repository's `check` profile, not the complete release matrix,
embedded verification, Miri, or a new performance comparison. No runtime API
was implemented, so there is no claimed validation of proposed view signatures,
Rust borrowing behavior, planner execution, or new backend support.

## 6. Independent design model

The [model source](../../verification/set-algebra/set_semantics_oracle.py)
implements a logical key-to-labelled-representative oracle independently from
a recursive lazy enumeration model. It imports no Frost code. The
[raw result](../../verification/set-algebra/results.json) records:

- Eleven rewrite laws over 88 one-key presence combinations.
- 1,375 two-key ordered-input rewrite cases.
- 24,000 expression/input combinations checking membership, representatives,
  unique enumeration, and lookup agreement.
- 1,500 union/intersection association cases checking encounter order.
- Explicit candidate-cover and individually-sorted-input counterexamples.

It demonstrates why membership, provenance, and order require separate
contracts. It does not prove Rust soundness, arbitrary expression depth,
allocation bounds, concurrent behavior, or performance. The finite model's
encounter-order baseline is deliberately stronger than the proposed general
view's unspecified output-order contract; the model still helps identify which
rewrites would be invalid for a future ordered profile.

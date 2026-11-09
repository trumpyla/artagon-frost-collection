# Generic set algebra: design evidence

This directory contains an independent finite semantics model and the executed
baseline verification log for the [general set-algebra design](../../docs/design/set-algebra.md).
It does not implement or test a production generic expression API.

## Reproduce the model

From the repository root:

```sh
python3 verification/set-algebra/set_semantics_oracle.py
```

The standard-library-only script writes `results.json` next to itself. Logical
keys determine membership while source-labelled objects make representative
choice observable. Input membership is fixed and lookup agrees with unique
traversal. The reference denotation uses explicit set operations; the recursive
enumerator uses left filters and disjoint concatenation without intermediate
result collections.

## Executed scope

| Check | Cases |
| --- | ---: |
| Rewrite laws | 11 |
| One-key presence cases across those laws | 88 |
| Two-key ordered-input rewrite cases | 1,375 |
| Expression/input combinations | 24,000 |
| Union/intersection association order checks | 1,500 |

The 24,000 cases combine all five ordered subsets of a two-key universe for each
of three sources, all six source permutations, both binary tree shapes, and all
four operations at each internal node. Separate rewrite cases include repeated
leaves. Results confirm the XOR provenance, nonlocal cancellation, distribution,
candidate-cover, and sorted-order counterexamples described in the design.

The model is finite and written in Python. It does not check Rust signatures,
lifetimes, GAT coherence, ownership, arbitrary plan depth, byte streams, encoded
integer domains, allocations, or concurrency. The proposed generic view's
unspecified order also permits other correct enumerations than this baseline.

## Existing implementation verification

`baseline-check.log` records the successful Rust 1.99.1 command:

```sh
bash scripts/verify.sh check
```

Formatting, strict Clippy, strict rustdoc, 351 pre-change local documentation
links, 243 unit/integration tests, 58 doctests, and 11 distinct example tests
passed. Examples are executed twice by the script: 312 distinct tests and 323
test executions passed. These results validate existing code only.

After documentation changes, the link checker verified **383** repository-local
links, and the copied semantics oracle passed again with the counts above.
All **118** original source, test, example, benchmark, script, and principal
configuration files were compared against the input archive and were
byte-identical. No benchmark was rerun and no proposed Rust API was implemented
in this pass.

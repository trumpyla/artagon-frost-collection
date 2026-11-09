# Integer expression bounds model

This directory contains an independent executable review of the proposed
cardinality-bound formulas for borrowed integer expressions. It does not
implement or exercise a production view API, and it is not part of the main
release gate.

Run from the project root with ordinary Python 3:

```sh
python3 verification/integer-views/check_bounds.py
```

The model uses only Python's standard library. Do not use `python -O`: assertions
perform the checks, and the script rejects optimization mode.

## Domain and assertions

Each operand is a subset of a common universe of size `U`. Cardinality
intervals satisfy `0 <= lower <= upper <= U`. For every `U` from 0 through 6,
the script enumerates every subset pair and every interval enclosing each
operand's actual cardinality. It checks that the proposed union, intersection,
difference, and XOR bounds enclose the result.

It also records observed extrema for every pair of cardinality intervals and
checks that both endpoints of each proposed bound are attainable. This is
individual tightness when only the two independent cardinality intervals are
known. Additional information, such as repeated operands or support ranges,
can make stronger bounds possible.

For nonempty operand pairs whose minimum-to-maximum support ranges are
disjoint, the script separately checks that intersection is empty, difference
equals the left input, and union/XOR cardinalities equal the sum.
It also checks the stronger interval formulas against every enclosing
cardinality interval for those pairs: intersection `[0, 0]`, difference
`[la, ua]`, and union/XOR `[la + lb, min(U, ua + ub)]`. The upper cap remains
necessary because loose cardinality upper bounds can sum above U even when
the actual support ranges are disjoint.

The recorded run covers:

| Check | Cases |
| --- | ---: |
| Subset pairs, summed over `U = 0..6` | 5,461 |
| Subset pair plus enclosing cardinality intervals | 1,003,621 |
| Disjoint nonempty support-range pairs | 402 |
| Disjoint pair plus enclosing cardinality intervals | 63,404 |
| Selected wide-universe interval pairs | 1,568 |

## Arithmetic scope

The wide checks use `U = 2^32` and `U = 2^64`. Interval endpoints come from
`{0, 1, U/2 - 1, U/2, U/2 + 1, U - 1, U}`. For all interval pairs over
those points, the script checks bound ordering, range, realizable extreme
overlap counts at interval endpoints, and that the binary sums and doubled
universe fit `u128`.

Python uses arbitrary-precision integers. These checks do not execute Rust
arithmetic or prove its implementation overflow-free. In an unsigned Rust
implementation, expressions such as `max(0, la - ub)` require
`la.saturating_sub(ub)`; applying `max` after a raw subtraction is too late.
Binary intermediates are at most `2U = 2^65` under the stated preconditions.

Using the primitive encoded universe for a restricted key domain is a safe,
possibly loose cap. The model does not exercise codecs, cursor state,
partially consumed iterator hints, planning estimates, concurrency, allocation,
or materialization.

## Evidence and limits

[bounds-check.log](bounds-check.log) records the copied script's output.
[status.json](status.json) records its command, exit status, timestamps, Python
version, coverage counts, and SHA-256 digests of the script and log. The log's
timing is not a performance benchmark.

This is exhaustive finite testing plus selected wide arithmetic checks. It is
not a symbolic proof, a verification of an unimplemented evaluator, or evidence
for upstream dependency behavior.

See the [performance audit](../../docs/reviews/lazy-views-performance-audit.md)
for the proposed view's cost model and semantic boundaries.

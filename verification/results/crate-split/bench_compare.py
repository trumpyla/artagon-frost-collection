#!/usr/bin/env python3
"""Summarise the benchmark comparison recorded for the crate split.

Evidence tool for the crate split (frost-wlt.6). It is not a Make gate.

usage: bench_compare.py [<benchmarks directory>]

The directory defaults to final/benchmarks next to this file. It holds two
kinds of raw observations.

Same-binary runs of `make integer-ab` (CSV with a `case` and an `old_over_new`
column): `integer-ab.*.csv` compares the revision before the split with the
split tree; `integer-aa.*.csv` compares the split tree with an identical copy
of itself, which measures what the harness reports when nothing differs.

Separate-binary runs of the bench targets: `separate/pre-split/<bench>.csv` and
`separate/split/<bench>.csv`, as printed by the targets (a `case` and a
`median_ns` column). Cases whose name marks a native control (the standard
library, native Roaring, raw slices) run code the split did not touch, so
their spread is the noise of comparing two binaries.

Every ratio is old over new: above 1 the split tree is faster.
"""
import csv
import math
import os
import re
import statistics
import sys

BENCHES = [
    "set_operations",
    "performance_review",
    "ordered_sets",
    "adaptive_maps",
    "integer_sets",
    "integer_output",
    "snapshots",
]
CONTROL = re.compile(r"(^|[._:])(std|btree|raw|native)[a-z_]*([._]|$)")


def summary(label, ratios):
    if not ratios:
        print("%-34s no cases" % label)
        return
    ordered = sorted(ratios)
    count = len(ordered)
    beyond = sum(1 for ratio in ordered if ratio < 1 / 1.1 or ratio > 1.1)
    print(
        "%-34s n=%4d geomean=%.3f median=%.3f p5=%.3f p95=%.3f min=%.3f max=%.3f beyond10%%=%d (%.0f%%)"
        % (
            label,
            count,
            math.exp(sum(math.log(ratio) for ratio in ordered) / count),
            statistics.median(ordered),
            ordered[int(count * 0.05)],
            ordered[min(count - 1, int(count * 0.95))],
            ordered[0],
            ordered[-1],
            beyond,
            100.0 * beyond / count,
        )
    )


def same_binary(path):
    with open(path, encoding="utf-8", newline="") as handle:
        return {row["case"]: float(row["old_over_new"]) for row in csv.DictReader(handle)}


def medians(path):
    result = {}
    header = None
    with open(path, encoding="utf-8", newline="") as handle:
        for row in csv.reader(handle):
            if row and row[0] == "case":
                header = row
            elif header and len(row) == len(header):
                try:
                    result[row[0]] = float(row[header.index("median_ns")])
                except ValueError:
                    pass
    return result


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    root = sys.argv[1] if len(sys.argv) > 1 else os.path.join(here, "final", "benchmarks")

    print("Same binary (make integer-ab), ratio old/new")
    before = same_binary(os.path.join(root, "integer-ab.pre-split-vs-split.3-rounds.csv"))
    control = same_binary(os.path.join(root, "integer-aa.split-vs-split.3-rounds.csv"))
    summary("pre-split vs split", list(before.values()))
    summary("split vs identical copy (control)", list(control.values()))
    summary(
        "first divided by control",
        [before[case] / control[case] for case in before if case in control],
    )
    again = same_binary(os.path.join(root, "integer-ab.outliers.9-rounds.csv"))
    again_control = same_binary(os.path.join(root, "integer-aa.outliers.9-rounds.csv"))
    outliers = [case for case in again if case in again_control]
    print()
    print("Cases beyond 10% in the first run, nine rounds: pre-split vs split, control")
    for case in sorted(outliers, key=lambda name: again[name]):
        print("  %-62s %.3f  %.3f" % (case, again[case], again_control[case]))

    print()
    print("Separate binaries (bench targets), ratio pre-split/split of median_ns")
    controls = []
    measured = []
    for bench in BENCHES:
        old = medians(os.path.join(root, "separate", "pre-split", bench + ".csv"))
        new = medians(os.path.join(root, "separate", "split", bench + ".csv"))
        missing = sorted(set(old) ^ set(new))
        if missing:
            print("%s: %d case(s) on one side only, first %s" % (bench, len(missing), missing[0]))
        ratios = {case: old[case] / new[case] for case in old if case in new and new[case] > 0}
        summary(bench, list(ratios.values()))
        for case, ratio in ratios.items():
            (controls if CONTROL.search(case) else measured).append(ratio)
    print()
    summary("native controls, all targets", controls)
    summary("Frost cases, all targets", measured)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Summarize two counterbalanced seven-sample runs without discarding samples."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
import statistics


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path, nargs="?", default=Path(__file__).resolve().parent)
    parser.add_argument("--case", help="case group selected by compare.py, when applicable")
    options = parser.parse_args()
    suffix = f"-{options.case}" if options.case else ""
    rows = []
    for profile in ("baseline", "v3"):
        data = {}
        for version in ("before", "after"):
            for repeat in (1, 2):
                path = options.directory / f"{version}-{profile}-bench{suffix}-{repeat}.csv"
                with path.open() as source:
                    data[version, repeat] = {row["case"]: row for row in csv.DictReader(source)}
        cases = list(data["before", 1])
        if not cases or any(set(values) != set(cases) for values in data.values()):
            raise SystemExit(f"Missing or mismatched cases in the {profile} comparison")
        for case in cases:
            result = {"profile": profile, "case": case}
            for version in ("before", "after"):
                samples = []
                for repeat in (1, 2):
                    source = data[version, repeat][case]
                    result[f"{version}_run_{repeat}_median_ns"] = float(source["median_ns"])
                    samples.extend(float(source[f"sample_{index}_ns"]) for index in range(1, 8))
                result[f"{version}_pooled_median_ns"] = statistics.median(samples)
            result["after_over_before"] = result["after_pooled_median_ns"] / result["before_pooled_median_ns"]
            rows.append(result)
    with (options.directory / f"comparison{suffix}.csv").open("w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    (options.directory / f"comparison{suffix}.json").write_text(json.dumps(rows, indent=2) + "\n")
    print(f"Summarized {len(rows)} profile/case rows; raw samples are unchanged")


if __name__ == "__main__":
    main()

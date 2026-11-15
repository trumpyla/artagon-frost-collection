# Integer and family optimizations: evidence

This folder holds the comparison harness and the raw results for the series
recorded in the
[validation record](../../crates/frost-collections-integer/docs/integer-optimization-validation.md). Read that
record first; it states what the figures support and what they do not.

## Scope

The harness is a timing comparison, not a proof and not a correctness oracle
for the crate. Each case checks its answer against the standard library before
it is timed, with one fixed dataset per case. The repository's tests carry the
correctness obligations.

- Domain: the datasets and operations in `ab-harness/src/main.rs`. Six general
  datasets under five policies, plus directed series for construction,
  traversal, small updates, identity shortcuts, Dense kernels, Roaring
  construction, ordered cursors, and the non-integer families.
- Production code executed: the public API of the baseline revision and of the
  working tree, both with the `int-set` and `std` features.
- Not covered: other features, other targets, concurrency, allocation counts,
  peak memory, and any host other than the one in
  [environment.json](environment.json).

## Layout

| Path | Content |
| --- | --- |
| `ab-harness/` | Isolated package with its own manifest and lockfile. It is excluded from the published crate with the rest of `verification/`. |
| `ab-harness/prepare-baseline.sh` | Archives a revision into `ab-harness/baseline-workspace/` with its packages renamed `frost-baseline*`. A workspace revision is extracted whole; a revision from before the workspace layout is extracted as the facade package alone. The folder is not tracked. The committed lockfile matches a workspace baseline; for an older one run `make integer-ab AB_LOCKED=`. |
| `results/ab-main-vs-90f2a4f-run{1,2}.csv` | Final runs: `main` at `22697a8` against the final tree, nine rounds. |
| `results/aa-82f4d5c-vs-82f4d5c.csv` | Control: one revision against itself, nine rounds. |
| `results/at-82f4d5c/` | The two full runs before the score field was narrowed. |
| `results/superseded-ce24b6c/` | Runs and control for two designs that measured slower and were withdrawn or replaced. |
| `results/review-differential-on-branch.log` | Output of the independent differential suite on the final commit. The suite itself is not in the repository. |

CSV columns: `case, old_min_ns, new_min_ns, old_over_new, old_repr, new_repr`.
A time is the minimum over the rounds of the median of five samples. The
representation columns show the layout each side selected, where a case has one.

## Run

```sh
make integer-ab AB_BASELINE=22697a8 AB_ROUNDS=9 > run.csv
```

`AB_FILTER=construct,fold` in the environment restricts the cases by substring.
Pin the process to one CPU and run nothing else. For a control, pass the
revision under test as `AB_BASELINE`.

## Limits found while using it

- The control gives ratios from 0.62 to 1.72 for identical source. Do not read
  a single ratio below about 1.5 as a change.
- Absolute times for the same baseline case differ by up to a factor of two
  between executables built from different candidate trees.
- `git archive` writes commit times. Before the script used `tar -m`, Cargo
  kept a newer baseline build when the script switched to an older revision.
  Runs taken in that state were discarded.

# Reproducing the initial mixed-probe measurements

These are complete source files for the initial implementation measured in
`../mixed-probe-before-1.csv` and `../mixed-probe-before-2.csv`. They preserve the
original smaller-cardinality probing rule and the 195-case benchmark.

- `storage-before-mixed-probe.rs` replaces `src/integer/storage.rs`.
- `integer-sets-before-mixed-probe.rs` replaces `benches/integer_sets.rs`.

Make those substitutions in a separate copy of the delivered source tree, then
use its pinned compiler, lockfile, and the same build command:

```sh
cargo bench --locked --features int-set --bench integer_sets --no-run
```

Cargo prints the resulting executable path. Run that executable serially with
CPU affinity as recorded in `../mixed-probe-before-execution.json`. The original
machine and compilation environment are recorded in `../environment.json`.

Both baseline file hashes match the hashes recorded immediately after the
original executions. `manifest.json` records these hashes and the originally
measured executable hash. Rebuilding on another host need not reproduce binary
identity or timing results; these files make the measured source reproducible.

The current benchmark adds 18 skewed mixed-pair cases. Those cases have no
initial counterpart in the two 195-case baseline CSVs. Before/after comparisons
must match case names, not row positions or overall case counts.

# Independent result-policy oracle

`policy-oracle.log` records **507,840 passing checks** against the merged integer
implementation. `policy-oracle.rs` is the exact program executed. This is a
finite correctness matrix, not a timing benchmark or a proof of every possible
input.

The matrix covers all 16 Sorted/Dense/Roaring/Chunked operand pairs, all four
operations, and five output policies (the four forced representations plus
Auto). It varies Dense caps over 0, 8, 16, and 1,024 bytes and Roaring caps over
0, 1, and 2 upper-32-bit partitions. Selected inputs include empties, identical
contents, local and multiple-prefix shapes, maximum u64 codes, and run-count
boundaries. Ineligible forced input representations are skipped explicitly.

For every result policy, the program checks either the exact expected policy
rejection or the returned values, logical equality, retained limits, permitted
representation, and full canonical Hasher event stream against a Sorted oracle.
The expected members come from the standard BTreeSet operations.

## Executed commands and build provenance

These commands were run from the main project directory with Rust 1.99.1:

```bash
export PATH=/home/agent/.cargo/bin:$PATH
rustc --edition=2024 /workspace/scratch/50bb50b82113/storage-review/policy_oracle.rs \
  --extern frost=/workspace/scratch/50bb50b82113/frost-collections/target/debug/deps/libfrost-9d7d95afdb961a00.rlib \
  -L dependency=/workspace/scratch/50bb50b82113/frost-collections/target/debug/deps \
  -o /workspace/scratch/50bb50b82113/storage-review/policy_oracle
/workspace/scratch/50bb50b82113/storage-review/policy_oracle \
  > /workspace/scratch/50bb50b82113/storage-review/policy_oracle.log 2>&1
```

The rlib came from the integration's existing all-features debug build; the
oracle did not run another Cargo build or alter production files. The relevant
integer logic files predate the oracle executable. The same rlib path was later
rebuilt for documentation changes, so no hash of that later artifact is presented
as the exact artifact linked by the oracle. `policy-oracle-provenance.json`
records the logical-source hashes reviewed, timestamps, and evidence boundaries.

## Portable rerun

An isolated manifest is included for a future rerun from the repository root:

```bash
cargo run --manifest-path verification/revision2-integration/policy-oracle/Cargo.toml
```

This manifest was added after the recorded execution and has **not** itself been
executed in this review. Its first run creates its own lockfile; normal crate
builds do not include this verification package. The recorded run instead used
the main integration's existing locked all-features dependency build.

The oracle does not test arbitrary custom codecs, allocator failure, all machine
word sizes, or inaccessible cardinalities. Separate main-suite and embedded
evidence applies to those executed domains. No time-based assertions are used.

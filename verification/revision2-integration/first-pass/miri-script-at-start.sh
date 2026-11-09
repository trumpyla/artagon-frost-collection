#!/usr/bin/env bash
# Fail-fast, composable verification. Normal commands use rust-toolchain.toml.
set -euo pipefail
cd "$(dirname "$0")/.."

frost_mode="${1:-all}"
if [[ $# -gt 1 ]]; then
  printf 'Usage: %s [check|profiles|features|integers|examples|package|embedded|miri|all]\n' "$0" >&2
  exit 2
fi

metadata() {
  cargo metadata --format-version 1 --no-deps --locked
}

check() {
  cargo fmt --all --check
  python3 scripts/check-doc-links.py
  cargo clippy --all-targets --all-features --locked -- -D warnings
  RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --locked
  cargo test --all-features --locked
  cargo test --examples --all-features --locked
}

profiles() {
  cargo clippy --lib --no-default-features --locked -- -D warnings
  cargo test --no-default-features --locked
  RUSTDOCFLAGS="-D warnings" cargo doc --no-default-features --no-deps --locked
  cargo clippy --lib --no-default-features --features alloc --locked -- -D warnings
  cargo test --no-default-features --features alloc --locked
  RUSTDOCFLAGS="-D warnings" cargo doc --no-default-features --features alloc --no-deps --locked
  cargo test --locked
}

features() {
  local frost_metadata frost_features frost_feature
  frost_metadata="$(metadata)"
  frost_features="$(python3 -c '
import json, sys
package = next(p for p in json.load(sys.stdin)["packages"] if p["name"] == "frost-collections")
for name in sorted(package["features"]):
    if name not in {"default", "alloc", "std", "advanced", "concurrent", "frameworks"}:
        print(name)
' <<< "$frost_metadata")"
  while IFS= read -r frost_feature; do
    printf '\nChecking isolated feature: %s\n' "$frost_feature"
    cargo clippy --lib --no-default-features --features "$frost_feature" --locked -- -D warnings
    cargo test --no-default-features --features "$frost_feature" --locked
    RUSTDOCFLAGS="-D warnings" cargo doc --no-default-features --features "$frost_feature" --no-deps --locked
  done <<< "$frost_features"
}

examples() {
  local frost_metadata frost_examples frost_example
  frost_metadata="$(metadata)"
  frost_examples="$(python3 -c '
import json, sys
package = next(p for p in json.load(sys.stdin)["packages"] if p["name"] == "frost-collections")
for target in package["targets"]:
    if "example" in target["kind"]:
        print(target["name"])
' <<< "$frost_metadata")"
  while IFS= read -r frost_example; do
    cargo run --example "$frost_example" --all-features --locked
  done <<< "$frost_examples"
}

integers() {
  cargo test --locked --no-default-features --features int-set --test integer_sets --test integer_output
  cargo test --locked --release --features int-set,arc-swap-1 --test integer_sets --test integer_output
  cargo test --locked --manifest-path verification/int-key-downstream/Cargo.toml
}

package() {
  local frost_metadata frost_package_manifest
  frost_metadata="$(metadata)"
  frost_package_manifest="$(python3 -c '
import json, pathlib, sys
data = json.load(sys.stdin)
p = next(p for p in data["packages"] if p["name"] == "frost-collections")
print(pathlib.Path(data["target_directory"]) / "package" / (p["name"] + "-" + p["version"]) / "Cargo.toml")
' <<< "$frost_metadata")"
  cargo package --locked --all-features --allow-dirty
  cargo test --manifest-path "$frost_package_manifest" --all-features --locked
  cargo test --manifest-path "$frost_package_manifest" --examples --all-features --locked
}

embedded() {
  cargo build --lib --locked --no-default-features --features int-set --target thumbv6m-none-eabi
  cargo build --lib --locked --no-default-features --features int-set --target thumbv7em-none-eabihf
  cargo build --lib --locked --no-default-features --target thumbv7em-none-eabihf
  cargo build --lib --locked --no-default-features \
    --features alloc,hashbrown-0_17,indexmap-2,smallvec-1,arrayvec-0_7,fixedbitset-0_5,slotmap-1,phf-0_14,serde-1,borsh-1,schemars-1 \
    --target thumbv7em-none-eabihf
}

miri() {
  local frost_nightly="${FROST_MIRI_TOOLCHAIN:-nightly-2026-11-08}"
  cargo +"$frost_nightly" test --doc --all-features --locked
  MIRIFLAGS="-Zmiri-strict-provenance" \
    cargo +"$frost_nightly" miri test --locked --test inline_safety --test families --test core_review -- --skip under_random_ops
  MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-strict-provenance" \
    cargo +"$frost_nightly" miri test --locked --test inline_safety
  MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-strict-provenance" \
    cargo +"$frost_nightly" miri test --locked --test exhaustive_collections inline_ownership_exhausts_three_operation_traces
  MIRIFLAGS="-Zmiri-strict-provenance" \
    cargo +"$frost_nightly" miri test --locked --no-default-features --features int-set \
      --target i686-unknown-linux-gnu --test integer_sets dense_ -- --test-threads=1
  MIRIFLAGS="-Zmiri-strict-provenance" \
    cargo +"$frost_nightly" miri test --locked --no-default-features --features int-set \
      --target i686-unknown-linux-gnu --lib integer::storage::word::tests -- --test-threads=1
  MIRIFLAGS="-Zmiri-strict-provenance" \
    cargo +"$frost_nightly" miri test --locked --no-default-features --features int-set \
      --target i686-unknown-linux-gnu --lib integer::storage::tests -- --test-threads=1
  # New shared-storage kernels: bounded cursor/run cases on a 32-bit target.
  local frost_storage_case
  for frost_storage_case in \
    integer::chunked::tests::partial_fold_resumes_near_chunk_ends_without_repeating_next_chunk \
    integer::chunked::tests::full_chunk_runs_and_early_break_are_bounded \
    integer::chunked::tests::unchanged_overlapping_containers_keep_their_allocations \
    integer::chunked::tests::dense_promotion_splits_windows_on_container_boundaries \
    integer::dense::tests::deleting_from_an_empty_window_is_a_no_op \
    integer::dense::tests::dense_runs_preserve_word_edges_and_the_maximum_code; do
    MIRIFLAGS="-Zmiri-strict-provenance" \
      cargo +"$frost_nightly" miri test --locked --no-default-features --features int-set \
        --target i686-unknown-linux-gnu --lib "$frost_storage_case" \
        -- --exact --test-threads=1
  done
  local frost_integer_case
  for frost_integer_case in canonical_hash_events_match_across_representations_and_boundaries \
    canonical_hash_scout_thresholds_are_representation_independent \
    equality_and_hash_ignore_dense_padding_and_construction_history \
    empty_operands_and_identity_shortcuts_do_not_bypass_limits \
    output_policy_overrides_are_checked_and_retained; do
    MIRIFLAGS="-Zmiri-strict-provenance" \
      cargo +"$frost_nightly" miri test --locked --no-default-features --features int-set \
        --target i686-unknown-linux-gnu --test integer_output "$frost_integer_case" \
        -- --exact --test-threads=1
  done
}

case "$frost_mode" in
  check|profiles|features|integers|examples|package|embedded|miri) "$frost_mode" ;;
  all)
    check
    profiles
    features
    integers
    examples
    package
    if [[ "${FROST_NO_STD:-0}" == 1 ]]; then embedded; fi
    if [[ "${FROST_MIRI:-0}" == 1 ]]; then miri; fi
    ;;
  *) printf 'Unknown verification scope: %s\n' "$frost_mode" >&2; exit 2 ;;
esac
printf '\nVerification scope %s passed.\n' "$frost_mode"

#!/usr/bin/env bash
# Runs a test command and fails when a selected test binary executed no test.
#
# A lane that selects tests by name exits 0 when the name matches nothing, so
# a rename or a move can empty the lane without failing it. Every
# "test result:" line of the command must report at least one passed test.
# With --total only the sum must be positive, for commands whose package has a
# test binary that is legitimately empty.
#
#   scripts/require-tests.sh cargo test --lib some::module::tests
#   scripts/require-tests.sh --total cargo test --manifest-path other/Cargo.toml
set -euo pipefail

mode=each
if [ "${1:-}" = "--total" ]; then
  mode=total
  shift
fi
if [ "$#" -eq 0 ]; then
  echo "usage: $0 [--total] <test command>..." >&2
  exit 2
fi

log=$(mktemp)
trap 'rm -f "$log"' EXIT

"$@" 2>&1 | tee "$log"

counts=$(sed -n -E 's/^test result: ok\. ([0-9]+) passed;.*/\1/p' "$log")
total=0
empty=0
for count in $counts; do
  total=$((total + count))
  if [ "$count" -eq 0 ]; then
    empty=$((empty + 1))
  fi
done
if [ "$total" -eq 0 ] || { [ "$mode" = each ] && [ "$empty" -ne 0 ]; }; then
  echo "error: a selected test binary executed no test: $*" >&2
  exit 1
fi

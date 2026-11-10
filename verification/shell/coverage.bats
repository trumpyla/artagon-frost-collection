#!/usr/bin/env bats
# Target: Make recipes under Bash 3.2+, macOS/Linux with jq.
# Cargo is replaced by an inert fixture; these tests exercise gate decisions,
# not LLVM instrumentation or a real coverage run. Effects stay in Bats temp.

setup() {
  frost_bash=${FROST_TEST_BASH:-bash}
  frost_make=${FROST_TEST_MAKE:-make}
  frost_fixture="$BATS_TEST_TMPDIR/project with spaces"
  frost_cargo="$BATS_TEST_TMPDIR/cargo stub"
  export RUSTC="$BATS_TEST_TMPDIR/default rustc stub"
  frost_baseline="$frost_fixture/verification/crate-standards-validation/baseline-coverage.json"
  mkdir -p "${frost_baseline%/*}"
  cp "$BATS_TEST_DIRNAME/../../Makefile" "$frost_fixture/Makefile"
  cp "$BATS_TEST_DIRNAME/../crate-standards-validation/baseline-coverage.json" "$frost_baseline"
  export FROST_COVERAGE_HOST=aarch64-apple-darwin
  export FROST_COVERAGE_TOOL_VERSION=0.9.1
  export FROST_COVERAGE_EXIT=0
  export FROST_COVERAGE_JSON
  FROST_COVERAGE_JSON=$(jq '{data:[{totals:.totals}]}' "$frost_baseline")
  unset CARGO_BUILD_TARGET RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
  cat >"$frost_cargo" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
if [[ $1 == -vV ]]; then
  printf 'cargo 1.99.1\nrelease: 1.99.1\nhost: %s\n' "$FROST_COVERAGE_HOST"
  exit 0
fi
if [[ $1 == llvm-cov && ${2:-} == --version ]]; then
  printf 'cargo-llvm-cov %s\n' "$FROST_COVERAGE_TOOL_VERSION"
  exit 0
fi
if [[ $1 != llvm-cov ]]; then
  printf 'unexpected fixture invocation\n' >&2
  exit 64
fi
while [[ $# -gt 0 ]]; do
  if [[ $1 == --output-path && $# -ge 2 ]]; then
    printf '%s\n' "$FROST_COVERAGE_JSON" >"$2"
    exit "$FROST_COVERAGE_EXIT"
  fi
  shift
done
printf 'coverage output path missing\n' >&2
exit 64
STUB
  chmod +x "$frost_cargo"
  cat >"$RUSTC" <<'STUB'
#!/usr/bin/env bash
printf 'rustc 1.99.1\nrelease: 1.99.1\nhost: %s\n' "$FROST_COVERAGE_HOST"
STUB
  chmod +x "$RUSTC"
}

run_coverage_gate() {
  run "$frost_make" --no-print-directory -C "$frost_fixture" \
    "SHELL=$frost_bash" "CARGO=$frost_cargo" coverage-gate "$@"
}

@test "coverage ratchet accepts its exact baseline with an unquoted scope" {
  run_coverage_gate
  [ "$status" -eq 0 ]
  jq -e '.host_target == "aarch64-apple-darwin" and
    .ratchet_mode == "scoped_exact_ratchet_enforced"' \
    "$frost_fixture/target/coverage/scope-metadata.json"
}

@test "coverage ratchet rejects a decrease still above the portable floor" {
  FROST_COVERAGE_JSON=$(printf '%s\n' "$FROST_COVERAGE_JSON" | jq '
    .data[0].totals.lines.covered -= 1 |
    .data[0].totals.lines.percent =
      (.data[0].totals.lines.covered * 100 / .data[0].totals.lines.count)')
  run_coverage_gate
  [ "$status" -ne 0 ]
  [ -s "$frost_fixture/target/coverage/source-summary.json" ]
  [ -s "$frost_fixture/target/coverage/scope-metadata.json" ]
}

@test "coverage ratchet rejects malformed baseline metric types" {
  jq '.totals.lines.percent = "unknown"' "$frost_baseline" >"$frost_baseline.tmp"
  mv "$frost_baseline.tmp" "$frost_baseline"
  run_coverage_gate
  [ "$status" -ne 0 ]
}

@test "a coverage command failure preserves the emitted summary and scope" {
  FROST_COVERAGE_EXIT=1 run_coverage_gate
  [ "$status" -ne 0 ]
  [ -s "$frost_fixture/target/coverage/source-summary.json" ]
  [ -s "$frost_fixture/target/coverage/scope-metadata.json" ]
}

@test "coverage ratchet rejects a missing baseline metric" {
  jq 'del(.totals.lines)' "$frost_baseline" >"$frost_baseline.tmp"
  mv "$frost_baseline.tmp" "$frost_baseline"
  run_coverage_gate
  [ "$status" -ne 0 ]
}

@test "cross-target coverage identifies adoption-floor-only scope" {
  FROST_COVERAGE_HOST=x86_64-unknown-linux-gnu run_coverage_gate
  [ "$status" -eq 0 ]
  jq -e '.host_target == "x86_64-unknown-linux-gnu" and
    .ratchet_mode == "portable_adoption_floor_only_cross_target"' \
    "$frost_fixture/target/coverage/scope-metadata.json"
}

@test "a different coverage tool cannot claim an exact baseline scope" {
  FROST_COVERAGE_TOOL_VERSION=0.9.2 run_coverage_gate
  if [[ $status -eq 0 ]]; then
    jq -e '.ratchet_mode != "scoped_exact_ratchet_enforced"' \
      "$frost_fixture/target/coverage/scope-metadata.json"
  fi
}

@test "a different exclusion cannot claim an exact baseline scope" {
  run_coverage_gate 'COVERAGE_IGNORE_REGEX=(^|/)(tests|examples)/'
  if [[ $status -eq 0 ]]; then
    jq -e '.ratchet_mode != "scoped_exact_ratchet_enforced"' \
      "$frost_fixture/target/coverage/scope-metadata.json"
  fi
}

@test "an explicit different build target cannot claim the host baseline" {
  CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu run_coverage_gate
  if [[ $status -eq 0 ]]; then
    jq -e '.ratchet_mode != "scoped_exact_ratchet_enforced"' \
      "$frost_fixture/target/coverage/scope-metadata.json"
  fi
}

@test "an overridden compiler cannot claim the Cargo executable version as its scope" {
  frost_rustc="$BATS_TEST_TMPDIR/rustc stub"
  cat >"$frost_rustc" <<'STUB'
#!/usr/bin/env bash
printf 'rustc 1.99.2\nrelease: 1.99.2\nhost: aarch64-apple-darwin\n'
STUB
  chmod +x "$frost_rustc"
  RUSTC="$frost_rustc" run_coverage_gate
  if [[ $status -eq 0 ]]; then
    jq -e '.ratchet_mode != "scoped_exact_ratchet_enforced"' \
      "$frost_fixture/target/coverage/scope-metadata.json"
  fi
}

@test "the default compiler identity is measured independently of Cargo" {
  frost_compiler_bin="$BATS_TEST_TMPDIR/compiler bin"
  mkdir -p "$frost_compiler_bin"
  cat >"$frost_compiler_bin/rustc" <<'STUB'
#!/usr/bin/env bash
printf 'rustc 1.99.2\nrelease: 1.99.2\nhost: aarch64-apple-darwin\n'
STUB
  chmod +x "$frost_compiler_bin/rustc"
  unset RUSTC
  PATH="$frost_compiler_bin:$PATH" run_coverage_gate
  if [[ $status -eq 0 ]]; then
    jq -e '.ratchet_mode != "scoped_exact_ratchet_enforced"' \
      "$frost_fixture/target/coverage/scope-metadata.json"
  fi
}

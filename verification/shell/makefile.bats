#!/usr/bin/env bats

setup() {
  frost_bash=${FROST_TEST_BASH:-bash}
  frost_make=${FROST_TEST_MAKE:-make}
  frost_fixture="$BATS_TEST_TMPDIR/project with spaces"
  frost_cargo="$BATS_TEST_TMPDIR/cargo stub"
  mkdir -p "$frost_fixture"
  cp "$BATS_TEST_DIRNAME/../../Makefile" "$frost_fixture/Makefile"
  export FROST_TEST_LOG="$BATS_TEST_TMPDIR/calls"
  export FROST_TEST_FAIL_COMMAND=
  unset RUSTDOCFLAGS
  export FROST_TEST_METADATA='{"target_directory":"build area","packages":[{
    "name":"frost-collections","version":"0.4.0",
    "features":{"default":[],"alloc":[],"std":[],"advanced":[],
      "concurrent":[],"frameworks":[],"alpha-feature":[],"zeta-feature":[]},
    "targets":[{"name":"frost","kind":["lib"]},
      {"name":"first","kind":["example"]},{"name":"second","kind":["example"]}]
  }]}'
  cat >"$frost_cargo" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
{
  printf 'CALL'
  printf ' <%s>' "$@"
  printf '\n'
} >>"$FROST_TEST_LOG"
if [[ $1 == "$FROST_TEST_FAIL_COMMAND" ]]; then
  exit 31
fi
if [[ $1 == metadata ]]; then
  printf '%s\n' "$FROST_TEST_METADATA"
fi
STUB
  chmod +x "$frost_cargo"
}

run_make() {
  run "$frost_make" --no-print-directory -C "$frost_fixture" \
    "SHELL=$frost_bash" "CARGO=$frost_cargo" "$@"
}

use_crlf_jq() {
  frost_jq="$BATS_TEST_TMPDIR/jq stub"
  export FROST_TEST_JQ_REAL
  FROST_TEST_JQ_REAL=$(command -v jq)
  cat >"$frost_jq" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
lines=$("$FROST_TEST_JQ_REAL" "$@")
while IFS= read -r line; do
  printf '%s\r\n' "$line"
done <<< "$lines"
STUB
  chmod +x "$frost_jq"
}

@test "help and dry runs do not execute Cargo or parse metadata" {
  run_make help
  [ "$status" -eq 0 ]
  [ ! -e "$FROST_TEST_LOG" ]
  run_make -n features examples package
  [ "$status" -eq 0 ]
  [ ! -e "$FROST_TEST_LOG" ]
}

@test "the default is a build and phony targets ignore same-named files" {
  touch "$frost_fixture/all" "$frost_fixture/build"
  run_make
  [ "$status" -eq 0 ]
  [ "$(<"$FROST_TEST_LOG")" = 'CALL <build> <--locked>' ]
}

@test "metadata failures stop discovery before any verification command" {
  FROST_TEST_FAIL_COMMAND=metadata run_make features
  [ "$status" -ne 0 ]
  [ "$(wc -l <"$FROST_TEST_LOG")" -eq 1 ]
  [[ "$(<"$FROST_TEST_LOG")" == 'CALL <metadata>'* ]]
}

@test "invalid JSON fails closed before packaging" {
  FROST_TEST_METADATA='not JSON' run_make package
  [ "$status" -ne 0 ]
  [ "$(wc -l <"$FROST_TEST_LOG")" -eq 1 ]
}

@test "an empty feature matrix fails instead of silently succeeding" {
  FROST_TEST_METADATA='{"packages":[{"name":"frost-collections","features":{}}]}' \
    run_make features
  [ "$status" -ne 0 ]
  [[ "$output" == *'no isolated features found'* ]]
  [ "$(wc -l <"$FROST_TEST_LOG")" -eq 1 ]
}

@test "isolated features exclude bundles and retain per-feature checks" {
  run_make features
  [ "$status" -eq 0 ]
  [ "$(wc -l <"$FROST_TEST_LOG")" -eq 7 ]
  [[ "$(<"$FROST_TEST_LOG")" == *'<clippy> <--lib> <--no-default-features> <--features> <alpha-feature>'* ]]
  [[ "$(<"$FROST_TEST_LOG")" == *'<test> <--no-default-features> <--features> <zeta-feature>'* ]]
  [[ "$(<"$FROST_TEST_LOG")" != *'<--features> <default>'* ]]
}

@test "a failed feature check stops the loop before tests or later features" {
  FROST_TEST_FAIL_COMMAND=clippy run_make features
  [ "$status" -ne 0 ]
  [ "$(wc -l <"$FROST_TEST_LOG")" -eq 2 ]
  [[ "$(<"$FROST_TEST_LOG")" != *'<zeta-feature>'* ]]
}

@test "example discovery runs only examples and propagates execution failure" {
  FROST_TEST_FAIL_COMMAND=run run_make examples
  [ "$status" -ne 0 ]
  [ "$(wc -l <"$FROST_TEST_LOG")" -eq 2 ]
  [[ "$(<"$FROST_TEST_LOG")" == *'<run> <--example> <first> <--all-features> <--locked>'* ]]
  [[ "$(<"$FROST_TEST_LOG")" != *'<second>'* ]]
}

@test "package tests preserve the manifest path as one argument" {
  run_make package
  [ "$status" -eq 0 ]
  [ "$(wc -l <"$FROST_TEST_LOG")" -eq 4 ]
  [[ "$(<"$FROST_TEST_LOG")" == *'<--manifest-path> <build area/package/frost-collections-0.4.0/Cargo.toml>'* ]]
}

@test "failed packaging never runs tests on an old extracted package" {
  FROST_TEST_FAIL_COMMAND=package run_make package
  [ "$status" -ne 0 ]
  [ "$(wc -l <"$FROST_TEST_LOG")" -eq 2 ]
  [[ "$(<"$FROST_TEST_LOG")" != *'CALL <test>'* ]]
}

@test "native Windows jq line endings do not enter feature arguments" {
  use_crlf_jq
  run_make features "JQ=$frost_jq"
  [ "$status" -eq 0 ]
  [ "$(wc -l <"$FROST_TEST_LOG")" -eq 7 ]
  [[ "$(<"$FROST_TEST_LOG")" != *$'\r'* ]]
  [[ "$(<"$FROST_TEST_LOG")" == *'<--features> <alpha-feature>'* ]]
  [[ "$(<"$FROST_TEST_LOG")" == *'<--features> <zeta-feature>'* ]]
}

@test "native Windows jq line endings do not enter example names" {
  use_crlf_jq
  run_make examples "JQ=$frost_jq"
  [ "$status" -eq 0 ]
  [ "$(wc -l <"$FROST_TEST_LOG")" -eq 3 ]
  [[ "$(<"$FROST_TEST_LOG")" != *$'\r'* ]]
  [[ "$(<"$FROST_TEST_LOG")" == *'<--example> <first>'* ]]
  [[ "$(<"$FROST_TEST_LOG")" == *'<--example> <second>'* ]]
}

@test "native Windows jq line endings do not enter package manifest paths" {
  use_crlf_jq
  run_make package "JQ=$frost_jq"
  [ "$status" -eq 0 ]
  [ "$(wc -l <"$FROST_TEST_LOG")" -eq 4 ]
  [[ "$(<"$FROST_TEST_LOG")" != *$'\r'* ]]
  [[ "$(<"$FROST_TEST_LOG")" == *'<--manifest-path> <build area/package/frost-collections-0.4.0/Cargo.toml>'* ]]
}

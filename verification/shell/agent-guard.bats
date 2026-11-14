#!/usr/bin/env bats

setup() {
  frost_guard="$BATS_TEST_DIRNAME/../../scripts/agent-guard-unsafe.sh"
  frost_buf='crates/frost-collections-core/src/adaptive/seq/buf.rs'
}

guard() {
  # guard <path> <field> <text>
  run "${FROST_TEST_BASH:-bash}" "$frost_guard" \
    <<<"$(jq -n --arg path "$1" --arg text "$3" --arg field "$2" \
      '{tool_name: "Edit", tool_input: {file_path: $path, ($field): $text}}')"
}

@test "the unsafe guard refuses a new unsafe block in another Rust file" {
  guard "/work/crates/frost-collections-core/src/family/ordered.rs" new_string \
    'let x = unsafe { *ptr };'
  [ "$status" -eq 2 ]
  [[ "$output" == *'Unsafe code stays in'* ]]
}

@test "the unsafe guard refuses unsafe items in a whole-file write" {
  guard "crates/frost-collections-integer/src/lib.rs" content \
    $'pub struct T;\nunsafe impl Send for T {}'
  [ "$status" -eq 2 ]
  guard "crates/frost-collections-integer/src/lib.rs" content 'pub unsafe fn f() {}'
  [ "$status" -eq 2 ]
}

@test "the unsafe guard allows the one module that may hold unsafe code" {
  guard "/work/$frost_buf" new_string 'unsafe { ptr.write(value) }'
  [ "$status" -eq 0 ]
  guard "$frost_buf" new_string 'unsafe { ptr.write(value) }'
  [ "$status" -eq 0 ]
}

@test "the unsafe guard allows safe Rust, lint attributes and prose about unsafe code" {
  guard "crates/frost-collections/src/lib.rs" new_string '#![forbid(unsafe_code)]'
  [ "$status" -eq 0 ]
  guard "crates/frost-collections-core/verification/kani/src/lib.rs" new_string \
    '// Reuse the exact unsafe implementation, not a rewritten model of it.'
  [ "$status" -eq 0 ]
  guard "crates/frost-collections-core/src/plan.rs" new_string 'let unsafe_count = 0;'
  [ "$status" -eq 0 ]
}

@test "the unsafe guard ignores files that are not Rust and input it cannot read" {
  guard "docs/design/architecture.md" new_string 'unsafe { example }'
  [ "$status" -eq 0 ]
  run "${FROST_TEST_BASH:-bash}" "$frost_guard" <<<'not json'
  [ "$status" -eq 0 ]
  run "${FROST_TEST_BASH:-bash}" "$frost_guard" <<<'{"tool_input":{}}'
  [ "$status" -eq 0 ]
}

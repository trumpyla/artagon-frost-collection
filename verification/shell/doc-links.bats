#!/usr/bin/env bats

setup() {
  frost_bash=${FROST_TEST_BASH:-bash}
  frost_script="$BATS_TEST_DIRNAME/../../scripts/check-doc-links.sh"
  frost_fixture="$BATS_TEST_TMPDIR/project with spaces"
  mkdir -p "$frost_fixture/docs/supplied"
}

@test "checks root and nested links with encoded spaces, UTF-8, and fragments" {
  touch "$frost_fixture/a file.md" "$frost_fixture/café.md"
  printf '%s\n' '[space](a%20file.md#heading)' '[utf8](caf%C3%A9.md?raw=1)' \
    '[directory](docs/)' >"$frost_fixture/README.md"
  printf '%s\n' '[parent](../a%20file.md)' >"$frost_fixture/docs/guide.md"
  run "$frost_bash" "$frost_script" "$frost_fixture"
  [ "$status" -eq 0 ]
  [ "$output" = 'Verified 4 repository-local documentation links.' ]
}

@test "ignores external URLs, anchors, queries, and supplied historical inputs" {
  printf '%s\n' '[web](https://example.invalid/x)' '[mail](mailto:test@example.invalid)' \
    '[network](//example.invalid/x)' '[anchor](#heading)' '[query](?raw=1)' \
    >"$frost_fixture/README.md"
  printf '%s\n' '[old](missing.md)' >"$frost_fixture/docs/supplied/old.md"
  run "$frost_bash" "$frost_script" "$frost_fixture"
  [ "$status" -eq 0 ]
  [ "$output" = 'Verified 0 repository-local documentation links.' ]
}

@test "reports every missing local link and returns failure" {
  printf '%s\n' '[one](one.md) and [two](two.md)' >"$frost_fixture/README.md"
  run "$frost_bash" "$frost_script" "$frost_fixture"
  [ "$status" -eq 1 ]
  [[ "$output" == *'README.md: missing one.md'* ]]
  [[ "$output" == *'README.md: missing two.md'* ]]
  [[ "$output" != *'Verified '* ]]
}

@test "preserves newline filenames and multiline labels" {
  touch "$frost_fixture/line"$'\n'"break.md"
  printf '[multi\nline](line%%0Abreak.md)\n' >"$frost_fixture/docs/a"$'\n'"b.md"
  run "$frost_bash" "$frost_script" "$frost_fixture"
  [ "$status" -eq 1 ]
  # The reference is relative to docs, so a same-named file at the root is insufficient.
  mv "$frost_fixture/line"$'\n'"break.md" "$frost_fixture/docs/line"$'\n'"break.md"
  run "$frost_bash" "$frost_script" "$frost_fixture"
  [ "$status" -eq 0 ]
  [ "$output" = 'Verified 1 repository-local documentation links.' ]
}

@test "keeps malformed percent escapes literal and decodes encoded punctuation" {
  touch "$frost_fixture/100%done.md" "$frost_fixture/a#b?.md"
  printf '%s\n' '[literal](100%done.md)' '[encoded](a%23b%3F.md)' \
    >"$frost_fixture/README.md"
  run "$frost_bash" "$frost_script" "$frost_fixture"
  [ "$status" -eq 0 ]
  [ "$output" = 'Verified 2 repository-local documentation links.' ]
}

@test "rejects encoded NUL instead of silently truncating the path" {
  touch "$frost_fixture/existing"
  printf '%s\n' '[nul](existing%00missing)' >"$frost_fixture/README.md"
  run "$frost_bash" "$frost_script" "$frost_fixture"
  [ "$status" -eq 1 ]
  [[ "$output" == *'invalid NUL'* ]]
}

@test "decoded shell syntax remains an inert filename" {
  printf '%s\n' '[command](%24%28touch%20owned%29)' >"$frost_fixture/README.md"
  cd "$frost_fixture" || return 1
  run "$frost_bash" "$frost_script" "$frost_fixture"
  [ "$status" -eq 1 ]
  [ ! -e "$frost_fixture/owned" ]
}

@test "accepts existing symlink destinations and rejects broken ones" {
  touch "$frost_fixture/target.md"
  ln -s target.md "$frost_fixture/link.md"
  printf '%s\n' '[link](link.md)' >"$frost_fixture/README.md"
  run "$frost_bash" "$frost_script" "$frost_fixture"
  [ "$status" -eq 0 ]
  rm "$frost_fixture/target.md"
  run "$frost_bash" "$frost_script" "$frost_fixture"
  [ "$status" -eq 1 ]
}

@test "checks links inside symlinked Markdown files" {
  printf '%s\n' '[missing](absent.md)' >"$frost_fixture/source.txt"
  ln -s source.txt "$frost_fixture/linked.md"
  run "$frost_bash" "$frost_script" "$frost_fixture"
  [ "$status" -eq 1 ]
  [[ "$output" == *'linked.md: missing absent.md'* ]]
}

@test "propagates discovery failure instead of reporting zero links" {
  mkdir "$BATS_TEST_TMPDIR/bin"
  printf '#!/bin/sh\nexit 74\n' >"$BATS_TEST_TMPDIR/bin/find"
  chmod +x "$BATS_TEST_TMPDIR/bin/find"
  PATH="$BATS_TEST_TMPDIR/bin:$PATH" run "$frost_bash" "$frost_script" "$frost_fixture"
  [ "$status" -eq 74 ]
  [[ "$output" != *'Verified '* ]]
}

@test "rejects unknown options, extra arguments, and a missing root" {
  run "$frost_bash" "$frost_script" --unknown
  [ "$status" -eq 2 ]
  run "$frost_bash" "$frost_script" "$frost_fixture" extra
  [ "$status" -eq 2 ]
  run "$frost_bash" "$frost_script" "$frost_fixture/missing"
  [ "$status" -ne 0 ]
}

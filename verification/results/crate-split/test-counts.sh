#!/usr/bin/env bash
# Evidence tool for the crate split (frost-wlt); not a Make gate.
# Reads `cargo test` output on stdin and prints one sorted line per test
# target: "<target>\t<passed>\t<failed>\t<ignored>". Results of one target
# reported in several batches (doctests) are summed. A target that silently
# runs fewer tests after a layout change shows up as a changed line.
set -euo pipefail

target=""
while IFS= read -r line; do
  case "$line" in
    *"Running unittests "*)
      target="unit:$(printf '%s\n' "$line" | sed -E 's/.*Running unittests ([^ ]+) .*/\1/')"
      ;;
    *"Running "*" ("*)
      target="$(printf '%s\n' "$line" | sed -E 's/.*Running ([^ ]+) .*/\1/')"
      ;;
    *"Doc-tests "*)
      target="doc:$(printf '%s\n' "$line" | sed -E 's/.*Doc-tests ([^ ]+).*/\1/')"
      ;;
    "test result: "*)
      counts="$(printf '%s\n' "$line" \
        | sed -E 's/^test result: [a-zA-Z]+\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored;.*/\1\t\2\t\3/')"
      printf '%s\t%s\n' "$target" "$counts"
      ;;
  esac
done \
  | awk -F'\t' '{ p[$1] += $2; f[$1] += $3; i[$1] += $4 }
    END { for (t in p) printf "%s\t%d\t%d\t%d\n", t, p[t], f[t], i[t] }' \
  | LC_ALL=C sort

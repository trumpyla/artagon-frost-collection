#!/usr/bin/env bash
# Bash 3.2+ executable for macOS/Linux; uses jq when it is installed.
# Pre-edit guard for agent CLIs: refuse an edit that adds `unsafe` code to a
# Rust file other than the one module that may hold it.
#
# Input: the JSON an agent CLI sends to a pre-tool hook on standard input, with
# the edited path in .tool_input.file_path and the new text in
# .tool_input.new_string (an edit) or .tool_input.content (a whole file).
# Exit 0 allows the edit. Exit 2 refuses it and prints the reason on standard
# error, which Claude Code shows to the agent.
#
# This is early feedback, not the gate: `make check` runs the ast-grep rule
# that enforces the boundary. Without jq the guard allows everything.
set -u

allowed='crates/frost-collections-core/src/adaptive/seq/buf.rs'

command -v jq >/dev/null 2>&1 || exit 0
input=$(cat)
path=$(printf '%s' "$input" | jq -r '.tool_input.file_path // empty' 2>/dev/null) || exit 0
case "$path" in
  *.rs) ;;
  *) exit 0 ;;
esac
case "$path" in
  "$allowed" | */"$allowed") exit 0 ;;
esac
text=$(printf '%s' "$input" \
  | jq -r '.tool_input.new_string // .tool_input.content // empty' 2>/dev/null) || exit 0
if printf '%s' "$text" \
  | grep -Eq '(^|[^[:alnum:]_])unsafe[[:space:]]*(\{|(fn|impl|trait|extern)([^[:alnum:]_]|$))'; then
  printf 'Refused: this edit adds unsafe code to %s. Unsafe code stays in %s (AGENTS.md).\n' \
    "$path" "$allowed" >&2
  exit 2
fi
exit 0

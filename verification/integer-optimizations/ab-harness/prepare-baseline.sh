#!/usr/bin/env bash
# Bash 3.2+ executable for macOS/Linux; requires Git, tar, and sed.
# Archive a baseline revision of this repository next to the harness and rename
# its packages to `frost-baseline*`, so the harness links baseline and working
# tree into one executable. Usage: ./prepare-baseline.sh <git-rev>
#
# The harness depends on ./baseline-workspace/crates/frost-collections. Two
# layouts are handled. A workspace revision has its packages under crates/; the
# whole workspace is extracted into ./baseline-workspace, whose root manifest
# its members inherit from. A revision from before the workspace layout has one
# package at the repository root; it is extracted into the same facade path.
set -euo pipefail

cd "$(dirname "$0")"
rev="${1:?usage: prepare-baseline.sh <git-rev>}"
root="$(git -C ../../.. rev-parse --show-toplevel)"
commit="$(git -C "$root" rev-parse "$rev^{commit}")"

# Both names are fixed and relative to this script's directory. `baseline` is
# where earlier versions of this script extracted.
rm -rf baseline baseline-workspace
facade_dir=baseline-workspace/crates/frost-collections
facade=$facade_dir/Cargo.toml

rename_manifest() {
  # Rename one package; keep the library name its dependents' sources use.
  local manifest=$1
  sed -e 's/^name = "frost-collections\(-[a-z-]*\)\{0,1\}"$/name = "frost-baseline\1"/' \
    "$manifest" >"$manifest.new"
  mv "$manifest.new" "$manifest"
}

if git -C "$root" cat-file -e "$commit:crates/frost-collections/Cargo.toml" 2>/dev/null; then
  mkdir -p baseline-workspace
  # `-m` gives the extracted files the current time. Archive times are the
  # commit time, and Cargo treats sources older than its last build as
  # unchanged, so a switch to an older baseline would silently keep the
  # previous one.
  git -C "$root" archive --format=tar "$commit" Cargo.toml Cargo.lock crates \
    | tar -x -m -C baseline-workspace
  for manifest in baseline-workspace/crates/*/Cargo.toml; do
    rename_manifest "$manifest"
  done
  # Members name each other through the workspace table. Keep each dependency
  # key, which is the name the sources import, and point it at the renamed
  # package.
  workspace=baseline-workspace/Cargo.toml
  sed -e 's/^\(frost-collections\(-[a-z-]*\)\{0,1\}\) = { path = /\1 = { package = "frost-baseline\2", path = /' \
    "$workspace" >"$workspace.new"
  mv "$workspace.new" "$workspace"
else
  mkdir -p "$facade_dir"
  git -C "$root" archive --format=tar "$commit" Cargo.toml Cargo.lock src README.md \
    | tar -x -m -C "$facade_dir"
  rename_manifest "$facade"
  # Bench, example, and test targets are not archived, so drop their sections:
  # a section runs from its header to the next blank line.
  sed -e '/^\[\[bench\]\]$/,/^$/d' \
    -e '/^\[\[example\]\]$/,/^$/d' \
    -e '/^\[\[test\]\]$/,/^$/d' \
    "$facade" >"$facade.new"
  mv "$facade.new" "$facade"
fi

# The facade's library is `frost` in both layouts; the harness imports the
# baseline as `frost_baseline`.
if grep -q '^name = "frost"$' "$facade"; then
  sed -e 's/^name = "frost"$/name = "frost_baseline"/' "$facade" >"$facade.new"
  mv "$facade.new" "$facade"
else
  printf '\n[lib]\nname = "frost_baseline"\n' >>"$facade"
fi

printf '%s\n' "$commit" >baseline-workspace/BASELINE_REV
printf 'prepared baseline %s\n' "$commit" >&2

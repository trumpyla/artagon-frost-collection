#!/usr/bin/env bash
# Archive a baseline revision of this repository into ./baseline and rename its
# package to `frost-baseline`, so the harness links baseline and working tree
# into one executable. Usage: ./prepare-baseline.sh <git-rev>
set -euo pipefail

cd "$(dirname "$0")"
rev="${1:?usage: prepare-baseline.sh <git-rev>}"
root="$(git -C ../../.. rev-parse --show-toplevel)"
commit="$(git -C "$root" rev-parse "$rev^{commit}")"

rm -rf baseline
mkdir -p baseline
git -C "$root" archive --format=tar "$commit" Cargo.toml Cargo.lock src README.md \
  | tar -x -C baseline

manifest=baseline/Cargo.toml
# Rename the package and library. Bench, example, and test targets are not
# archived, so drop their sections: a section runs from its header to the next
# blank line.
sed -e 's/^name = "frost-collections"$/name = "frost-baseline"/' \
  -e '/^\[\[bench\]\]$/,/^$/d' \
  -e '/^\[\[example\]\]$/,/^$/d' \
  -e '/^\[\[test\]\]$/,/^$/d' \
  "$manifest" >"$manifest.new"
mv "$manifest.new" "$manifest"
if grep -q '^name = "frost"$' "$manifest"; then
  sed -e 's/^name = "frost"$/name = "frost_baseline"/' "$manifest" >"$manifest.new"
  mv "$manifest.new" "$manifest"
else
  printf '\n[lib]\nname = "frost_baseline"\n' >>"$manifest"
fi

printf '%s\n' "$commit" >baseline/BASELINE_REV
printf 'prepared baseline %s\n' "$commit"

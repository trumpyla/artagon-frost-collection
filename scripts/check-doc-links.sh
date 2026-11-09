#!/usr/bin/env bash
# Executable, Bash 3.2+; BSD/GNU find (-print0), mktemp, and rm on macOS/Linux.
# Checks the same inline Markdown-link subset as the original checker.
set -euo pipefail
export LC_ALL=C

if ((BASH_VERSINFO[0] < 3 || (BASH_VERSINFO[0] == 3 && BASH_VERSINFO[1] < 2))); then
  printf 'Bash 3.2 or newer is required.\n' >&2
  exit 2
fi
if (($# > 1)) || [[ ${1:-} == -* ]]; then
  printf 'Usage: %s [project-root]\n' "$0" >&2
  exit 2
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P) || exit 1
root=$(cd -- "${1:-$script_dir/..}" && pwd -P) || exit 1
readonly root
temporary=$(mktemp -d "${TMPDIR:-/tmp}/frost-doc-links.XXXXXX") || exit 1
readonly temporary
trap 'rm -rf -- "$temporary"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# A file list preserves spaces/newlines and lets discovery failures propagate.
find "$root" -maxdepth 1 \( -type f -o -type l \) -name '*.md' \
  -print0 >"$temporary/files"
if [[ -d $root/docs ]]; then
  find "$root/docs" -type d -name supplied -prune -o \
    \( -type f -o -type l \) -name '*.md' -print0 >>"$temporary/files"
fi

# Set decoded in the caller; never evaluate the URL as shell code.
decode_path() {
  local remaining=$1 byte hex prefix
  decoded=
  while [[ $remaining == *%* ]]; do
    prefix=${remaining%%\%*}
    decoded+=$prefix
    remaining=${remaining#*%}
    hex=${remaining:0:2}
    if [[ $hex =~ ^[[:xdigit:]]{2}$ ]]; then
      if [[ $hex == 00 ]]; then
        return 1
      fi
      printf -v byte '%b' "\\x$hex"
      decoded+=$byte
      remaining=${remaining:2}
    else
      decoded+=%
    fi
  done
  decoded+=$remaining
}

readonly link_pattern='\[[^]]*\]\(([^[:space:])]+)\)'
readonly scheme_pattern='^[[:alpha:]][[:alnum:]+.-]*:'
checked=0
errors=0
decoded=
while IFS= read -r -d '' file; do
  if [[ ! -f $file || ! -r $file ]]; then
    printf 'Cannot read Markdown file: %s\n' "${file#"$root"/}" >&2
    exit 1
  fi
  content=$(<"$file")
  while [[ $content =~ $link_pattern ]]; do
    matched=${BASH_REMATCH[0]}
    target=${BASH_REMATCH[1]}
    content=${content#*"$matched"}
    if [[ $target =~ $scheme_pattern || $target == //* ]]; then
      continue
    fi
    path=${target%%[?#]*}
    if [[ -z $path ]]; then
      continue
    fi
    checked=$((checked + 1))
    if ! decode_path "$path"; then
      printf '%s: invalid NUL in link %s\n' "${file#"$root"/}" "$target" >&2
      errors=$((errors + 1))
      continue
    fi
    if [[ $decoded == /* ]]; then
      destination=$decoded
    else
      destination=${file%/*}/$decoded
    fi
    if [[ ! -e $destination ]]; then
      printf '%s: missing %s\n' "${file#"$root"/}" "$target" >&2
      errors=$((errors + 1))
    fi
  done
done <"$temporary/files"

if ((errors > 0)); then
  exit 1
fi
printf 'Verified %s repository-local documentation links.\n' "$checked"

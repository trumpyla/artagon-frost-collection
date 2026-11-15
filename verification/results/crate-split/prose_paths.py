#!/usr/bin/env python3
"""Rewrite backticked source paths in prose after the crate split.

Migration tool for the crate split (frost-wlt.17). It is not a Make gate.

usage: prose_paths.py [--apply] <markdown file>...

Documents written before the split name files relative to the single package:
`src/adaptive/seq/buf.rs`, `tests/families.rs`, `verification/kani`. For each
backticked token that starts with one of those prefixes, the tool looks for the
same relative path under each package (and for the moved harness directories).
If exactly one location exists, the token is rewritten to the repository-relative
path. If none or several exist, the token is left alone and reported, so a
person or an agent can decide from the surrounding sentence.

Only whole backticked tokens are considered; a trailing `:line` suffix, a
trailing `/`, and glob characters are kept. Links are not touched: relink.py
handles those.
"""
import os
import re
import sys

PACKAGES = [
    "crates/frost-collections-core",
    "crates/frost-collections-integer",
    "crates/frost-collections-concurrent",
    "crates/frost-collections",
    "crates/frost-collections-test-support",
]
HARNESSES = {
    "verification/kani": "crates/frost-collections-core/verification/kani",
    "verification/verus": "crates/frost-collections-core/verification/verus",
    "verification/fuzz": "crates/frost-collections-integer/fuzz",
    "verification/loom": "crates/frost-collections-concurrent/verification/loom",
    "verification/int-key-downstream": "crates/frost-collections/verification/int-key-downstream",
}
SPECIAL = {
    "tests/support/workload.rs": "crates/frost-collections-test-support/src/workload.rs",
    "benches/support/mod.rs": "crates/frost-collections-test-support/src/bench.rs",
    "benches/support": "crates/frost-collections-test-support/src",
    "tests/support": "crates/frost-collections-test-support/src",
}
TOKEN = re.compile(r"`((?:src|tests|benches|examples|verification)/[^`\s]*)`")


def candidates(path):
    core = path.split(":")[0].rstrip("/")
    core = core.split("*")[0].rstrip("/") or core
    if core in SPECIAL:
        return [SPECIAL[core] + path[len(core):]]
    for old, new in HARNESSES.items():
        if core == old or core.startswith(old + "/"):
            return [new + path[len(old):]]
    if core.startswith("verification/"):
        return None
    found = []
    for package in PACKAGES:
        if os.path.exists(os.path.join(package, core)):
            found.append(package + "/" + path)
    return found


def main():
    apply = "--apply" in sys.argv
    files = [arg for arg in sys.argv[1:] if not arg.startswith("--")]
    if not files:
        sys.exit(__doc__)
    changed = 0
    for path in files:
        with open(path, encoding="utf-8") as handle:
            text = handle.read()
        notes = []

        def rewrite(match):
            nonlocal changed
            token = match.group(1)
            found = candidates(token)
            if found is None:
                return match.group(0)
            if len(found) == 1:
                changed += 1
                return "`" + found[0] + "`"
            line = text.count("\n", 0, match.start()) + 1
            kind = "ambiguous" if found else "not found"
            notes.append("%s:%d: %s `%s`%s" % (
                path, line, kind, token,
                " -> " + ", ".join(f.split("/")[1] for f in found) if found else ""))
            return match.group(0)

        updated = TOKEN.sub(rewrite, text)
        if apply and updated != text:
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(updated)
        for note in notes:
            print(note)
    print("%d token(s) %s" % (changed, "rewritten" if apply else "would be rewritten"))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Move test and bench targets from the facade to their owning packages.

Migration tool for the crate split (frost-wlt.13). It is not a Make gate.

usage: move_targets.py <kind> <package dir> <library name> <target>...

<kind> is `tests` or `benches`. Each <target> is a file or directory name
under `crates/frost-collections/<kind>/`. The tool moves it with `git mv` to
`<package dir>/<kind>/` and, for each moved target root (a `*.rs` file directly
under <kind>, or `<dir>/main.rs`), inserts

    extern crate <library name> as frost;

after the file's leading inner doc comments and inner attributes, so the body
keeps the `frost::` paths it was written with. With `--drop-gate <feature>`
(repeatable, before the targets) it also removes a leading
`#![cfg(feature = "<feature>")]` line, for a feature the new package always
has. Nothing else in a file changes.
"""
import os
import subprocess
import sys

FACADE = "crates/frost-collections"


def insert_alias(path, library, drop):
    with open(path, encoding="utf-8") as handle:
        lines = handle.read().split("\n")
    alias = "extern crate %s as frost;" % library
    if alias in lines:
        return
    kept = []
    dropped = []
    for line in lines:
        if any(line.strip() == '#![cfg(feature = "%s")]' % feature for feature in drop):
            dropped.append(line.strip())
            continue
        kept.append(line)
    lines = kept
    index = 0
    last = -1
    in_attr = False
    while index < len(lines):
        text = lines[index].strip()
        if in_attr:
            last = index
            if text.endswith("]"):
                in_attr = False
        elif text.startswith("//!") or text.startswith("//") or text == "":
            if text.startswith("//!"):
                last = index
        elif text.startswith("#!["):
            last = index
            if not text.endswith("]"):
                in_attr = True
        else:
            break
        index += 1
    position = last + 1
    block = ["", "// The package under test, under the name its users import.", alias]
    if position < len(lines) and lines[position].strip() != "":
        block.append("")
    lines[position:position] = block
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines))
    print("  alias in %s%s" % (path, " (dropped %s)" % ", ".join(dropped) if dropped else ""))


def main():
    args = sys.argv[1:]
    drop = []
    rest = []
    index = 0
    while index < len(args):
        if args[index] == "--drop-gate":
            drop.append(args[index + 1])
            index += 2
        else:
            rest.append(args[index])
            index += 1
    if len(rest) < 4:
        sys.exit(__doc__)
    kind, package, library = rest[0], rest[1], rest[2]
    destination = os.path.join(package, kind)
    os.makedirs(destination, exist_ok=True)
    for target in rest[3:]:
        source = os.path.join(FACADE, kind, target)
        subprocess.run(["git", "mv", source, os.path.join(destination, target)], check=True)
        moved = os.path.join(destination, target)
        if os.path.isdir(moved):
            root = os.path.join(moved, "main.rs")
            if os.path.exists(root):
                insert_alias(root, library, drop)
        elif moved.endswith(".rs"):
            insert_alias(moved, library, drop)
        print("moved %s" % moved)


if __name__ == "__main__":
    main()

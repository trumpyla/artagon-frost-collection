#!/usr/bin/env python3
"""Add a hidden crate alias to the doctests of an internal Frost package.

Migration tool for the crate split (frost-wlt). It is not a Make gate.

usage: doctest_alias.py <library name> <src dir>

Doctests are written against the facade library `frost`. In an internal
package they would not resolve, so each doctest whose body names `frost` gets
`# extern crate <library name> as frost;` as its first line. The line is
hidden in rendered documentation. Running the tool twice changes nothing.
"""
import os
import re
import sys

FENCE = re.compile(r"^(\s*(?://[/!] ?)?)```(.*)$")


def process(path, library):
    alias = "# extern crate %s as frost;" % library
    with open(path, encoding="utf-8") as handle:
        lines = handle.read().split("\n")
    out = []
    added = 0
    index = 0
    while index < len(lines):
        line = lines[index]
        match = FENCE.match(line)
        out.append(line)
        index += 1
        if not match:
            continue
        prefix = match.group(1)
        body = []
        while index < len(lines) and not FENCE.match(lines[index]):
            body.append(lines[index])
            index += 1
        text = "\n".join(body)
        if re.search(r"\bfrost\b", text) and alias not in text:
            if prefix.strip():
                out.append(prefix.rstrip() + " " + alias)
            else:
                out.append(prefix + alias)
            added += 1
        out.extend(body)
        if index < len(lines):
            out.append(lines[index])
            index += 1
    if added:
        with open(path, "w", encoding="utf-8") as handle:
            handle.write("\n".join(out))
    return added


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    library, root = sys.argv[1], sys.argv[2]
    total = 0
    for directory, _, files in os.walk(root):
        for name in sorted(files):
            if name.endswith(".rs"):
                total += process(os.path.join(directory, name), library)
    print("%d doctest(s) aliased" % total)


if __name__ == "__main__":
    main()

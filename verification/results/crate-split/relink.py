#!/usr/bin/env python3
"""Rewrite repository-local Markdown links after files move.

Migration tool for the crate split (frost-wlt). It is not a Make gate.

usage: relink.py [--dry] OLD=NEW [OLD=NEW]...

Each OLD=NEW pair maps a repository-relative path or directory prefix before
the move to its location after it. Run it after the files have moved. For
every Markdown file it resolves each relative link against the file's
location before the move, maps the target, and writes the link relative to
the file's location now. A link is rewritten only when its file or its target
moved. Anchors and query strings are kept. Links to targets that do not exist
after mapping are reported and left unchanged.

Inline links `[text](target)`, images, and reference definitions
`[label]: target` are handled. Code blocks are not parsed; a link-shaped
string inside one is treated as a link, as the repository's link checker does.
"""
import os
import re
import subprocess
import sys

INLINE = re.compile(r"(\]\()([^)\s]+)(\))")
REFERENCE = re.compile(r"^(\[[^\]]+\]:\s+)(\S+)(.*)$", re.M)
SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")


def norm(path):
    return os.path.normpath(path).replace(os.sep, "/")


class Mapping:
    def __init__(self, pairs):
        self.by_old = sorted(pairs, key=lambda pair: -len(pair[0]))
        self.by_new = sorted(pairs, key=lambda pair: -len(pair[1]))

    def forward(self, path):
        for old, new in self.by_old:
            if path == old or path.startswith(old + "/"):
                return norm(new + path[len(old):])
        return path

    def backward(self, path):
        for old, new in self.by_new:
            if path == new or path.startswith(new + "/"):
                return norm(old + path[len(new):])
        return path


def main():
    args = sys.argv[1:]
    dry = "--dry" in args
    pairs = []
    for arg in args:
        if arg.startswith("--"):
            continue
        old, new = arg.split("=", 1)
        pairs.append((norm(old), norm(new)))
    if not pairs:
        sys.exit(__doc__)
    mapping = Mapping(pairs)
    files = subprocess.run(
        ["git", "ls-files", "-co", "--exclude-standard", "*.md"],
        check=True, capture_output=True, text=True,
    ).stdout.split("\n")
    changed = 0
    missing = []
    for path in filter(None, files):
        if not os.path.isfile(path):
            continue
        old_file = mapping.backward(path)
        old_dir = os.path.dirname(old_file)
        new_dir = os.path.dirname(path)
        with open(path, encoding="utf-8") as handle:
            text = handle.read()

        def rewrite(match):
            target = match.group(2)
            if SCHEME.match(target) or target.startswith("#") or target.startswith("/"):
                return match.group(0)
            cut = len(target)
            for mark in "#?":
                if mark in target:
                    cut = min(cut, target.index(mark))
            link, suffix = target[:cut], target[cut:]
            if not link:
                return match.group(0)
            old_target = norm(os.path.join(old_dir, link))
            if old_target.startswith(".."):
                return match.group(0)
            new_target = mapping.forward(old_target)
            if new_target == old_target and old_file == path:
                return match.group(0)
            if not os.path.exists(new_target):
                missing.append("%s: %s -> %s" % (path, target, new_target))
                return match.group(0)
            relative = os.path.relpath(new_target, new_dir or ".").replace(os.sep, "/")
            if link.endswith("/") and not relative.endswith("/"):
                relative += "/"
            if relative == link:
                return match.group(0)
            return match.group(1) + relative + suffix + match.group(3)

        updated = INLINE.sub(rewrite, text)
        updated = REFERENCE.sub(rewrite, updated)
        if updated != text:
            changed += 1
            if not dry:
                with open(path, "w", encoding="utf-8") as handle:
                    handle.write(updated)
    print("%d file(s) %s" % (changed, "would change" if dry else "changed"))
    for line in missing:
        print("missing target: " + line)


if __name__ == "__main__":
    main()

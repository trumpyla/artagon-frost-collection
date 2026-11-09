#!/usr/bin/env python3
"""Check repository-local Markdown links, excluding unmodified supplied evidence."""

from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parent.parent
FILES = list(ROOT.glob("*.md")) + list((ROOT / "docs").rglob("*.md"))
errors = []
checked = 0
for path in sorted(FILES):
    if "supplied" in path.relative_to(ROOT).parts:
        continue
    for match in re.finditer(r"\[[^\]]*\]\(([^\s)]+)\)", path.read_text()):
        target = match.group(1)
        parsed = urlsplit(target)
        if parsed.scheme or parsed.netloc or not parsed.path:
            continue
        destination = (path.parent / unquote(parsed.path)).resolve()
        checked += 1
        if not destination.exists():
            errors.append(f"{path.relative_to(ROOT)}: missing {target}")
if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
print(f"Verified {checked} repository-local documentation links.")

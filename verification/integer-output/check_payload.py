#!/usr/bin/env python3
"""Run the candidate-only exact payload reproducer outside the library graph."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tomllib


def run(command: list[str], cwd: Path, env: dict[str, str], log: Path) -> None:
    with log.open("wb") as output:
        completed = subprocess.run(command, cwd=cwd, env=env, stdout=output, stderr=subprocess.STDOUT)
    if completed.returncode:
        raise SystemExit(f"Command failed ({completed.returncode}); inspect {log}")


def pins(path: Path) -> set[tuple[str, str, str, str]]:
    return {
        (package["name"], package["version"], package.get("source", ""), package.get("checksum", ""))
        for package in tomllib.loads(path.read_text())["package"]
        if "source" in package
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--online", action="store_true")
    options = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    work = options.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(root / "verification/integer-output/payload.rs", work / "main.rs")
    shutil.copyfile(root / "Cargo.lock", work / "Cargo.lock")
    (work / "Cargo.toml").write_text(
        "[package]\n"
        'name = "frost-integer-payload-check"\n'
        'version = "0.0.0"\n'
        'edition = "2024"\n'
        'rust-version = "1.99.1"\n'
        "publish = false\n\n"
        "[workspace]\n\n"
        "[[bin]]\n"
        'name = "integer-payload-check"\n'
        'path = "main.rs"\n\n'
        "[dependencies]\n"
        f'frost = {{ package = "frost-collections", path = {json.dumps(str(root))}, '
        'default-features = false, features = ["std", "int-set"] }\n'
    )
    rustup = shutil.which("rustup") or str(Path.home() / ".cargo/bin/rustup")
    environment = os.environ.copy()
    for variable in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_RUSTFLAGS"):
        environment.pop(variable, None)
    environment["CARGO_TARGET_DIR"] = str(work / "target")
    offline = [] if options.online else ["--offline"]
    run([rustup, "run", "1.99.1", "cargo", "update", *offline, "--workspace"], work, environment, work / "payload-lock.log")
    if pins(work / "Cargo.lock") - pins(root / "Cargo.lock"):
        raise SystemExit("Payload reproducer selected a dependency absent from the source lockfile")
    run([rustup, "run", "1.99.1", "cargo", "build", *offline, "--locked", "--release"], work, environment, work / "payload-build.log")
    with (work / "payload-check.csv").open("wb") as output, (work / "payload-check.log").open("wb") as errors:
        completed = subprocess.run([str(work / "target/release/integer-payload-check")], cwd=work, env=environment, stdout=output, stderr=errors)
    if completed.returncode:
        raise SystemExit(f"Payload checks failed; inspect {work / 'payload-check.log'}")
    metadata = {
        "toolchain": "1.99.1",
        "source": str(root),
        "features": ["std", "int-set"],
        "harness_sha256": hashlib.sha256((work / "main.rs").read_bytes()).hexdigest(),
        "measurement": "Direct contiguous_payload_bytes(); actual Sorted/Dense buffer bytes, excluding wrapper and allocator overhead",
        "roaring": "None means the API does not report discontiguous resident allocation; no resident-byte claim is made",
        "timing": "None; complete logical outputs and expected representation/payload/limits are asserted",
    }
    (work / "payload-metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print((work / "payload-check.csv").read_text(), end="")


if __name__ == "__main__":
    main()

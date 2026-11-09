#!/usr/bin/env python3
"""Build/verify/time the identical integer-output harness against both sources.

The baseline archive is extracted into an isolated work directory. Neither the
archive nor the current crate's manifest or source files is modified. Build is
the default phase; running timers requires --phase bench. Execute timing phases
without other builds, tests, or benchmarks competing for the machine.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tomllib
import zipfile


BASELINE_SHA256 = "516a019e46cf7755bb495fbb92356ea3044a0560863d075ef4f2b4dea9195886"
TOOLCHAIN = "1.99.1"
PROFILES = {
    "baseline": "-C target-cpu=x86-64",
    "v3": "-C target-cpu=x86-64-v3",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command: list[str], log: Path, env: dict[str, str], cwd: Path) -> None:
    print("Running:", " ".join(command), flush=True)
    with log.open("wb") as output:
        completed = subprocess.run(command, cwd=cwd, env=env, stdout=output, stderr=subprocess.STDOUT)
    if completed.returncode:
        print(log.read_text(errors="replace"), file=sys.stderr)
        raise SystemExit(f"Command failed ({completed.returncode}); see {log}")


def extract_baseline(archive: Path, destination: Path) -> Path:
    if sha256(archive) != BASELINE_SHA256:
        raise SystemExit("Baseline archive digest does not match the reviewed source archive")
    if not destination.exists():
        destination.mkdir(parents=True)
        with zipfile.ZipFile(archive) as source:
            for item in source.infolist():
                path = Path(item.filename)
                if path.is_absolute() or ".." in path.parts:
                    raise SystemExit(f"Invalid archive member: {item.filename}")
            source.extractall(destination)
    result = destination / "frost"
    if not (result / "Cargo.toml").is_file():
        raise SystemExit("Baseline extraction does not contain frost/Cargo.toml")
    return result


def package_pins(lock: Path) -> set[tuple[str, str, str, str]]:
    return {
        (package["name"], package["version"], package.get("source", ""), package.get("checksum", ""))
        for package in tomllib.loads(lock.read_text())["package"]
        if "source" in package
    }


def source_digests(source: Path) -> dict[str, str]:
    paths = [source / "Cargo.toml", source / "Cargo.lock"]
    paths.extend((source / "src").rglob("*.rs"))
    paths.extend(source.glob("rust-toolchain*"))
    return {str(path.relative_to(source)): sha256(path) for path in sorted(paths)}


def prepare_harness(source: Path, candidate: Path, destination: Path) -> str:
    destination.mkdir(parents=True, exist_ok=True)
    (destination / "support").mkdir(exist_ok=True)
    shutil.copyfile(candidate / "benches/integer_output.rs", destination / "main.rs")
    shutil.copyfile(candidate / "benches/support/mod.rs", destination / "support/mod.rs")
    source_literal = json.dumps(str(source))
    (destination / "Cargo.toml").write_text(
        "[package]\n"
        'name = "frost-integer-output-comparison"\n'
        'version = "0.0.0"\n'
        'edition = "2024"\n'
        'rust-version = "1.99.1"\n'
        "publish = false\n\n"
        "[workspace]\n\n"
        "[[bin]]\n"
        'name = "integer-output"\n'
        'path = "main.rs"\n\n'
        "[dependencies]\n"
        f'frost = {{ package = "frost-collections", path = {source_literal}, '
        'default-features = false, features = ["std", "int-set", "roaring-0_12"] }\n\n'
        "[profile.release]\n"
        "opt-level = 3\n"
        "debug = false\n"
        "lto = false\n"
        "codegen-units = 16\n"
        "incremental = false\n"
        "overflow-checks = false\n"
        'panic = "unwind"\n'
    )
    # Seed every comparison with the crate's checked-in dependency versions.
    # Cargo updates the standalone workspace package, not its dependencies.
    shutil.copyfile(source / "Cargo.lock", destination / "Cargo.lock")
    return sha256(destination / "main.rs")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work-dir", type=Path, required=True, help="isolated build/output directory")
    parser.add_argument("--phase", choices=("build", "verify", "bench"), default="build")
    parser.add_argument("--cpu", type=int, default=0, help="CPU affinity for timed execution (default: 0)")
    parser.add_argument("--profile", choices=("baseline", "v3", "both"), default="both")
    parser.add_argument("--case", choices=("mixed", "distant", "sorted_roaring", "equality_hash", "construction", "intersection", "contains"))
    parser.add_argument("--online", action="store_true", help="allow Cargo network access instead of --offline")
    options = parser.parse_args()

    candidate = Path(__file__).resolve().parents[2]
    work = options.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=True)
    if platform.machine().lower() not in ("x86_64", "amd64"):
        raise SystemExit("These paired profiles are x86-64 specific; choose matching flags for another architecture")
    profiles = list(PROFILES) if options.profile == "both" else [options.profile]
    if options.phase != "build" and "v3" in profiles and Path("/proc/cpuinfo").exists():
        cpuinfo = Path("/proc/cpuinfo").read_text()
        if not all(flag in cpuinfo.split() for flag in ("avx2", "bmi2", "fma")):
            raise SystemExit("This host does not advertise the features needed to execute x86-64-v3")

    rustup = shutil.which("rustup") or str(Path.home() / ".cargo/bin/rustup")
    environment = os.environ.copy()
    for variable in ("CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_RUSTFLAGS", "FROST_OUTPUT_VERIFY_ONLY", "FROST_OUTPUT_CASE"):
        environment.pop(variable, None)
    environment["CARGO_INCREMENTAL"] = "0"
    offline = [] if options.online else ["--offline"]
    baseline = extract_baseline(candidate / "verification/integer-output/baseline.zip", work / "baseline-source")
    sources = {"before": baseline, "after": candidate}
    metadata = {
        "toolchain": TOOLCHAIN,
        "phase": options.phase,
        "host": platform.platform(),
        "baseline_archive_sha256": BASELINE_SHA256,
        "source_paths": {key: str(value) for key, value in sources.items()},
        "source_digests": {key: source_digests(value) for key, value in sources.items()},
        "profiles": {key: PROFILES[key] for key in profiles},
        "benchmark_scope": "warm-cache wall clock; input setup excluded; result allocation/destruction included",
        "hash_control_scope": "Roaring has no native Hash; custom member and payload controls have separate protocols",
        "memory_scope": "Sorted payload exact; bounds-derived Dense payload minimum; Roaring serialized bytes are not resident bytes",
        "case": options.case or "all",
        "harness_sha256": sha256(candidate / "benches/integer_output.rs"),
        "support_sha256": sha256(candidate / "benches/support/mod.rs"),
        "cpu_affinity": options.cpu if options.phase == "bench" else None,
        "timed_version_order_per_profile": ["before", "after", "after", "before"] if options.phase == "bench" else None,
    }
    if Path("/proc/cpuinfo").exists():
        shutil.copyfile("/proc/cpuinfo", work / "cpuinfo.txt")
    run([rustup, "run", TOOLCHAIN, "rustc", "-Vv"], work / "rustc.txt", environment, candidate)

    for version, source in sources.items():
        harness = work / f"harness-{version}"
        assert prepare_harness(source, candidate, harness) == metadata["harness_sha256"]
        run([rustup, "run", TOOLCHAIN, "cargo", "update", *offline, "--workspace"], work / f"{version}-lock.log", environment, harness)
        unexpected = package_pins(harness / "Cargo.lock") - package_pins(source / "Cargo.lock")
        if unexpected:
            raise SystemExit(f"Standalone harness resolved dependencies absent from the source lockfile: {unexpected}")
    before_pins = package_pins(work / "harness-before/Cargo.lock")
    after_pins = package_pins(work / "harness-after/Cargo.lock")
    if before_pins != after_pins:
        raise SystemExit("The comparison resolved unequal dependency versions; align its features and locks before measuring")
    metadata["matched_dependency_pins"] = [list(pin) for pin in sorted(before_pins)]

    for profile in profiles:
        flags = PROFILES[profile]
        environment["RUSTFLAGS"] = flags
        target = work / f"target-{profile}"
        environment["CARGO_TARGET_DIR"] = str(target)
        for version in sources:
            harness = work / f"harness-{version}"
            run([rustup, "run", TOOLCHAIN, "cargo", "build", *offline, "--locked", "--release"], work / f"{version}-{profile}-build.log", environment, harness)
            binary = work / f"{version}-{profile}-integer-output"
            shutil.copy2(target / "release/integer-output", binary)

    if metadata["source_digests"] != {key: source_digests(value) for key, value in sources.items()}:
        raise SystemExit("A source changed during the build; repeat against a frozen source tree")
    # Serial execution, after all compilation finishes, avoids build traffic
    # contaminating another version's samples. The counterbalanced run order
    # exposes drift without asserting that order removes environmental noise.
    if options.phase != "build":
        if options.phase == "bench":
            if not hasattr(os, "sched_getaffinity") or options.cpu not in os.sched_getaffinity(0):
                raise SystemExit(f"Cannot pin timed execution to CPU {options.cpu}; select an available CPU explicitly")
            os.sched_setaffinity(0, {options.cpu})
        for profile in profiles:
            if options.phase == "bench":
                versions = ("before", "after", "after", "before")
            else:
                versions = ("before", "after") if profile == "baseline" else ("after", "before")
            repetitions = {"before": 0, "after": 0}
            for version in versions:
                environment["RUSTFLAGS"] = PROFILES[profile]
                if options.phase == "verify":
                    environment["FROST_OUTPUT_VERIFY_ONLY"] = "1"
                if options.case:
                    environment["FROST_OUTPUT_CASE"] = options.case
                binary = work / f"{version}-{profile}-integer-output"
                label = f"{version}-{profile}-{options.phase}" + (f"-{options.case}" if options.case else "")
                repetitions[version] += 1
                if options.phase == "bench":
                    label += f"-{repetitions[version]}"
                print(f"Running {label} serially", flush=True)
                with (work / f"{label}.csv").open("wb") as output, (work / f"{label}.log").open("wb") as errors:
                    completed = subprocess.run([str(binary)], cwd=work, env=environment, stdout=output, stderr=errors)
                if completed.returncode:
                    raise SystemExit(f"{label} failed with {completed.returncode}; inspect its .log file")
    (work / f"{options.phase}-metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(f"Completed {options.phase}; outputs: {work}")


if __name__ == "__main__":
    main()

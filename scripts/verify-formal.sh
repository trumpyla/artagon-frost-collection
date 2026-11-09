#!/usr/bin/env bash
# Each requested mode must execute its checker successfully; no silent skips.
set -euo pipefail
cd "$(dirname "$0")/.."

case "${1:-bounded}" in
  bounded)
    cargo test --locked --test exhaustive_collections --features fixedbitset-0_5
    cargo test --locked --test publication_model -- --nocapture
    # These ordinary tests execute the proof properties but do not run a solver.
    cargo test --manifest-path verification/kani/Cargo.toml --locked
    ;;
  loom)
    # This isolated package has no effect on the library's dependency graph.
    cargo test --manifest-path verification/loom/Cargo.toml --release
    ;;
  kani)
    if ! cargo kani --version; then
      printf 'Kani is unavailable. Install a compatible pinned verifier; no proof ran.\n' >&2
      exit 1
    fi
    cd verification/kani
    cargo kani --lib
    ;;
  *) printf 'Usage: %s [bounded|loom|kani]\n' "$0" >&2; exit 2 ;;
esac

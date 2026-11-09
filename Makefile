# GNU Make 3.81+; recipes use Bash 3.2+. Cargo owns Rust dependency tracking.
.DEFAULT_GOAL := all
SHELL := bash
.SHELLFLAGS := -eu -o pipefail -c
.SUFFIXES:
.DELETE_ON_ERROR:

CARGO ?= cargo
JQ ?= jq
BATS ?= bats
SHELLCHECK ?= shellcheck
SHFMT ?= shfmt
AST_GREP ?= ast-grep
MIRI_TOOLCHAIN ?= nightly-2026-11-08
EMBEDDED_TARGET ?= thumbv7em-none-eabihf

.PHONY: all build help check verify fmt fmt-check doc-links clippy docs test \
	test-examples profiles features integers examples package embedded miri \
	bounded loom kani shell-check shell-test tools-check search-index clean

all: build

build:
	"$(CARGO)" build --locked

help:
	@printf '%s\n' \
	  'make [all|build]  Build the default library profile' \
	  'make check        Formatting, links, Clippy, rustdoc, tests, examples' \
	  'make verify       All host verification scopes (no Miri/proof solver)' \
	  'make profiles     Core-only, alloc-only, and default profiles' \
	  'make features     Check each optional feature independently (requires jq)' \
	  'make integers     Debug/release integer tests and downstream codec crate' \
	  'make examples     Run every example program (requires jq)' \
	  'make package      Build/test the extracted Cargo package (requires jq)' \
	  'make embedded     Bare-metal builds; install EMBEDDED_TARGET first' \
	  'make miri         Ownership diagnostics with MIRI_TOOLCHAIN' \
	  'make bounded      Finite collection/retry models and native Kani helpers' \
	  'make loom|kani    Optional protocol model or symbolic proofs' \
	  'make shell-check  ShellCheck, shfmt, and shell/Make behavior tests' \
	  'make tools-check  Test and scan the optional ast-grep rules' \
	  'make search-index Refresh the local tgrep index' \
	  'make fmt|clean    Format Rust sources or remove Cargo build artifacts'

check: fmt-check doc-links clippy docs test test-examples

# Independent prerequisites may run with -j; Cargo coordinates its build locks.
verify: check profiles features integers examples package

fmt:
	"$(CARGO)" fmt --all

fmt-check:
	"$(CARGO)" fmt --all --check

doc-links:
	"$(SHELL)" scripts/check-doc-links.sh

clippy:
	"$(CARGO)" clippy --all-targets --all-features --locked -- -D warnings

docs:
	RUSTDOCFLAGS="$${RUSTDOCFLAGS:+$${RUSTDOCFLAGS} }-D warnings" \
	  "$(CARGO)" doc --all-features --no-deps --locked

test:
	"$(CARGO)" test --all-features --locked

test-examples:
	"$(CARGO)" test --examples --all-features --locked

profiles:
	"$(CARGO)" clippy --lib --no-default-features --locked -- -D warnings
	"$(CARGO)" test --no-default-features --locked
	RUSTDOCFLAGS="$${RUSTDOCFLAGS:+$${RUSTDOCFLAGS} }-D warnings" \
	  "$(CARGO)" doc --no-default-features --no-deps --locked
	"$(CARGO)" clippy --lib --no-default-features --features alloc --locked -- -D warnings
	"$(CARGO)" test --no-default-features --features alloc --locked
	RUSTDOCFLAGS="$${RUSTDOCFLAGS:+$${RUSTDOCFLAGS} }-D warnings" \
	  "$(CARGO)" doc --no-default-features --features alloc --no-deps --locked
	"$(CARGO)" test --locked

# Keep metadata discovery in recipes: make help/-n never executes Cargo.
# GNU Make 3.81 ignores .SHELLFLAGS, so compound recipes set strict options too.
features:
	@set -euo pipefail; \
	metadata=$$("$(CARGO)" metadata --format-version 1 --no-deps --locked); \
	features=$$(printf '%s\n' "$$metadata" | "$(JQ)" -er \
	  '[.packages[] | select(.name == "frost-collections")] | \
	   if length != 1 then error("expected one Frost package") else .[0] end | \
	   .features | keys | map(select(. != "default" and . != "alloc" and \
	   . != "std" and . != "advanced" and . != "concurrent" and . != "frameworks")) | \
	   if length == 0 then error("no isolated features found") else .[] end'); \
	while IFS= read -r feature; do \
	  printf '\nChecking isolated feature: %s\n' "$$feature"; \
	  "$(CARGO)" clippy --lib --no-default-features --features "$$feature" --locked -- -D warnings; \
	  "$(CARGO)" test --no-default-features --features "$$feature" --locked; \
	  RUSTDOCFLAGS="$${RUSTDOCFLAGS:+$${RUSTDOCFLAGS} }-D warnings" \
	    "$(CARGO)" doc --no-default-features --features "$$feature" --no-deps --locked; \
	done <<< "$$features"

examples:
	@set -euo pipefail; \
	metadata=$$("$(CARGO)" metadata --format-version 1 --no-deps --locked); \
	examples=$$(printf '%s\n' "$$metadata" | "$(JQ)" -er \
	  '[.packages[] | select(.name == "frost-collections")] | \
	   if length != 1 then error("expected one Frost package") else .[0] end | \
	   [.targets[] | select(.kind | index("example")) | .name] | \
	   if length == 0 then error("no examples found") else .[] end'); \
	while IFS= read -r example; do \
	  "$(CARGO)" run --example "$$example" --all-features --locked; \
	done <<< "$$examples"

integers:
	"$(CARGO)" test --locked --no-default-features --features int-set --test integer_sets --test integer_output
	"$(CARGO)" test --locked --release --features int-set,arc-swap-1 --test integer_sets --test integer_output
	"$(CARGO)" test --locked --manifest-path verification/int-key-downstream/Cargo.toml

package:
	@set -euo pipefail; \
	metadata=$$("$(CARGO)" metadata --format-version 1 --no-deps --locked); \
	manifest=$$(printf '%s\n' "$$metadata" | "$(JQ)" -er \
	  '.target_directory as $$target | \
	   [.packages[] | select(.name == "frost-collections")] | \
	   if length != 1 then error("expected one Frost package") else .[0] end | \
	   [$$target, "package", (.name + "-" + .version), "Cargo.toml"] | join("/")'); \
	"$(CARGO)" package --locked --all-features --allow-dirty; \
	"$(CARGO)" test --manifest-path "$$manifest" --all-features --locked; \
	"$(CARGO)" test --manifest-path "$$manifest" --examples --all-features --locked

embedded:
	"$(CARGO)" build --lib --locked --no-default-features --features int-set --target "$(EMBEDDED_TARGET)"
	"$(CARGO)" build --lib --locked --no-default-features --target "$(EMBEDDED_TARGET)"
	"$(CARGO)" build --lib --locked --no-default-features \
	  --features alloc,hashbrown-0_17,indexmap-2,smallvec-1,arrayvec-0_7,fixedbitset-0_5,slotmap-1,phf-0_14,serde-1,borsh-1,schemars-1 \
	  --target "$(EMBEDDED_TARGET)"

miri:
	"$(CARGO)" "+$(MIRI_TOOLCHAIN)" test --doc --all-features --locked
	MIRIFLAGS='-Zmiri-strict-provenance' \
	  "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test --locked --test inline_safety --test families --test core_review -- --skip under_random_ops
	MIRIFLAGS='-Zmiri-tree-borrows -Zmiri-strict-provenance' \
	  "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test --locked --test inline_safety
	MIRIFLAGS='-Zmiri-tree-borrows -Zmiri-strict-provenance' \
	  "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test --locked --test exhaustive_collections inline_ownership_exhausts_three_operation_traces
	MIRIFLAGS='-Zmiri-strict-provenance' \
	  "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test --locked --no-default-features --features int-set \
	    --target i686-unknown-linux-gnu --test integer_sets dense_ -- --test-threads=1
	MIRIFLAGS='-Zmiri-strict-provenance' \
	  "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test --locked --no-default-features --features int-set \
	    --target i686-unknown-linux-gnu --lib integer::storage::word::tests -- --test-threads=1
	MIRIFLAGS='-Zmiri-strict-provenance' \
	  "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test --locked --no-default-features --features int-set \
	    --target i686-unknown-linux-gnu --lib integer::storage::tests -- --test-threads=1
	@set -euo pipefail; \
	for case_name in canonical_hash_events_match_across_representations_and_boundaries \
	  canonical_hash_scout_thresholds_are_representation_independent \
	  equality_and_hash_ignore_dense_padding_and_construction_history \
	  empty_operands_and_identity_shortcuts_do_not_bypass_limits \
	  output_policy_overrides_are_checked_and_retained; do \
	  MIRIFLAGS='-Zmiri-strict-provenance' \
	    "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test --locked --no-default-features --features int-set \
	      --target i686-unknown-linux-gnu --test integer_output "$$case_name" -- --exact --test-threads=1; \
	done

bounded:
	"$(CARGO)" test --locked --test exhaustive_collections --features fixedbitset-0_5
	"$(CARGO)" test --locked --test publication_model -- --nocapture
	# Native property helpers; this does not execute a symbolic solver.
	"$(CARGO)" test --manifest-path verification/kani/Cargo.toml --locked

loom:
	"$(CARGO)" test --manifest-path verification/loom/Cargo.toml --release --locked

kani:
	"$(CARGO)" kani --version
	cd verification/kani && "$(CARGO)" kani --lib

shell-check:
	"$(SHELLCHECK)" --shell=bash --severity=info scripts/check-doc-links.sh verification/shell/*.bats
	"$(SHFMT)" -ln bash -d -i 2 -ci -bn scripts/check-doc-links.sh
	"$(SHFMT)" -ln bats -d -i 2 -ci -bn verification/shell/*.bats
	+$(MAKE) shell-test

shell-test:
	FROST_TEST_BASH="$(SHELL)" FROST_TEST_MAKE="$(MAKE)" "$(BATS)" verification/shell

tools-check:
	"$(AST_GREP)" test --config sgconfig.yml
	"$(AST_GREP)" scan --config sgconfig.yml --inspect summary src

search-index:
	tgrep index --hidden --exclude .git --exclude .idea .

clean:
	"$(CARGO)" clean

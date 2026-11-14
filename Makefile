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
# Verus release pinned in docs/verification.md; its own driver uses Rust 1.99.1.
VERUS ?= verus
VERUS_VERSION ?= 0.2026.11.02.8bf68ee
MIRI_TOOLCHAIN ?= nightly-2026-11-08
MIRI_SURVEY_JOBS ?= 4
MIRI_SURVEY_SHARDS := $(foreach shape,0 1 2 3 4 5 6 7 8 9 10,$(foreach width,32 64,$(foreach policy,0 1 2 3,miri-survey-$(shape)-$(width)-$(policy))))
FUZZ_TOOLCHAIN ?= $(MIRI_TOOLCHAIN)
EMBEDDED_TARGET ?= thumbv7em-none-eabihf
EMBEDDED_NO_ATOMICS_TARGET ?= thumbv6m-none-eabi
NEXTEST_PROFILE ?= default
COVERAGE_TARGET_DIR ?= target/coverage
COVERAGE_OUTPUT_DIR ?= target/coverage
COVERAGE_IGNORE_REGEX ?= (^|/)(tests|benches|examples|verification)/
COVERAGE_MIN_LINES ?= 77
COVERAGE_MIN_FUNCTIONS ?= 70
COVERAGE_MIN_REGIONS ?= 77
MUTANTS_CONFIG ?= .config/mutants.toml
MUTANTS_OUTPUT_DIR ?= target/mutants
FUZZ_TARGET ?= ordered_set_algebra
FUZZ_TARGET_DIR ?= target/fuzz
FUZZ_WORK_CORPUS_DIR ?= target/fuzz-corpus
FUZZ_RUNS ?= 64
FUZZ_MAX_TIME ?= 10
CATALOG_TARGET_DIR ?= target/catalog
CATALOG_INVENTORY_DIR ?= target/catalog-inventory
AB_BASELINE ?= origin/main
AB_ROUNDS ?= 3
AB_HARNESS_DIR := verification/integer-optimizations/ab-harness
# The facade package. Recipes name a package with -p; none relies on feature
# unification across workspace members.
FACADE ?= frost-collections
# The packages that own the test targets some lanes select by name.
CORE ?= frost-collections-core
INTEGER ?= frost-collections-integer
CONCURRENT ?= frost-collections-concurrent
PACKAGE_CHECK_DIR ?= target/package-check
# Fails a lane whose test selection matched nothing.
REQUIRE_TESTS := "$(SHELL)" scripts/require-tests.sh

.PHONY: all build help check verify fmt fmt-check doc-links clippy docs test \
	test-examples profiles feature-isolation feature-forwarding features integers examples package embedded miri \
	doctest-codes \
	bounded loom kani verus nextest coverage coverage-gate coverage-branch mutants \
	fuzz-check fuzz-smoke catalog catalog-inventory shell-check shell-test \
	tools-check search-index integer-ab miri-survey $(MIRI_SURVEY_SHARDS) clean

all: build

build:
	"$(CARGO)" build --locked

help:
	@printf '%s\n' \
	  'make [all|build]  Build the default library profile' \
	  'make check        Formatting, links, Clippy, rustdoc, tests, examples, catalog' \
	  'make verify       All host verification scopes (no Miri/proof solver)' \
	  'make profiles     Core-only, alloc-only, and default profiles' \
	  'make features     Check each optional feature independently (requires jq)' \
	  'make integers     Debug/release integer tests and downstream codec crate' \
	  'make examples     Run every example program (requires jq)' \
	  'make package      Build/test the extracted Cargo package (requires jq)' \
	  'make embedded     Install EMBEDDED_TARGET and EMBEDDED_NO_ATOMICS_TARGET first' \
	  'make miri         Ownership diagnostics with MIRI_TOOLCHAIN' \
	  'make miri-survey  Full planner matrix in isolated 32-bit Miri processes' \
	  'make bounded      Finite collection/retry models and native Kani helpers' \
	  'make doctest-codes Check compile_fail error codes with the pinned nightly' \
	  'make loom|kani    Optional protocol model or symbolic proofs' \
	  'make verus        Spec-level algebra proofs with the pinned Verus release' \
	  'make nextest      Run test suite with cargo-nextest (.config/nextest.toml)' \
	  'make coverage     Export source-only cargo-llvm-cov JSON summary and scope' \
	  'make coverage-gate Enforce portable floor (77/70/77) and scoped ratchet' \
	  'make coverage-branch Export nightly branch-coverage JSON summary' \
	  'make mutants      Run cargo-mutants with .config/mutants.toml' \
	  'make fuzz-check   Run deterministic fuzz-oracle regressions on stable' \
	  'make fuzz-smoke   Run bounded libFuzzer smoke with FUZZ_TOOLCHAIN' \
	  'make catalog      Run verification/catalog obligation inventory checks' \
	  'make catalog-inventory Verify benchmark FROST_BENCH_LIST inventory (requires jq)' \
	  'make shell-check  ShellCheck, shfmt, and shell/Make behavior tests' \
	  'make tools-check  Test and scan the optional ast-grep rules' \
	  'make integer-ab   Time AB_BASELINE against the working tree in one executable' \
	  'make search-index Refresh the local tgrep index' \
	  'make fmt|clean    Format Rust sources or remove Cargo build artifacts'

check: fmt-check doc-links clippy docs test test-examples catalog catalog-inventory

# Independent prerequisites may run with -j; Cargo coordinates its build locks.
verify: check profiles features integers examples package

fmt:
	"$(CARGO)" fmt --all

fmt-check:
	"$(CARGO)" fmt --all --check

doc-links:
	"$(SHELL)" scripts/check-doc-links.sh

clippy:
	"$(CARGO)" clippy --workspace --all-targets --all-features --locked -- -D warnings

docs:
	RUSTDOCFLAGS="$${RUSTDOCFLAGS:+$${RUSTDOCFLAGS} }-D warnings" \
	  "$(CARGO)" doc --workspace --all-features --no-deps --locked

test:
	"$(CARGO)" test --workspace --all-features --locked

test-examples:
	"$(CARGO)" test -p "$(FACADE)" --examples --all-features --locked

# Keep metadata discovery in recipes: make help/-n never executes Cargo.
# GNU Make 3.81 ignores .SHELLFLAGS, so compound recipes set strict options too.
# Native Windows jq emits CRLF. Strip only each record's trailing carriage return
# before passing feature names, example names, or package paths to Cargo.
#
# Profile and feature lanes run once per library package, each in its own Cargo
# invocation, so one package's features never reach another's build.
profiles: feature-isolation feature-forwarding
	@set -euo pipefail; \
	metadata=$$("$(CARGO)" metadata --format-version 1 --no-deps --locked); \
	packages=$$(printf '%s\n' "$$metadata" | "$(JQ)" -er \
	  '[.packages[] | select(any(.targets // [] | .[]; .kind | index("lib"))) | \
	    select(.features | has("alloc")) | .name] | \
	   if length == 0 then error("no library packages found") else .[] end'); \
	while IFS= read -r package; do \
	  package=$${package%$$'\r'}; \
	  printf '\nChecking profiles of package: %s\n' "$$package"; \
	  "$(CARGO)" clippy -p "$$package" --lib --no-default-features --locked -- -D warnings; \
	  "$(CARGO)" test -p "$$package" --no-default-features --locked; \
	  RUSTDOCFLAGS="$${RUSTDOCFLAGS:+$${RUSTDOCFLAGS} }-D warnings" \
	    "$(CARGO)" doc -p "$$package" --no-default-features --no-deps --locked; \
	  "$(CARGO)" clippy -p "$$package" --lib --no-default-features --features alloc --locked -- -D warnings; \
	  "$(CARGO)" test -p "$$package" --no-default-features --features alloc --locked; \
	  RUSTDOCFLAGS="$${RUSTDOCFLAGS:+$${RUSTDOCFLAGS} }-D warnings" \
	    "$(CARGO)" doc -p "$$package" --no-default-features --features alloc --no-deps --locked; \
	  "$(CARGO)" test -p "$$package" --locked; \
	done <<< "$$packages"

# The core-only, alloc-only and integer profiles of the facade must resolve the
# core package without `std`, and the core-only profile without `alloc`. A
# package that always enables `std` is never selected together with them.
feature-isolation:
	@set -euo pipefail; \
	check() { \
	  forbidden=$$1; shift; \
	  line=$$("$(CARGO)" tree -p "$(FACADE)" -e normal --prefix none -f '{p} [{f}]' --locked "$$@" \
	    | { grep '^frost-collections-core ' || true; } | sort -u); \
	  if [[ -z $$line ]]; then \
	    printf 'error: the core package is not in the tree of %s\n' "$$*" >&2; \
	    exit 1; \
	  fi; \
	  resolved=$${line##*[}; \
	  resolved=",$${resolved%]},"; \
	  for feature in $$forbidden; do \
	    if [[ $$resolved == *",$$feature,"* ]]; then \
	      printf 'error: %s resolves the core package with %s\n' "$$*" "$$feature" >&2; \
	      exit 1; \
	    fi; \
	  done; \
	}; \
	check 'std alloc' --no-default-features; \
	check 'std' --no-default-features --features alloc; \
	check 'std' --no-default-features --features int-set

# Every library package declares each feature of the workspace packages it
# depends on, under the same name, so a feature gate means the same thing in
# every layer and no layer silently drops a forward.
feature-forwarding:
	@set -euo pipefail; \
	metadata=$$("$(CARGO)" metadata --format-version 1 --no-deps --locked); \
	result=$$(printf '%s\n' "$$metadata" | "$(JQ)" -er \
	  '[.packages[] | select(any(.targets // [] | .[]; .kind | index("lib")))] as $$libraries | \
	   [$$libraries[] | . as $$package | (.dependencies // [])[] | select(.kind == null) | .name as $$lower | \
	     ($$libraries[] | select(.name == $$lower) | .features | keys | map(select(. != "default"))) - \
	     ($$package.features | keys) | select(length > 0) | \
	     "\($$package.name) does not declare \(join(", ")) of \($$lower)"] | \
	   if length > 0 then error(join("; ")) else "every layer declares the features of the layers below it" end'); \
	printf '%s\n' "$$result"

features:
	@set -euo pipefail; \
	metadata=$$("$(CARGO)" metadata --format-version 1 --no-deps --locked); \
	pairs=$$(printf '%s\n' "$$metadata" | "$(JQ)" -er \
	  '[.packages[] | select(any(.targets // [] | .[]; .kind | index("lib"))) | \
	    .name as $$package | .features | keys | \
	    map(select(. != "default" and . != "alloc" and . != "std" and \
	      . != "advanced" and . != "concurrent" and . != "frameworks")) | \
	    .[] | [$$package, .] | @tsv] | \
	   if length == 0 then error("no isolated features found") else .[] end'); \
	while IFS=$$'\t' read -r package feature; do \
	  feature=$${feature%$$'\r'}; \
	  printf '\nChecking isolated feature: %s of %s\n' "$$feature" "$$package"; \
	  "$(CARGO)" clippy -p "$$package" --lib --no-default-features --features "$$feature" --locked -- -D warnings; \
	  "$(CARGO)" test -p "$$package" --no-default-features --features "$$feature" --locked; \
	  RUSTDOCFLAGS="$${RUSTDOCFLAGS:+$${RUSTDOCFLAGS} }-D warnings" \
	    "$(CARGO)" doc -p "$$package" --no-default-features --features "$$feature" --no-deps --locked; \
	done <<< "$$pairs"

examples:
	@set -euo pipefail; \
	metadata=$$("$(CARGO)" metadata --format-version 1 --no-deps --locked); \
	examples=$$(printf '%s\n' "$$metadata" | "$(JQ)" -er \
	  '[.packages[] | select(.name == "$(FACADE)")] | \
	   if length != 1 then error("expected one Frost package") else .[0] end | \
	   [.targets[] | select(.kind | index("example")) | .name] | \
	   if length == 0 then error("no examples found") else .[] end'); \
	while IFS= read -r example; do \
	  example=$${example%$$'\r'}; \
	  "$(CARGO)" run -p "$(FACADE)" --example "$$example" --all-features --locked; \
	done <<< "$$examples"

integers:
	$(REQUIRE_TESTS) "$(CARGO)" test -p "$(INTEGER)" --locked --no-default-features --features int-set --test integer_sets --test integer_output
	$(REQUIRE_TESTS) "$(CARGO)" test -p "$(INTEGER)" --locked --release --test integer_sets --test integer_output
	$(REQUIRE_TESTS) "$(CARGO)" test -p "$(CONCURRENT)" --locked --release --features int-set,arc-swap-1 --test integer_sets --test integer_output
	"$(CARGO)" test --locked --manifest-path verification/int-key-downstream/Cargo.toml

# Nothing is published, and `cargo package` cannot resolve unpublished path
# siblings. So the file set each member would package is copied into a fresh
# scratch workspace and built and tested there (CRATES.md CR-DEP-5). Each run
# makes a new directory; `make clean` removes them with the rest of target/.
package:
	@set -euo pipefail; \
	metadata=$$("$(CARGO)" metadata --format-version 1 --no-deps --locked); \
	members=$$(printf '%s\n' "$$metadata" | "$(JQ)" -er \
	  '[.packages[] | [.name, .manifest_path] | @tsv] | \
	   if length == 0 then error("no workspace members found") else .[] end'); \
	root=$$(pwd -P); \
	mkdir -p "$(PACKAGE_CHECK_DIR)"; \
	scratch=$$(mktemp -d "$(PACKAGE_CHECK_DIR)/workspace.XXXXXX"); \
	cp Cargo.toml Cargo.lock rust-toolchain.toml "$$scratch/"; \
	while IFS=$$'\t' read -r name manifest; do \
	  manifest=$${manifest%$$'\r'}; \
	  source_dir=$$(cd "$$(dirname "$$manifest")" && pwd -P); \
	  member=$${source_dir#"$$root"/}; \
	  files=$$("$(CARGO)" package --list --locked --allow-dirty -p "$$name"); \
	  while IFS= read -r file; do \
	    file=$${file%$$'\r'}; \
	    if [[ -f "$$source_dir/$$file" && $$file != Cargo.lock ]]; then \
	      mkdir -p "$$scratch/$$member/$$(dirname "$$file")"; \
	      cp "$$source_dir/$$file" "$$scratch/$$member/$$file"; \
	    fi; \
	  done <<< "$$files"; \
	done <<< "$$members"; \
	"$(CARGO)" test --manifest-path "$$scratch/Cargo.toml" \
	  --target-dir "$(PACKAGE_CHECK_DIR)/target" --workspace --all-features --locked

embedded:
	"$(CARGO)" build -p "$(FACADE)" --lib --locked --no-default-features --features int-set --target "$(EMBEDDED_NO_ATOMICS_TARGET)"
	"$(CARGO)" build -p "$(FACADE)" --lib --locked --no-default-features --features int-set --target "$(EMBEDDED_TARGET)"
	"$(CARGO)" build -p "$(FACADE)" --lib --locked --no-default-features --target "$(EMBEDDED_TARGET)"
	"$(CARGO)" build -p "$(FACADE)" --lib --locked --no-default-features \
	  --features alloc,hashbrown-0_17,indexmap-2,smallvec-1,arrayvec-0_7,fixedbitset-0_5,slotmap-1,phf-0_14,serde-1,borsh-1,schemars-1 \
	  --target "$(EMBEDDED_TARGET)"

# Stable rustdoc accepts any error for a `compile_fail` doctest. The pinned
# nightly checks the stated error code, so a doctest cannot pass for a wrong
# reason such as an unresolved import.
doctest-codes:
	$(REQUIRE_TESTS) --total "$(CARGO)" "+$(MIRI_TOOLCHAIN)" test --workspace --doc --all-features --locked

miri: doctest-codes
	MIRIFLAGS='-Zmiri-strict-provenance' \
	  $(REQUIRE_TESTS) "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test -p "$(CORE)" --locked --test inline_safety --test families --test core_review -- --skip under_random_ops
	MIRIFLAGS='-Zmiri-tree-borrows -Zmiri-strict-provenance' \
	  $(REQUIRE_TESTS) "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test -p "$(CORE)" --locked --test inline_safety
	MIRIFLAGS='-Zmiri-tree-borrows -Zmiri-strict-provenance' \
	  $(REQUIRE_TESTS) "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test -p "$(CORE)" --locked --test exhaustive_collections inline_ownership_exhausts_three_operation_traces
	MIRIFLAGS='-Zmiri-strict-provenance' \
	  $(REQUIRE_TESTS) "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test -p "$(INTEGER)" --locked --no-default-features --features int-set \
	    --target i686-unknown-linux-gnu --test integer_sets dense_ -- --test-threads=1
	MIRIFLAGS='-Zmiri-strict-provenance' \
	  $(REQUIRE_TESTS) "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test -p "$(INTEGER)" --locked --no-default-features --features int-set \
	    --target i686-unknown-linux-gnu --lib integer::storage::word::tests -- --test-threads=1
	MIRIFLAGS='-Zmiri-strict-provenance' \
	  $(REQUIRE_TESTS) "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test -p "$(INTEGER)" --locked --no-default-features --features int-set \
	    --target i686-unknown-linux-gnu --lib integer::storage::tests -- --test-threads=1 \
	    --skip integer::storage::tests::early_survey_exit_selects_the_same_backend_as_a_complete_survey
	@set -euo pipefail; \
	for case_name in \
	  integer::chunked::tests::partial_fold_resumes_near_chunk_ends_without_repeating_next_chunk \
	  integer::chunked::tests::full_chunk_runs_and_early_break_are_bounded \
	  integer::chunked::tests::unchanged_overlapping_containers_keep_their_allocations \
	  integer::chunked::tests::dense_promotion_splits_windows_on_container_boundaries \
	  integer::dense::tests::deleting_from_an_empty_window_is_a_no_op \
	  integer::dense::tests::dense_runs_preserve_word_edges_and_the_maximum_code; do \
	  MIRIFLAGS='-Zmiri-strict-provenance' \
	    $(REQUIRE_TESTS) "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test -p "$(INTEGER)" --locked --no-default-features --features int-set \
	      --target i686-unknown-linux-gnu --lib "$$case_name" -- --exact --test-threads=1; \
	done
	@set -euo pipefail; \
	for case_name in canonical_hash_events_match_across_representations_and_boundaries \
	  canonical_hash_scout_thresholds_are_representation_independent \
	  equality_and_hash_ignore_dense_padding_and_construction_history \
	  empty_operands_and_identity_shortcuts_do_not_bypass_limits \
	  output_policy_overrides_are_checked_and_retained; do \
	  MIRIFLAGS='-Zmiri-strict-provenance' \
	    $(REQUIRE_TESTS) "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test -p "$(INTEGER)" --locked --no-default-features --features int-set \
	      --target i686-unknown-linux-gnu --test integer_output "$$case_name" -- --exact --test-threads=1; \
	done
	+$(MAKE) miri-survey

# Preserve every planner input, word width, and policy while resetting Miri's
# finite simulated address space between cases. No validation is disabled.
miri-survey:
	+$(MAKE) -j$(MIRI_SURVEY_JOBS) $(MIRI_SURVEY_SHARDS)

$(MIRI_SURVEY_SHARDS):
	FROST_MIRI_SURVEY_CASE="$(patsubst miri-survey-%,%,$@)" \
	  MIRIFLAGS='-Zmiri-strict-provenance -Zmiri-env-forward=FROST_MIRI_SURVEY_CASE' \
	  $(REQUIRE_TESTS) "$(CARGO)" "+$(MIRI_TOOLCHAIN)" miri test -p "$(INTEGER)" --locked --no-default-features --features int-set \
	    --target i686-unknown-linux-gnu --lib \
	    integer::storage::tests::early_survey_exit_selects_the_same_backend_as_a_complete_survey \
	    -- --exact --test-threads=1

bounded:
	$(REQUIRE_TESTS) "$(CARGO)" test -p "$(CORE)" --locked --test exhaustive_collections --features fixedbitset-0_5
	$(REQUIRE_TESTS) "$(CARGO)" test -p "$(CONCURRENT)" --locked --test publication_model -- --nocapture
	# Native property helpers; this does not execute a symbolic solver.
	$(REQUIRE_TESTS) --total "$(CARGO)" test --manifest-path verification/kani/Cargo.toml --locked

loom:
	$(REQUIRE_TESTS) --total "$(CARGO)" test --manifest-path verification/loom/Cargo.toml --release --locked

kani:
	"$(CARGO)" kani --version
	cd verification/kani && "$(CARGO)" kani --lib

# Fails when Verus is absent or is not the pinned release.
verus:
	"$(VERUS)" --version | grep -F "$(VERUS_VERSION)"
	"$(VERUS)" --crate-type=lib verification/verus/src/lib.rs

nextest:
	"$(CARGO)" nextest --version
	"$(CARGO)" nextest run --workspace --config-file .config/nextest.toml --profile "$(NEXTEST_PROFILE)" --all-features --locked

coverage:
	"$(CARGO)" llvm-cov --version
	mkdir -p "$(COVERAGE_OUTPUT_DIR)"
	set -euo pipefail; \
	rustc_info=$$("$${RUSTC:-rustc}" -vV); \
	  host_line=$$(printf '%s\n' "$$rustc_info" | "$(JQ)" -rR 'select(startswith("host: ")) | sub("^host: "; "")'); \
	  toolchain_ver=$$(printf '%s\n' "$$rustc_info" | "$(JQ)" -rR 'select(startswith("release: ")) | sub("^release: "; "")'); \
	  rustc_env="$${RUSTC:-}"; \
	  rustup_tc="$${RUSTUP_TOOLCHAIN:-}"; \
	  effective_target="$${CARGO_BUILD_TARGET:-$$host_line}"; \
	  llvm_ver=$$("$(CARGO)" llvm-cov --version); \
	  "$(JQ)" -n \
	    --arg host "$$effective_target" \
	    --arg toolchain "$$toolchain_ver" \
	    --arg rustc_override "$$rustc_env" \
	    --arg rustup_toolchain "$$rustup_tc" \
	    --arg llvm_cov "$$llvm_ver" \
	    --arg ignore "$(COVERAGE_IGNORE_REGEX)" \
	    '{schema:"frost-coverage-scope/v1",host_target:$$host,toolchain_version:$$toolchain,rustc_override:$$rustc_override,rustup_toolchain_override:$$rustup_toolchain,tool_version:$$llvm_cov,features:"all-features",ignore_filename_regex:$$ignore,baseline_reference:"verification/crate-standards-validation/baseline-coverage.json",baseline_target:"aarch64-apple-darwin"}' \
	    >"$(COVERAGE_OUTPUT_DIR)/scope-metadata.json"
	CARGO_TARGET_DIR="$(COVERAGE_TARGET_DIR)" \
	  "$(CARGO)" llvm-cov --workspace --all-features --locked --json --summary-only \
	    --ignore-filename-regex "$(COVERAGE_IGNORE_REGEX)" \
	    --output-path "$(COVERAGE_OUTPUT_DIR)/source-summary.json"

coverage-gate:
	"$(CARGO)" llvm-cov --version
	mkdir -p "$(COVERAGE_OUTPUT_DIR)"
	set -euo pipefail; \
	rustc_info=$$("$${RUSTC:-rustc}" -vV); \
	  host_line=$$(printf '%s\n' "$$rustc_info" | "$(JQ)" -rR 'select(startswith("host: ")) | sub("^host: "; "")'); \
	  toolchain_ver=$$(printf '%s\n' "$$rustc_info" | "$(JQ)" -rR 'select(startswith("release: ")) | sub("^release: "; "")'); \
	  rustc_env="$${RUSTC:-}"; \
	  rustup_tc="$${RUSTUP_TOOLCHAIN:-}"; \
	  rustflags_env="$${RUSTFLAGS:-}$${CARGO_ENCODED_RUSTFLAGS:-}"; \
	  effective_target="$${CARGO_BUILD_TARGET:-$$host_line}"; \
	  llvm_ver=$$("$(CARGO)" llvm-cov --version); \
	  if [[ $$effective_target == aarch64-apple-darwin && $$toolchain_ver == 1.99.1 && ( -z "$$rustup_tc" || $$rustup_tc == 1.99.1 ) && -z "$$rustflags_env" && $$llvm_ver == "cargo-llvm-cov 0.9.1" && "$(COVERAGE_IGNORE_REGEX)" == "(^|/)(tests|benches|examples|verification)/" ]]; then \
	    ratchet_mode="scoped_exact_ratchet_enforced"; \
	  else \
	    ratchet_mode="portable_adoption_floor_only_cross_target"; \
	  fi; \
	  "$(JQ)" -n \
	    --arg host "$$effective_target" \
	    --arg toolchain "$$toolchain_ver" \
	    --arg rustc_override "$$rustc_env" \
	    --arg rustup_toolchain "$$rustup_tc" \
	    --arg llvm_cov "$$llvm_ver" \
	    --arg ignore "$(COVERAGE_IGNORE_REGEX)" \
	    --arg mode "$$ratchet_mode" \
	    '{schema:"frost-coverage-scope/v1",host_target:$$host,toolchain_version:$$toolchain,rustc_override:$$rustc_override,rustup_toolchain_override:$$rustup_toolchain,tool_version:$$llvm_cov,features:"all-features",ignore_filename_regex:$$ignore,portable_floor:{lines:$(COVERAGE_MIN_LINES),functions:$(COVERAGE_MIN_FUNCTIONS),regions:$(COVERAGE_MIN_REGIONS)},ratchet_mode:$$mode,baseline_reference:"verification/crate-standards-validation/baseline-coverage.json",baseline_target:"aarch64-apple-darwin"}' \
	    >"$(COVERAGE_OUTPUT_DIR)/scope-metadata.json"; \
	  CARGO_TARGET_DIR="$(COVERAGE_TARGET_DIR)" \
	    "$(CARGO)" llvm-cov --workspace --all-features --locked --json --summary-only \
	      --ignore-filename-regex "$(COVERAGE_IGNORE_REGEX)" \
	      --fail-under-lines "$(COVERAGE_MIN_LINES)" \
	      --fail-under-functions "$(COVERAGE_MIN_FUNCTIONS)" \
	      --fail-under-regions "$(COVERAGE_MIN_REGIONS)" \
	      --output-path "$(COVERAGE_OUTPUT_DIR)/source-summary.json"; \
	  if [[ $$ratchet_mode == scoped_exact_ratchet_enforced ]]; then \
	    "$(JQ)" -e --slurpfile base verification/crate-standards-validation/baseline-coverage.json \
	      '($$base[0].totals.lines.percent | type == "number") and ($$base[0].totals.functions.percent | type == "number") and ($$base[0].totals.regions.percent | type == "number") and (.data[0].totals.lines.percent | type == "number") and (.data[0].totals.functions.percent | type == "number") and (.data[0].totals.regions.percent | type == "number") and (.data[0].totals.lines.percent >= $$base[0].totals.lines.percent) and (.data[0].totals.functions.percent >= $$base[0].totals.functions.percent) and (.data[0].totals.regions.percent >= $$base[0].totals.regions.percent)' \
	      "$(COVERAGE_OUTPUT_DIR)/source-summary.json" >/dev/null; \
	  fi

coverage-branch:
	"$(CARGO)" llvm-cov --version
	mkdir -p "$(COVERAGE_OUTPUT_DIR)"
	CARGO_TARGET_DIR="$(COVERAGE_TARGET_DIR)" \
	  "$(CARGO)" "+$(MIRI_TOOLCHAIN)" llvm-cov --branch --workspace --all-features --locked --json --summary-only \
	    --ignore-filename-regex "$(COVERAGE_IGNORE_REGEX)" \
	    --output-path "$(COVERAGE_OUTPUT_DIR)/branch-summary.json"

mutants:
	"$(CARGO)" mutants --version
	"$(CARGO)" mutants --config "$(MUTANTS_CONFIG)" --output "$(MUTANTS_OUTPUT_DIR)"

fuzz-check:
	"$(CARGO)" test --manifest-path verification/fuzz/Cargo.toml \
	  --target-dir "$(FUZZ_TARGET_DIR)" --lib --locked

fuzz-smoke: fuzz-check
	"$(CARGO)" fuzz --version
	mkdir -p "$(FUZZ_WORK_CORPUS_DIR)/$(FUZZ_TARGET)"
	"$(CARGO)" "+$(FUZZ_TOOLCHAIN)" fuzz run --fuzz-dir verification/fuzz \
	  --target-dir "$(FUZZ_TARGET_DIR)" "$(FUZZ_TARGET)" \
	  "$(FUZZ_WORK_CORPUS_DIR)/$(FUZZ_TARGET)" \
	  "verification/fuzz/corpus/$(FUZZ_TARGET)" -- \
	  -runs="$(FUZZ_RUNS)" -max_total_time="$(FUZZ_MAX_TIME)"

catalog:
	"$(CARGO)" test --locked --manifest-path verification/catalog/Cargo.toml \
	  --target-dir "$(CATALOG_TARGET_DIR)"
	"$(CARGO)" run --locked --manifest-path verification/catalog/Cargo.toml \
	  --target-dir "$(CATALOG_TARGET_DIR)" -- check --root .

# Each package with benchmark targets keeps its inventory under its own
# verification/metadata, with target names relative to the package.
catalog-inventory:
	@set -euo pipefail; \
	mkdir -p "$(CATALOG_INVENTORY_DIR)"; \
	metadata=$$("$(CARGO)" metadata --format-version 1 --no-deps --locked); \
	benches=$$(printf '%s\n' "$$metadata" | "$(JQ)" -er \
	  '[.packages[] | . as $$package | .targets[] | select(.kind | index("bench")) | \
	    [$$package.name, $$package.manifest_path, .name] | @tsv] | \
	   if length == 0 then error("no bench targets found") else .[] end'); \
	while IFS=$$'\t' read -r package manifest bench; do \
	  bench=$${bench%$$'\r'}; \
	  list_file="$(CATALOG_INVENTORY_DIR)/$$bench.list"; \
	  FROST_BENCH_LIST=1 "$(CARGO)" bench -p "$$package" --locked --all-features --bench "$$bench" >"$$list_file"; \
	  "$(CARGO)" run --locked --manifest-path verification/catalog/Cargo.toml \
	    --target-dir "$(CATALOG_TARGET_DIR)" -- inventory "benches/$$bench.rs" "$$list_file" \
	    --root "$$(dirname "$$manifest")"; \
	done <<< "$$benches"

shell-check:
	"$(SHELLCHECK)" --shell=bash --severity=info scripts/check-doc-links.sh scripts/require-tests.sh "$(AB_HARNESS_DIR)/prepare-baseline.sh" verification/shell/*.bats
	"$(SHFMT)" -ln bash -d -i 2 -ci -bn scripts/check-doc-links.sh scripts/require-tests.sh "$(AB_HARNESS_DIR)/prepare-baseline.sh"
	"$(SHFMT)" -ln bats -d -i 2 -ci -bn verification/shell/*.bats
	+$(MAKE) shell-test

shell-test:
	FROST_TEST_BASH="$(SHELL)" FROST_TEST_MAKE="$(MAKE)" "$(BATS)" verification/shell

# Same-executable timing comparison; see verification/integer-optimizations.
# Run it alone on a quiet host. The output is CSV on stdout.
integer-ab:
	@"$(CARGO)" --version >&2
	@git --version >&2
	@tar --version >&2
	@"$(SHELL)" "$(AB_HARNESS_DIR)/prepare-baseline.sh" "$(AB_BASELINE)" >&2
	@"$(CARGO)" build --release --locked --manifest-path "$(AB_HARNESS_DIR)/Cargo.toml" >&2
	@"$(AB_HARNESS_DIR)/target/release/frost-integer-optimizations-ab" "$(AB_ROUNDS)"

tools-check:
	"$(AST_GREP)" test --config sgconfig.yml
	"$(AST_GREP)" scan --config sgconfig.yml --inspect summary crates

search-index:
	tgrep index --hidden --exclude .git --exclude .idea .

clean:
	"$(CARGO)" clean

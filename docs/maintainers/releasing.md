# Release procedure

This revision prepares the crate for release review; it does not publish it.
`publish = false` is intentional until the owner supplies release metadata and
accepts the completed evidence. Keep this distinct from a compilation failure.

## Required evidence

1. Choose a clean candidate commit and record Rust, nightly diagnostic version,
   targets, lockfile hash, and platform. Run `make verify` for the host matrix.
2. Run `make embedded` and `make miri`. Review
   warnings and filtered tests; never count a filtered case as a pass. Install
   both default embedded targets, `thumbv7em-none-eabihf` and `thumbv6m-none-eabi`,
   to check integer storage with and without pointer atomics. The Miri target
   includes the bounded Dense/Chunked regressions and the 32-bit integer lane.
3. Run `make bounded` and `make loom`. For any
   symbolic-proof claim, run the Kani harnesses with a compatible pinned tool
   and retain solver results and unwind assertions. An unavailable or
   incompatible tool is an unresolved gate for that claim, not a successful run.
4. Run a current RustSec audit against the committed lockfile. Record the
   database revision, investigate advisories, and explicitly document any
   accepted exception. Do not substitute a stale offline database for a current
   audit without stating its age.
5. Run `make package`, inspect the packaged file list, and test
   the extracted package. Confirm runtime dependencies exclude proof tooling.
6. Compare performance with the agreed baseline on supported targets. Include
   balanced and skewed cases, tiny collections, allocations, long-lived snapshot
   readers, and representative application workloads. Performance results are
   workload evidence, not a universal release certification.
7. Check public API changes, compile-fail contracts, feature combinations,
   changelog, docs links, licensing, and migration notes. The normal checks cover
   individual integrations and their union, not the entire feature powerset.
   Review the supported host CI results separately from local results. Changes
   to Make recipes or shell helpers also require `make shell-check`.

## Owner decisions before distribution

Confirm the canonical repository/documentation URLs, maintainers, supported target
matrix, version/support policy, and the private security channel described in
[SECURITY.md](../../SECURITY.md). A source import does not establish a registry
publishing or support policy. Enable the required CI jobs and branch protection
in the actual host repository. Review license compatibility for the features you
distribute.

Choose the release version for these unreleased APIs and update Cargo/changelog
consistently. Only then deliberately enable publishing, verify `cargo publish
--dry-run`, and use the owner's normal authenticated release process. Never put
registry credentials, private data, or a production publishing action into the
benchmark/test archive.

## Evidence record

Record each executed gate with its command, exit status, meaningful case count,
log location, and known limits. The
[integration validation](../revision2-integration-validation.md) describes the
reviewed source and its original script-based execution; the
[GitHub import and Make checks](../../verification/revision2-integration/github-import-ci/README.md)
describe the later PR checkout. Preserve those scopes when adding release
evidence. Do not overwrite historical evidence or relabel earlier numbers as
fresh results. Store the source fingerprint alongside benchmark and test output
so a future maintainer can identify the exact implementation.

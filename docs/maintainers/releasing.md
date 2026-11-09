# Release procedure

This revision prepares the crate for release review; it does not publish it.
`publish = false` is intentional until the owner supplies release metadata and
accepts the completed evidence. Keep this distinct from a compilation failure.

## Required evidence

1. Choose a clean candidate commit and record Rust, nightly diagnostic version,
   targets, lockfile hash, and platform. Run `./scripts/verify.sh all`.
2. Run `./scripts/verify.sh embedded` and `./scripts/verify.sh miri`. Review
   warnings and filtered tests; never count a filtered case as a pass.
3. Run `./scripts/verify-formal.sh bounded` and the Loom protocol model. For any
   symbolic-proof claim, run the Kani harnesses with a compatible pinned tool
   and retain solver results and unwind assertions. An unavailable or
   incompatible tool is an unresolved gate for that claim, not a successful run.
4. Run a current RustSec audit against the committed lockfile. Record the
   database revision, investigate advisories, and explicitly document any
   accepted exception. Do not substitute a stale offline database for a current
   audit without stating its age.
5. Run `./scripts/verify.sh package`, inspect the packaged file list, and test
   the extracted package. Confirm runtime dependencies exclude proof tooling.
6. Compare performance with the agreed baseline on supported targets. Include
   balanced and skewed cases, tiny collections, allocations, long-lived snapshot
   readers, and representative application workloads. Performance results are
   workload evidence, not a universal release certification.
7. Check public API changes, compile-fail contracts, feature combinations,
   changelog, docs links, licensing, and migration notes. The normal checks cover
   individual integrations and their union, not the entire feature powerset.

## Owner decisions before distribution

Set the canonical repository/documentation URLs, maintainers, supported target
matrix, version/support policy, and the private security channel described in
[SECURITY.md](../../SECURITY.md). These facts cannot be inferred from an attached
archive. Enable the required CI jobs and branch protection in the actual host
repository. Review license compatibility for the features you distribute.

Choose the release version for these unreleased APIs and update Cargo/changelog
consistently. Only then deliberately enable publishing, verify `cargo publish
--dry-run`, and use the owner's normal authenticated release process. Never put
registry credentials, private data, or a production publishing action into the
benchmark/test archive.

## Evidence record

Update [current validation](../integer-output-validation.md) with each executed gate,
command, exit status, meaningful case count, log location, and known limits.
Do not overwrite historical evidence or relabel the previous implementation's
numbers as fresh results. Store the source fingerprint alongside benchmark and
test output so a future maintainer can identify the exact implementation.

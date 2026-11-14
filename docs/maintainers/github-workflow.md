# GitHub intake and Beads

GitHub issue forms collect public reports, proposals, and research or verification
requests. They help people provide enough context for triage; they do not establish
that a reported behavior or verification result has been confirmed. Contributors
can use a blank issue when the forms do not fit. Do not put secrets or private
production data in issues or pull requests. For suspected vulnerabilities, follow
[SECURITY.md](../../SECURITY.md); it describes the current limits of private
reporting and does not promise a monitored contact.

Beads is the authoritative tracker for project work, dependencies, and status.
During triage, maintainers connect relevant GitHub issues and pull requests to
their Beads items and preserve links in both directions where available. A
GitHub issue or PR alone does not establish Beads status. Include acceptance
criteria and scenario or tag links where useful, and distinguish observed
evidence from proposed work and unrun checks.

The committed `.beads/issues.jsonl` is a portable public export; database files
and runtime state stay local. Use a public actor name for tracker mutations and
check creator fields on issues and dependency records before publishing an
export. Legacy private creator addresses are recorded as `[redacted]`; do not
publish machine-specific paths. An export from an older local database can
restore historical creator values, so repeat this check after exporting.

The [crate standard](../../CRATES.md) is canonical for layout, dependency,
coverage, and test metadata rules. Frost is a Cargo workspace: the library
`frost-collections` re-exports the internal packages `frost-collections-core`,
`-integer` and `-concurrent`. The standards and testing epic is `frost-wou`;
the crate reorganization epic is `frost-wlt`.

PR descriptions should explain the concrete problem and behavior, scope, affected
public APIs and feature or target profiles, contract and documentation changes,
checks actually run, and remaining limitations. Include relevant metadata or
catalog IDs and links to related work where available.
For performance work, follow the [benchmark guidance](benchmarks.md) and retain
workload details, native controls, raw observations, and slower results. Use the
[verification guide](../verification.md) for the applicable gates; a form or a
successful structural check is not evidence that those gates passed.

GitHub enables these templates after they reach the repository's default branch.
See GitHub's [issue form syntax](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms),
[form schema](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-githubs-form-schema),
[template chooser configuration](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/configuring-issue-templates-for-your-repository),
and [PR template documentation](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/creating-a-pull-request-template-for-your-repository).

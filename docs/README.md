# Frost documentation

Start with the [crate README](../README.md) for API examples and features.

| Document | Purpose |
| --- | --- |
| [Design philosophy](design/philosophy.md) | Durable principles and review criteria |
| [Architecture](design/architecture.md) | Families, capabilities, profiles, and backend boundaries |
| [Design decisions](design/decisions.md) | Why the original architecture was selected |
| [Snapshot publication](design/snapshots.md) | Ownership, identity, retries, and linearization |
| [Typed integer profile](design/integer-sets.md) | Canonical key codecs, private storage, policies, and encoded algebra |
| [Integer adversarial review](reviews/integer-design-audit.md) | Design risks, corrections, and scope decisions |
| [Integer output review](reviews/integer-output-review.md) | Disposition of the supplied follow-up findings and operation contracts |
| [Current validation](integer-output-validation.md) | Executed output-policy, feature, concurrency, and verification gates |
| [Current integer benchmarks](integer-output-benchmarks.md) | Same-harness baseline/candidate comparisons and CPU target controls |
| [Integer roadmap](design/integer-roadmap.md) | Proposed future families and their acceptance criteria |
| [Lazy views research review](reviews/lazy-views-adversarial-review.md) | Adversarial findings, corrected source claims, and design decisions |
| [Borrowed integer expressions](design/integer-views.md) | Proposed IntSetExpr API, bounds, cursors, evaluation, and admission tests |
| [Persistent integer storage](design/persistent-integer-storage.md) | Separate proposed ownership profile, sharing costs, and snapshot invariants |
| [Search-source audit](reviews/lazy-views-source-audit.md) | Versioned Rust/FST/Lucene/Tantivy contracts and checked issue/PR statuses |
| [Views performance audit](reviews/lazy-views-performance-audit.md) | Paper evidence, experiment limits, and future benchmark design |
| [Expression bounds model](../verification/integer-views/README.md) | Executed finite mathematics checks; no production views implementation |
| [Initial integer validation](integer-validation.md) | Historical evidence before output-policy corrections |
| [Initial integer benchmarks](integer-benchmarks.md) | Historical representation tradeoffs and mixed-probe improvement |
| [Set operations](set-operations.md) | Algebra, representatives, complexity, and bitmap universes |
| [General set algebra](design/set-algebra.md) | Proposed typed views for all set categories, representative semantics, execution, and ownership |
| [Generic algebra review](reviews/generic-set-algebra-review.md) | Coverage gaps, research iteration, baseline checks, and finite semantic evidence |
| [Generic algebra model](../verification/set-algebra/README.md) | Reproduced representative/order counterexamples; no production view implementation |
| [Maintainer guide](maintainers/guide.md) | Code map, invariants, and change-specific checks |
| [Benchmarks](maintainers/benchmarks.md) | Fair measurement and reproduction |
| [Verification](verification.md) | Executable models, proof bounds, and limitations |
| [Release procedure](maintainers/releasing.md) | Concrete gates before publishing |
| [Integration review](reviews/incoming-assessment.md) | Disposition of the attached patch and review claims |
| [Prior integration validation](production-readiness.md) | Historical checks before the integer profile was added |
| [Previous performance review](reviews/performance-review.md) | Earlier kernel improvements and raw evidence |

Historical review and verification documents under `reviews/` preserve earlier
findings. The original attached review and patch under `reviews/supplied/` are
unmodified evidence; their claims are not automatically current project policy.

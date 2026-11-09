# Security and supported releases

This source distribution is an unreleased 0.4.0 development revision with
`publish = false`. No maintained release branch, public repository owner, or
private reporting address was supplied with the project. This document does
not invent a support commitment or a monitored contact.

Before publishing, the project owner must configure a monitored private
security channel, document supported versions and response expectations, and
replace this paragraph with those details. If a canonical repository has
private security advisories enabled, use that private channel for a suspected
memory-safety or exploitable denial-of-service issue. Do not include secrets or
private production data in a reproducer.

## Scope for security review

- The inline buffer's initialized-prefix, ownership-transfer, and drop
  invariants are the crate's local unsafe-code boundary.
- Untrusted keys can supply expensive hashing, equality, ordering, and drop
  implementations. Hashers are deliberately preserved; users choose a hasher
  appropriate for their inputs. No bounded-latency guarantee covers arbitrary
  user callbacks or destructors.
- Decoders follow their documented duplicate and ordering rules. Use strict
  decoding helpers when canonical input is required; caller resource limits
  remain necessary for unbounded serialized collections.
- Snapshot publication protects collection structure and version lifetime;
  it does not make shared interior state immutable. Retaining old versions
  retains their memory. Update callbacks may run repeatedly under contention.
- Live backend operations retain their native locking/reentrancy restrictions.
  Dependencies' atomics and reclamation are not formally proved by Frost's
  wrapper models.

The dependency audit CI job checks the committed lockfile against the RustSec
database. A current successful audit is a release gate; the presence of a
workflow is not evidence that it has run. Update the database and record its
revision when assessing a release.

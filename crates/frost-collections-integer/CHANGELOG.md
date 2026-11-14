# Changelog: frost-collections-integer

Changes to this package. Entries from before the package existed are in the
[repository changelog](../../CHANGELOG.md).

## Unreleased

- Created by splitting `frost-collections` under tracker epic `frost-wlt`.
  The typed integer-set profile (the `integer` module) moved into this
  package.
- No public path under `frost::` changed.
- Fuzz oracle: an empty result under an `Auto` output policy may be an empty
  operand's storage, with that operand's representation. New seed
  `seed-empty-dense-operands` and a replay test. No library change.

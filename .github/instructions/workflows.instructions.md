---
applyTo: ".github/workflows/**/*.yml"
---

- Jobs run on Linux runners only. macOS and Windows runners are disabled for
  now because of their cost.
- `make miri` runs only in the nightly and release tiers of
  `verification.yml`, never on push or pull request.
- Run build and verification steps through `make` targets.
- Keep `permissions: contents: read`, pin each action by commit SHA, and give
  every job a `timeout-minutes`.

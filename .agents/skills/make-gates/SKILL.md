---
name: make-gates
description: Runs the Makefile gates that the current change calls for and reports the results as they are. Use before review or when asked to verify a change.
---

Run the repository's gates for what changed and report the result as it is.

1. Run `make check` first. It formats, lints, tests, builds the documents, and
   checks links, the test catalog and the benchmark inventory.
2. Add the gates the change calls for:
   - Cargo features or manifests changed: `make profiles` and `make features`.
   - A shell script, the Makefile or a `.bats` file changed: `make shell-check`.
   - The fuzz package or its seeds changed: `make fuzz-check`.
   - The integer package changed: `make integers`.
3. Do not run `make miri`, `make kani`, `make mutants` or `make fuzz-smoke`
   unless the maintainer asks for that lane.
4. Report every gate you ran with its result. Say which gates you did not run
   and why. Do not report a pass you did not see.

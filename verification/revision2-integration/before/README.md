# Before-integration evidence

These logs capture input provenance, environment setup, reviewed-main and supplied-revision2 checks, and the initial pointer-atomic portability reproducer. They are historical inputs to this integration, not final-source validation results.

`baseline-integers.log` passed 18 integer-output and 23 integer-set tests on reviewed main. `revision2-integers.log` passed 30 supplied revision-2 integer-set tests. `baseline-thumbv6m.log` passed, while `revision2-thumbv6m.log` reproduces the unconditional `alloc::sync::Arc` build regression. `integrated-thumbv6m.log` is an early working-tree check with warnings; use the parent directory's final validation logs for the integrated source.

The source tree reconstructed during provenance analysis is intentionally not included here. The attached input archive hashes are preserved in `input-provenance.json`. Environment-setup logs demonstrate tool availability, not successful Miri execution of this project's tests.

# Integration patch

`revision2-integration-code.diff` is the complete code, test, benchmark, and
verification-script delta against the supplied reviewed main archive. It applies
once to that reviewed input. It is not based on the older parent of the incoming
revision-2 diffs. The complete source archive also includes merged design docs,
original supplied reviews, and verification evidence; those are not duplicated
inside this code-only patch.

Apply from the reviewed repository root:

```sh
git apply --check patches/revision2-integration-code.diff
git apply patches/revision2-integration-code.diff
```

If applying to an unpacked input, place the patch there first. The integrated
archive already has these changes; do not apply the patch to it again.

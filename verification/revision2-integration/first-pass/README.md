# First integration measurement

These are complete, unmodified measurements of the first merged candidate.
They exposed an avoidable native Roaring `optimize()` call after tiny cloned
updates. The final candidate optimizes native binary results at the native
kernel boundary, preserving main's direct small-update behavior.

`integer/storage.rs` and `integer/storage/word.rs` retain the exact two affected
source files before that adjustment. Other production source files did not
change for the adjustment. Original raw samples and slower observations are
retained; current measurements are in the parent directory.

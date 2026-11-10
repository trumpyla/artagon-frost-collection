# Additional public API obligations

The source coverage baseline in [baseline-coverage.json](baseline-coverage.json)
identified weak execution coverage in mutable vectors, ordered map entry APIs,
and native capability implementations. [api_contracts.rs](../../tests/api_contracts.rs)
adds the following independent contract checks. These checks do not establish
complete public API coverage.

| Test selector | Obligation | Oracle and bound |
| --- | --- | --- |
| `mutable_vec_exhausts_bounded_operation_traces` | Mutation ordering, return values, partial drain, split/append, and freeze/thaw preserve contents | `std::vec::Vec`; five initial vectors and all 16³ three-operation traces (20,480 traces), checking every prefix |
| `mutable_vec_capacity_failure_preserves_contents` | Successful reservation supplies room; capacity overflow leaves contents and capacity intact | Capacity lower bounds and fixed expected contents; non-zero-sized `u64` with `usize::MAX` additional capacity forces overflow without attempting an enormous allocation |
| `mutable_vec_partial_consumption_drops_each_non_clone_value_once` | Partial drain and partial owning iteration neither leak nor double-drop transferred elements | Eight distinct non-Clone values with per-value drop counters, checked before and after each ownership boundary |
| `ordered_entry_preserves_representatives_and_lazy_callbacks` | Equivalent entries retain the stored key; default callbacks run only when vacant; removal returns the stored representative | Keys compare by borrowed string but carry distinct observable representative tags; explicit callback counts and fixed values |
| `ordered_mutation_ranges_and_named_iterators_match_btree` | Mutable values, ordered ranges, exact remaining iterator counts, fusion, and removals agree with the ordered contract | `std::collections::BTreeMap` plus repeated exhaustion checks; fixed four-key domain |
| `borrowed_map_capabilities_preserve_stored_keys` | Generic insertion retains the first key and last value; borrowed lookup and removal expose the stored key | Explicit representative tags and return values across the ordered family and BTreeMap; hash family and HashMap additionally run with `std` |

All six tests require `alloc`. The test harness uses the host standard library,
while the library can be built with only its `alloc` feature. No test support
dependency enters the library graph. No test uses sleeps or timing assertions.

The finite trace domain does not cover arbitrary lengths, every mutator,
panic-unwinding behavior, allocator failures, or all element types. The entry
tests do not establish the behavior of every family. Other feature combinations,
embedded targets, mutation scores, and symbolic or scheduler proofs require
their own evidence. Source line/function coverage is an execution measure and
cannot substitute for this obligation map.

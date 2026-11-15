# Downstream integer-key compilation fixture

This isolated Cargo package compiles against Frost as a path dependency with
only the `int-set` profile enabled. It implements application `UserId`, `OrderId`,
and signed `Timestamp` keys outside Frost, then exercises generic construction,
associated storage-word inference, Send/Sync inference, value capabilities,
freeze plans, and mixed representation algebra. Its compile-fail doctest checks
that two domains sharing u64 storage cannot be combined.

```sh
cargo +1.99.1 test --manifest-path verification/int-key-downstream/Cargo.toml
```

After initial dependency resolution, retain the isolated Cargo.lock and rerun
with `--locked`. This fixture does not add a normal library dependency or a new
public backend type.

# Verus proofs of the algebra laws

[`src/lib.rs`](src/lib.rs) proves collection-algebra laws from
[`docs/design/algebra`](../../docs/design/algebra/README.md) with
[Verus](https://github.com/verus-lang/verus), over vstd's finite `Set`, `Map` and
`Seq` and over natural-number counts. It is the spec-level layer of the
[formal validation tools report](../../docs/research/reports/formal-validation-tools.md).

These proofs are about the reference definitions. Nothing here links to Frost,
and no Frost type is shown to implement a law by them. The links to real code
are the bounded Kani harnesses in [`verification/kani`](../kani/src/set_laws.rs)
and the finite models in [`tests/formal_models`](../../crates/frost-collections/tests/formal_models/main.rs).

| Law | Proof |
| --- | --- |
| `SET-BOOLEAN` | Associativity, commutativity, idempotence, absorption, both distributive laws, symmetric difference |
| `SET-REP` | Left-representative union and intersection associate; union keeps the left object |
| `DOM-IMAGE` | Image preserves union and maps intersection into the intersection of images; preimage preserves union, intersection and difference |
| `BAG-LAWS`, `BAG-SUPPORT` | Pointwise count laws and the identities tying min, monus, max and absolute difference |
| `COUNT-PARTIAL` | Both groupings of a bounded checked sum succeed exactly when the total fits |
| `SEQ-MONOID` | Concatenation is associative with the empty sequence as unit |
| `BIMAP-PARTIAL` | Downward closure, and checked union succeeds in either grouping exactly when the whole union is a partial bijection |
| `KNOW-BOUNDS` | The union, intersection and difference transfer rules are sound |

Not proved here: `REL-COMP` and `BAGREL-COMP`, `MAP-LIFT`, `BIMAP-CLOSED` and
`BIMAP-IDENTITY`, `DELTA-REPLAY`, the necessity directions of `DOM-IMAGE`, and
the domain laws.

## Running

Install the pinned release (`VERUS_VERSION` in the root `Makefile`) from the
Verus releases page, put its `verus` on `PATH`, and run:

```sh
make verus
```

The target fails when Verus is absent or is a different release. The release
bundles vstd and Z3 and uses Rust 1.99.1, the crate's pinned toolchain. The file
is not a Cargo package, so Cargo never builds it and it adds no dependency.

The catalog tool does not discover proof functions inside `verus!`, so these
proofs have no case records. `docs/design/algebra/catalog.json` lists them per
law under `verus`.

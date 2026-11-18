# frost-interval

frost-interval provides value domains with order-preserving encodings, and
interval sets and maps over them. A value type declares its domain at the type
level: its extrema, its density, and an order-preserving encoding into a word.
The sets and maps store words only, so every domain with the same word type
shares one compiled kernel.

The package is `no_std`, always uses `alloc`, and forbids unsafe code. It
depends on no other Frost package. The facade re-exports it as
`frost::interval` behind the feature `interval`.

## Contents

- Traits: `Domain`, `OrderedDomain`, `DiscreteDomain`, `CoverPreserving`,
  `Ranked`, `CyclicDomain`, `FiniteDomain`, `PrefixDomain`, `Poset`,
  `Lattice`, `BoundedLattice`.
- `Cut` and `Interval`, with canonical cuts, and `IntoInterval` for every std
  range form.
- `FrozenIntervalSet`, `FrozenIntervalMap`, and the mutable `IntervalSet`.
- `FiniteSet`, `DownSet`, `LatticeInterval`, `ProductRegion` and
  `CyclicInterval`, with the `Region`, `Convex` and `BooleanRegion` traits.
- Domains for the integer types, `NonZero`, `char`, `Ipv4Addr`, floats as
  `NotNanF32` and `NotNanF64`, `Duration`, and, with `std`, `SystemTime` and
  `Instant`. `newtype_domain!` gives a newtype the domain of its field.

## Features

| Feature | Adds |
| --- | --- |
| `std` (default) | The `SystemTime` and `Instant` domains |
| `chrono-0_4` | chrono date and time domains |
| `jiff-0_2` | jiff date, time, timestamp and duration domains |
| `time-0_3` | time-crate date, time and duration domains |

## Design

The [design record](../../docs/design/proposals/frozen-box.md) gives the
rules, the traits, the kernel, the evidence, and the open items.

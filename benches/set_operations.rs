//! Small dependency-free comparison of scalar set kernels with native APIs.
//!
//! Run `cargo bench --bench set_operations --all-features --locked`.
//! Construction is outside timing. The hash baseline scans the smaller side,
//! as the standard implementation does. Hash tables share one deterministic
//! hasher. See `performance_review` for the larger workload matrix.

mod support;

use frost::backend::{fixedbitset::FixedBitSet, roaring::RoaringBitmap};
use frost::prelude::*;
use std::collections::{BTreeSet, HashSet};
use std::hash::{BuildHasherDefault, DefaultHasher};
use std::hint::black_box;
use support::{as_u64, measure};

type Hasher = BuildHasherDefault<DefaultHasher>;

fn main() {
    support::header();
    for size in [128_u32, 4096, 65_536] {
        let left_std: BTreeSet<_> = (0..size).map(|value| value * 2).collect();
        let right_std: BTreeSet<_> = (0..size).map(|value| value * 3).collect();
        let expected = as_u64(left_std.intersection(&right_std).count());
        let left: FrozenOrderedSet<_> = left_std.iter().copied().collect();
        let right: FrozenOrderedSet<_> = right_std.iter().copied().collect();
        measure(&format!("ordered.{size}.frost_count"), expected, || {
            as_u64(black_box(&left).intersection_len(black_box(&right)))
        });
        measure(&format!("ordered.{size}.std_count"), expected, || {
            as_u64(
                black_box(&left_std)
                    .intersection(black_box(&right_std))
                    .count(),
            )
        });
    }

    let mut large_std: HashSet<_, Hasher> = (0..65_536_u32).collect();
    let mut small_std: HashSet<_, Hasher> = (0..64_u32).map(|value| value * 1024).collect();
    large_std.shrink_to_fit();
    small_std.shrink_to_fit();
    let large: FrozenHashSet<_, Hasher> = large_std.clone().into();
    let small: FrozenHashSet<_, Hasher> = small_std.clone().into();
    let common = as_u64(large_std.intersection(&small_std).count());
    measure("hash.skew_65536_64.frost_count", common, || {
        as_u64(black_box(&large).intersection_len(black_box(&small)))
    });
    measure("hash.skew_65536_64.std_count", common, || {
        as_u64(
            black_box(&large_std)
                .intersection(black_box(&small_std))
                .count(),
        )
    });

    let left_native: RoaringBitmap = (0..262_144_u32).step_by(2).collect();
    let right_native: RoaringBitmap = (0..262_144_u32).step_by(3).collect();
    let common = left_native.intersection_len(&right_native);
    let left = left_native.clone().seal();
    let right = right_native.clone().seal();
    measure("roaring.frost_count", common, || {
        black_box(&left).intersection_len(black_box(&right))
    });
    measure("roaring.native_count", common, || {
        black_box(&left_native).intersection_len(black_box(&right_native))
    });

    let mut left_native = FixedBitSet::with_capacity(262_144);
    left_native.extend((0..262_144).step_by(2));
    let mut right_native = FixedBitSet::with_capacity(262_144);
    right_native.extend((0..262_144).step_by(3));
    let common = as_u64(left_native.intersection_count(&right_native));
    let left = left_native.clone().seal();
    let right = right_native.clone().seal();
    measure("dense.frost_count", common, || {
        as_u64(black_box(&left).intersection_len(black_box(&right)))
    });
    measure("dense.native_count", common, || {
        as_u64(black_box(&left_native).intersection_count(black_box(&right_native)))
    });
}

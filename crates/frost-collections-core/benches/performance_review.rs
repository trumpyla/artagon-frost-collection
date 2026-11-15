//! Reproducible workload matrix for the performance review.
//!
//! Run on both source revisions with the same compiler and flags. Native
//! baselines use the same values; hash baselines also use a deterministic
//! hasher so randomized table layout does not dominate comparisons. This
//! benchmark has warm inputs, excludes input construction, and reports seven samples.
//! It does not calibrate adaptive representation thresholds or cold-cache cost.

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

use frost_collections_test_support::bench as support;

use frost::adaptive::{AdaptiveHash, AdaptiveHashMap, AdaptiveOrdered, AdaptiveOrderedMap};
use frost::backend::{fixedbitset::FixedBitSet, roaring::RoaringBitmap};
use frost::prelude::*;
use std::collections::{BTreeSet, HashSet};
use std::hash::{BuildHasherDefault, DefaultHasher};
use std::hint::black_box;
use support::{as_u64, measure};

fn ordered_case(name: &str, left_values: Vec<u32>, right_values: Vec<u32>) {
    let native_left: BTreeSet<_> = left_values.iter().copied().collect();
    let native_right: BTreeSet<_> = right_values.iter().copied().collect();
    let expected = as_u64(native_left.intersection(&native_right).count());
    let checksum: u64 = native_left
        .intersection(&native_right)
        .map(|&value| u64::from(value))
        .sum();
    let left: FrozenOrderedSet<_> = left_values.into_iter().collect();
    let right: FrozenOrderedSet<_> = right_values.into_iter().collect();
    measure(&format!("ordered.{name}.frost_count"), expected, || {
        as_u64(black_box(&left).intersection_len(black_box(&right)))
    });
    measure(
        &format!("ordered.{name}.frost_reverse_count"),
        expected,
        || as_u64(black_box(&right).intersection_len(black_box(&left))),
    );
    measure(&format!("ordered.{name}.frost_iter"), checksum, || {
        black_box(&left)
            .intersection(black_box(&right))
            .map(|&value| u64::from(value))
            .sum()
    });
    measure(&format!("ordered.{name}.frost_next"), checksum, || {
        let mut sum = 0;
        for &value in black_box(&left).intersection(black_box(&right)) {
            sum += u64::from(value);
        }
        sum
    });
    measure(&format!("ordered.{name}.std_btree_count"), expected, || {
        as_u64(
            black_box(&native_left)
                .intersection(black_box(&native_right))
                .count(),
        )
    });
    let (small, large) = if left.len() <= right.len() {
        (&left, &right)
    } else {
        (&right, &left)
    };
    measure(
        &format!("ordered.{name}.probe_small_count"),
        expected,
        || {
            as_u64(
                black_box(small)
                    .iter()
                    .filter(|item| black_box(large).contains(*item))
                    .count(),
            )
        },
    );
    let subset = u64::from(if native_left.len() <= native_right.len() {
        native_left.is_subset(&native_right)
    } else {
        native_right.is_subset(&native_left)
    });
    measure(&format!("ordered.{name}.frost_subset"), subset, || {
        u64::from(black_box(small).is_subset(black_box(large)))
    });
}

fn hash_cases() {
    type Hasher = BuildHasherDefault<DefaultHasher>;
    for size in [128_u32, 65_536] {
        let mut native: HashSet<u32, Hasher> = HashSet::with_hasher(Hasher::default());
        native.extend(0..size);
        native.shrink_to_fit();
        let frozen: FrozenHashSet<_, Hasher> = native.clone().into();
        let mut native_small: HashSet<u32, Hasher> = HashSet::with_hasher(Hasher::default());
        native_small.extend((0..64).map(|value| value * (size / 64)));
        native_small.shrink_to_fit();
        let small: FrozenHashSet<_, Hasher> = native_small.clone().into();
        measure(
            &format!("hash.{size}.frost_iter_count"),
            u64::from(size),
            || as_u64(black_box(&frozen).iter().count()),
        );
        measure(
            &format!("hash.{size}.std_iter_count"),
            u64::from(size),
            || as_u64(black_box(&native).iter().count()),
        );
        measure(&format!("hash.{size}.frost_intersection"), 64, || {
            as_u64(black_box(&frozen).intersection_len(black_box(&small)))
        });
        measure(&format!("hash.{size}.std_intersection"), 64, || {
            as_u64(
                black_box(&native)
                    .intersection(black_box(&native_small))
                    .count(),
            )
        });
    }
}

fn dense_case(left_len: usize, right_len: usize) {
    let mut native_left = FixedBitSet::with_capacity(left_len);
    native_left.extend((0..left_len).step_by(2));
    let mut native_right = FixedBitSet::with_capacity(right_len);
    native_right.extend((0..right_len).step_by(3));
    // These native baselines intentionally have clean padding. Dirty-padding
    // semantics are tested separately because Frost masks retained padding.
    let intersection = as_u64(native_left.intersection_count(&native_right));
    let union = as_u64(native_left.union_count(&native_right));
    let difference = as_u64(native_left.difference_count(&native_right));
    let symmetric = as_u64(native_left.symmetric_difference_count(&native_right));
    let left = native_left.clone().seal();
    let right = native_right.clone().seal();
    let name = format!("dense.{left_len}_{right_len}");
    measure(&format!("{name}.frost_intersection"), intersection, || {
        as_u64(black_box(&left).intersection_len(black_box(&right)))
    });
    measure(&format!("{name}.native_intersection"), intersection, || {
        as_u64(black_box(&native_left).intersection_count(black_box(&native_right)))
    });
    measure(&format!("{name}.frost_union"), union, || {
        as_u64(black_box(&left).union_len(black_box(&right)))
    });
    measure(&format!("{name}.frost_difference"), difference, || {
        as_u64(black_box(&left).difference_len(black_box(&right)))
    });
    measure(&format!("{name}.frost_symmetric"), symmetric, || {
        as_u64(black_box(&left).symmetric_difference_len(black_box(&right)))
    });
    measure(
        &format!("{name}.frost_materialize_intersection"),
        intersection,
        || {
            black_box(&left)
                .intersection(black_box(&right))
                .cardinality()
        },
    );
}

fn adaptive_cases() {
    for size in [8_u64, 64] {
        let mut hash = AdaptiveHashMap::with_hasher_and_policy(
            BuildHasherDefault::<DefaultHasher>::default(),
            AdaptiveHash::linear_up_to(8),
        );
        let mut ordered = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(32));
        for key in 0..size {
            hash.insert(key, key);
            ordered.insert(key, key);
        }
        measure(&format!("adaptive.hash.{size}.entry_hit"), size - 1, || {
            *black_box(&mut hash).get_or_insert_with(black_box(size - 1), || unreachable!())
        });
        measure(
            &format!("adaptive.ordered.{size}.entry_hit"),
            size - 1,
            || *black_box(&mut ordered).get_or_insert_with(black_box(size - 1), || unreachable!()),
        );
    }
}

fn main() {
    support::header();
    for size in [128_u32, 4096, 65_536] {
        ordered_case(
            &format!("balanced_{size}"),
            (0..size).map(|value| value * 2).collect(),
            (0..size).map(|value| value * 3).collect(),
        );
    }
    ordered_case("skew_tail_1", (0..65_536).collect(), vec![65_535]);
    ordered_case("skew_head_64", (0..65_536).collect(), (0..64).collect());
    ordered_case(
        "skew_tail_64",
        (0..65_536).collect(),
        (65_472..65_536).collect(),
    );
    ordered_case(
        "skew_spread_64",
        (0..65_536).collect(),
        (0..64).map(|value| value * 1024).collect(),
    );
    for ratio in [8_u32, 16, 32] {
        ordered_case(
            &format!("ratio_{ratio}"),
            (0..8192).collect(),
            (0..8192 / ratio).map(|value| value * ratio).collect(),
        );
    }
    ordered_case("disjoint", (0..4096).collect(), (10_000..14_096).collect());
    ordered_case(
        "interleaved_miss",
        (0..4096).map(|value| value * 2).collect(),
        (0..4096).map(|value| value * 2 + 1).collect(),
    );
    hash_cases();
    for (left_len, right_len) in [
        (64, 64),
        (65, 65),
        (4096, 4096),
        (262_144, 262_144),
        (262_147, 262_147),
        (65, 262_147),
        (262_147, 65),
    ] {
        dense_case(left_len, right_len);
    }
    adaptive_cases();
    let native_left: RoaringBitmap = (0..262_144).step_by(2).collect();
    let native_right: RoaringBitmap = (0..262_144).step_by(3).collect();
    let expected = native_left.intersection_len(&native_right);
    let left = native_left.clone().seal();
    let right = native_right.clone().seal();
    measure("roaring.frost_intersection", expected, || {
        black_box(&left).intersection_len(black_box(&right))
    });
    measure("roaring.native_intersection", expected, || {
        black_box(&native_left).intersection_len(black_box(&native_right))
    });
}

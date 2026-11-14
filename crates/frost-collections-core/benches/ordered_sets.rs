//! Ordered-set workload controls for comparing source revisions.
//!
//! Run `cargo bench --bench ordered_sets --features std --locked`.
//! Run the same file and `support` module in each revision with identical
//! compiler, profile, feature, and CPU settings. Every answer is checked
//! against `BTreeSet` before timing; construction is excluded. Measurements
//! use warm inputs. They do not establish cold-cache or arbitrary-key results.

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

use frost_collections_test_support::bench as support;

use frost::family::FrozenOrderedSet;
use std::collections::BTreeSet;
use std::hint::black_box;
use support::workload::{Class, Emit, Layout, Order, Partition, SetPairSpec, WorkloadSpec};
use support::{as_u64, measure};

/// Factory names include the complete reproducible spec and seed. Construction
/// and the `BTreeSet` oracle stay outside timing as for hand-written cases.
fn factory_case(label: &str, spec: &SetPairSpec) {
    let pair = spec
        .build()
        .and_then(|pair| pair.to_u32())
        .unwrap_or_else(|error| panic!("ordered.{label}: {error}"));
    eprintln!("ordered.{label} := {}", pair.id);
    case(&format!("{label}_{}", pair.id), pair.left, pair.right);
}

fn case(name: &str, left: Vec<u32>, right: Vec<u32>) {
    let left_std: BTreeSet<_> = left.iter().copied().collect();
    let right_std: BTreeSet<_> = right.iter().copied().collect();
    let common = as_u64(left_std.intersection(&right_std).count());
    let checksum = left_std
        .intersection(&right_std)
        .map(|&key| u64::from(key))
        .sum();
    let subset = u64::from(left_std.is_subset(&right_std));
    let disjoint = u64::from(left_std.is_disjoint(&right_std));
    let left: FrozenOrderedSet<_> = left.into_iter().collect();
    let right: FrozenOrderedSet<_> = right.into_iter().collect();

    measure(&format!("ordered.{name}.frost_count"), common, || {
        as_u64(black_box(&left).intersection_len(black_box(&right)))
    });
    measure(
        &format!("ordered.{name}.frost_reverse_count"),
        common,
        || as_u64(black_box(&right).intersection_len(black_box(&left))),
    );
    measure(&format!("ordered.{name}.frost_next"), checksum, || {
        let mut sum = 0;
        for &key in black_box(&left).intersection(black_box(&right)) {
            sum += u64::from(key);
        }
        sum
    });
    measure(&format!("ordered.{name}.frost_fold"), checksum, || {
        black_box(&left)
            .intersection(black_box(&right))
            .fold(0, |sum, &key| sum + u64::from(key))
    });
    measure(&format!("ordered.{name}.frost_subset"), subset, || {
        u64::from(black_box(&left).is_subset(black_box(&right)))
    });
    measure(&format!("ordered.{name}.frost_disjoint"), disjoint, || {
        u64::from(black_box(&left).is_disjoint(black_box(&right)))
    });
    measure(&format!("ordered.{name}.std_count"), common, || {
        as_u64(
            black_box(&left_std)
                .intersection(black_box(&right_std))
                .count(),
        )
    });
    measure(&format!("ordered.{name}.std_subset"), subset, || {
        u64::from(black_box(&left_std).is_subset(black_box(&right_std)))
    });
}

fn main() {
    support::header();
    case("empty", Vec::new(), (0..128).collect());
    case(
        "balanced_65536",
        (0..65_536).map(|key| key * 2).collect(),
        (0..65_536).map(|key| key * 3).collect(),
    );
    case(
        "balanced_interleaved_miss",
        (0..4096).map(|key| key * 2).collect(),
        (0..4096).map(|key| key * 2 + 1).collect(),
    );
    for big in [65_536_u32, 1_048_576] {
        for small in [64_u32, 1024] {
            if big == 65_536 && small == 1024 {
                continue;
            }
            case(
                &format!("spread_{small}_{big}"),
                (0..small).map(|key| key * (big / small) * 2 + 2).collect(),
                (0..big).map(|key| key * 2).collect(),
            );
        }
    }
    case("head_64_65536", (0..64).collect(), (0..65_536).collect());
    case(
        "tail_64_65536",
        (65_472..65_536).collect(),
        (0..65_536).collect(),
    );
    for ratio in [31_u32, 32, 33] {
        case(
            &format!("ratio_{ratio}"),
            (0..256).map(|key| key * ratio).collect(),
            (0..256 * ratio).collect(),
        );
    }
    // Initially equal cardinality, but one side has a long run between two
    // matches. This exposes the tradeoff of selecting strategy by initial size.
    let prefix: Vec<u32> = (0..4096).collect();
    let mut left = prefix.clone();
    left.extend(200_000..204_096);
    let mut right = prefix;
    right.extend(4096..8191);
    right.push(200_000);
    case("balanced_then_long_gap", left, right);

    case("separated", (0..4096).collect(), (8192..12_288).collect());
    factory_cases();
    let same: FrozenOrderedSet<_> = (0..65_536_u32).collect();
    measure("ordered.same_storage.frost_count", 65_536, || {
        as_u64(black_box(&same).intersection_len(black_box(&same)))
    });
}

/// Seeded shapes the hand-written cases cannot express: sparse uniform skew,
/// run/gap blocks, and shuffled duplicate-bearing input.
fn factory_cases() {
    factory_case(
        "factory_uniform_skew",
        &SetPairSpec::new(
            1,
            65_536,
            64,
            32,
            Layout::Uniform {
                base: 0,
                span: 1 << 18,
            },
        ),
    );
    factory_case(
        "factory_runs_blocks",
        &SetPairSpec::new(
            2,
            4096,
            4096,
            1024,
            Layout::Runs {
                base: 0,
                run: 256,
                gap: 768,
            },
        )
        .with_partition(Partition::Blocks([
            Class::LeftOnly,
            Class::Shared,
            Class::RightOnly,
        ])),
    );
    factory_case(
        "factory_shuffled_duplicates",
        &SetPairSpec::new(
            3,
            8192,
            8192,
            4096,
            Layout::Uniform {
                base: 0,
                span: 1 << 20,
            },
        )
        .with_partition(Partition::Shuffled)
        .with_emit(
            Emit {
                order: Order::Shuffled,
                duplicates: 1024,
            },
            Emit::ASCENDING,
        ),
    );
}

//! Encoded integer profiles on identical logical datasets and native controls.
//!
//! Run `cargo bench --locked --features int-set --bench integer_sets`.
//! Input generation is excluded. Construction cases include validation,
//! normalization, representation construction and destruction. Materialization
//! includes output allocation and destruction. All full answers are checked
//! before timing; no byte estimate is presented as allocated resident memory.

mod support;

use frost::backend::roaring::{RoaringBitmap, RoaringTreemap};
use frost::family::FrozenOrderedSet;
use frost::integer::{FrozenIntSet, IntKey, IntSetBackend, IntSetError, IntSetPolicy};
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::hint::black_box;
use support::workload::{Layout, Partition, SetPairSpec, WorkloadSpec};
use support::{as_u64, measure};

const POLICIES: [(&str, IntSetPolicy); 5] = [
    ("auto", IntSetPolicy::auto()),
    ("sorted", IntSetPolicy::sorted()),
    ("dense", IntSetPolicy::dense()),
    ("roaring", IntSetPolicy::roaring()),
    ("chunked", IntSetPolicy::chunked()),
];

fn profiles<K: IntKey + Debug>(name: &str, values: &[K]) -> Vec<(&'static str, FrozenIntSet<K>)> {
    POLICIES
        .into_iter()
        .filter_map(|(label, policy)| {
            match FrozenIntSet::try_from_iter(values.iter().copied(), policy) {
                Ok(set) => {
                    eprintln!(
                        "{name}.{label}: {:?}; cardinality={}; span={}; partitions={}",
                        set.representation(),
                        set.cardinality(),
                        set.statistics().span,
                        set.statistics().high32_partitions
                    );
                    Some((label, set))
                }
                Err(error) if matches!(error, IntSetError::DenseBudgetExceeded { .. }) => {
                    eprintln!("{name}.{label}: excluded by checked dense budget: {error}");
                    None
                }
                Err(error) => panic!("unexpected construction failure for {name}.{label}: {error}"),
            }
        })
        .collect()
}

fn same_profile<K: IntKey + Debug>(
    name: &str,
    label: &str,
    left: &FrozenIntSet<K>,
    right: &FrozenIntSet<K>,
    expected: &[Vec<K>; 4],
) {
    let [intersection, union, difference, symmetric] = expected;
    assert_eq!(
        left.intersection(right).iter().collect::<Vec<_>>(),
        *intersection
    );
    assert_eq!(left.union(right).iter().collect::<Vec<_>>(), *union);
    assert_eq!(
        left.difference(right).iter().collect::<Vec<_>>(),
        *difference
    );
    assert_eq!(
        left.symmetric_difference(right).iter().collect::<Vec<_>>(),
        *symmetric
    );
    assert_eq!(left.is_disjoint(right), intersection.is_empty());
    assert_eq!(left.is_subset(right), difference.is_empty());
    measure(
        &format!("{name}.{label}.intersection_count"),
        as_u64(intersection.len()),
        || black_box(left).intersection_len(black_box(right)),
    );
    measure(
        &format!("{name}.{label}.intersection_materialize"),
        as_u64(intersection.len()),
        || black_box(black_box(left).intersection(black_box(right))).cardinality(),
    );
    measure(
        &format!("{name}.{label}.union_materialize"),
        as_u64(union.len()),
        || black_box(black_box(left).union(black_box(right))).cardinality(),
    );
    measure(
        &format!("{name}.{label}.difference_materialize"),
        as_u64(difference.len()),
        || black_box(black_box(left).difference(black_box(right))).cardinality(),
    );
    measure(
        &format!("{name}.{label}.symmetric_difference_materialize"),
        as_u64(symmetric.len()),
        || black_box(black_box(left).symmetric_difference(black_box(right))).cardinality(),
    );
}

fn compare<K: IntKey + Debug>(name: &str, left: &[K], right: &[K], mixed: bool) {
    let left_std: BTreeSet<_> = left.iter().copied().collect();
    let right_std: BTreeSet<_> = right.iter().copied().collect();
    let intersection: Vec<_> = left_std.intersection(&right_std).copied().collect();
    let union: Vec<_> = left_std.union(&right_std).copied().collect();
    let difference: Vec<_> = left_std.difference(&right_std).copied().collect();
    let symmetric: Vec<_> = left_std.symmetric_difference(&right_std).copied().collect();
    let common = as_u64(intersection.len());
    let expected = [intersection, union, difference, symmetric];
    let left_profiles = profiles(&format!("{name}.left"), left);
    let right_profiles = profiles(&format!("{name}.right"), right);
    for (label, left) in &left_profiles {
        let right = &right_profiles
            .iter()
            .find(|(other, _)| other == label)
            .expect("matching dataset storage eligibility")
            .1;
        same_profile(name, label, left, right, &expected);
    }

    measure(&format!("{name}.btree.intersection_count"), common, || {
        as_u64(
            black_box(&left_std)
                .intersection(black_box(&right_std))
                .count(),
        )
    });
    measure(
        &format!("{name}.btree.intersection_materialize"),
        common,
        || {
            let result: BTreeSet<_> = black_box(&left_std)
                .intersection(black_box(&right_std))
                .copied()
                .collect();
            as_u64(black_box(result).len())
        },
    );
    let left_ordered: FrozenOrderedSet<_> = left_std.iter().copied().collect();
    let right_ordered: FrozenOrderedSet<_> = right_std.iter().copied().collect();
    measure(
        &format!("{name}.frozen_ordered.intersection_count"),
        common,
        || as_u64(black_box(&left_ordered).intersection_len(black_box(&right_ordered))),
    );

    if mixed {
        for (left_label, left) in &left_profiles {
            for (right_label, right) in &right_profiles {
                if *left_label == "auto" || *right_label == "auto" {
                    continue;
                }
                assert_eq!(
                    left.intersection(right).iter().collect::<Vec<_>>(),
                    expected[0]
                );
                measure(
                    &format!("{name}.mixed_{left_label}_{right_label}.intersection_count"),
                    common,
                    || black_box(left).intersection_len(black_box(right)),
                );
                measure(
                    &format!("{name}.mixed_{left_label}_{right_label}.intersection_materialize"),
                    common,
                    || black_box(black_box(left).intersection(black_box(right))).cardinality(),
                );
                // New in this revision: mixed union/difference outputs and their
                // representation, which used to degrade to sorted codes.
                let union = left.union(right);
                assert_eq!(union.iter().collect::<Vec<_>>(), expected[1]);
                eprintln!(
                    "{name}.mixed_{left_label}_{right_label}.union: {:?}; payload={} bytes",
                    union.representation(),
                    union.payload_bytes()
                );
                measure(
                    &format!("{name}.mixed_{left_label}_{right_label}.union_materialize"),
                    as_u64(expected[1].len()),
                    || black_box(black_box(left).union(black_box(right))).cardinality(),
                );
                let difference = left.difference(right);
                assert_eq!(difference.iter().collect::<Vec<_>>(), expected[2]);
                measure(
                    &format!("{name}.mixed_{left_label}_{right_label}.difference_materialize"),
                    as_u64(expected[2].len()),
                    || black_box(black_box(left).difference(black_box(right))).cardinality(),
                );
            }
        }
    }
}

/// Snapshot-style updates: one new member joins a large set, repeatedly.
fn snapshot<K: IntKey + Debug>(name: &str, values: &[K], additions: &[K]) {
    for (label, policy) in POLICIES {
        let Ok(base) = FrozenIntSet::try_from_iter(values.iter().copied(), policy) else {
            continue;
        };
        let singletons: Vec<_> = additions
            .iter()
            .map(|&value| FrozenIntSet::try_from_iter([value], IntSetPolicy::sorted()).unwrap())
            .collect();
        let mut expected: BTreeSet<_> = values.iter().copied().collect();
        expected.extend(additions.iter().copied());
        let mut current = base.clone();
        for singleton in &singletons {
            current = current.union(singleton);
        }
        assert_eq!(
            current.iter().collect::<Vec<_>>(),
            expected.iter().copied().collect::<Vec<_>>()
        );
        eprintln!(
            "{name}.{label}.snapshot: {:?} -> {:?}; shared_chunks={}",
            base.representation(),
            current.representation(),
            current.shared_chunks(&base)
        );
        let expected_len = as_u64(expected.len());
        measure(
            &format!("{name}.{label}.snapshot_update"),
            expected_len,
            || {
                let mut current = black_box(&base).clone();
                for singleton in black_box(&singletons) {
                    current = current.union(singleton);
                }
                current.cardinality()
            },
        );
        measure(&format!("{name}.{label}.clone"), base.cardinality(), || {
            black_box(black_box(&base).clone()).cardinality()
        });
        let twin = FrozenIntSet::try_from_iter(values.iter().copied(), policy).unwrap();
        assert_eq!(base, twin);
        measure(&format!("{name}.{label}.equality"), 1, || {
            u64::from(black_box(&base) == black_box(&twin))
        });
    }
}

fn construction<K: IntKey + Debug>(name: &str, input: &[K]) {
    let expected_values: Vec<_> = input
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let expected = as_u64(expected_values.len());
    for (label, policy) in POLICIES {
        match FrozenIntSet::try_from_iter(input.iter().copied(), policy) {
            Ok(set) => assert_eq!(set.iter().collect::<Vec<_>>(), expected_values),
            Err(error) => {
                assert_eq!(policy.backend(), IntSetBackend::Dense);
                assert!(matches!(error, IntSetError::DenseBudgetExceeded { .. }));
                continue;
            }
        }
        measure(&format!("{name}.{label}.construct"), expected, || {
            black_box(
                FrozenIntSet::try_from_iter(black_box(input).iter().copied(), policy).unwrap(),
            )
            .cardinality()
        });
    }
}

fn native32(name: &str, left: &[u32], right: &[u32]) {
    let mut left_raw: RoaringBitmap = left.iter().copied().collect();
    let mut right_raw: RoaringBitmap = right.iter().copied().collect();
    left_raw.optimize();
    right_raw.optimize();
    let left_std: BTreeSet<_> = left.iter().copied().collect();
    let right_std: BTreeSet<_> = right.iter().copied().collect();
    let expected = as_u64(left_std.intersection(&right_std).count());
    assert_eq!(
        (&left_raw & &right_raw).iter().collect::<Vec<_>>(),
        left_std
            .intersection(&right_std)
            .copied()
            .collect::<Vec<_>>()
    );
    measure(
        &format!("{name}.native_roaring.intersection_count"),
        expected,
        || black_box(&left_raw).intersection_len(black_box(&right_raw)),
    );
    measure(
        &format!("{name}.native_roaring.intersection_materialize"),
        expected,
        || black_box(black_box(&left_raw) & black_box(&right_raw)).len(),
    );
}

fn native64(name: &str, left: &[u64], right: &[u64]) {
    let mut left_raw: RoaringTreemap = left.iter().copied().collect();
    let mut right_raw: RoaringTreemap = right.iter().copied().collect();
    left_raw.optimize();
    right_raw.optimize();
    let left_std: BTreeSet<_> = left.iter().copied().collect();
    let right_std: BTreeSet<_> = right.iter().copied().collect();
    let expected = as_u64(left_std.intersection(&right_std).count());
    assert_eq!(
        (&left_raw & &right_raw).iter().collect::<Vec<_>>(),
        left_std
            .intersection(&right_std)
            .copied()
            .collect::<Vec<_>>()
    );
    measure(
        &format!("{name}.native_roaring.intersection_count"),
        expected,
        || black_box(&left_raw).intersection_len(black_box(&right_raw)),
    );
    measure(
        &format!("{name}.native_roaring.intersection_materialize"),
        expected,
        || black_box(black_box(&left_raw) & black_box(&right_raw)).len(),
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct UserId(u64);

impl IntKey for UserId {
    type Word = u64;
    fn encode(self) -> u64 {
        self.0
    }
    fn decode(word: u64) -> Option<Self> {
        Some(Self(word))
    }
}

fn main() {
    support::header();
    let left32: Vec<u32> = (0..16_384).step_by(2).map(|v| 1_000_003 + v).collect();
    let right32: Vec<u32> = (0..16_384).step_by(3).map(|v| 1_000_034 + v).collect();
    compare("u32.cluster", &left32, &right32, false);
    construction("u32.cluster", &left32);
    native32("u32.cluster", &left32, &right32);

    let large: Vec<u32> = (0..65_536).collect();
    let small: Vec<u32> = (0..64).map(|v| v * 1024).collect();
    compare("u32.skew", &large, &small, true);
    native32("u32.skew", &large, &small);

    let runs_left: Vec<u32> = (0..4)
        .flat_map(|block| (0..1024).map(move |v| block * 65_536 + v))
        .collect();
    let runs_right: Vec<u32> = runs_left.iter().map(|v| v + 512).collect();
    compare("u32.runs", &runs_left, &runs_right, false);
    native32("u32.runs", &runs_left, &runs_right);

    let base = 1_000_000_000_000_003_u64;
    let clustered_left: Vec<u64> = (0..16_384).step_by(2).map(|v| base + v).collect();
    let clustered_right: Vec<u64> = (0..16_384).step_by(3).map(|v| base + 79 + v).collect();
    compare("u64.large_base", &clustered_left, &clustered_right, true);
    construction("u64.large_base", &clustered_left);
    native64("u64.large_base", &clustered_left, &clustered_right);

    let scattered_left: Vec<u64> = (0..4096)
        .map(|v| ((v * 104_729) << 32) | (v * 13))
        .collect();
    let scattered_right: Vec<u64> = (0..4096)
        .map(|v| ((v * 209_458) << 32) | (v * 26))
        .collect();
    compare(
        "u64.scattered_prefixes",
        &scattered_left,
        &scattered_right,
        false,
    );
    construction("u64.scattered_prefixes", &scattered_left);
    native64("u64.scattered_prefixes", &scattered_left, &scattered_right);

    let additions: Vec<u64> = (0..64).map(|v| base + 1_000_003 + v * 97).collect();
    snapshot("u64.large_base", &clustered_left, &additions);
    let window_additions: Vec<u32> = (0..64).map(|v| 2_000_000 + v * 97).collect();
    snapshot("u32.window", &large, &window_additions);

    // Factory names retain their complete reproducible specs and seeds.
    // Oracles are computed by `compare` outside timing.
    let chunked = SetPairSpec::new(
        31,
        16_384,
        16_384,
        4096,
        Layout::Chunks {
            first_chunk: 2,
            chunk_bits: 16,
            per_chunk: 512,
        },
    )
    .with_partition(Partition::Shuffled)
    .build()
    .and_then(|pair| pair.to_u32())
    .expect("u32 chunk-boundary dataset");
    eprintln!("u32.factory_chunks := {}", chunked.id);
    let chunked_name = format!("u32.factory_chunks_{}", chunked.id);
    compare(&chunked_name, &chunked.left, &chunked.right, true);
    native32(&chunked_name, &chunked.left, &chunked.right);

    let sparse64 = SetPairSpec::new(
        32,
        8192,
        8192,
        2048,
        Layout::Uniform {
            base: 1 << 40,
            span: 1 << 24,
        },
    )
    .build()
    .expect("u64 sparse uniform dataset");
    eprintln!("u64.factory_uniform := {}", sparse64.id);
    let sparse_name = format!("u64.factory_uniform_{}", sparse64.id);
    compare(&sparse_name, &sparse64.left, &sparse64.right, false);
    construction(&sparse_name, &sparse64.left);
    native64(&sparse_name, &sparse64.left, &sparse64.right);

    let signed_left: Vec<i64> = (-8192..8192).step_by(2).collect();
    let signed_right: Vec<i64> = (-8161..8223).step_by(3).collect();
    compare("i64.across_zero", &signed_left, &signed_right, false);
    let ids_left: Vec<UserId> = clustered_left.iter().copied().map(UserId).collect();
    let ids_right: Vec<UserId> = clustered_right.iter().copied().map(UserId).collect();
    compare("user_id.large_base", &ids_left, &ids_right, false);
}

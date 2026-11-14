//! Independently reproduced output-size and wrapper-cost review workloads.
//!
//! This file deliberately uses the public API shared by the before/after
//! versions so it can be compiled unchanged against both. Run competing
//! versions serially with identical compiler flags. `FROST_OUTPUT_CASE` selects
//! `mixed`, `distant`, `sorted_roaring`, `equality_hash`, `construction`,
//! `intersection`, or `contains`; the default runs all groups.
//! `FROST_OUTPUT_VERIFY_ONLY=1` executes
//! every full-answer check and one untimed invocation instead of collecting
//! samples. Inputs and oracle checks are outside timing; result allocation and
//! destruction are inside. Native controls use the same compiler and flags.
//!
//! A native Roaring serialized size is explicitly a wire-format observation,
//! not allocated memory. Bounds-derived Dense bytes below are a minimum payload
//! calculation because the historical API has no actual payload accessor.
//! Roaring has native Eq but no native Hash in the pinned 0.12.0 release. The
//! custom member-hash and raw-payload hash controls are labeled separately and
//! are not presented as the same canonical logical Hash protocol.

mod support;

use frost::backend::roaring::{RoaringBitmap, RoaringTreemap};
use frost::integer::{FrozenIntSet, IntSetError, IntSetPolicy, IntSetRepr};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::hint::black_box;
use support::{as_u64, measure};

fn checked(values: impl IntoIterator<Item = u64>, policy: IntSetPolicy) -> FrozenIntSet<u64> {
    FrozenIntSet::try_from_iter(values, policy).unwrap()
}

fn timed(name: &str, expected: u64, mut operation: impl FnMut() -> u64) {
    if std::env::var_os("FROST_OUTPUT_VERIFY_ONLY").is_some() {
        assert_eq!(operation(), expected, "untimed benchmark check: {name}");
    } else {
        measure(name, expected, operation);
    }
}

fn digest(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn native_member_digest(values: &RoaringTreemap) -> u64 {
    let mut hasher = DefaultHasher::new();
    values.len().hash(&mut hasher);
    for value in values {
        value.hash(&mut hasher);
    }
    hasher.finish()
}

fn describe(name: &str, set: &FrozenIntSet<u64>) {
    let minimum_dense_bytes = set.bounds().map_or(0, |(minimum, maximum)| {
        (u128::from(maximum / 64) - u128::from(minimum / 64) + 1) * 8
    });
    match set.representation() {
        IntSetRepr::Sorted => eprintln!(
            "{name}: repr=Sorted; cardinality={}; sorted_payload_bytes={}",
            set.cardinality(),
            u128::from(set.cardinality()) * 8
        ),
        IntSetRepr::Dense => eprintln!(
            "{name}: repr=Dense; cardinality={}; minimum_dense_payload_bytes={minimum_dense_bytes}",
            set.cardinality()
        ),
        representation => eprintln!(
            "{name}: repr={representation:?}; cardinality={}; resident_bytes=unmeasured",
            set.cardinality()
        ),
    }
}

fn mixed_output() {
    const SPAN: u64 = 2_000_000;
    let dense = checked((0..SPAN).step_by(2), IntSetPolicy::dense());
    let singleton = checked([1], IntSetPolicy::roaring());
    let present = checked([64], IntSetPolicy::roaring());
    describe("mixed.input", &dense);
    let union = dense.union(&singleton);
    let symmetric = dense.symmetric_difference(&singleton);
    let difference = dense.difference(&present);
    assert!(
        union
            .iter()
            .eq((0..SPAN).filter(|value| value % 2 == 0 || *value == 1))
    );
    assert!(symmetric.iter().eq(union.iter()));
    assert!(
        difference
            .iter()
            .eq((0..SPAN).step_by(2).filter(|value| *value != 64))
    );
    describe("mixed.union", &union);
    describe("mixed.difference", &difference);
    timed("million_even.dense_roaring.union", SPAN / 2 + 1, || {
        black_box(black_box(&dense).union(black_box(&singleton))).cardinality()
    });
    timed("million_even.dense_roaring.xor", SPAN / 2 + 1, || {
        black_box(black_box(&dense).symmetric_difference(black_box(&singleton))).cardinality()
    });
    timed(
        "million_even.dense_roaring.difference",
        SPAN / 2 - 1,
        || black_box(black_box(&dense).difference(black_box(&present))).cardinality(),
    );

    let words = vec![0x5555_5555_5555_5555_u64; usize::try_from(SPAN / 64).unwrap()];
    let mut updated = words.clone();
    updated[0] |= 2;
    assert_eq!(
        updated
            .iter()
            .map(|word| u64::from(word.count_ones()))
            .sum::<u64>(),
        SPAN / 2 + 1
    );
    timed(
        "million_even.raw_dense.clone_and_set_bit",
        SPAN / 64,
        || {
            let mut result = black_box(&words).clone();
            result[0] |= 2;
            as_u64(black_box(result).len())
        },
    );
    let native = RoaringTreemap::from_sorted_iter((0..SPAN).step_by(2)).unwrap();
    let mut native_updated = native.clone();
    native_updated.insert(1);
    assert!(native_updated.iter().eq(union.iter()));
    timed(
        "million_even.native_roaring.clone_and_insert",
        SPAN / 2 + 1,
        || {
            let mut result = black_box(&native).clone();
            result.insert(black_box(1));
            black_box(result).len()
        },
    );

    // The same logical update stream used by SnapshotStore, without measuring
    // atomic publication or conflating its cost with representation decisions.
    let additions: Vec<_> = (1..64)
        .step_by(2)
        .map(|value| checked([value], IntSetPolicy::roaring()))
        .collect();
    let mut updated = dense.clone();
    for addition in &additions {
        updated = updated.union(addition);
    }
    assert!(
        updated
            .iter()
            .eq((0..SPAN).filter(|value| *value < 64 || value % 2 == 0))
    );
    describe("mixed.after_32_updates", &updated);
    timed(
        "million_even.mixed_updates_32_including_initial_clone",
        SPAN / 2 + 32,
        || {
            let mut result = black_box(&dense).clone();
            for addition in black_box(&additions) {
                result = result.union(addition);
            }
            black_box(result).cardinality()
        },
    );

    let tiny = IntSetPolicy::dense().with_dense_budget(8);
    let left = checked([0], tiny);
    let right = checked([1 << 20], tiny);
    let combined = left.union(&right);
    assert!(combined.iter().eq([0, 1 << 20]));
    describe("eight_byte_budget.union", &combined);
}

fn distant_runs() {
    const RUN_LENGTH: u64 = 1 << 22;
    const GAP: u64 = 1 << 27;
    let left = checked(0..RUN_LENGTH, IntSetPolicy::dense());
    let right = checked(GAP..GAP + RUN_LENGTH, IntSetPolicy::dense());
    describe("distant.input_left", &left);
    describe("distant.input_right", &right);
    let output = left.union(&right);
    assert!(
        output
            .iter()
            .eq((0..RUN_LENGTH).chain(GAP..GAP + RUN_LENGTH))
    );
    describe("distant.union", &output);
    let mut native_left = RoaringTreemap::new();
    native_left.insert_range(0..RUN_LENGTH);
    native_left.optimize();
    let mut native_right = RoaringTreemap::new();
    native_right.insert_range(GAP..GAP + RUN_LENGTH);
    native_right.optimize();
    let native_output = &native_left | &native_right;
    assert!(native_output.iter().eq(output.iter()));
    eprintln!(
        "distant.native_union: serialized_bytes={}; resident_bytes=unmeasured",
        native_output.serialized_size()
    );
    timed("distant_runs.dense_dense.union", 2 * RUN_LENGTH, || {
        black_box(black_box(&left).union(black_box(&right))).cardinality()
    });
    timed("distant_runs.native_roaring.union", 2 * RUN_LENGTH, || {
        black_box(black_box(&native_left) | black_box(&native_right)).len()
    });
}

fn balanced_sorted_roaring() {
    const SPAN: u64 = 2_000_000;
    let left_values: Vec<_> = (0..SPAN).step_by(2).collect();
    let right_values: Vec<_> = (0..SPAN).filter(|value| value % 4 < 2).collect();
    let sorted = checked(left_values.iter().copied(), IntSetPolicy::sorted());
    let roaring = checked(right_values.iter().copied(), IntSetPolicy::roaring());
    let mut native_left = RoaringTreemap::from_sorted_iter(left_values.iter().copied()).unwrap();
    native_left.optimize();
    let mut native_right = RoaringTreemap::from_sorted_iter(right_values.iter().copied()).unwrap();
    native_right.optimize();
    let union = sorted.union(&roaring);
    let difference = sorted.difference(&roaring);
    let reverse_difference = roaring.difference(&sorted);
    assert!(union.iter().eq((0..SPAN).filter(|value| value % 4 != 3)));
    assert!(
        difference
            .iter()
            .eq((0..SPAN).filter(|value| value % 4 == 2))
    );
    assert!(
        reverse_difference
            .iter()
            .eq((0..SPAN).filter(|value| value % 4 == 1))
    );
    assert!((&native_left | &native_right).iter().eq(union.iter()));
    assert!((&native_left - &native_right).iter().eq(difference.iter()));
    assert!(
        (&native_right - &native_left)
            .iter()
            .eq(reverse_difference.iter())
    );
    describe("balanced_sorted_roaring.union", &union);
    describe("balanced_sorted_roaring.difference", &difference);
    timed("balanced64.sorted_roaring.union", SPAN * 3 / 4, || {
        black_box(black_box(&sorted).union(black_box(&roaring))).cardinality()
    });
    timed("balanced64.sorted_roaring.difference", SPAN / 4, || {
        black_box(black_box(&sorted).difference(black_box(&roaring))).cardinality()
    });
    timed("balanced64.roaring_sorted.difference", SPAN / 4, || {
        black_box(black_box(&roaring).difference(black_box(&sorted))).cardinality()
    });
    timed(
        "balanced64.native_roaring.prepared_union",
        SPAN * 3 / 4,
        || black_box(black_box(&native_left) | black_box(&native_right)).len(),
    );
    timed(
        "balanced64.native_roaring.prepared_difference",
        SPAN / 4,
        || black_box(black_box(&native_left) - black_box(&native_right)).len(),
    );
    timed(
        "balanced64.native_roaring.sorted_build_and_union",
        SPAN * 3 / 4,
        || {
            let mut converted =
                RoaringTreemap::from_sorted_iter(black_box(&left_values).iter().copied()).unwrap();
            converted.optimize();
            black_box(&converted | black_box(&native_right)).len()
        },
    );
    timed(
        "balanced64.native_roaring.sorted_build_and_difference",
        SPAN / 4,
        || {
            let mut converted =
                RoaringTreemap::from_sorted_iter(black_box(&left_values).iter().copied()).unwrap();
            converted.optimize();
            black_box(&converted - black_box(&native_right)).len()
        },
    );
}

fn skew_sorted_compressed() {
    let values: Vec<_> = (0_u64..2_000_000).step_by(2).collect();
    let sorted = checked(values.iter().copied(), IntSetPolicy::sorted());
    let small_values = [1024_u64, 1026];
    for (label, policy) in [
        ("dense", IntSetPolicy::dense()),
        ("roaring", IntSetPolicy::roaring()),
    ] {
        let small = checked(small_values, policy);
        for (direction, left, right) in [
            ("sorted_first", &sorted, &small),
            ("compressed_first", &small, &sorted),
        ] {
            assert!(left.intersection(right).iter().eq(small_values));
            assert_eq!(left.intersection_len(right), 2);
            timed(
                &format!("skew64.{label}.{direction}.intersection_count_2_of_1m"),
                2,
                || black_box(left).intersection_len(black_box(right)),
            );
            timed(
                &format!("skew64.{label}.{direction}.intersection_materialize_2_of_1m"),
                2,
                || black_box(black_box(left).intersection(black_box(right))).cardinality(),
            );
        }
    }
    timed("skew64.raw_sorted.binary_search_2_of_1m", 2, || {
        as_u64(
            black_box(&small_values)
                .iter()
                .filter(|value| black_box(&values).binary_search(value).is_ok())
                .count(),
        )
    });
    let native = RoaringTreemap::from_sorted_iter(values.iter().copied()).unwrap();
    let small_native = RoaringTreemap::from_sorted_iter(small_values).unwrap();
    assert!((&native & &small_native).iter().eq(small_values));
    timed(
        "skew64.native_roaring.prepared_intersection_count_2_of_1m",
        2,
        || black_box(&native).intersection_len(black_box(&small_native)),
    );
    timed(
        "skew64.native_roaring.prepared_intersection_materialize_2_of_1m",
        2,
        || black_box(black_box(&native) & black_box(&small_native)).len(),
    );
}

fn sorted_roaring() {
    balanced_sorted_roaring();
    skew_sorted_compressed();
}

fn equality_hash_case(name: &str, values: &[u64]) {
    let mut native = RoaringTreemap::from_sorted_iter(values.iter().copied()).unwrap();
    native.optimize();
    let native_copy = native.clone();
    let sorted_copy = values.to_vec();
    assert!(native.iter().eq(values.iter().copied()));
    timed(&format!("{name}.native_roaring.eq"), 1, || {
        u64::from(black_box(&native) == black_box(&native_copy))
    });
    timed(&format!("{name}.raw_sorted.eq"), 1, || {
        u64::from(black_box(values) == black_box(sorted_copy.as_slice()))
    });
    let expected_native_hash = native_member_digest(&native);
    timed(
        &format!("{name}.native_roaring.custom_member_hash"),
        expected_native_hash,
        || native_member_digest(black_box(&native)),
    );
    let expected_payload_hash = digest(&values);
    timed(
        &format!("{name}.raw_sorted.payload_hash"),
        expected_payload_hash,
        || digest(black_box(&values)),
    );

    let mut expected_logical_hash = None;
    for (label, policy) in [
        ("sorted", IntSetPolicy::sorted()),
        ("dense", IntSetPolicy::dense()),
        ("roaring", IntSetPolicy::roaring()),
    ] {
        let set = match FrozenIntSet::try_from_iter(values.iter().copied(), policy) {
            Ok(set) => set,
            Err(IntSetError::DenseBudgetExceeded { .. }) => {
                eprintln!(
                    "{name}.{label}: skipped because the full window exceeds the dense budget"
                );
                continue;
            }
            Err(error) => panic!("unexpected fixture failure: {error}"),
        };
        let copy = set.clone();
        assert!(set.iter().eq(values.iter().copied()));
        let hash = digest(&set);
        assert_eq!(*expected_logical_hash.get_or_insert(hash), hash);
        timed(&format!("{name}.{label}.eq"), 1, || {
            u64::from(black_box(&set) == black_box(&copy))
        });
        timed(&format!("{name}.{label}.logical_hash"), hash, || {
            digest(black_box(&set))
        });
        if label == "dense" {
            let base = values[0] / 64;
            let length = usize::try_from(values[values.len() - 1] / 64 - base + 1).unwrap();
            let mut words = vec![0_u64; length];
            for value in values {
                words[usize::try_from(value / 64 - base).unwrap()] |= 1_u64 << (value % 64);
            }
            let other_words = words.clone();
            assert_eq!(
                words
                    .iter()
                    .map(|word| u64::from(word.count_ones()))
                    .sum::<u64>(),
                as_u64(values.len())
            );
            timed(&format!("{name}.raw_dense.eq"), 1, || {
                u64::from(black_box(&words) == black_box(&other_words))
            });
            let expected = digest(&words);
            timed(&format!("{name}.raw_dense.payload_hash"), expected, || {
                digest(black_box(&words))
            });
        }
    }
}

fn equality_hash() {
    equality_hash_case("million_run", &(0..1_000_000).collect::<Vec<_>>());
    equality_hash_case(
        "million_alternating",
        &(0..2_000_000).step_by(2).collect::<Vec<_>>(),
    );
    equality_hash_case(
        "eight_runs_32768",
        &(0..8)
            .flat_map(|run| run * 8192..run * 8192 + 4096)
            .collect::<Vec<_>>(),
    );
    equality_hash_case(
        "seventeen_runs_69632",
        &(0..17)
            .flat_map(|run| run * 8192..run * 8192 + 4096)
            .collect::<Vec<_>>(),
    );
    equality_hash_case(
        "sparse_local_2048",
        &(0..2048).map(|value| value * 509).collect::<Vec<_>>(),
    );
    equality_hash_case(
        "sparse_prefixes_2048",
        &(0..2048)
            .map(|value| (value << 40) + value)
            .collect::<Vec<_>>(),
    );
}

fn construction_case(name: &str, values: &[u64]) {
    let expected = as_u64(values.len());
    let mut collected: RoaringTreemap = values.iter().copied().collect();
    collected.optimize();
    let mut sorted = RoaringTreemap::from_sorted_iter(values.iter().copied()).unwrap();
    sorted.optimize();
    let frozen = checked(values.iter().copied(), IntSetPolicy::roaring());
    assert!(collected.iter().eq(values.iter().copied()));
    assert_eq!(collected, sorted);
    assert!(frozen.iter().eq(values.iter().copied()));
    timed(
        &format!("{name}.frost_roaring.validated_build"),
        expected,
        || {
            black_box(checked(
                black_box(values).iter().copied(),
                IntSetPolicy::roaring(),
            ))
            .cardinality()
        },
    );
    timed(
        &format!("{name}.native_roaring.collect_optimize"),
        expected,
        || {
            let mut result: RoaringTreemap = black_box(values).iter().copied().collect();
            result.optimize();
            black_box(result).len()
        },
    );
    timed(
        &format!("{name}.native_roaring.sorted_iter_optimize"),
        expected,
        || {
            let mut result =
                RoaringTreemap::from_sorted_iter(black_box(values).iter().copied()).unwrap();
            result.optimize();
            black_box(result).len()
        },
    );
}

fn construction() {
    construction_case("million_run", &(0..1_000_000).collect::<Vec<_>>());
    construction_case(
        "million_alternating",
        &(0..2_000_000).step_by(2).collect::<Vec<_>>(),
    );
    construction_case(
        "sparse_prefixes_2048",
        &(0..2048)
            .map(|value| (value << 40) + value)
            .collect::<Vec<_>>(),
    );
}

fn dense_roaring_intersection() {
    const SPAN: u32 = 2_000_000;
    let dense = FrozenIntSet::try_from_iter((0..SPAN).step_by(2), IntSetPolicy::dense()).unwrap();
    let other_dense = FrozenIntSet::try_from_iter(
        (0..SPAN).filter(|value| value % 4 < 2),
        IntSetPolicy::dense(),
    )
    .unwrap();
    let other_roaring = other_dense.with_policy(IntSetPolicy::roaring()).unwrap();
    let native = RoaringBitmap::from_sorted_iter((0..SPAN).filter(|value| value % 4 < 2)).unwrap();
    let bytes = vec![0x55_u8; usize::try_from(SPAN / 8).unwrap()];
    let promoted = RoaringBitmap::from_lsb0_bytes(0, &bytes);
    assert!(promoted.iter().eq((0..SPAN).step_by(2)));
    assert!(other_roaring.iter().eq(native.iter()));
    for actual in [
        dense.intersection(&other_dense),
        dense.intersection(&other_roaring),
    ] {
        assert!(actual.iter().eq((0..SPAN).step_by(4)));
    }
    assert!((&promoted & &native).iter().eq((0..SPAN).step_by(4)));
    let expected = u64::from(SPAN / 4);
    timed("overlap32.dense_dense.count", expected, || {
        black_box(&dense).intersection_len(black_box(&other_dense))
    });
    timed("overlap32.dense_roaring.count", expected, || {
        black_box(&dense).intersection_len(black_box(&other_roaring))
    });
    timed("overlap32.dense_roaring.materialize", expected, || {
        black_box(black_box(&dense).intersection(black_box(&other_roaring))).cardinality()
    });
    timed(
        "overlap32.native_roaring.cached_promotion_count",
        expected,
        || black_box(&promoted).intersection_len(black_box(&native)),
    );
    timed(
        "overlap32.native_roaring.promotion_and_count",
        expected,
        || {
            RoaringBitmap::from_lsb0_bytes(0, black_box(&bytes))
                .intersection_len(black_box(&native))
        },
    );
    timed(
        "overlap32.native_roaring.promotion_and_materialize",
        expected,
        || {
            black_box(&RoaringBitmap::from_lsb0_bytes(0, black_box(&bytes)) & black_box(&native))
                .len()
        },
    );
    let words = vec![0x5555_5555_5555_5555_u64; usize::try_from(SPAN / 64).unwrap()];
    let other_words = vec![0x3333_3333_3333_3333_u64; words.len()];
    timed("overlap32.raw_dense.and_popcount", expected, || {
        black_box(&words)
            .iter()
            .zip(black_box(&other_words))
            .map(|(left, right)| u64::from((left & right).count_ones()))
            .sum()
    });
}

fn contains() {
    let queries: Vec<u32> = (0..1024).map(|value| value * 37 % 2000).collect();
    let expected = as_u64(queries.iter().filter(|value| **value < 1000).count());
    for (label, policy) in [
        ("auto", IntSetPolicy::auto()),
        ("sorted", IntSetPolicy::sorted()),
        ("dense", IntSetPolicy::dense()),
        ("roaring", IntSetPolicy::roaring()),
    ] {
        let set = FrozenIntSet::try_from_iter(0_u32..1000, policy).unwrap();
        eprintln!(
            "small_run32.{label}: repr={:?}; cardinality=1000",
            set.representation()
        );
        assert!(set.iter().eq(0..1000));
        for query in &queries {
            assert_eq!(set.contains(query), *query < 1000);
        }
        timed(
            &format!("small_run32.{label}.contains_1024_queries"),
            expected,
            || {
                as_u64(
                    black_box(&queries)
                        .iter()
                        .filter(|query| black_box(&set).contains(query))
                        .count(),
                )
            },
        );
    }
    let mut native = RoaringBitmap::from_sorted_iter(0..1000).unwrap();
    native.optimize();
    timed(
        "small_run32.native_roaring.contains_1024_queries",
        expected,
        || {
            as_u64(
                black_box(&queries)
                    .iter()
                    .filter(|query| black_box(&native).contains(**query))
                    .count(),
            )
        },
    );
}

fn main() {
    let selected = std::env::var("FROST_OUTPUT_CASE").ok();
    support::header();
    let groups: [(&str, fn()); 7] = [
        ("mixed", mixed_output),
        ("distant", distant_runs),
        ("sorted_roaring", sorted_roaring),
        ("equality_hash", equality_hash),
        ("construction", construction),
        ("intersection", dense_roaring_intersection),
        ("contains", contains),
    ];
    if let Some(selected) = selected.as_deref() {
        assert!(
            groups.iter().any(|(name, _)| *name == selected),
            "unknown benchmark group {selected}"
        );
    }
    for (name, group) in groups {
        if selected.as_deref().is_none_or(|selected| selected == name) {
            group();
        }
    }
}

//! Output policy, promotion, and logical equality/hash regression tests.
//!
//! These reproduce the supplied review summary independently. Size assertions
//! concern dense bitmap payload limits, not total resident allocation.
#![cfg(feature = "int-set")]

use frost::integer::{FrozenIntSet, IntKey, IntSetError, IntSetPolicy, IntSetRepr};
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

const POLICIES: [IntSetPolicy; 4] = [
    IntSetPolicy::sorted(),
    IntSetPolicy::dense(),
    IntSetPolicy::roaring(),
    IntSetPolicy::chunked(),
];

fn set<K: IntKey>(values: impl IntoIterator<Item = K>, policy: IntSetPolicy) -> FrozenIntSet<K> {
    FrozenIntSet::try_from_iter(values, policy).unwrap()
}

fn assert_members<K: IntKey + Debug>(actual: &FrozenIntSet<K>, expected: &BTreeSet<K>) {
    assert_eq!(
        actual.iter().collect::<Vec<_>>(),
        expected.iter().copied().collect::<Vec<_>>()
    );
    assert_eq!(actual.cardinality(), u64::try_from(expected.len()).unwrap());
    assert_eq!(
        actual.bounds(),
        expected.first().copied().zip(expected.last().copied())
    );
    for value in expected {
        assert!(actual.contains(value));
    }
}

fn answers<K: IntKey>(left: &BTreeSet<K>, right: &BTreeSet<K>) -> [BTreeSet<K>; 4] {
    [
        left.union(right).copied().collect(),
        left.intersection(right).copied().collect(),
        left.difference(right).copied().collect(),
        left.symmetric_difference(right).copied().collect(),
    ]
}

fn assert_algebra<K: IntKey + Debug>(left: &FrozenIntSet<K>, right: &FrozenIntSet<K>) {
    let left_reference = left.iter().collect();
    let right_reference = right.iter().collect();
    let expected = answers(&left_reference, &right_reference);
    let actual = [
        left.union(right),
        left.intersection(right),
        left.difference(right),
        left.symmetric_difference(right),
    ];
    let counts = [
        left.union_len(right),
        left.intersection_len(right),
        left.difference_len(right),
        left.symmetric_difference_len(right),
    ];
    for ((actual, expected), count) in actual.iter().zip(&expected).zip(counts) {
        assert_members(actual, expected);
        assert_eq!(count, actual.cardinality());
    }
    assert_eq!(
        left.is_subset(right),
        left_reference.is_subset(&right_reference)
    );
    assert_eq!(
        left.is_superset(right),
        left_reference.is_superset(&right_reference)
    );
    assert_eq!(
        left.is_disjoint(right),
        left_reference.is_disjoint(&right_reference)
    );
}

#[test]
fn tiny_mixed_updates_preserve_a_dense_window() {
    let dense = set((0_u64..8192).step_by(2), IntSetPolicy::dense());
    for policy in [IntSetPolicy::sorted(), IntSetPolicy::roaring()] {
        let absent = set([1], policy);
        let present = set([64], policy);
        for (left, right) in [(&dense, &absent), (&absent, &dense)] {
            assert_algebra(left, right);
            assert_eq!(left.union(right).representation(), IntSetRepr::Dense);
            assert_eq!(left.union(right).contiguous_payload_bytes(), Some(1024));
            assert_eq!(
                left.symmetric_difference(right).representation(),
                IntSetRepr::Dense
            );
        }
        assert_algebra(&dense, &present);
        assert_eq!(
            dense.difference(&present).representation(),
            IntSetRepr::Dense
        );
    }
}

#[test]
fn distant_dense_runs_promote_without_a_sorted_expansion() {
    // Scaled member counts retain the original over-budget window shape.
    // Neither input nor the expected answer needs a multi-megabyte allocation.
    for gap in [1_u64 << 27, 1_u64 << 40] {
        let left = set(0..4096, IntSetPolicy::dense());
        let right = set(gap..gap + 4096, IntSetPolicy::dense());
        assert_algebra(&left, &right);
        for result in [left.union(&right), left.symmetric_difference(&right)] {
            assert_eq!(result.representation(), IntSetRepr::Roaring);
            assert_eq!(result.cardinality(), 8192);
        }
    }
}

#[test]
fn compact_native_intersection_survives_auto_but_respects_output_limits() {
    const SPAN: u64 = 65_536;
    let dense = set((0..SPAN).step_by(2), IntSetPolicy::dense());
    let roaring = set(
        (0..SPAN).filter(|value| value % 4 < 2),
        IntSetPolicy::roaring(),
    );
    let expected: BTreeSet<_> = (0..SPAN).step_by(4).collect();
    let automatic = dense.intersection(&roaring);
    assert_members(&automatic, &expected);
    assert_eq!(automatic.representation(), IntSetRepr::Roaring);
    let forced_dense = dense
        .intersection_with_policy(&roaring, IntSetPolicy::dense())
        .unwrap();
    assert_members(&forced_dense, &expected);
    assert_eq!(forced_dense.representation(), IntSetRepr::Dense);
    assert_eq!(forced_dense.contiguous_payload_bytes(), Some(8192));
    let no_roaring = dense
        .intersection_with_policy(
            &roaring,
            IntSetPolicy::auto().with_roaring_partition_limit(0),
        )
        .unwrap();
    assert_members(&no_roaring, &expected);
    assert_ne!(no_roaring.representation(), IntSetRepr::Roaring);
    assert_eq!(no_roaring.limits().max_roaring_partitions(), 0);
    let sorted = dense
        .intersection_with_policy(
            &roaring,
            IntSetPolicy::auto()
                .with_dense_budget(0)
                .with_roaring_partition_limit(0),
        )
        .unwrap();
    assert_members(&sorted, &expected);
    assert_eq!(sorted.representation(), IntSetRepr::Sorted);
    assert_eq!(sorted.contiguous_payload_bytes(), Some(16_384 * 8));
}

#[test]
fn mixed_promotion_handles_word_partition_and_unsigned_end_boundaries() {
    for base in [
        0_u64,
        (1_u64 << 32) - 193,
        (1_u64 << 48) + 5,
        u64::MAX - 511,
    ] {
        let left: Vec<_> = (0..512)
            .filter(|offset| offset % 3 != 0)
            .map(|offset| base + offset)
            .collect();
        let right: Vec<_> = (0..512)
            .filter(|offset| offset % 5 < 3)
            .map(|offset| base + offset)
            .collect();
        let expected = answers(
            &left.iter().copied().collect(),
            &right.iter().copied().collect(),
        );
        for left_policy in POLICIES {
            for right_policy in POLICIES {
                let left = set(left.iter().copied(), left_policy);
                let right = set(right.iter().copied(), right_policy);
                assert_algebra(&left, &right);
                for output_policy in POLICIES {
                    let actual = [
                        left.union_with_policy(&right, output_policy).unwrap(),
                        left.intersection_with_policy(&right, output_policy)
                            .unwrap(),
                        left.difference_with_policy(&right, output_policy).unwrap(),
                        left.symmetric_difference_with_policy(&right, output_policy)
                            .unwrap(),
                    ];
                    for (actual, expected) in actual.iter().zip(&expected) {
                        assert_members(actual, expected);
                    }
                }
            }
        }
    }
}

#[test]
fn signed_and_restricted_domains_survive_promoted_algebra() {
    use std::num::NonZeroU64;
    for left_policy in POLICIES {
        for right_policy in POLICIES {
            let left = set((-130_i64..130).filter(|value| value % 3 != 0), left_policy);
            let right = set((-65_i64..195).filter(|value| value % 5 != 0), right_policy);
            assert_algebra(&left, &right);
            let left = set(
                (1_u64..513)
                    .step_by(3)
                    .map(|value| NonZeroU64::new(value).unwrap()),
                left_policy,
            );
            let right = set(
                (2_u64..514)
                    .step_by(5)
                    .map(|value| NonZeroU64::new(value).unwrap()),
                right_policy,
            );
            assert_algebra(&left, &right);
        }
    }
}

#[test]
fn construction_limits_survive_default_algebra() {
    let strict = IntSetPolicy::dense()
        .with_dense_budget(8)
        .with_roaring_partition_limit(1);
    let loose = IntSetPolicy::dense()
        .with_dense_budget(128)
        .with_roaring_partition_limit(7);
    let left = set([1_u64], strict);
    let right = set([1_u64 << 20], loose);
    for result in [
        left.union(&right),
        right.union(&left),
        left.symmetric_difference(&right),
        right.symmetric_difference(&left),
    ] {
        assert_eq!(result.limits().max_dense_bytes(), 8);
        assert_eq!(result.limits().max_roaring_partitions(), 1);
        // Even the minimum possible dense window here exceeds eight bytes.
        assert_ne!(result.representation(), IntSetRepr::Dense);
    }
    let empty = left.intersection(&right);
    assert!(empty.is_empty());
    assert_eq!(empty.limits(), left.limits());
    assert_eq!(left.difference(&right).limits(), left.limits());
    assert_eq!(right.difference(&left).limits(), right.limits());
    let repeated = left.union(&right).union(&set([2], IntSetPolicy::roaring()));
    assert_eq!(repeated.limits(), left.limits());
    assert_ne!(repeated.representation(), IntSetRepr::Dense);
    assert_eq!(left.clone().limits(), left.limits());
    assert_eq!(left.contiguous_payload_bytes(), Some(8));
}

#[test]
fn empty_operands_and_identity_shortcuts_do_not_bypass_limits() {
    let strict_empty = set(
        [],
        IntSetPolicy::sorted()
            .with_dense_budget(0)
            .with_roaring_partition_limit(0),
    );
    let dense = set(0_u64..512, IntSetPolicy::dense());
    for result in [
        dense.union(&strict_empty),
        strict_empty.union(&dense),
        dense.symmetric_difference(&strict_empty),
        strict_empty.symmetric_difference(&dense),
    ] {
        assert_eq!(result.limits(), strict_empty.limits());
        assert_eq!(result.representation(), IntSetRepr::Sorted);
        assert_eq!(result, dense);
    }
    let strict_equal = dense
        .with_policy(
            IntSetPolicy::sorted()
                .with_dense_budget(0)
                .with_roaring_partition_limit(0),
        )
        .unwrap();
    let common = dense.intersection(&strict_equal);
    assert_eq!(common, dense);
    assert_eq!(common.representation(), IntSetRepr::Sorted);
    assert_eq!(common.limits(), strict_equal.limits());
    assert_eq!(dense.difference(&strict_empty).limits(), dense.limits());
}

#[test]
fn both_limits_are_combined_independently() {
    let left = set(
        [1_u64, 65],
        IntSetPolicy::sorted()
            .with_dense_budget(8)
            .with_roaring_partition_limit(4),
    );
    let right = set(
        [65_u64, 129],
        IntSetPolicy::sorted()
            .with_dense_budget(24)
            .with_roaring_partition_limit(0),
    );
    for result in [
        left.union(&right),
        left.intersection(&right),
        left.symmetric_difference(&right),
    ] {
        assert_eq!(result.limits().max_dense_bytes(), 8);
        assert_eq!(result.limits().max_roaring_partitions(), 0);
        assert_ne!(result.representation(), IntSetRepr::Roaring);
        if result.cardinality() > 1 {
            assert_ne!(result.representation(), IntSetRepr::Dense);
        }
    }
    assert_eq!(left.difference(&right).limits(), left.limits());
    assert_eq!(right.difference(&left).limits(), right.limits());
}

#[test]
fn contiguous_payload_diagnostics_report_actual_buffer_bytes() {
    assert_eq!(
        set([1_u32, 2, 3], IntSetPolicy::sorted()).contiguous_payload_bytes(),
        Some(12)
    );
    assert_eq!(
        set([1_u64, 2, 3], IntSetPolicy::sorted()).contiguous_payload_bytes(),
        Some(24)
    );
    assert_eq!(
        set([63_u64, 64], IntSetPolicy::dense()).contiguous_payload_bytes(),
        Some(16)
    );
    assert_eq!(
        set([1_u64, 2, 3], IntSetPolicy::roaring()).contiguous_payload_bytes(),
        None
    );
    assert_eq!(
        set([0_u64; 0], IntSetPolicy::sorted()).contiguous_payload_bytes(),
        Some(0)
    );
}

#[test]
fn output_policy_overrides_are_checked_and_retained() {
    let left = set([0_u64, 63], IntSetPolicy::dense().with_dense_budget(8));
    let right = set([64_u64, 127], IntSetPolicy::dense().with_dense_budget(8));
    assert!(matches!(
        left.union_with_policy(&right, IntSetPolicy::dense().with_dense_budget(8)),
        Err(IntSetError::DenseBudgetExceeded {
            required_bytes: 16,
            max_bytes: 8
        })
    ));
    let policy = IntSetPolicy::dense()
        .with_dense_budget(16)
        .with_roaring_partition_limit(2);
    let combined = left.union_with_policy(&right, policy).unwrap();
    assert_eq!(combined.representation(), IntSetRepr::Dense);
    assert_eq!(combined.limits().max_dense_bytes(), 16);
    assert_eq!(combined.limits().max_roaring_partitions(), 2);
    assert_eq!(combined.contiguous_payload_bytes(), Some(16));
    assert_eq!(combined.iter().collect::<Vec<_>>(), [0, 63, 64, 127]);
    assert_eq!(left.limits().max_dense_bytes(), 8);
    for policy in POLICIES {
        let expected_repr = set([0_u64], policy).representation();
        assert_eq!(
            left.union_with_policy(&right, policy)
                .unwrap()
                .representation(),
            expected_repr
        );
        assert_eq!(
            combined
                .intersection_with_policy(&left, policy)
                .unwrap()
                .representation(),
            expected_repr
        );
        assert_eq!(
            combined
                .difference_with_policy(&left, policy)
                .unwrap()
                .representation(),
            expected_repr
        );
        assert_eq!(
            combined
                .symmetric_difference_with_policy(&left, policy)
                .unwrap()
                .representation(),
            expected_repr
        );
    }
}

#[test]
fn roaring_partition_limits_apply_to_promoted_outputs() {
    let policy = IntSetPolicy::dense()
        .with_dense_budget(8)
        .with_roaring_partition_limit(1);
    let left = set([1_u64], policy);
    let right = set([(1_u64 << 32) + 1], policy);
    let combined = left.union(&right);
    assert_eq!(combined.representation(), IntSetRepr::Sorted);
    assert_eq!(combined.limits(), left.limits());
    assert!(matches!(
        left.union_with_policy(
            &right,
            IntSetPolicy::roaring().with_roaring_partition_limit(1)
        ),
        Err(IntSetError::RoaringPartitionLimit {
            partitions: 2,
            max_partitions: 1
        })
    ));
    let combined = left
        .union_with_policy(
            &right,
            IntSetPolicy::roaring().with_roaring_partition_limit(2),
        )
        .unwrap();
    assert_eq!(combined.representation(), IntSetRepr::Roaring);
    assert_eq!(combined.limits().max_roaring_partitions(), 2);
    assert_eq!(combined.iter().collect::<Vec<_>>(), [1, (1_u64 << 32) + 1]);
}

#[cfg(not(miri))]
#[test]
fn explicit_large_dense_budget_is_not_clamped_to_the_default() {
    // One output allocation of 8 MiB + 8 bytes; avoid this scan in Miri.
    let gap = 1_u64 << 26;
    let policy = IntSetPolicy::dense().with_dense_budget(16 * 1024 * 1024);
    let left = set([0_u64], policy);
    let right = set([gap], policy);
    let automatic = left.union(&right);
    assert_eq!(automatic.limits().max_dense_bytes(), 16 * 1024 * 1024);
    let forced = left.union_with_policy(&right, policy).unwrap();
    assert_eq!(forced.representation(), IntSetRepr::Dense);
    assert_eq!(forced.limits().max_dense_bytes(), 16 * 1024 * 1024);
    assert_eq!(forced.contiguous_payload_bytes(), Some(8 * 1024 * 1024 + 8));
    assert_eq!(forced.iter().collect::<Vec<_>>(), [0, gap]);
}

#[derive(Debug, Default, PartialEq, Eq)]
struct HashEvents(Vec<(&'static str, Vec<u8>)>);

macro_rules! record_write {
    ($($method:ident($ty:ty)),+ $(,)?) => {
        $(fn $method(&mut self, value: $ty) {
            self.0.push((stringify!($method), value.to_ne_bytes().to_vec()));
        })+
    };
}

impl Hasher for HashEvents {
    fn finish(&self) -> u64 {
        0
    }
    fn write(&mut self, bytes: &[u8]) {
        self.0.push(("write", bytes.to_vec()));
    }
    record_write!(
        write_u8(u8),
        write_u16(u16),
        write_u32(u32),
        write_u64(u64),
        write_u128(u128),
        write_usize(usize),
        write_i8(i8),
        write_i16(i16),
        write_i32(i32),
        write_i64(i64),
        write_i128(i128),
        write_isize(isize),
    );
}

fn hash_events<K: IntKey>(set: &FrozenIntSet<K>) -> HashEvents {
    let mut events = HashEvents::default();
    set.hash(&mut events);
    events
}

fn hash_every_representation<K: IntKey + Debug>(values: &[K]) {
    let sets: Vec<_> = POLICIES
        .into_iter()
        .filter_map(
            |policy| match FrozenIntSet::try_from_iter(values.iter().copied(), policy) {
                Ok(set) => Some(set),
                Err(IntSetError::DenseBudgetExceeded { .. }) => None,
                Err(error) => panic!("unexpected construction failure: {error}"),
            },
        )
        .collect();
    let expected = hash_events(&sets[0]);
    for left in &sets {
        assert_eq!(hash_events(left), expected);
        for right in &sets {
            assert_eq!(left, right);
            assert_eq!(left.cmp(right), std::cmp::Ordering::Equal);
        }
    }
}

#[test]
fn canonical_hash_events_match_across_representations_and_boundaries() {
    hash_every_representation::<u64>(&[]);
    hash_every_representation::<u32>(&[]);
    hash_every_representation(&[0_u32]);
    hash_every_representation(&(0_u32..130).collect::<Vec<_>>());
    hash_every_representation(&((1_u32 << 16) - 130..(1_u32 << 16) + 130).collect::<Vec<_>>());
    hash_every_representation(&(u32::MAX - 130..=u32::MAX).collect::<Vec<_>>());
    hash_every_representation(&[0_u64, 63, 64, 65, 127, 128, 129]);
    hash_every_representation(&((1_u64 << 32) - 130..(1_u64 << 32) + 130).collect::<Vec<_>>());
    hash_every_representation(&(u64::MAX - 130..=u64::MAX).collect::<Vec<_>>());
    hash_every_representation(&[-65_i32, -64, -1, 0, 1, 63, 64, 65]);
    hash_every_representation(&(-130_i64..130).collect::<Vec<_>>());
    hash_every_representation(&(i64::MAX - 130..=i64::MAX).collect::<Vec<_>>());
    // The full-width sparse case has no eligible Dense representation.
    hash_every_representation(&[0_u64, 1, u64::MAX - 1, u64::MAX]);
}

#[test]
fn canonical_hash_scout_thresholds_are_representation_independent() {
    // The 64/65 cardinality boundary must use the same hashing strategy for
    // equal sets, including when the first member of a third run is appended.
    for length in [64_u64, 65] {
        let few_runs: Vec<_> = (0..length)
            .map(|value| value / 32 * 96 + value % 32)
            .collect();
        let singletons: Vec<_> = (0..length).map(|value| value * 2).collect();
        hash_every_representation(&few_runs);
        hash_every_representation(&singletons);
    }
    // Cardinality exceeds 64 on both sides of the 16/17-run scouting boundary.
    // The first run crosses a container/prefix boundary in two windows, so raw
    // backend range fragments must be coalesced before counting logical runs.
    for base in [
        0_u64,
        (1_u64 << 6) - 2,
        (1_u64 << 16) - 2,
        (1_u64 << 32) - 2,
        u64::MAX - 164,
    ] {
        for runs in [16_u64, 17] {
            let values: Vec<_> = (0..runs)
                .flat_map(|run| (0..5).map(move |offset| base + run * 10 + offset))
                .collect();
            hash_every_representation(&values);
        }
    }
    for runs in [16_i64, 17] {
        let values: Vec<_> = (0..runs)
            .flat_map(|run| (0..5).map(move |offset| -2 + run * 10 + offset))
            .collect();
        hash_every_representation(&values);
    }
}

#[test]
fn equality_and_hash_ignore_dense_padding_and_construction_history() {
    let original = set(0_u64..512, IntSetPolicy::dense());
    let removed = set((0_u64..71).chain(250..512), IntSetPolicy::roaring());
    let changed = original
        .difference_with_policy(&removed, IntSetPolicy::dense())
        .unwrap();
    let direct = set(71_u64..250, IntSetPolicy::dense());
    assert_eq!(changed, direct);
    assert_eq!(hash_events(&changed), hash_events(&direct));
    for policy in POLICIES {
        let rebuilt = direct.with_policy(policy).unwrap();
        let joined = set(71_u64..130, policy)
            .union_with_policy(&set(130..250, policy), policy)
            .unwrap();
        assert_eq!(joined, direct);
        assert_eq!(hash_events(&rebuilt), hash_events(&changed));
        assert_eq!(hash_events(&joined), hash_events(&changed));
    }
}

#[test]
fn equality_checks_members_even_when_metadata_matches() {
    for left_policy in POLICIES {
        for right_policy in POLICIES {
            let left = set([1_u64, 63, 64, 129], left_policy);
            let right = set([1_u64, 62, 65, 129], right_policy);
            assert_eq!(left.cardinality(), right.cardinality());
            assert_eq!(left.bounds(), right.bounds());
            assert_ne!(left, right);
        }
    }
}

#[test]
fn limits_are_excluded_from_logical_equality_and_hash() {
    let strict = set(
        0_u64..130,
        IntSetPolicy::sorted()
            .with_dense_budget(0)
            .with_roaring_partition_limit(0),
    );
    for policy in POLICIES {
        let other = set(0_u64..130, policy);
        assert_ne!(strict.limits(), other.limits());
        assert_eq!(strict, other);
        assert_eq!(strict.cmp(&other), std::cmp::Ordering::Equal);
        assert_eq!(hash_events(&strict), hash_events(&other));
    }
}

#[cfg(feature = "arc-swap-1")]
#[test]
fn repeated_snapshot_updates_preserve_dense_storage_and_old_versions() {
    use frost::concurrent::SnapshotStore;
    let initial: BTreeSet<_> = (0_u64..8192).step_by(2).collect();
    let store = SnapshotStore::new(set(initial.iter().copied(), IntSetPolicy::dense()));
    let retained = store.load();
    let mut expected = initial.clone();
    for value in (1_u64..64).step_by(2) {
        let addition = set([value], IntSetPolicy::roaring());
        let previous = store.update(|snapshot| snapshot.union(&addition));
        assert_members(&previous, &expected);
        expected.insert(value);
        let current = store.load();
        assert_eq!(current.representation(), IntSetRepr::Dense);
        assert_members(&current, &expected);
    }
    assert_members(&retained, &initial);
}

#[test]
fn identity_sharing_obeys_forced_representations_and_inherited_limits() {
    let values: Vec<_> = (0_u64..130).collect();
    let sets: Vec<_> = POLICIES
        .into_iter()
        .map(|policy| set(values.iter().copied(), policy))
        .collect();
    for (index, original) in sets.iter().enumerate() {
        let unchanged = original.with_policy(POLICIES[index]).unwrap();
        assert!(unchanged.shares_storage_with(original));
        for other in &sets {
            // For equal sets, prefer a policy-compatible operand over rebuilding
            // an incompatible operand merely because it came first.
            let left_result = original
                .intersection_with_policy(other, POLICIES[index])
                .unwrap();
            let right_result = other.union_with_policy(original, POLICIES[index]).unwrap();
            assert!(left_result.shares_storage_with(original));
            assert!(right_result.shares_storage_with(original));
        }
    }

    let strict = sets[0]
        .with_policy(
            IntSetPolicy::sorted()
                .with_dense_budget(0)
                .with_roaring_partition_limit(0),
        )
        .unwrap();
    for other in &sets[1..3] {
        let result = other.intersection(&strict);
        assert_eq!(result.limits(), strict.limits());
        assert!(result.shares_storage_with(&strict));
    }

    // Chunked storage is outside the dense/Roaring limits. Tightening those
    // limits can retain its allocation without weakening the carried policy.
    let chunked = sets[3]
        .with_policy(
            IntSetPolicy::chunked()
                .with_dense_budget(0)
                .with_roaring_partition_limit(0),
        )
        .unwrap();
    assert!(chunked.shares_storage_with(&sets[3]));
    assert_eq!(chunked.limits(), strict.limits());
    assert_eq!(chunked.contiguous_payload_bytes(), None);
    assert!(matches!(
        chunked.union_with_policy(
            &chunked,
            IntSetPolicy::roaring().with_roaring_partition_limit(0),
        ),
        Err(IntSetError::RoaringPartitionLimit { .. })
    ));
}

//! Typed integer storage: codec laws, all representation pairs, and boundaries.
#![cfg(feature = "int-set")]

use core::fmt::Debug;
use core::iter::FusedIterator;
use core::num::{NonZeroU32, NonZeroU64};
use frost::TryFreezeWith;
use frost::family::{FrozenOrderedSet, MutableOrderedSet};
use frost::integer::{FrozenIntSet, IntKey, IntSetError, IntSetPolicy, IntSetRepr};
use frost::read::{Cardinality, SetContains, ValueElements};
use proptest::prelude::*;
use std::cell::Cell;
use std::collections::{BTreeSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::panic::{AssertUnwindSafe, catch_unwind};

const POLICIES: [IntSetPolicy; 3] = [
    IntSetPolicy::sorted(),
    IntSetPolicy::dense(),
    IntSetPolicy::roaring(),
];

fn checked<K: IntKey>(values: &[K], policy: IntSetPolicy) -> FrozenIntSet<K> {
    FrozenIntSet::try_from_iter(values.iter().copied(), policy).unwrap()
}

fn assert_cursor<T: Copy + Debug + Eq, I: Iterator<Item = T> + Clone + FusedIterator>(
    iter: &I,
    expected: &[T],
) {
    for consumed in 0..=expected.len() {
        let mut tail = iter.clone();
        for value in &expected[..consumed] {
            assert_eq!(tail.next(), Some(*value));
        }
        let remaining = &expected[consumed..];
        let (lower, upper) = tail.size_hint();
        assert!(lower <= remaining.len());
        assert!(upper.is_none_or(|upper| upper >= remaining.len()));
        assert_eq!(tail.clone().count(), remaining.len());
        assert_eq!(tail.by_ref().collect::<Vec<_>>(), remaining);
        assert_eq!(tail.next(), None);
        assert_eq!(tail.next(), None);
    }
}

fn assert_members<K: IntKey + Debug>(actual: &FrozenIntSet<K>, expected: &[K]) {
    assert_eq!(actual.iter().collect::<Vec<_>>(), expected);
    assert_eq!(actual.cardinality(), u64::try_from(expected.len()).unwrap());
    assert_eq!(actual.try_len(), Some(expected.len()));
    assert_eq!(actual.is_empty(), expected.is_empty());
    assert_eq!(
        actual.bounds(),
        expected.first().copied().zip(expected.last().copied())
    );
    for member in expected {
        assert!(actual.contains(member));
    }
}

fn assert_algebra<K: IntKey + Debug>(
    left: &FrozenIntSet<K>,
    right: &FrozenIntSet<K>,
    left_reference: &BTreeSet<K>,
    right_reference: &BTreeSet<K>,
) {
    let expected = [
        left_reference
            .union(right_reference)
            .copied()
            .collect::<Vec<_>>(),
        left_reference
            .intersection(right_reference)
            .copied()
            .collect(),
        left_reference
            .difference(right_reference)
            .copied()
            .collect(),
        left_reference
            .symmetric_difference(right_reference)
            .copied()
            .collect(),
    ];
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
    for ((result, expected), count) in actual.iter().zip(&expected).zip(counts) {
        assert_members(result, expected);
        assert_eq!(count, u64::try_from(expected.len()).unwrap());
    }
    assert_eq!(
        left.is_subset(right),
        left_reference.is_subset(right_reference)
    );
    assert_eq!(
        left.is_superset(right),
        left_reference.is_superset(right_reference)
    );
    assert_eq!(
        left.is_disjoint(right),
        left_reference.is_disjoint(right_reference)
    );
}

fn every_pair<K: IntKey + Debug>(left: &[K], right: &[K]) {
    let left_reference = left.iter().copied().collect();
    let right_reference = right.iter().copied().collect();
    for left_policy in POLICIES {
        let left = checked(left, left_policy);
        for right_policy in POLICIES {
            let right = checked(right, right_policy);
            assert_algebra(&left, &right, &left_reference, &right_reference);
        }
    }
}

#[test]
fn builtin_codecs_preserve_width_boundaries_order_and_round_trips() {
    let signed32 = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX];
    let expected32 = [0, 1, 0x7fff_ffff, 0x8000_0000, 0x8000_0001, u32::MAX];
    assert_eq!(signed32.map(IntKey::encode), expected32);
    for (value, word) in signed32.into_iter().zip(expected32) {
        assert_eq!(i32::decode(word), Some(value));
    }
    let signed64 = [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX];
    let expected64 = [
        0,
        1,
        0x7fff_ffff_ffff_ffff,
        0x8000_0000_0000_0000,
        0x8000_0000_0000_0001,
        u64::MAX,
    ];
    assert_eq!(signed64.map(IntKey::encode), expected64);
    for (value, word) in signed64.into_iter().zip(expected64) {
        assert_eq!(i64::decode(word), Some(value));
    }
    for policy in [
        IntSetPolicy::sorted(),
        IntSetPolicy::roaring(),
        IntSetPolicy::auto(),
    ] {
        assert_members(&checked(&signed32, policy), &signed32);
        assert_members(&checked(&signed64, policy), &signed64);
        let unsigned32 = [0, 1, u32::MAX - 1, u32::MAX];
        let unsigned64 = [0, 1, u64::MAX - 1, u64::MAX];
        assert_members(&checked(&unsigned32, policy), &unsigned32);
        assert_members(&checked(&unsigned64, policy), &unsigned64);
    }
}

#[test]
fn forced_backends_and_iterators_preserve_typed_value_order() {
    let values = [-5_i64, -3, -1, 0, 1, 65, 66];
    for (policy, repr) in
        POLICIES
            .into_iter()
            .zip([IntSetRepr::Sorted, IntSetRepr::Dense, IntSetRepr::Roaring])
    {
        let set = checked(&values, policy);
        assert_eq!(set.representation(), repr);
        assert_cursor(&set.iter(), &values);
        assert_cursor(&set.encoded_values(), &values.map(IntKey::encode));
        assert_eq!((&set).into_iter().collect::<Vec<_>>(), values);
        assert_eq!(set.clone(), set);
    }
}

#[test]
fn all_backend_pairs_align_distinct_dense_windows_and_signed_zero() {
    every_pair(&[1_u32, 63, 64, 127, 130], &[62, 63, 65, 127, 191]);
    every_pair(&[37_u64, 64, 65, 127, 193], &[64, 66, 127, 129, 257]);
    every_pair(&[-130_i32, -65, -1, 0, 64], &[-65, -2, 0, 1, 65]);
    every_pair(&[-130_i64, -65, -1, 0, 64], &[-65, -2, 0, 1, 65]);
}

#[test]
fn mixed_backend_dispatch_preserves_algebra_at_balanced_and_skewed_sizes() {
    fn check_mixed<K: IntKey + Debug>(sorted_values: &[K], compressed_values: &[K]) {
        let sorted = checked(sorted_values, IntSetPolicy::sorted());
        let sorted_reference = sorted_values.iter().copied().collect();
        let compressed_reference = compressed_values.iter().copied().collect();
        for (policy, representation) in [
            (IntSetPolicy::dense(), IntSetRepr::Dense),
            (IntSetPolicy::roaring(), IntSetRepr::Roaring),
        ] {
            let compressed = checked(compressed_values, policy);
            if !compressed.is_empty() {
                assert_eq!(compressed.representation(), representation);
            }
            assert_algebra(
                &sorted,
                &compressed,
                &sorted_reference,
                &compressed_reference,
            );
            assert_algebra(
                &compressed,
                &sorted,
                &compressed_reference,
                &sorted_reference,
            );
        }
    }

    // A smaller compressed operand need not be the cheaper side to traverse:
    // probing sorted storage has a different cost from dense/Roaring lookup.
    let balanced_sorted: Vec<_> = (0_u32..8_192).map(|value| value * 2).collect();
    let balanced_compressed: Vec<_> = (0_u32..5_462).map(|value| value * 3 + 1).collect();
    let large: Vec<_> = (0_u32..4_096).map(|value| value * 2).collect();
    let small = [0, 1, 63, 64, 2_047, 4_095, 4_096, 8_192];
    let disjoint: Vec<_> = (0_u32..8).map(|value| value * 2 + 1).collect();
    let subset: Vec<_> = (0_u32..8).map(|value| value * 128).collect();
    let cases: [(&[u32], &[u32]); 8] = [
        (&balanced_sorted, &balanced_compressed),
        (&small, &large),
        (&large, &small),
        (&large, &disjoint),
        (&subset, &large),
        (&large, &subset),
        (&[], &large),
        (&large, &[]),
    ];
    for (sorted, compressed) in cases {
        check_mixed(sorted, compressed);
        let widen = |values: &[u32]| {
            values
                .iter()
                .map(|&value| (1_u64 << 48) + 37 + u64::from(value))
                .collect::<Vec<_>>()
        };
        check_mixed(&widen(sorted), &widen(compressed));
    }
}

#[test]
fn dense_windows_near_u64_max_align_without_overflow() {
    let top = u64::MAX;
    every_pair(
        &[top - 193, top - 129, top - 64, top],
        &[top - 192, top - 129, top - 63, top],
    );
    let signed_top = i64::MAX;
    every_pair(
        &[signed_top - 130, signed_top - 64, signed_top],
        &[signed_top - 129, signed_top - 64, signed_top],
    );
    let bottom = i64::MIN;
    every_pair(
        &[bottom, bottom + 64, bottom + 130],
        &[bottom + 1, bottom + 64, bottom + 131],
    );
}

#[test]
fn dense_full_u64_span_is_rejected_before_narrowing_or_allocating() {
    let error =
        FrozenIntSet::<u64>::try_from_iter([0, u64::MAX], IntSetPolicy::dense()).unwrap_err();
    assert!(
        matches!(error, IntSetError::DenseBudgetExceeded { required_bytes, .. } if required_bytes == 1_u128 << 61)
    );
    let sorted = checked(&[0_u64, u64::MAX], IntSetPolicy::sorted());
    assert_eq!(sorted.statistics().span, 1_u128 << 64);
    assert_eq!(sorted.cardinality(), 2);
    assert!(sorted.with_policy(IntSetPolicy::dense()).is_err());
    assert_members(&sorted, &[0, u64::MAX]);
}

#[test]
fn far_apart_dense_operands_do_not_allocate_the_gap() {
    let left = checked(&[0_u64, 1], IntSetPolicy::dense());
    let right = checked(&[u64::MAX - 1, u64::MAX], IntSetPolicy::dense());
    let union = left.union(&right);
    assert_members(&union, &[0, 1, u64::MAX - 1, u64::MAX]);
    assert_ne!(union.representation(), IntSetRepr::Dense);
    assert_members(
        &left.symmetric_difference(&right),
        &[0, 1, u64::MAX - 1, u64::MAX],
    );
    assert!(left.intersection(&right).is_empty());
    assert!(left.is_disjoint(&right));
    assert_eq!(left.difference(&right), left);
}

#[test]
fn finite_five_member_domains_exhaust_all_representation_pairs() {
    const VALUES: [u32; 5] = [61, 64, 65, 127, 130];
    for left_mask in 0_u8..32 {
        let left: Vec<_> = VALUES
            .into_iter()
            .enumerate()
            .filter_map(|(bit, value)| ((left_mask & (1 << bit)) != 0).then_some(value))
            .collect();
        for right_mask in 0_u8..32 {
            let right: Vec<_> = VALUES
                .into_iter()
                .enumerate()
                .filter_map(|(bit, value)| ((right_mask & (1 << bit)) != 0).then_some(value))
                .collect();
            every_pair(&left, &right);
            let left64: Vec<_> = left.iter().copied().map(u64::from).collect();
            let right64: Vec<_> = right.iter().copied().map(u64::from).collect();
            every_pair(&left64, &right64);
        }
    }
}

#[test]
fn empty_and_duplicate_inputs_have_set_semantics_in_every_backend() {
    for policy in POLICIES {
        let empty = checked::<u64>(&[], policy);
        assert_members(&empty, &[]);
        assert_cursor(&empty.iter(), &[]);
        let set = checked(&[9, 3, 9, 5, 3, 5, 9], policy);
        assert_members(&set, &[3, 5, 9]);
        assert_eq!(set.union(&empty), set);
        assert_eq!(empty.union(&set), set);
        assert_eq!(set.intersection(&empty), empty);
        assert_eq!(set.difference(&set), empty);
        assert!(empty.is_subset(&set));
        assert!(!set.is_subset(&empty));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Sparse {
    First,
    Middle,
    Last,
}

impl IntKey for Sparse {
    type Word = u32;
    fn encode(self) -> u32 {
        match self {
            Self::First => 1,
            Self::Middle => 4,
            Self::Last => 7,
        }
    }
    fn decode(word: u32) -> Option<Self> {
        match word {
            1 => Some(Self::First),
            4 => Some(Self::Middle),
            7 => Some(Self::Last),
            _ => None,
        }
    }
}

#[test]
fn restricted_domains_reject_imported_gaps_and_algebra_only_selects_members() {
    for policy in POLICIES {
        assert!(FrozenIntSet::<Sparse>::try_from_encoded([1, 2, 7], policy).is_err());
        assert!(FrozenIntSet::<NonZeroU32>::try_from_encoded([0, 1], policy).is_err());
        assert!(FrozenIntSet::<NonZeroU64>::try_from_encoded([0, u64::MAX], policy).is_err());
        let imported = FrozenIntSet::<Sparse>::try_from_encoded([7, 1, 4, 1], policy).unwrap();
        assert_members(&imported, &[Sparse::First, Sparse::Middle, Sparse::Last]);
        let nonzero = FrozenIntSet::<NonZeroU32>::try_from_encoded([3, 1, 3], policy).unwrap();
        assert_eq!(
            nonzero.iter().map(NonZeroU32::get).collect::<Vec<_>>(),
            [1, 3]
        );
    }
    every_pair(
        &[Sparse::First, Sparse::Last],
        &[Sparse::Middle, Sparse::Last],
    );
    let universe: FrozenIntSet<_> = [Sparse::First, Sparse::Middle, Sparse::Last]
        .into_iter()
        .collect();
    let excluded: FrozenIntSet<_> = [Sparse::Middle].into_iter().collect();
    assert_members(
        &universe.difference(&excluded),
        &[Sparse::First, Sparse::Last],
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Collision(u32);
impl IntKey for Collision {
    type Word = u32;
    fn encode(self) -> u32 {
        self.0 & 1
    }
    fn decode(word: u32) -> Option<Self> {
        Some(Self(word))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Reverse(u32);
impl IntKey for Reverse {
    type Word = u32;
    fn encode(self) -> u32 {
        u32::MAX - self.0
    }
    fn decode(word: u32) -> Option<Self> {
        Some(Self(u32::MAX - word))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct NonCanonical(u32);
impl IntKey for NonCanonical {
    type Word = u32;
    fn encode(self) -> u32 {
        self.0
    }
    fn decode(word: u32) -> Option<Self> {
        Some(Self(word & !1))
    }
}

#[test]
fn broken_codec_laws_are_checked_without_unsafe_value_construction() {
    for policy in POLICIES {
        assert!(matches!(
            FrozenIntSet::try_from_iter([Collision(2)], policy),
            Err(IntSetError::CodecRoundTrip { .. })
        ));
        assert!(matches!(
            FrozenIntSet::try_from_iter([Reverse(1), Reverse(2)], policy),
            Err(IntSetError::NonMonotonicEncoding { .. })
        ));
        assert!(matches!(
            FrozenIntSet::<NonCanonical>::try_from_encoded([3], policy),
            Err(IntSetError::NonCanonicalCode { .. })
        ));
    }
    let failure = catch_unwind(|| [Collision(2)].into_iter().collect::<FrozenIntSet<_>>());
    assert!(failure.is_err());
}

thread_local! {
    static DECODING_DISABLED: Cell<bool> = const { Cell::new(false) };
    static ENCODE_CALLS: Cell<usize> = const { Cell::new(0) };
    static DECODE_CALLS: Cell<usize> = const { Cell::new(0) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Unstable(u32);
impl IntKey for Unstable {
    type Word = u32;
    fn encode(self) -> u32 {
        self.0
    }
    fn decode(word: u32) -> Option<Self> {
        DECODING_DISABLED.with(|disabled| (!disabled.get()).then_some(Self(word)))
    }
}

#[test]
fn a_decoder_that_changes_after_construction_causes_a_checked_panic() {
    let set: FrozenIntSet<_> = [Unstable(3)].into_iter().collect();
    DECODING_DISABLED.set(true);
    let failure = catch_unwind(AssertUnwindSafe(|| set.iter().next()));
    DECODING_DISABLED.set(false);
    assert!(failure.is_err());
    assert_eq!(set.iter().collect::<Vec<_>>(), [Unstable(3)]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Counted(u32);
impl IntKey for Counted {
    type Word = u32;
    fn encode(self) -> u32 {
        ENCODE_CALLS.set(ENCODE_CALLS.get() + 1);
        self.0
    }
    fn decode(word: u32) -> Option<Self> {
        DECODE_CALLS.set(DECODE_CALLS.get() + 1);
        Some(Self(word))
    }
}

#[test]
fn bulk_algebra_and_replanning_do_not_reencode_or_decode_keys() {
    for left_policy in POLICIES {
        for right_policy in POLICIES {
            let left = checked(&[Counted(1), Counted(64), Counted(129)], left_policy);
            let right = checked(&[Counted(64), Counted(65), Counted(130)], right_policy);
            ENCODE_CALLS.set(0);
            DECODE_CALLS.set(0);
            assert_eq!(left.intersection_len(&right), 1);
            assert_eq!(left.union_len(&right), 5);
            assert_eq!(left.difference_len(&right), 2);
            assert_eq!(left.symmetric_difference_len(&right), 4);
            assert!(!left.is_subset(&right));
            assert!(!left.is_disjoint(&right));
            let union = left.union(&right);
            let common = left.intersection(&right);
            let difference = left.difference(&right);
            let symmetric = left.symmetric_difference(&right);
            let rebuilt = left.with_policy(right_policy).unwrap();
            assert_eq!(
                union.encoded_values().collect::<Vec<_>>(),
                [1, 64, 65, 129, 130]
            );
            assert_eq!(common.encoded_values().collect::<Vec<_>>(), [64]);
            assert_eq!(difference.encoded_values().collect::<Vec<_>>(), [1, 129]);
            assert_eq!(
                symmetric.encoded_values().collect::<Vec<_>>(),
                [1, 65, 129, 130]
            );
            assert_eq!(rebuilt, left);
            assert_eq!(
                ENCODE_CALLS.get(),
                0,
                "bulk operations re-encoded logical keys"
            );
            assert_eq!(
                DECODE_CALLS.get(),
                0,
                "bulk operations decoded logical keys"
            );
            assert!(left.contains(&Counted(64)));
            assert_eq!(
                ENCODE_CALLS.get(),
                1,
                "membership must encode its query once"
            );
            assert_eq!(DECODE_CALLS.get(), 0);
        }
    }
}

#[test]
fn freeze_plan_preserves_failed_sources_and_ordinary_pairings() {
    let original = vec![u64::MAX, 0, u64::MAX];
    let rejected = original
        .clone()
        .try_freeze_with(IntSetPolicy::dense())
        .unwrap_err();
    assert_eq!(rejected.input, original);
    let tree: MutableOrderedSet<_> = original.iter().copied().collect();
    let rejected_tree = tree
        .try_freeze_with(IntSetPolicy::dense())
        .unwrap_err()
        .input;
    assert_eq!(
        rejected_tree.iter().copied().collect::<Vec<_>>(),
        [0, u64::MAX]
    );
    let ordinary: FrozenOrderedSet<_> = rejected_tree.freeze();
    let rejected_frozen = ordinary
        .try_freeze_with(IntSetPolicy::dense())
        .unwrap_err()
        .input;
    assert_eq!(
        rejected_frozen.iter().copied().collect::<Vec<_>>(),
        [0, u64::MAX]
    );
    let int_set = rejected_frozen
        .try_freeze_with(IntSetPolicy::roaring())
        .unwrap();
    assert_members(&int_set, &[0, u64::MAX]);
}

#[test]
fn explicit_budget_and_partition_limits_are_enforced() {
    assert!(checked::<u32>(&[], IntSetPolicy::dense().with_dense_budget(0)).is_empty());
    assert!(
        FrozenIntSet::try_from_iter([1_u32], IntSetPolicy::dense().with_dense_budget(7)).is_err()
    );
    assert!(
        FrozenIntSet::try_from_iter([1_u32, 64], IntSetPolicy::dense().with_dense_budget(8))
            .is_err()
    );
    let one_word = checked(&[1_u32, 63], IntSetPolicy::dense().with_dense_budget(8));
    assert_members(&one_word, &[1, 63]);
    let two_partitions = [1_u64, (1_u64 << 32) + 1];
    assert!(
        FrozenIntSet::try_from_iter(
            two_partitions,
            IntSetPolicy::roaring().with_roaring_partition_limit(1)
        )
        .is_err()
    );
    assert_members(
        &checked(
            &two_partitions,
            IntSetPolicy::roaring().with_roaring_partition_limit(2),
        ),
        &two_partitions,
    );
    let automatic = checked(
        &two_partitions,
        IntSetPolicy::auto()
            .with_dense_budget(0)
            .with_roaring_partition_limit(0),
    );
    assert_eq!(automatic.representation(), IntSetRepr::Sorted);
}

#[test]
fn auto_planner_respects_local_span_and_scattered_upper_prefixes() {
    let base = 1_000_000_000_000_000_u64;
    let window: Vec<_> = (0..2048).map(|offset| base + offset * 2).collect();
    let automatic = checked(&window, IntSetPolicy::auto());
    assert_ne!(automatic.representation(), IntSetRepr::Sorted);
    assert_members(&automatic, &window);
    let scattered: Vec<_> = (0..32_u64).map(|prefix| (prefix << 32) + prefix).collect();
    let bounded = checked(
        &scattered,
        IntSetPolicy::auto().with_roaring_partition_limit(4),
    );
    assert_eq!(bounded.representation(), IntSetRepr::Sorted);
    assert_eq!(bounded.statistics().high32_partitions, 32);
}

fn hash_value(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn logical_equality_order_and_hash_ignore_representation() {
    let values = [-3_i32, 0, 5, 65];
    let sets = POLICIES.map(|policy| checked(&values, policy));
    for left in &sets {
        for right in &sets {
            assert_eq!(left, right);
            assert_eq!(left.cmp(right), core::cmp::Ordering::Equal);
            assert_eq!(hash_value(left), hash_value(right));
        }
    }
    let later = checked(&[-3_i32, 0, 6, 65], IntSetPolicy::dense());
    assert!(sets[0] < later);
}

#[test]
fn capability_traits_expose_reconstructed_values_and_wide_cardinality() {
    fn require_thread_safe<T: Send + Sync>() {}
    fn generic_thread_safe<K: IntKey + Send + Sync>() {
        require_thread_safe::<FrozenIntSet<K>>();
    }
    fn collect_values<C: ValueElements<Item = u64>>(collection: &C) -> Vec<u64> {
        collection.values().collect()
    }
    generic_thread_safe::<i32>();
    generic_thread_safe::<u64>();
    let set: FrozenIntSet<u64> = [5, 1, 3].into_iter().collect();
    assert_eq!(collect_values(&set), [1, 3, 5]);
    assert_eq!(Cardinality::cardinality(&set), 3);
    assert!(SetContains::contains(&set, &3));
    assert_eq!(FrozenIntSet::<u64>::default().statistics().span, 0);
}

#[cfg(feature = "arc-swap-1")]
#[test]
fn snapshot_publication_retains_integer_profile_versions() {
    use frost::concurrent::SnapshotStore;
    let original = checked(&[1_u64, 65], IntSetPolicy::dense());
    let addition = checked(&[65_u64, 130], IntSetPolicy::roaring());
    let store = SnapshotStore::new(original);
    let old = store.load_guard();
    let displaced = store.update(|snapshot| snapshot.union(&addition));
    assert_members(&old, &[1, 65]);
    assert_members(&displaced, &[1, 65]);
    assert_eq!(
        store.peek(|snapshot| snapshot.iter().collect::<Vec<_>>()),
        [1, 65, 130]
    );
}

#[cfg(feature = "arc-swap-1")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ConcurrentId(u64);

#[cfg(feature = "arc-swap-1")]
impl IntKey for ConcurrentId {
    type Word = u64;
    fn encode(self) -> u64 {
        self.0
    }
    fn decode(word: u64) -> Option<Self> {
        Some(Self(word))
    }
}

#[cfg(feature = "arc-swap-1")]
#[test]
fn concurrent_integer_updates_publish_complete_versions_without_lost_ids() {
    use frost::concurrent::SnapshotStore;
    use std::sync::Barrier;

    const WRITERS: usize = 4;
    const UPDATES: u64 = 64;
    const BASE: u64 = 1_000_000_000_000_000;
    let store = SnapshotStore::new(checked(&[ConcurrentId(BASE - 1)], IntSetPolicy::dense()));
    let retained = store.load();
    let start = Barrier::new(WRITERS + 2);
    std::thread::scope(|scope| {
        let reader = scope.spawn(|| {
            let initial = store.load_guard();
            start.wait();
            // Fixed work keeps the regression bounded even if a writer panics.
            // Each individually loaded version must remain complete and sorted.
            for _ in 0..256 {
                let current = store.load_guard();
                let values = current.iter().collect::<Vec<_>>();
                assert_eq!(values.first(), Some(&ConcurrentId(BASE - 1)));
                assert!(values.windows(2).all(|pair| pair[0] < pair[1]));
                assert!(values.iter().all(|id| {
                    (BASE - 1..BASE + u64::try_from(WRITERS).unwrap() * UPDATES).contains(&id.0)
                }));
                assert_eq!(current.cardinality(), u64::try_from(values.len()).unwrap());
                std::thread::yield_now();
            }
            assert_members(&initial, &[ConcurrentId(BASE - 1)]);
        });
        let writers: Vec<_> = (0..WRITERS)
            .map(|writer| {
                let store = &store;
                let start = &start;
                scope.spawn(move || {
                    start.wait();
                    for step in 0..UPDATES {
                        let id =
                            ConcurrentId(BASE + u64::try_from(writer).unwrap() * UPDATES + step);
                        let singleton = checked(&[id], IntSetPolicy::roaring());
                        store.update(|current| current.union(&singleton));
                    }
                })
            })
            .collect();
        start.wait();
        for writer in writers {
            writer.join().unwrap();
        }
        reader.join().unwrap();
    });
    let expected: Vec<_> = (BASE - 1..BASE + u64::try_from(WRITERS).unwrap() * UPDATES)
        .map(ConcurrentId)
        .collect();
    assert_members(&store.load(), &expected);
    assert_members(&retained, &[ConcurrentId(BASE - 1)]);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn signed_encodings_preserve_generated_order(a in any::<i32>(), b in any::<i32>(), c in any::<i64>(), d in any::<i64>()) {
        prop_assert_eq!(a.cmp(&b), a.encode().cmp(&b.encode()));
        prop_assert_eq!(c.cmp(&d), c.encode().cmp(&d.encode()));
        prop_assert_eq!(i32::decode(a.encode()), Some(a));
        prop_assert_eq!(i64::decode(c.encode()), Some(c));
    }

    #[test]
    fn windowed_u64_algebra_matches_standard_sets_in_every_backend(
        base in 0_u64..=u64::MAX - 511,
        left in proptest::collection::vec(0_u16..512, 0..65),
        right in proptest::collection::vec(0_u16..512, 0..65),
    ) {
        let left: Vec<_> = left.into_iter().map(|offset| base + u64::from(offset)).collect();
        let right: Vec<_> = right.into_iter().map(|offset| base + u64::from(offset)).collect();
        every_pair(&left, &right);
    }

    #[test]
    fn arbitrary_u64_sorted_and_roaring_algebra_match_standard_sets(
        left in proptest::collection::vec(any::<u64>(), 0..65),
        right in proptest::collection::vec(any::<u64>(), 0..65),
    ) {
        let left_reference = left.iter().copied().collect();
        let right_reference = right.iter().copied().collect();
        for left_policy in [IntSetPolicy::sorted(), IntSetPolicy::roaring()] {
            for right_policy in [IntSetPolicy::sorted(), IntSetPolicy::roaring()] {
                assert_algebra(&checked(&left, left_policy), &checked(&right, right_policy), &left_reference, &right_reference);
            }
        }
    }
}

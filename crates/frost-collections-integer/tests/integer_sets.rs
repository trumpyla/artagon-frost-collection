//! Typed integer storage: codec laws, all representation pairs, and boundaries.

// The package under test, under the name its users import.
extern crate frost_collections_integer as frost;

use core::fmt::Debug;
use core::iter::FusedIterator;
use core::num::{NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64};
use frost::TryFreezeWith;
use frost::family::{FrozenOrderedSet, MutableOrderedSet};
use frost::integer::{FrozenIntSet, IntKey, IntSetError, IntSetPolicy, IntSetRepr};
use frost::read::{Cardinality, SetContains, ValueElements};
use proptest::prelude::*;
use std::cell::Cell;
use std::collections::{BTreeSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::panic::{AssertUnwindSafe, catch_unwind};

const POLICIES: [IntSetPolicy; 4] = [
    IntSetPolicy::sorted(),
    IntSetPolicy::dense(),
    IntSetPolicy::roaring(),
    IntSetPolicy::chunked(),
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
        // `fold` selects the representation once; it must agree with `next`.
        let folded = tail.clone().fold(Vec::new(), |mut seen, value| {
            seen.push(value);
            seen
        });
        assert_eq!(folded, remaining);
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

/// Build with a policy, skipping only a dense window the budget rejects.
fn eligible<K: IntKey>(values: &[K], policy: IntSetPolicy) -> Option<FrozenIntSet<K>> {
    match FrozenIntSet::try_from_iter(values.iter().copied(), policy) {
        Ok(set) => Some(set),
        Err(IntSetError::DenseBudgetExceeded { .. }) => None,
        Err(error) => panic!("unexpected construction failure: {error}"),
    }
}

fn every_pair<K: IntKey + Debug>(left: &[K], right: &[K]) {
    let left_reference = left.iter().copied().collect();
    let right_reference = right.iter().copied().collect();
    for left_policy in POLICIES {
        let Some(left) = eligible(left, left_policy) else {
            continue;
        };
        for right_policy in POLICIES {
            let Some(right) = eligible(right, right_policy) else {
                continue;
            };
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
    for (policy, repr) in POLICIES.into_iter().zip([
        IntSetRepr::Sorted,
        IntSetRepr::Dense,
        IntSetRepr::Roaring,
        IntSetRepr::Chunked,
    ]) {
        let set = checked(&values, policy);
        assert_eq!(set.representation(), repr);
        assert_cursor(&set.iter(), &values);
        assert_cursor(&set.encoded_values(), &values.map(IntKey::encode));
        assert_eq!((&set).into_iter().collect::<Vec<_>>(), values);
        assert_eq!(set.clone(), set);
    }
}

/// The public iterators forward `fold` to the storage traversal. A partially
/// consumed iterator must fold exactly its remaining members in every
/// representation, including Chunked Array, Bitmap, and Full containers and
/// positions on both sides of a container boundary.
#[test]
fn public_iterator_folds_resume_after_partial_consumption() {
    let mut values: Vec<u32> = (0..40).map(|value| value * 1_000).collect(); // Array
    values.extend((65_536..75_536).step_by(2)); // Bitmap (5,000 members)
    values.extend(131_072..196_608); // Full
    values.extend([196_608, 262_143, u32::MAX]); // Array, isolated maximum
    values.sort_unstable();
    let boundaries = [
        0, 1, 39, 40, 41, 5_039, 5_040, 5_041, 70_575, 70_576, 70_577,
    ];
    let mut covered = BTreeSet::new();
    // The complete span exceeds the Dense budget; the shorter one fits it.
    let spans = [&values[..], &values[..values.len() - 1]];
    for (values, policy) in spans.into_iter().flat_map(|values| {
        POLICIES
            .into_iter()
            .chain([IntSetPolicy::auto()])
            .map(move |policy| (values, policy))
    }) {
        let Some(set) = eligible(values, policy) else {
            continue;
        };
        covered.insert(format!("{:?}", set.representation()));
        for consumed in boundaries
            .into_iter()
            .chain([values.len() - 1, values.len()])
        {
            let mut keys = set.iter();
            let mut codes = set.encoded_values();
            for expected in &values[..consumed] {
                assert_eq!(keys.next(), Some(*expected));
                assert_eq!(codes.next(), Some(*expected));
            }
            let remaining = &values[consumed..];
            let sum: u64 = remaining.iter().map(|&value| u64::from(value)).sum();
            assert_eq!(keys.clone().map(u64::from).sum::<u64>(), sum);
            assert_eq!(
                codes.clone().fold(0, |total, code| total + u64::from(code)),
                sum
            );
            let mut visited = 0;
            keys.clone().for_each(|_| visited += 1);
            assert_eq!(visited, remaining.len());
            assert_eq!(keys.clone().last(), remaining.last().copied());
            assert!(keys.eq(remaining.iter().copied()));
            assert!(codes.eq(remaining.iter().copied()));
        }
    }
    assert_eq!(covered.len(), 4, "every representation was traversed");
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
            (IntSetPolicy::chunked(), IntSetRepr::Chunked),
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
    // Four members fit more compactly in sorted storage than in two native
    // partitions. Larger distant runs have a separate Roaring regression test.
    assert_eq!(union.representation(), IntSetRepr::Sorted);
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

/// A codec with one fault of each kind at a fixed code, for comparing error
/// reporting across construction paths. Code 13 does not decode, code 21
/// decodes to a key with another code, and the keys for codes 40 and 41
/// compare in the opposite order to their codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Quirk(u32);
impl Quirk {
    const fn rank(self) -> u32 {
        match self.0 {
            40 => 41,
            41 => 40,
            other => other,
        }
    }
}
impl PartialOrd for Quirk {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Quirk {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.rank().cmp(&other.rank())
    }
}
impl IntKey for Quirk {
    type Word = u32;
    fn encode(self) -> u32 {
        self.0
    }
    fn decode(word: u32) -> Option<Self> {
        match word {
            13 => None,
            21 => Some(Self(22)),
            other => Some(Self(other)),
        }
    }
}

/// The two-pass construction contract, written without any fast path: check
/// each key in arrival order, sort and deduplicate the codes, then check each
/// code in sorted order. Indices are arrival positions in the first pass and
/// sorted positions in the second.
fn reference_construction<K: IntKey>(
    keys: Option<&[K]>,
    codes: &[K::Word],
) -> Result<Vec<K::Word>, IntSetError> {
    let mut words = Vec::new();
    if let Some(keys) = keys {
        for (index, &key) in keys.iter().enumerate() {
            let word = key.encode();
            let decoded = K::decode(word).ok_or(IntSetError::InvalidCode { index })?;
            if decoded != key || decoded.cmp(&key) != core::cmp::Ordering::Equal {
                return Err(IntSetError::CodecRoundTrip { index });
            }
            if decoded.encode() != word {
                return Err(IntSetError::NonCanonicalCode { index });
            }
            words.push(word);
        }
    } else {
        for (index, &word) in codes.iter().enumerate() {
            let key = K::decode(word).ok_or(IntSetError::InvalidCode { index })?;
            if key.encode() != word {
                return Err(IntSetError::NonCanonicalCode { index });
            }
            words.push(word);
        }
    }
    words.sort_unstable();
    words.dedup();
    let mut previous: Option<K> = None;
    for (index, &word) in words.iter().enumerate() {
        let key = K::decode(word).ok_or(IntSetError::InvalidCode { index })?;
        if key.encode() != word {
            return Err(IntSetError::NonCanonicalCode { index });
        }
        if previous.is_some_and(|prior| prior.cmp(&key) != core::cmp::Ordering::Less) {
            return Err(IntSetError::NonMonotonicEncoding { index });
        }
        previous = Some(key);
    }
    Ok(words)
}

/// Deterministic arrival orders of one multiset of codes.
fn arrival_orders(sorted: &[u32], seed: u64) -> Vec<Vec<u32>> {
    let mut state = seed | 1;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut shuffled = sorted.to_vec();
    for index in (1..shuffled.len()).rev() {
        let other = usize::try_from(next() % (index as u64 + 1)).unwrap();
        shuffled.swap(index, other);
    }
    let mut descending = sorted.to_vec();
    descending.reverse();
    let mut duplicated = shuffled.clone();
    duplicated.extend(sorted.iter().step_by(3));
    let mut one_swap = sorted.to_vec();
    if one_swap.len() > 1 {
        let last = one_swap.len() - 1;
        one_swap.swap(last - 1, last);
    }
    let mut repeated_tail = sorted.to_vec();
    repeated_tail.extend(sorted.last());
    vec![
        sorted.to_vec(),
        descending,
        shuffled,
        duplicated,
        one_swap,
        repeated_tail,
    ]
}

/// Ascending input skips the sort, dense unsorted input is sorted through a
/// bitmap, and other input is sorted by comparison. All three must produce the
/// same members under every policy, on both sides of the bitmap threshold
/// (the span equals the staged bytes) and at the ends of both word widths.
#[test]
fn construction_normalizes_every_arrival_order_identically() {
    let shapes: Vec<Vec<u32>> = vec![
        vec![],
        vec![7],
        (0..63).collect(),                            // below the bitmap minimum
        (0..64).map(|code| code * 32 + 31).collect(), // 64 codes, span 2,048 bits: bitmap
        (0..64).map(|code| code * 33).collect(),      // 64 codes, span 2,080 bits: comparison sort
        (0..5_000).map(|code| code * 7 + 3).collect(),
        (0..300).map(|code| u32::MAX - code * 5).rev().collect(),
        (0..200).map(|code| code * 1_000_003).collect(),
        (0..70_000).collect(),
    ];
    for (seed, shape) in shapes.iter().enumerate() {
        let expected: BTreeSet<u32> = shape.iter().copied().collect();
        let expected: Vec<u32> = expected.into_iter().collect();
        for order in arrival_orders(shape, seed as u64 + 1) {
            for policy in POLICIES.into_iter().chain([IntSetPolicy::auto()]) {
                let built = FrozenIntSet::try_from_iter(order.iter().copied(), policy);
                let imported = FrozenIntSet::<u32>::try_from_encoded(order.iter().copied(), policy);
                match (built, imported) {
                    (Ok(built), Ok(imported)) => {
                        assert_members(&built, &expected);
                        assert_eq!(built, imported);
                    }
                    (Err(left), Err(right)) => {
                        assert_eq!(left, right);
                        assert!(matches!(left, IntSetError::DenseBudgetExceeded { .. }));
                    }
                    other => panic!("construction paths disagree: {other:?}"),
                }
            }
        }
    }
    // Signed keys and the top of the 64-bit domain, shuffled into a dense span.
    let signed: Vec<i64> = (-200..200).map(|value| value * 3).collect();
    let top: Vec<u64> = (0..400).map(|value| u64::MAX - value * 2).rev().collect();
    let mut signed_shuffled = signed.clone();
    signed_shuffled.rotate_left(137);
    signed_shuffled.swap(3, 250);
    let mut top_shuffled = top.clone();
    top_shuffled.rotate_left(91);
    for policy in POLICIES.into_iter().chain([IntSetPolicy::auto()]) {
        assert_members(&checked(&signed_shuffled, policy), &signed);
        assert_members(&checked(&top_shuffled, policy), &top);
    }
}

/// Every construction path reports the error the two-pass contract reports:
/// the same variant and the same index, for keys and for imported codes, for
/// ascending, unsorted sparse, and unsorted dense (bitmap-sorted) input.
#[test]
fn construction_reports_contract_errors_on_every_path() {
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut outcomes = BTreeSet::new();
    for case in 0..4_000 {
        // Short inputs take the comparison sort; long dense ones the bitmap.
        let length = if case % 4 == 0 {
            64 + next() % 40
        } else {
            next() % 9
        };
        let universe = if case % 3 == 0 { 48 } else { 400 };
        let faults = [13_u32, 21, 40, 41];
        let mut codes: Vec<u32> = (0..length)
            .map(|_| {
                let roll = next();
                if roll % 23 == 0 {
                    faults[usize::try_from((roll >> 8) % 4).unwrap()]
                } else {
                    // Keep ordinary codes clear of the fault codes.
                    100 + u32::try_from((roll >> 8) % universe).unwrap()
                }
            })
            .collect();
        match case % 5 {
            0 => codes.sort_unstable(),
            1 => {
                codes.sort_unstable();
                codes.dedup();
            }
            _ => {}
        }
        let keys: Vec<Quirk> = codes.iter().copied().map(Quirk).collect();
        for policy in [IntSetPolicy::auto(), IntSetPolicy::sorted()] {
            let expected = reference_construction::<Quirk>(Some(&keys), &[]);
            let actual = FrozenIntSet::try_from_iter(keys.iter().copied(), policy)
                .map(|set| set.encoded_values().collect::<Vec<_>>());
            assert_eq!(actual, expected, "keys {codes:?}");
            let expected = reference_construction::<Quirk>(None, &codes);
            let actual = FrozenIntSet::<Quirk>::try_from_encoded(codes.iter().copied(), policy)
                .map(|set| set.encoded_values().collect::<Vec<_>>());
            assert_eq!(actual, expected, "codes {codes:?}");
            outcomes.insert(match expected {
                Ok(_) => "ok",
                Err(IntSetError::InvalidCode { .. }) => "invalid",
                Err(IntSetError::NonCanonicalCode { .. }) => "noncanonical",
                Err(IntSetError::NonMonotonicEncoding { .. }) => "nonmonotonic",
                Err(_) => "other",
            });
        }
    }
    assert_eq!(
        outcomes.into_iter().collect::<Vec<_>>(),
        ["invalid", "noncanonical", "nonmonotonic", "ok"]
    );
    // The first pass completes before any order violation is reported, even
    // when ascending input reveals the violation earlier.
    assert_eq!(
        FrozenIntSet::try_from_iter([40, 41, 50, 13].map(Quirk), IntSetPolicy::auto()).err(),
        Some(IntSetError::InvalidCode { index: 3 })
    );
    assert_eq!(
        FrozenIntSet::try_from_iter([39, 40, 41, 50].map(Quirk), IntSetPolicy::auto()).err(),
        Some(IntSetError::NonMonotonicEncoding { index: 2 })
    );
    assert_eq!(
        FrozenIntSet::try_from_iter([41, 40].map(Quirk), IntSetPolicy::auto()).err(),
        Some(IntSetError::NonMonotonicEncoding { index: 1 })
    );
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
    fn two_cluster_u64_algebra_matches_standard_sets_in_every_backend(
        base in 0_u64..=u64::MAX - (1 << 41),
        gap in 1_u64..(1 << 40),
        left in proptest::collection::vec((0_u16..512, any::<bool>()), 0..65),
        right in proptest::collection::vec((0_u16..512, any::<bool>()), 0..65),
    ) {
        let place = |(offset, far): (u16, bool)| base + u64::from(offset) + if far { gap } else { 0 };
        let left: Vec<_> = left.into_iter().map(place).collect();
        let right: Vec<_> = right.into_iter().map(place).collect();
        every_pair(&left, &right);
    }

    #[test]
    fn arbitrary_u64_sorted_and_roaring_algebra_match_standard_sets(
        left in proptest::collection::vec(any::<u64>(), 0..65),
        right in proptest::collection::vec(any::<u64>(), 0..65),
    ) {
        let left_reference = left.iter().copied().collect();
        let right_reference = right.iter().copied().collect();
        for left_policy in [IntSetPolicy::sorted(), IntSetPolicy::roaring(), IntSetPolicy::chunked()] {
            for right_policy in [IntSetPolicy::sorted(), IntSetPolicy::roaring(), IntSetPolicy::chunked()] {
                assert_algebra(&checked(&left, left_policy), &checked(&right, right_policy), &left_reference, &right_reference);
            }
        }
    }
}

fn payload_stays_compact<K: IntKey + Debug>(
    result: &FrozenIntSet<K>,
    left: &FrozenIntSet<K>,
    right: &FrozenIntSet<K>,
) {
    // A result never costs more than the operands it was derived from, plus
    // a fixed allowance for one container or window of slack.
    let allowance = 16 * 1024;
    assert!(
        result.payload_bytes() <= left.payload_bytes() + right.payload_bytes() + allowance,
        "result payload {} exceeds operands {} + {} ({:?} from {:?} and {:?})",
        result.payload_bytes(),
        left.payload_bytes(),
        right.payload_bytes(),
        result.representation(),
        left.representation(),
        right.representation()
    );
}

#[test]
fn mixed_results_keep_the_compact_operand_representation() {
    let base = 1_u64 << 40;
    let big: Vec<u64> = (0..100_000_u64).map(|i| base + i * 2).collect();
    let dense = checked(&big, IntSetPolicy::dense());
    let singleton = checked(&[base + 1], IntSetPolicy::roaring());
    let union = dense.union(&singleton);
    assert_eq!(union.representation(), IntSetRepr::Dense);
    assert_eq!(union.cardinality(), 100_001);
    payload_stays_compact(&union, &dense, &singleton);
    let tiny_sorted = checked(
        &[base + 2, base + 4, base + 6, base + 7],
        IntSetPolicy::sorted(),
    );
    let difference = dense.difference(&tiny_sorted);
    assert_eq!(difference.representation(), IntSetRepr::Dense);
    assert_eq!(difference.cardinality(), 99_997);
    payload_stays_compact(&difference, &dense, &tiny_sorted);
    let symmetric = dense.symmetric_difference(&tiny_sorted);
    assert_eq!(symmetric.representation(), IntSetRepr::Dense);
    assert_eq!(symmetric.cardinality(), 99_998);
    let roaring = checked(&big, IntSetPolicy::roaring());
    let plus = roaring.union(&tiny_sorted);
    assert_eq!(plus.representation(), IntSetRepr::Roaring);
    assert_eq!(plus.cardinality(), 100_001);
    let minus = roaring.difference(&tiny_sorted);
    assert_eq!(minus.representation(), IntSetRepr::Roaring);
    assert_eq!(minus.cardinality(), 99_997);
    // Balanced dense/Roaring pairs promote the window and use native kernels.
    let shifted: Vec<u64> = (0..100_000_u64).map(|i| base + i * 3 + 1).collect();
    let other = checked(&shifted, IntSetPolicy::roaring());
    let common = dense.intersection(&other);
    assert_eq!(common.representation(), IntSetRepr::Roaring);
    assert_eq!(common.cardinality(), dense.intersection_len(&other));
    let reference: BTreeSet<u64> = big.iter().copied().collect();
    let shifted_reference: BTreeSet<u64> = shifted.iter().copied().collect();
    assert_eq!(
        common.iter().collect::<Vec<_>>(),
        reference
            .intersection(&shifted_reference)
            .copied()
            .collect::<Vec<_>>()
    );
    // Chunked operands absorb the other representation per container.
    let chunked = checked(&big, IntSetPolicy::chunked());
    for other in [&singleton, &tiny_sorted, &dense, &roaring] {
        let union = chunked.union(other);
        assert_eq!(union.representation(), IntSetRepr::Chunked);
        assert_eq!(union.cardinality(), chunked.union_len(other));
        payload_stays_compact(&union, &chunked, other);
    }
    // The snapshot pattern stays in its representation across updates.
    let mut current = dense.clone();
    for step in 0..32_u64 {
        let addition = checked(&[base + 3_000_000 + step], IntSetPolicy::roaring());
        current = current.union(&addition);
        assert_eq!(current.representation(), IntSetRepr::Dense);
    }
    assert_eq!(current.cardinality(), 100_032);
}

#[test]
fn budgets_are_inherited_by_results() {
    let low = checked(&[0_u64, 5], IntSetPolicy::dense().with_dense_budget(8));
    let high = checked(
        &[1_u64 << 20, (1_u64 << 20) + 5],
        IntSetPolicy::dense().with_dense_budget(8),
    );
    let union = low.union(&high);
    assert_ne!(union.representation(), IntSetRepr::Dense);
    assert_eq!(union.policy().max_dense_bytes(), 8);
    assert_members(&union, &[0, 5, 1 << 20, (1 << 20) + 5]);

    let near: Vec<u64> = (0..4096).collect();
    let far: Vec<u64> = (131_072..131_072 + 4096).collect();
    let small_budget = IntSetPolicy::dense().with_dense_budget(16 * 1024);
    let capped = checked(&near, small_budget).union(&checked(&far, small_budget));
    assert_eq!(capped.representation(), IntSetRepr::Roaring);
    assert_eq!(capped.cardinality(), 8192);
    let wide_budget = IntSetPolicy::dense().with_dense_budget(32 * 1024);
    let widened = checked(&near, wide_budget).union(&checked(&far, wide_budget));
    assert_eq!(widened.representation(), IntSetRepr::Dense);
    assert_eq!(widened.policy().max_dense_bytes(), 32 * 1024);
    // Preserve the main project's contract: the smaller limit controls a
    // mixed-budget union, rather than silently widening the strict operand.
    let mixed = checked(&near, small_budget).union(&checked(&far, wide_budget));
    assert_eq!(mixed.representation(), IntSetRepr::Roaring);
    assert_eq!(mixed.policy().max_dense_bytes(), 16 * 1024);
}

#[test]
fn cloning_and_identity_results_share_storage() {
    for policy in POLICIES {
        let set = checked(&[1_u32, 70, 300, 70_000], policy);
        let copy = set.clone();
        assert!(copy.shares_storage_with(&set));
        assert_eq!(copy, set);
        let empty = FrozenIntSet::<u32>::new();
        assert!(set.union(&empty).shares_storage_with(&set));
        assert!(empty.union(&set).shares_storage_with(&set));
        assert!(set.difference(&empty).shares_storage_with(&set));
        assert!(set.union(&set).shares_storage_with(&set));
        assert!(set.intersection(&set).shares_storage_with(&set));
        let subset = checked(&[70_u32, 300], IntSetPolicy::sorted());
        assert!(set.union(&subset).shares_storage_with(&set));
        assert!(subset.union(&set).shares_storage_with(&set));
        assert!(set.intersection(&subset).shares_storage_with(&subset));
        assert!(subset.intersection(&set).shares_storage_with(&subset));
        let disjoint = checked(&[2_u32, 71], IntSetPolicy::sorted());
        assert!(set.difference(&disjoint).shares_storage_with(&set));
        assert!(!set.union(&disjoint).shares_storage_with(&set));
    }
}

/// A sorted operand with more occupied partitions than the native budget takes
/// the direct merge path. That path must still share an equal eligible operand.
#[test]
fn scattered_sorted_results_share_an_equal_operand() {
    let codes: Vec<u64> = (0..4097_u64).map(|index| (index << 32) | 7).collect();
    let set = checked(&codes, IntSetPolicy::auto());
    assert_eq!(set.representation(), IntSetRepr::Sorted);
    let equal = checked(&codes, IntSetPolicy::auto());
    assert!(!equal.shares_storage_with(&set));
    let half: Vec<u64> = codes.iter().copied().step_by(2).collect();
    let subsets = [
        vec![codes[0]],
        vec![codes[codes.len() - 1]],
        half,
        codes.clone(),
    ];
    for subset in &subsets {
        for policy in POLICIES {
            let Ok(other) = FrozenIntSet::<u64>::try_from_iter(subset.iter().copied(), policy)
            else {
                continue;
            };
            let left = set.union(&other);
            assert!(
                left.shares_storage_with(&set),
                "{policy:?} {}",
                subset.len()
            );
            let right = other.union(&set);
            assert!(
                right.shares_storage_with(&set) || right.shares_storage_with(&other),
                "{policy:?} {}",
                subset.len()
            );
            assert_eq!(left, set);
            assert_eq!(right, set);
        }
    }
    assert!(set.union(&equal).shares_storage_with(&set));
    assert!(equal.union(&set).shares_storage_with(&equal));
    let grown = set.union(&checked(&[9_u64], IntSetPolicy::auto()));
    assert!(!grown.shares_storage_with(&set));
    assert_eq!(grown.cardinality(), 4098);
}

/// An empty result shares an empty operand that satisfies the result policy,
/// so the operation allocates no new storage header.
#[test]
fn empty_results_share_an_eligible_empty_operand() {
    let empty = FrozenIntSet::<u32>::new();
    for policy in POLICIES {
        let set = checked(&[1_u32, 70, 300, 70_000], policy);
        assert!(set.intersection(&empty).shares_storage_with(&empty));
        assert!(empty.intersection(&set).shares_storage_with(&empty));
        assert!(empty.difference(&set).shares_storage_with(&empty));
        assert!(empty.difference(&empty).shares_storage_with(&empty));
        assert!(
            empty
                .symmetric_difference(&empty)
                .shares_storage_with(&empty)
        );
        let other_empty = FrozenIntSet::<u32>::new();
        assert!(empty.intersection(&other_empty).shares_storage_with(&empty));
        assert!(empty.difference(&other_empty).shares_storage_with(&empty));
        // No operand is empty: the result needs its own header.
        let vanished = set.difference(&set);
        assert!(vanished.is_empty());
        assert!(!vanished.shares_storage_with(&empty));
        // The forced policy still controls the result representation.
        let forced = set.intersection_with_policy(&empty, policy).unwrap();
        assert!(forced.is_empty());
        let built = checked::<u32>(&[], policy);
        assert_eq!(forced.representation(), built.representation());
        assert_eq!(forced.limits(), built.limits());
    }
}

#[test]
fn chunked_results_share_untouched_containers() {
    let values: Vec<u32> = (0..8)
        .flat_map(|chunk| (0..100).map(move |i| chunk * 65_536 + i * 7))
        .collect();
    let snapshot = checked(&values, IntSetPolicy::chunked());
    assert_eq!(snapshot.chunk_count(), 8);
    let inside = checked(&[65_536 * 3 + 1], IntSetPolicy::chunked());
    let updated = snapshot.union(&inside);
    assert_eq!(updated.chunk_count(), 8);
    assert_eq!(updated.shared_chunks(&snapshot), 7);
    assert_eq!(updated.cardinality(), 801);
    let outside = checked(&[65_536 * 9], IntSetPolicy::sorted());
    let extended = snapshot.union(&outside);
    assert_eq!(extended.chunk_count(), 9);
    assert_eq!(extended.shared_chunks(&snapshot), 8);
    let member = checked(&[65_536 * 3 + 7], IntSetPolicy::chunked());
    let removed = snapshot.difference(&member);
    assert_eq!(removed.shared_chunks(&snapshot), 7);
    assert_eq!(removed.cardinality(), 799);
    // Removing a non-member changes nothing and shares everything.
    assert!(snapshot.difference(&inside).shares_storage_with(&snapshot));
    // Shared containers short-circuit without reading their payload.
    assert_eq!(snapshot.intersection(&updated), snapshot);
    assert!(snapshot.is_subset(&updated));
    assert_eq!(
        updated.difference(&snapshot).iter().collect::<Vec<_>>(),
        [65_536 * 3 + 1]
    );
    assert_eq!(snapshot.symmetric_difference(&updated).cardinality(), 1);
    let mut current = snapshot.clone();
    let mut expected: BTreeSet<u32> = values.iter().copied().collect();
    for step in 0..64_u32 {
        let value = 65_536 * (step % 8) + 65_000 + step;
        let addition = checked(&[value], IntSetPolicy::chunked());
        let next = current.union(&addition);
        assert_eq!(next.shared_chunks(&current), 7);
        expected.insert(value);
        current = next;
    }
    assert_eq!(
        current.iter().collect::<Vec<_>>(),
        expected.into_iter().collect::<Vec<_>>()
    );
    // Full containers are recognized and need no payload.
    let full: Vec<u32> = (0..65_536).collect();
    let full_set = checked(&full, IntSetPolicy::chunked());
    // Full containers have no member buffer; the reported index cost depends
    // on the target's pointer width and alignment.
    assert!(full_set.payload_bytes() <= 32);
    let partial = checked(&[5_u32, 9], IntSetPolicy::chunked());
    assert!(full_set.union(&partial).shares_storage_with(&full_set));
    assert!(
        partial
            .intersection(&full_set)
            .shares_storage_with(&partial)
    );
    assert_eq!(full_set.difference(&partial).cardinality(), 65_534);
}

#[test]
fn full_containers_take_part_in_every_representation_pair() {
    let mut full_plus: Vec<u32> = (0..65_536).collect();
    full_plus.push(65_536 + 5);
    full_plus.push(65_536 * 4 + 1);
    let partial = [3_u32, 65_535, 65_536 + 5, 65_536 * 2 + 9, 65_536 * 4 + 2];
    every_pair(&full_plus, &partial);
    every_pair(&partial, &full_plus);
    let two_full: Vec<u32> = (0..65_536).chain(65_536 * 3..65_536 * 4).collect();
    every_pair(&full_plus, &two_full);
    let chunked_full = checked(&two_full, IntSetPolicy::chunked());
    assert_eq!(chunked_full.chunk_count(), 2);
    assert!(chunked_full.payload_bytes() <= 64);
    let nearly: Vec<u32> = (1..65_536).collect();
    every_pair(&nearly, &two_full);
    let complement = chunked_full.difference(&checked(&nearly, IntSetPolicy::chunked()));
    assert_eq!(complement.cardinality(), 65_537);
    assert!(complement.contains(&0));
    assert!(!complement.contains(&1));
}

#[test]
fn narrow_and_address_sized_keys_round_trip_and_reject_foreign_codes() {
    let bytes = [0_u8, 1, 127, 128, 255];
    let signed_bytes = [i8::MIN, -1, 0, 1, i8::MAX];
    let shorts = [0_u16, 1, u16::MAX];
    let signed_shorts = [i16::MIN, -1, 0, i16::MAX];
    let chars = ['a', 'z', '\u{10FFFF}', '\0'];
    let sizes = [0_usize, 1, usize::MAX];
    let signed_sizes = [isize::MIN, -1, 0, isize::MAX];
    for policy in POLICIES {
        assert_members(&checked(&bytes, policy), &bytes);
        assert_members(&checked(&signed_bytes, policy), &signed_bytes);
        assert_members(&checked(&shorts, policy), &shorts);
        assert_members(&checked(&signed_shorts, policy), &signed_shorts);
        let mut sorted_chars = chars;
        sorted_chars.sort_unstable();
        if let Some(set) = eligible(&chars, policy) {
            assert_members(&set, &sorted_chars);
        }
        // Address-sized extremes span the whole word; dense windows decline.
        if let Some(set) = eligible(&sizes, policy) {
            assert_members(&set, &sizes);
        } else {
            assert_eq!(policy.backend(), frost::integer::IntSetBackend::Dense);
        }
        if let Some(set) = eligible(&signed_sizes, policy) {
            assert_members(&set, &signed_sizes);
        }
        assert!(FrozenIntSet::<u8>::try_from_encoded([256], policy).is_err());
        assert!(FrozenIntSet::<i8>::try_from_encoded([256], policy).is_err());
        assert!(FrozenIntSet::<u16>::try_from_encoded([65_536], policy).is_err());
        assert!(FrozenIntSet::<char>::try_from_encoded([0xD800], policy).is_err());
        assert!(FrozenIntSet::<NonZeroU8>::try_from_encoded([0], policy).is_err());
        assert!(FrozenIntSet::<NonZeroU16>::try_from_encoded([0, 70_000], policy).is_err());
        let nonzero = FrozenIntSet::<NonZeroU8>::try_from_encoded([3, 1], policy).unwrap();
        assert_eq!(
            nonzero.iter().map(NonZeroU8::get).collect::<Vec<_>>(),
            [1, 3]
        );
    }
    assert_eq!(i8::decode(i8::MIN.encode()), Some(i8::MIN));
    assert!(i8::MIN.encode() < 0_i8.encode() && 0_i8.encode() < i8::MAX.encode());
    assert!(isize::MIN.encode() < 0_isize.encode() && 0_isize.encode() < isize::MAX.encode());
}

//! Differential fuzz oracles for Frost's ordered-family and integer-set profiles.
//!
//! The library target exposes deterministic bounded decoders and typed oracle
//! entry points so `make fuzz-check` replays seed corpora, synthetic byte
//! sweeps, and direct typed edge cases under the pinned stable Rust 1.99.1
//! compiler, while `make fuzz-smoke` drives the binary targets under
//! `cargo-fuzz` / `libFuzzer` on the diagnostic nightly toolchain.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use arbitrary::{Arbitrary, Result as ArbitraryResult, Unstructured};
use frost::TryFreezeWith;
use frost::family::{FrozenOrderedMap, FrozenOrderedSet, MutableOrderedMap, MutableOrderedSet};
use frost::integer::{FrozenIntSet, IntSetPolicy, IntSetRepr};

/// Maximum number of elements decoded per operand to bound allocation during
/// fuzzing before collection construction begins.
pub const MAX_ITEMS: usize = 64;

/// A key whose logical equivalence and ordering depend only on `id`, while
/// `representative` remains observable to verify first-key and left-side
/// representative retention.
#[derive(Arbitrary, Clone, Copy, Debug)]
pub struct RepKey {
    pub id: i16,
    pub representative: u8,
}

impl PartialEq for RepKey {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for RepKey {}

impl PartialOrd for RepKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RepKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for RepKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Clone, Debug)]
pub struct OrderedAlgebraCase {
    pub left: Vec<RepKey>,
    pub right: Vec<RepKey>,
    pub prefix_consumption: u8,
}

impl<'a> Arbitrary<'a> for OrderedAlgebraCase {
    fn arbitrary(u: &mut Unstructured<'a>) -> ArbitraryResult<Self> {
        if u.is_empty() {
            return Ok(Self {
                left: Vec::new(),
                right: Vec::new(),
                prefix_consumption: 0,
            });
        }
        let left_req = usize::from(u.arbitrary::<u8>()?) % (MAX_ITEMS + 1);
        let right_req = if u.is_empty() {
            0
        } else {
            usize::from(u.arbitrary::<u8>()?) % (MAX_ITEMS + 1)
        };
        let prefix_consumption = if u.is_empty() {
            0
        } else {
            u.arbitrary::<u8>()?
        };

        let available_keys = u.len() / 3;
        let (left_cap, right_cap) = balance_capacities(left_req, right_req, available_keys);

        let mut left = Vec::with_capacity(left_cap);
        for _ in 0..left_cap {
            if u.len() < 3 {
                break;
            }
            let lo = u.arbitrary::<u8>()?;
            let hi = u.arbitrary::<u8>()?;
            let rep = u.arbitrary::<u8>()?;
            left.push(RepKey {
                id: i16::from_le_bytes([lo, hi]),
                representative: rep,
            });
        }

        let mut right = Vec::with_capacity(right_cap);
        for _ in 0..right_cap {
            if u.len() < 3 {
                break;
            }
            let lo = u.arbitrary::<u8>()?;
            let hi = u.arbitrary::<u8>()?;
            let rep = u.arbitrary::<u8>()?;
            right.push(RepKey {
                id: i16::from_le_bytes([lo, hi]),
                representative: rep,
            });
        }

        Ok(Self {
            left,
            right,
            prefix_consumption,
        })
    }
}

fn balance_capacities(left_req: usize, right_req: usize, available: usize) -> (usize, usize) {
    let left_req = left_req.min(MAX_ITEMS);
    let right_req = right_req.min(MAX_ITEMS);
    if left_req + right_req <= available {
        return (left_req, right_req);
    }
    if available == 0 {
        return (0, 0);
    }
    if right_req == 0 {
        return (left_req.min(available), 0);
    }
    if left_req == 0 {
        return (0, right_req.min(available));
    }
    let half = available.div_ceil(2);
    let left_cap = left_req.min(half).min(MAX_ITEMS);
    let right_cap = right_req
        .min(available.saturating_sub(left_cap))
        .min(MAX_ITEMS);
    (left_cap, right_cap)
}

fn canonical_first_rep(items: &[RepKey]) -> BTreeMap<i16, u8> {
    let mut oracle = BTreeMap::new();
    for item in items.iter().take(MAX_ITEMS) {
        oracle.entry(item.id).or_insert(item.representative);
    }
    oracle
}

/// Decodes a bounded `OrderedAlgebraCase` from raw fuzz bytes.
pub fn decode_ordered_case(bytes: &[u8]) -> Option<OrderedAlgebraCase> {
    let mut unstructured = Unstructured::new(bytes);
    OrderedAlgebraCase::arbitrary(&mut unstructured).ok()
}

/// Differential oracle for `MutableOrderedSet`, `FrozenOrderedSet`, and
/// `MutableOrderedMap` / `FrozenOrderedMap`:
/// - First equivalent representative wins during construction.
/// - Last value wins during map construction without replacing the stored key.
/// - Left representative wins in `union` and `intersection`.
/// - Scalar counts (`union_len`, `intersection_len`, `difference_len`,
///   `symmetric_difference_len`), predicates (`is_subset`, `is_superset`,
///   `is_disjoint`) against an independent `BTreeSet` oracle, and
///   partial-iterator `count`/`fold` match the oracle.
pub fn check_ordered_set_algebra(bytes: &[u8]) {
    if let Some(case) = decode_ordered_case(bytes) {
        run_ordered_case(&case);
    }
}

pub fn run_ordered_case(case: &OrderedAlgebraCase) {
    let left_items: &[RepKey] = &case.left[..case.left.len().min(MAX_ITEMS)];
    let right_items: &[RepKey] = &case.right[..case.right.len().min(MAX_ITEMS)];

    let left_oracle = canonical_first_rep(left_items);
    let right_oracle = canonical_first_rep(right_items);
    let left_ids: BTreeSet<i16> = left_oracle.keys().copied().collect();
    let right_ids: BTreeSet<i16> = right_oracle.keys().copied().collect();

    let left_mutable: MutableOrderedSet<RepKey> = left_items.iter().copied().collect();
    let left_frozen: FrozenOrderedSet<RepKey> = left_items.iter().copied().collect();
    let right_frozen: FrozenOrderedSet<RepKey> = right_items.iter().copied().collect();

    let expected_left: Vec<(i16, u8)> = left_oracle.iter().map(|(&id, &rep)| (id, rep)).collect();
    assert_eq!(
        left_mutable
            .iter()
            .map(|k| (k.id, k.representative))
            .collect::<Vec<_>>(),
        expected_left
    );
    assert_eq!(
        left_frozen
            .iter()
            .map(|k| (k.id, k.representative))
            .collect::<Vec<_>>(),
        expected_left
    );

    // Map construction: first key representative + last value wins.
    let mut map_oracle: BTreeMap<i16, (u8, usize)> = BTreeMap::new();
    for (idx, item) in left_items.iter().copied().enumerate() {
        map_oracle
            .entry(item.id)
            .and_modify(|entry| entry.1 = idx)
            .or_insert((item.representative, idx));
    }
    let map_mutable: MutableOrderedMap<RepKey, usize> = left_items
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, k)| (k, idx))
        .collect();
    let map_frozen: FrozenOrderedMap<RepKey, usize> = left_items
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, k)| (k, idx))
        .collect();
    let expected_map: Vec<(i16, u8, usize)> = map_oracle
        .iter()
        .map(|(&id, &(rep, val))| (id, rep, val))
        .collect();
    assert_eq!(
        map_mutable
            .iter()
            .map(|(k, &v)| (k.id, k.representative, v))
            .collect::<Vec<_>>(),
        expected_map
    );
    assert_eq!(
        map_frozen
            .iter()
            .map(|(k, &v)| (k.id, k.representative, v))
            .collect::<Vec<_>>(),
        expected_map
    );

    // Set algebra with left-representative retention.
    let mut expected_union = left_oracle.clone();
    for (&id, &rep) in &right_oracle {
        expected_union.entry(id).or_insert(rep);
    }
    let expected_intersection: Vec<(i16, u8)> = left_oracle
        .iter()
        .filter(|(id, _)| right_oracle.contains_key(id))
        .map(|(&id, &rep)| (id, rep))
        .collect();
    let expected_difference: Vec<(i16, u8)> = left_oracle
        .iter()
        .filter(|(id, _)| !right_oracle.contains_key(id))
        .map(|(&id, &rep)| (id, rep))
        .collect();
    let mut expected_sym_diff = BTreeMap::new();
    for (&id, &rep) in &left_oracle {
        if !right_oracle.contains_key(&id) {
            expected_sym_diff.insert(id, rep);
        }
    }
    for (&id, &rep) in &right_oracle {
        if !left_oracle.contains_key(&id) {
            expected_sym_diff.insert(id, rep);
        }
    }

    let actual_union: Vec<(i16, u8)> = left_frozen
        .union(&right_frozen)
        .map(|k| (k.id, k.representative))
        .collect();
    let actual_intersection: Vec<(i16, u8)> = left_frozen
        .intersection(&right_frozen)
        .map(|k| (k.id, k.representative))
        .collect();
    let actual_difference: Vec<(i16, u8)> = left_frozen
        .difference(&right_frozen)
        .map(|k| (k.id, k.representative))
        .collect();
    let actual_sym_diff: Vec<(i16, u8)> = left_frozen
        .symmetric_difference(&right_frozen)
        .map(|k| (k.id, k.representative))
        .collect();

    assert_eq!(
        actual_union,
        expected_union
            .iter()
            .map(|(&id, &rep)| (id, rep))
            .collect::<Vec<_>>()
    );
    assert_eq!(actual_intersection, expected_intersection);
    assert_eq!(actual_difference, expected_difference);
    assert_eq!(
        actual_sym_diff,
        expected_sym_diff
            .iter()
            .map(|(&id, &rep)| (id, rep))
            .collect::<Vec<_>>()
    );

    assert_eq!(left_frozen.union_len(&right_frozen), expected_union.len());
    assert_eq!(
        left_frozen.intersection_len(&right_frozen),
        expected_intersection.len()
    );
    assert_eq!(
        left_frozen.difference_len(&right_frozen),
        expected_difference.len()
    );
    assert_eq!(
        left_frozen.symmetric_difference_len(&right_frozen),
        expected_sym_diff.len()
    );

    // Independent BTreeSet oracle for subset/superset/disjoint predicates.
    assert_eq!(
        left_frozen.is_subset(&right_frozen),
        left_ids.is_subset(&right_ids)
    );
    assert_eq!(
        left_frozen.is_superset(&right_frozen),
        left_ids.is_superset(&right_ids)
    );
    assert_eq!(
        left_frozen.is_disjoint(&right_frozen),
        left_ids.is_disjoint(&right_ids)
    );

    // Partial iterator consumption + remaining count and fold verification.
    let prefix = usize::from(case.prefix_consumption) % (actual_union.len() + 1);
    let mut union_iter = left_frozen.union(&right_frozen);
    for expected in actual_union.iter().take(prefix) {
        let item = union_iter.next().expect("prefix within union length");
        assert_eq!((item.id, item.representative), *expected);
    }
    let (lower, upper) = union_iter.size_hint();
    let remaining_expected = actual_union.len() - prefix;
    assert!(lower <= remaining_expected);
    assert!(upper.is_none_or(|up| remaining_expected <= up));
    assert_eq!(union_iter.count(), remaining_expected);

    let mut fold_iter = left_frozen.union(&right_frozen);
    for _ in 0..prefix {
        let _ = fold_iter.next();
    }
    let folded_tail: Vec<(i16, u8)> = fold_iter.fold(Vec::new(), |mut acc, k| {
        acc.push((k.id, k.representative));
        acc
    });
    assert_eq!(&folded_tail[..], &actual_union[prefix..]);
}

#[derive(Arbitrary, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicySelector {
    Auto,
    Sorted,
    Dense,
    Roaring,
    Chunked,
}

impl PolicySelector {
    pub const ALL: [Self; 5] = [
        Self::Auto,
        Self::Sorted,
        Self::Dense,
        Self::Roaring,
        Self::Chunked,
    ];

    pub fn from_byte(byte: u8) -> Self {
        Self::ALL[usize::from(byte) % Self::ALL.len()]
    }

    pub const fn expected_forced_repr(self) -> Option<IntSetRepr> {
        match self {
            Self::Auto => None,
            Self::Sorted => Some(IntSetRepr::Sorted),
            Self::Dense => Some(IntSetRepr::Dense),
            Self::Roaring => Some(IntSetRepr::Roaring),
            Self::Chunked => Some(IntSetRepr::Chunked),
        }
    }

    pub fn to_policy(self, dense_budget: u16, roaring_partitions: u8) -> IntSetPolicy {
        let base = match self {
            Self::Auto => IntSetPolicy::auto(),
            Self::Sorted => IntSetPolicy::sorted(),
            Self::Dense => IntSetPolicy::dense(),
            Self::Roaring => IntSetPolicy::roaring(),
            Self::Chunked => IntSetPolicy::chunked(),
        };
        let bytes = usize::from(dense_budget).max(64);
        let partitions = usize::from(roaring_partitions).max(1);
        base.with_dense_budget(bytes)
            .with_roaring_partition_limit(partitions)
    }
}

#[derive(Clone, Debug)]
pub struct IntegerAlgebraCase {
    pub left: Vec<i16>,
    pub right: Vec<i16>,
    pub left_policy: PolicySelector,
    pub right_policy: PolicySelector,
    pub out_policy: PolicySelector,
    pub dense_budget: u16,
    pub roaring_partitions: u8,
}

impl<'a> Arbitrary<'a> for IntegerAlgebraCase {
    fn arbitrary(u: &mut Unstructured<'a>) -> ArbitraryResult<Self> {
        if u.is_empty() {
            return Ok(Self {
                left: Vec::new(),
                right: Vec::new(),
                left_policy: PolicySelector::Auto,
                right_policy: PolicySelector::Auto,
                out_policy: PolicySelector::Auto,
                dense_budget: 256,
                roaring_partitions: 4,
            });
        }
        let left_req = usize::from(u.arbitrary::<u8>()?) % (MAX_ITEMS + 1);
        let right_req = if u.is_empty() {
            0
        } else {
            usize::from(u.arbitrary::<u8>()?) % (MAX_ITEMS + 1)
        };
        let left_policy = PolicySelector::from_byte(if u.is_empty() {
            0
        } else {
            u.arbitrary::<u8>()?
        });
        let right_policy = PolicySelector::from_byte(if u.is_empty() {
            0
        } else {
            u.arbitrary::<u8>()?
        });
        let out_policy = PolicySelector::from_byte(if u.is_empty() {
            0
        } else {
            u.arbitrary::<u8>()?
        });
        let dense_budget = if u.len() >= 2 {
            u16::from_le_bytes([u.arbitrary::<u8>()?, u.arbitrary::<u8>()?])
        } else {
            256
        };
        let roaring_partitions = if u.is_empty() {
            4
        } else {
            u.arbitrary::<u8>()?
        };

        let available_items = u.len() / 2;
        let (left_cap, right_cap) = balance_capacities(left_req, right_req, available_items);

        let mut left = Vec::with_capacity(left_cap);
        for _ in 0..left_cap {
            if u.len() < 2 {
                break;
            }
            let lo = u.arbitrary::<u8>()?;
            let hi = u.arbitrary::<u8>()?;
            left.push(i16::from_le_bytes([lo, hi]));
        }

        let mut right = Vec::with_capacity(right_cap);
        for _ in 0..right_cap {
            if u.len() < 2 {
                break;
            }
            let lo = u.arbitrary::<u8>()?;
            let hi = u.arbitrary::<u8>()?;
            right.push(i16::from_le_bytes([lo, hi]));
        }

        Ok(Self {
            left,
            right,
            left_policy,
            right_policy,
            out_policy,
            dense_budget,
            roaring_partitions,
        })
    }
}

/// Decodes a bounded `IntegerAlgebraCase` from raw fuzz bytes.
pub fn decode_integer_case(bytes: &[u8]) -> Option<IntegerAlgebraCase> {
    let mut unstructured = Unstructured::new(bytes);
    IntegerAlgebraCase::arbitrary(&mut unstructured).ok()
}

/// Differential oracle for `FrozenIntSet<i32>` across all five `IntSetPolicy`
/// modes (`Auto` plus four explicit candidates: `Sorted`, `Dense`, `Roaring`,
/// `Chunked`), verifying:
/// - Materialized union, intersection, difference, and symmetric difference.
/// - Scalar cardinality kernels (`union_len`, `intersection_len`,
///   `difference_len`, `symmetric_difference_len`).
/// - Independent `BTreeSet` predicate equivalence (`is_subset`, `is_superset`,
///   `is_disjoint`) and iterator `fold` equivalence.
/// - Representation-independent equality and canonical `Hash` event streams.
pub fn check_integer_set_algebra(bytes: &[u8]) {
    if let Some(case) = decode_integer_case(bytes) {
        run_integer_case(&case);
    }
}

pub fn run_integer_case(case: &IntegerAlgebraCase) {
    let left_vec: Vec<i32> = case
        .left
        .iter()
        .take(MAX_ITEMS)
        .copied()
        .map(i32::from)
        .collect();
    let right_vec: Vec<i32> = case
        .right
        .iter()
        .take(MAX_ITEMS)
        .copied()
        .map(i32::from)
        .collect();

    let left_oracle: BTreeSet<i32> = left_vec.iter().copied().collect();
    let right_oracle: BTreeSet<i32> = right_vec.iter().copied().collect();

    let left_pol = case
        .left_policy
        .to_policy(case.dense_budget, case.roaring_partitions);
    let right_pol = case
        .right_policy
        .to_policy(case.dense_budget, case.roaring_partitions);
    let out_pol = case
        .out_policy
        .to_policy(case.dense_budget, case.roaring_partitions);

    let left = match left_vec.clone().try_freeze_with(left_pol) {
        Ok(set) => {
            assert_construction_repr(&set, case.left_policy);
            set
        }
        Err(rejected) => {
            assert_eq!(rejected.input, left_vec);
            rejected
                .input
                .try_freeze_with(IntSetPolicy::sorted())
                .expect("sorted policy always accepts valid i32 keys")
        }
    };
    let right = match right_vec.clone().try_freeze_with(right_pol) {
        Ok(set) => {
            assert_construction_repr(&set, case.right_policy);
            set
        }
        Err(rejected) => {
            assert_eq!(rejected.input, right_vec);
            rejected
                .input
                .try_freeze_with(IntSetPolicy::sorted())
                .expect("sorted policy always accepts valid i32 keys")
        }
    };

    verify_integer_algebra(&left, &right, &left_oracle, &right_oracle);

    if let Ok(forced_union) = left.union_with_policy(&right, out_pol) {
        assert_output_policy_repr(&forced_union, case.out_policy, &left, &right);
        let expected_union: Vec<i32> = left_oracle.union(&right_oracle).copied().collect();
        let sorted_reference = FrozenIntSet::try_from_iter(expected_union, IntSetPolicy::sorted())
            .expect("sorted reference succeeds");
        assert_eq!(forced_union, sorted_reference);
        assert_eq!(hash_events(&forced_union), hash_events(&sorted_reference));
    }
}

/// Strict oracle for inputs whose span and partition count are known to be
/// eligible under all five `IntSetPolicy` modes (`Auto`, `Sorted`, `Dense`,
/// `Roaring`, `Chunked`). Rejects any fallback to `Sorted` on construction or
/// `*_with_policy` output and verifies that forced policies produce their
/// exact requested `IntSetRepr`.
pub fn run_integer_case_strict(case: &IntegerAlgebraCase) {
    let left_vec: Vec<i32> = case
        .left
        .iter()
        .take(MAX_ITEMS)
        .copied()
        .map(i32::from)
        .collect();
    let right_vec: Vec<i32> = case
        .right
        .iter()
        .take(MAX_ITEMS)
        .copied()
        .map(i32::from)
        .collect();

    let left_oracle: BTreeSet<i32> = left_vec.iter().copied().collect();
    let right_oracle: BTreeSet<i32> = right_vec.iter().copied().collect();

    let left_pol = case
        .left_policy
        .to_policy(case.dense_budget, case.roaring_partitions);
    let right_pol = case
        .right_policy
        .to_policy(case.dense_budget, case.roaring_partitions);
    let out_pol = case
        .out_policy
        .to_policy(case.dense_budget, case.roaring_partitions);

    let left = left_vec
        .try_freeze_with(left_pol)
        .expect("strict eligible case must succeed under left_policy");
    let right = right_vec
        .try_freeze_with(right_pol)
        .expect("strict eligible case must succeed under right_policy");

    assert_construction_repr(&left, case.left_policy);
    assert_construction_repr(&right, case.right_policy);

    verify_integer_algebra(&left, &right, &left_oracle, &right_oracle);

    let expected_union: Vec<i32> = left_oracle.union(&right_oracle).copied().collect();
    let expected_intersection: Vec<i32> =
        left_oracle.intersection(&right_oracle).copied().collect();
    let expected_difference: Vec<i32> = left_oracle.difference(&right_oracle).copied().collect();
    let expected_sym_diff: Vec<i32> = left_oracle
        .symmetric_difference(&right_oracle)
        .copied()
        .collect();

    let forced_union = left
        .union_with_policy(&right, out_pol)
        .expect("strict eligible case must succeed under union_with_policy");
    let forced_intersection = left
        .intersection_with_policy(&right, out_pol)
        .expect("strict eligible case must succeed under intersection_with_policy");
    let forced_difference = left
        .difference_with_policy(&right, out_pol)
        .expect("strict eligible case must succeed under difference_with_policy");
    let forced_sym_diff = left
        .symmetric_difference_with_policy(&right, out_pol)
        .expect("strict eligible case must succeed under symmetric_difference_with_policy");

    assert_output_policy_repr(&forced_union, case.out_policy, &left, &right);
    assert_output_policy_repr(&forced_intersection, case.out_policy, &left, &right);
    assert_output_policy_repr(&forced_difference, case.out_policy, &left, &right);
    assert_output_policy_repr(&forced_sym_diff, case.out_policy, &left, &right);

    for (actual, expected) in [
        (&forced_union, expected_union),
        (&forced_intersection, expected_intersection),
        (&forced_difference, expected_difference),
        (&forced_sym_diff, expected_sym_diff),
    ] {
        let sorted_reference = FrozenIntSet::try_from_iter(expected, IntSetPolicy::sorted())
            .expect("sorted reference succeeds");
        assert_eq!(actual, &sorted_reference);
        assert_eq!(hash_events(actual), hash_events(&sorted_reference));
    }
}

fn assert_construction_repr(set: &FrozenIntSet<i32>, selector: PolicySelector) {
    if let Some(expected_repr) = selector.expected_forced_repr() {
        assert_eq!(set.representation(), expected_repr);
    } else if set.is_empty() {
        assert_eq!(set.representation(), IntSetRepr::Sorted);
    } else {
        assert_ne!(
            set.representation(),
            IntSetRepr::Chunked,
            "Auto construction never selects Chunked storage"
        );
    }
}

/// Checks the representation of an algebra result against its output policy.
///
/// A forced policy gives its own representation. Under `Auto`, an empty result
/// is `Sorted` unless it is an empty operand's storage: algebra reuses an
/// operand that equals the result and satisfies the output policy, whatever
/// representation that operand was built with.
fn assert_output_policy_repr(
    set: &FrozenIntSet<i32>,
    out_selector: PolicySelector,
    left: &FrozenIntSet<i32>,
    right: &FrozenIntSet<i32>,
) {
    if let Some(expected_repr) = out_selector.expected_forced_repr() {
        assert_eq!(set.representation(), expected_repr);
    } else if set.is_empty() {
        let reuses_empty_operand = [left, right]
            .into_iter()
            .any(|operand| operand.is_empty() && set.shares_storage_with(operand));
        if !reuses_empty_operand {
            assert_eq!(set.representation(), IntSetRepr::Sorted);
        }
    } else if left.representation() != IntSetRepr::Chunked
        && right.representation() != IntSetRepr::Chunked
    {
        assert_ne!(
            set.representation(),
            IntSetRepr::Chunked,
            "Auto output policy never promotes non-Chunked operands to Chunked"
        );
    }
}

fn verify_integer_algebra(
    left: &FrozenIntSet<i32>,
    right: &FrozenIntSet<i32>,
    left_oracle: &BTreeSet<i32>,
    right_oracle: &BTreeSet<i32>,
) {
    assert_eq!(
        left.iter().collect::<Vec<_>>(),
        left_oracle.iter().copied().collect::<Vec<_>>()
    );
    assert_eq!(
        right.iter().collect::<Vec<_>>(),
        right_oracle.iter().copied().collect::<Vec<_>>()
    );

    let expected_union: Vec<i32> = left_oracle.union(right_oracle).copied().collect();
    let expected_intersection: Vec<i32> = left_oracle.intersection(right_oracle).copied().collect();
    let expected_difference: Vec<i32> = left_oracle.difference(right_oracle).copied().collect();
    let expected_sym_diff: Vec<i32> = left_oracle
        .symmetric_difference(right_oracle)
        .copied()
        .collect();

    let union = left.union(right);
    let intersection = left.intersection(right);
    let difference = left.difference(right);
    let sym_diff = left.symmetric_difference(right);

    assert_eq!(union.iter().collect::<Vec<_>>(), expected_union);
    assert_eq!(
        union.iter().fold(Vec::new(), |mut acc, v| {
            acc.push(v);
            acc
        }),
        expected_union
    );
    assert_eq!(
        intersection.iter().collect::<Vec<_>>(),
        expected_intersection
    );
    assert_eq!(difference.iter().collect::<Vec<_>>(), expected_difference);
    assert_eq!(sym_diff.iter().collect::<Vec<_>>(), expected_sym_diff);

    assert_eq!(left.union_len(right) as usize, expected_union.len());
    assert_eq!(
        left.intersection_len(right) as usize,
        expected_intersection.len()
    );
    assert_eq!(
        left.difference_len(right) as usize,
        expected_difference.len()
    );
    assert_eq!(
        left.symmetric_difference_len(right) as usize,
        expected_sym_diff.len()
    );

    assert_eq!(left.is_subset(right), left_oracle.is_subset(right_oracle));
    assert_eq!(
        left.is_superset(right),
        left_oracle.is_superset(right_oracle)
    );
    assert_eq!(
        left.is_disjoint(right),
        left_oracle.is_disjoint(right_oracle)
    );
}

#[derive(Default)]
struct EventRecorder(Vec<(u8, u64)>);

impl Hasher for EventRecorder {
    fn finish(&self) -> u64 {
        0
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.push((0, bytes.len() as u64));
        for &byte in bytes {
            self.0.push((1, u64::from(byte)));
        }
    }

    fn write_u64(&mut self, i: u64) {
        self.0.push((8, i));
    }

    fn write_usize(&mut self, i: usize) {
        self.0.push((9, i as u64));
    }
}

fn hash_events<T: Hash>(value: &T) -> Vec<(u8, u64)> {
    let mut recorder = EventRecorder::default();
    value.hash(&mut recorder);
    recorder.0
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use frost::TryFreezeWith;
    use frost::integer::{FrozenIntSet, IntSetError, IntSetPolicy};

    use super::{
        IntegerAlgebraCase, MAX_ITEMS, OrderedAlgebraCase, PolicySelector, RepKey,
        check_integer_set_algebra, check_ordered_set_algebra, decode_integer_case,
        decode_ordered_case, run_integer_case_strict, run_ordered_case,
    };

    #[test]
    fn ordered_set_algebra_seed_corpus_and_edge_cases() {
        let seed_bytes =
            include_bytes!("../corpus/ordered_set_algebra/seed-duplicate-representatives");
        let seed_case = decode_ordered_case(seed_bytes).expect("seed decodes");
        assert!(
            !seed_case.left.is_empty() && !seed_case.right.is_empty(),
            "ordered seed corpus must decode to two non-empty operand vectors"
        );
        let left_unique: BTreeSet<i16> = seed_case.left.iter().map(|k| k.id).collect();
        let right_unique: BTreeSet<i16> = seed_case.right.iter().map(|k| k.id).collect();
        assert!(
            left_unique.len() < seed_case.left.len(),
            "ordered seed corpus must contain duplicate keys in left operand"
        );
        assert!(
            !left_unique.is_disjoint(&right_unique),
            "ordered seed corpus must contain overlapping keys across operands"
        );
        run_ordered_case(&seed_case);

        let direct_cases = [
            OrderedAlgebraCase {
                left: Vec::new(),
                right: Vec::new(),
                prefix_consumption: 0,
            },
            OrderedAlgebraCase {
                left: vec![
                    RepKey {
                        id: 10,
                        representative: 1,
                    },
                    RepKey {
                        id: 10,
                        representative: 2,
                    },
                    RepKey {
                        id: -5,
                        representative: 7,
                    },
                ],
                right: Vec::new(),
                prefix_consumption: 1,
            },
            OrderedAlgebraCase {
                left: Vec::new(),
                right: vec![
                    RepKey {
                        id: 3,
                        representative: 9,
                    },
                    RepKey {
                        id: 3,
                        representative: 10,
                    },
                ],
                prefix_consumption: 1,
            },
            OrderedAlgebraCase {
                left: vec![
                    RepKey {
                        id: 1,
                        representative: 10,
                    },
                    RepKey {
                        id: 2,
                        representative: 20,
                    },
                ],
                right: vec![
                    RepKey {
                        id: 3,
                        representative: 30,
                    },
                    RepKey {
                        id: 4,
                        representative: 40,
                    },
                ],
                prefix_consumption: 2,
            },
            OrderedAlgebraCase {
                left: vec![
                    RepKey {
                        id: 5,
                        representative: 11,
                    },
                    RepKey {
                        id: 5,
                        representative: 22,
                    },
                    RepKey {
                        id: 1,
                        representative: 33,
                    },
                    RepKey {
                        id: 9,
                        representative: 44,
                    },
                    RepKey {
                        id: 9,
                        representative: 55,
                    },
                ],
                right: vec![
                    RepKey {
                        id: 5,
                        representative: 99,
                    },
                    RepKey {
                        id: 5,
                        representative: 88,
                    },
                    RepKey {
                        id: 2,
                        representative: 77,
                    },
                    RepKey {
                        id: 9,
                        representative: 66,
                    },
                ],
                prefix_consumption: 3,
            },
        ];
        for case in &direct_cases {
            run_ordered_case(case);
        }

        let mut both_nonempty = 0_usize;
        let mut overlapping = 0_usize;
        for byte in 1_u8..=64 {
            let synthetic = [
                3,
                3,
                byte % 5,
                byte,
                0,
                1,
                byte,
                0,
                2,
                byte.wrapping_add(1),
                0,
                3,
                byte,
                0,
                9,
                byte.wrapping_add(2),
                0,
                8,
                byte.wrapping_sub(1),
                0,
                7,
            ];
            let decoded = decode_ordered_case(&synthetic).expect("synthetic decodes");
            assert!(decoded.left.len() <= MAX_ITEMS && decoded.right.len() <= MAX_ITEMS);
            if !decoded.left.is_empty() && !decoded.right.is_empty() {
                both_nonempty += 1;
            }
            let l_ids: BTreeSet<i16> = decoded.left.iter().map(|k| k.id).collect();
            let r_ids: BTreeSet<i16> = decoded.right.iter().map(|k| k.id).collect();
            if !l_ids.is_disjoint(&r_ids) {
                overlapping += 1;
            }
            check_ordered_set_algebra(&synthetic);
        }
        assert_eq!(both_nonempty, 64);
        assert_eq!(overlapping, 64);
    }

    #[test]
    fn integer_set_algebra_seed_corpus_and_policy_combinations() {
        let seed_bytes = include_bytes!("../corpus/integer_set_algebra/seed-representation-pairs");
        let seed_case = decode_integer_case(seed_bytes).expect("integer seed decodes");
        assert!(
            !seed_case.left.is_empty() && !seed_case.right.is_empty(),
            "integer seed corpus must decode to two non-empty operand vectors"
        );
        let left_unique: BTreeSet<i16> = seed_case.left.iter().copied().collect();
        let right_unique: BTreeSet<i16> = seed_case.right.iter().copied().collect();
        assert!(
            !left_unique.is_disjoint(&right_unique),
            "integer seed corpus must contain overlapping values across operands"
        );
        // Seed corpus uses ASCII '0','2' for dense_budget (0x3230 = 12848 bytes),
        // so all policies are strictly eligible.
        run_integer_case_strict(&seed_case);

        // Strictly exercise all 5 x 5 x 5 = 125 (left_policy, right_policy, out_policy)
        // combinations across overlapping dense/clustered/negative-positive inputs,
        // asserting that every construction and *_with_policy call succeeds without
        // fallback and matches its requested representation.
        let sample_left: Vec<i16> = vec![-128, -127, -1, 0, 1, 2, 3, 63, 64, 65, 512, 513];
        let sample_right: Vec<i16> = vec![-128, 0, 2, 4, 64, 66, 512, 1024];
        let mut productive_combinations = 0_usize;
        for &left_policy in &PolicySelector::ALL {
            for &right_policy in &PolicySelector::ALL {
                for &out_policy in &PolicySelector::ALL {
                    run_integer_case_strict(&IntegerAlgebraCase {
                        left: sample_left.clone(),
                        right: sample_right.clone(),
                        left_policy,
                        right_policy,
                        out_policy,
                        dense_budget: 512,
                        roaring_partitions: 8,
                    });
                    productive_combinations += 1;
                }
            }
        }
        assert_eq!(productive_combinations, 125);

        // Empty and disjoint edge cases under strict verification.
        run_integer_case_strict(&IntegerAlgebraCase {
            left: Vec::new(),
            right: sample_right.clone(),
            left_policy: PolicySelector::Dense,
            right_policy: PolicySelector::Roaring,
            out_policy: PolicySelector::Chunked,
            dense_budget: 256,
            roaring_partitions: 4,
        });
        run_integer_case_strict(&IntegerAlgebraCase {
            left: vec![1, 2, 3],
            right: vec![100, 101, 102],
            left_policy: PolicySelector::Sorted,
            right_policy: PolicySelector::Dense,
            out_policy: PolicySelector::Auto,
            dense_budget: 256,
            roaring_partitions: 4,
        });

        let mut both_nonempty = 0_usize;
        let mut overlapping = 0_usize;
        for mode in 0_u8..25 {
            let synthetic = [
                3,
                3,
                mode % 5,
                (mode / 5) % 5,
                (mode + 2) % 5,
                0,
                2,
                4,
                mode,
                0,
                mode.wrapping_add(1),
                0,
                mode.wrapping_add(2),
                0,
                mode.wrapping_add(1),
                0,
                mode.wrapping_add(3),
                0,
                mode.wrapping_add(4),
                0,
            ];
            let decoded = decode_integer_case(&synthetic).expect("synthetic decodes");
            if !decoded.left.is_empty() && !decoded.right.is_empty() {
                both_nonempty += 1;
            }
            let l_ids: BTreeSet<i16> = decoded.left.iter().copied().collect();
            let r_ids: BTreeSet<i16> = decoded.right.iter().copied().collect();
            if !l_ids.is_disjoint(&r_ids) {
                overlapping += 1;
            }
            run_integer_case_strict(&decoded);
            check_integer_set_algebra(&synthetic);
        }
        assert_eq!(both_nonempty, 25);
        assert_eq!(overlapping, 25);
    }

    /// Regression for the libFuzzer input `4444`: two empty operands forced to
    /// `Dense` and an `Auto` output. The result is an operand's storage, so it
    /// is `Dense`, not `Sorted`.
    #[test]
    fn integer_set_empty_operands_keep_their_representation_under_auto_output() {
        let seed_bytes = include_bytes!("../corpus/integer_set_algebra/seed-empty-dense-operands");
        let seed_case = decode_integer_case(seed_bytes).expect("integer seed decodes");
        assert!(seed_case.left.is_empty() && seed_case.right.is_empty());
        assert_eq!(seed_case.left_policy, PolicySelector::Dense);
        assert_eq!(seed_case.right_policy, PolicySelector::Dense);
        assert_eq!(seed_case.out_policy, PolicySelector::Auto);
        check_integer_set_algebra(seed_bytes);

        let empty = IntSetPolicy::dense()
            .with_dense_budget(256)
            .with_roaring_partition_limit(4);
        let left = Vec::<i32>::new()
            .try_freeze_with(empty)
            .expect("empty is eligible");
        let right = Vec::<i32>::new()
            .try_freeze_with(empty)
            .expect("empty is eligible");
        let union = left
            .union_with_policy(&right, IntSetPolicy::auto())
            .expect("an empty union is eligible");
        assert!(union.shares_storage_with(&right));

        // Every policy combination, with one or both operands empty.
        let sample: Vec<i16> = vec![-128, -1, 0, 1, 63, 64, 512];
        let operands = [
            (Vec::new(), Vec::new()),
            (Vec::new(), sample.clone()),
            (sample.clone(), Vec::new()),
        ];
        let mut combinations = 0_usize;
        for (left, right) in &operands {
            for &left_policy in &PolicySelector::ALL {
                for &right_policy in &PolicySelector::ALL {
                    for &out_policy in &PolicySelector::ALL {
                        run_integer_case_strict(&IntegerAlgebraCase {
                            left: left.clone(),
                            right: right.clone(),
                            left_policy,
                            right_policy,
                            out_policy,
                            dense_budget: 512,
                            roaring_partitions: 8,
                        });
                        combinations += 1;
                    }
                }
            }
        }
        assert_eq!(combinations, 375);
    }

    #[test]
    fn integer_set_ineligible_policy_limits_are_rejected_and_preserve_input() {
        let wide_input = vec![0_i32, 10_000];
        let tight_dense = IntSetPolicy::dense().with_dense_budget(64);
        let rejected = wide_input
            .clone()
            .try_freeze_with(tight_dense)
            .expect_err("10,000-code span exceeds 64-byte dense budget");
        assert_eq!(rejected.input, wide_input);
        assert!(matches!(
            rejected.error,
            IntSetError::DenseBudgetExceeded { .. }
        ));

        let left = FrozenIntSet::try_from_iter([0_i32, 1], IntSetPolicy::dense()).unwrap();
        let right =
            FrozenIntSet::try_from_iter([10_000_i32, 10_001], IntSetPolicy::dense()).unwrap();
        let err = left
            .union_with_policy(&right, tight_dense)
            .expect_err("union span exceeds 64-byte dense output budget");
        assert!(matches!(err, IntSetError::DenseBudgetExceeded { .. }));
    }
}

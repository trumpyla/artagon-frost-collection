//! Private word mapping, bounded representation planning, and encoded kernels.
//!
//! Dense bases are absolute multiples of 64. Operations align those coordinates
//! before combining words; equal local offsets alone never imply equal members.

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt;
use core::hash::Hash;
use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::mem::size_of;

use super::{IntSetBackend, IntSetError, IntSetPolicy, IntSetRepr};
use crate::family::ordered::set_ops;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operation {
    Union,
    Intersection,
    Difference,
    SymmetricDifference,
}

/// The only implementations are unsigned primitive widths. This trait and its
/// associated native types are deliberately private to the crate.
pub(crate) trait Word: Copy + Ord + Hash + fmt::Debug + 'static {
    type Roaring: Clone + Send + Sync;
    type RoaringIter<'a>: Iterator<Item = Self> + Clone
    where
        Self: 'a;

    fn to_u64(self) -> u64;
    fn try_from_u64(value: u64) -> Option<Self>;
    fn roaring_from_sorted(words: &[Self]) -> Self::Roaring;
    fn roaring_iter(value: &Self::Roaring) -> Self::RoaringIter<'_>;
    fn roaring_len(value: &Self::Roaring) -> u64;
    fn roaring_bounds(value: &Self::Roaring) -> Option<(Self, Self)>;
    fn roaring_contains(value: &Self::Roaring, member: Self) -> bool;
    fn roaring_intersection_len(left: &Self::Roaring, right: &Self::Roaring) -> u64;
    fn roaring_is_subset(left: &Self::Roaring, right: &Self::Roaring) -> bool;
    fn roaring_is_disjoint(left: &Self::Roaring, right: &Self::Roaring) -> bool;
    fn roaring_binary(left: &Self::Roaring, right: &Self::Roaring, op: Operation) -> Self::Roaring;
}

macro_rules! word {
    ($word:ty, $backend:ty, $iterator:ty) => {
        impl Word for $word {
            type Roaring = $backend;
            type RoaringIter<'a> = $iterator;

            fn to_u64(self) -> u64 {
                u64::from(self)
            }
            fn try_from_u64(value: u64) -> Option<Self> {
                Self::try_from(value).ok()
            }
            fn roaring_from_sorted(words: &[Self]) -> Self::Roaring {
                let mut result: Self::Roaring = words.iter().copied().collect();
                result.optimize();
                result
            }
            fn roaring_iter(value: &Self::Roaring) -> Self::RoaringIter<'_> {
                value.iter()
            }
            fn roaring_len(value: &Self::Roaring) -> u64 {
                value.len()
            }
            fn roaring_bounds(value: &Self::Roaring) -> Option<(Self, Self)> {
                Some((value.min()?, value.max()?))
            }
            fn roaring_contains(value: &Self::Roaring, member: Self) -> bool {
                value.contains(member)
            }
            fn roaring_intersection_len(left: &Self::Roaring, right: &Self::Roaring) -> u64 {
                left.intersection_len(right)
            }
            fn roaring_is_subset(left: &Self::Roaring, right: &Self::Roaring) -> bool {
                left.is_subset(right)
            }
            fn roaring_is_disjoint(left: &Self::Roaring, right: &Self::Roaring) -> bool {
                left.is_disjoint(right)
            }
            fn roaring_binary(
                left: &Self::Roaring,
                right: &Self::Roaring,
                op: Operation,
            ) -> Self::Roaring {
                match op {
                    Operation::Union => left | right,
                    Operation::Intersection => left & right,
                    Operation::Difference => left - right,
                    Operation::SymmetricDifference => left ^ right,
                }
            }
        }
    };
}
word!(u32, roaring::RoaringBitmap, roaring::bitmap::Iter<'a>);
word!(u64, roaring::RoaringTreemap, roaring::treemap::Iter<'a>);

#[derive(Clone)]
enum Repr<W: Word> {
    Sorted(Box<[W]>),
    Dense(Dense<W>),
    Roaring(W::Roaring),
}

/// Immutable cardinality and bounds are cached even for the partitioned backend.
#[derive(Clone)]
pub(crate) struct Storage<W: Word> {
    repr: Repr<W>,
    cardinality: u64,
    bounds: Option<(W, W)>,
}

impl<W: Word> Storage<W> {
    pub(crate) fn empty() -> Self {
        Self {
            repr: Repr::Sorted(Box::new([])),
            cardinality: 0,
            bounds: None,
        }
    }

    /// Caller supplies strictly ascending, unique, validated codes.
    pub(crate) fn from_sorted(words: Vec<W>, policy: IntSetPolicy) -> Result<Self, IntSetError> {
        debug_assert!(words.windows(2).all(|pair| pair[0] < pair[1]));
        let cardinality =
            u64::try_from(words.len()).map_err(|_| IntSetError::CardinalityOverflow)?;
        let bounds = words
            .first()
            .zip(words.last())
            .map(|(&low, &high)| (low, high));
        let (partitions, roaring_score) = match policy.backend {
            IntSetBackend::Auto => roaring_score(&words),
            IntSetBackend::Roaring => (partition_count(&words), 0),
            IntSetBackend::Sorted | IntSetBackend::Dense => (0, 0),
        };
        let dense_bytes = bounds.map_or(0, |(low, high)| dense_bytes(low.to_u64(), high.to_u64()));
        let dense_limit = policy.max_dense_bytes.min(isize::MAX as usize);
        let backend = match policy.backend {
            IntSetBackend::Auto => {
                if words.is_empty() {
                    IntSetBackend::Sorted
                } else {
                    let sorted_score = words.len() as u128 * size_of::<W>() as u128 + 16;
                    let mut backend = IntSetBackend::Sorted;
                    let mut score = sorted_score;
                    if dense_bytes <= dense_limit as u128 && dense_bytes + 24 < score {
                        backend = IntSetBackend::Dense;
                        score = dense_bytes + 24;
                    }
                    if partitions <= policy.max_roaring_partitions as u64 && roaring_score < score {
                        backend = IntSetBackend::Roaring;
                    }
                    backend
                }
            }
            explicit => explicit,
        };
        let repr = match backend {
            IntSetBackend::Sorted => Repr::Sorted(words.into_boxed_slice()),
            IntSetBackend::Dense => {
                if dense_bytes > dense_limit as u128 {
                    return Err(IntSetError::DenseBudgetExceeded {
                        required_bytes: dense_bytes,
                        max_bytes: dense_limit,
                    });
                }
                Repr::Dense(Dense::from_sorted(&words))
            }
            IntSetBackend::Roaring => {
                if partitions > policy.max_roaring_partitions as u64 {
                    return Err(IntSetError::RoaringPartitionLimit {
                        partitions,
                        max_partitions: policy.max_roaring_partitions,
                    });
                }
                Repr::Roaring(W::roaring_from_sorted(&words))
            }
            IntSetBackend::Auto => unreachable!("automatic planning chooses a concrete backend"),
        };
        Ok(Self {
            repr,
            cardinality,
            bounds,
        })
    }

    fn sorted(words: Vec<W>) -> Self {
        let cardinality = u64::try_from(words.len()).expect("addressable sorted length fits u64");
        let bounds = words
            .first()
            .zip(words.last())
            .map(|(&low, &high)| (low, high));
        Self {
            repr: Repr::Sorted(words.into_boxed_slice()),
            cardinality,
            bounds,
        }
    }

    fn roaring(value: W::Roaring) -> Self {
        let cardinality = W::roaring_len(&value);
        let bounds = W::roaring_bounds(&value);
        Self {
            repr: Repr::Roaring(value),
            cardinality,
            bounds,
        }
    }

    fn dense(value: Dense<W>) -> Self {
        let cardinality = value
            .words
            .iter()
            .map(|word| u64::from(word.count_ones()))
            .sum();
        let bounds = value.bounds();
        Self {
            repr: Repr::Dense(value),
            cardinality,
            bounds,
        }
    }

    pub(crate) const fn cardinality(&self) -> u64 {
        self.cardinality
    }
    pub(crate) const fn bounds(&self) -> Option<(W, W)> {
        self.bounds
    }

    pub(crate) fn representation(&self) -> IntSetRepr {
        match self.repr {
            Repr::Sorted(_) => IntSetRepr::Sorted,
            Repr::Dense(_) => IntSetRepr::Dense,
            Repr::Roaring(_) => IntSetRepr::Roaring,
        }
    }

    pub(crate) fn contains(&self, member: W) -> bool {
        match &self.repr {
            Repr::Sorted(words) => words.binary_search(&member).is_ok(),
            Repr::Dense(words) => words.contains(member),
            Repr::Roaring(words) => W::roaring_contains(words, member),
        }
    }

    pub(crate) fn iter(&self) -> Words<'_, W> {
        Words {
            inner: match &self.repr {
                Repr::Sorted(words) => WordIter::Sorted(words.iter().copied()),
                Repr::Dense(words) => WordIter::Dense(words.iter()),
                Repr::Roaring(words) => WordIter::Roaring(W::roaring_iter(words)),
            },
            remaining: self.cardinality,
        }
    }

    pub(crate) fn intersection_len(&self, other: &Self) -> u64 {
        if core::ptr::eq(self, other) {
            return self.cardinality;
        }
        if self.separated(other) {
            return 0;
        }
        match (&self.repr, &other.repr) {
            (Repr::Sorted(left), Repr::Sorted(right)) => {
                u64::try_from(set_ops::intersection_len(left, right)).expect("slice count fits u64")
            }
            (Repr::Dense(left), Repr::Dense(right)) => left.intersection_len(right),
            (Repr::Roaring(left), Repr::Roaring(right)) => W::roaring_intersection_len(left, right),
            (Repr::Sorted(left), Repr::Dense(right)) => {
                if self.cardinality <= other.cardinality {
                    probe_count(left.iter().copied(), |word| right.contains(word))
                } else {
                    probe_count(right.iter(), |word| left.binary_search(&word).is_ok())
                }
            }
            (Repr::Dense(left), Repr::Sorted(right)) => {
                if self.cardinality <= other.cardinality {
                    probe_count(left.iter(), |word| right.binary_search(&word).is_ok())
                } else {
                    probe_count(right.iter().copied(), |word| left.contains(word))
                }
            }
            (Repr::Sorted(left), Repr::Roaring(right)) => {
                if self.cardinality <= other.cardinality {
                    probe_count(left.iter().copied(), |word| {
                        W::roaring_contains(right, word)
                    })
                } else {
                    probe_count(W::roaring_iter(right), |word| {
                        left.binary_search(&word).is_ok()
                    })
                }
            }
            (Repr::Roaring(left), Repr::Sorted(right)) => {
                if self.cardinality <= other.cardinality {
                    probe_count(W::roaring_iter(left), |word| {
                        right.binary_search(&word).is_ok()
                    })
                } else {
                    probe_count(right.iter().copied(), |word| {
                        W::roaring_contains(left, word)
                    })
                }
            }
            (Repr::Dense(left), Repr::Roaring(right)) => {
                if self.cardinality <= other.cardinality {
                    probe_count(left.iter(), |word| W::roaring_contains(right, word))
                } else {
                    probe_count(W::roaring_iter(right), |word| left.contains(word))
                }
            }
            (Repr::Roaring(left), Repr::Dense(right)) => {
                if self.cardinality <= other.cardinality {
                    probe_count(W::roaring_iter(left), |word| right.contains(word))
                } else {
                    probe_count(right.iter(), |word| W::roaring_contains(left, word))
                }
            }
        }
    }

    fn separated(&self, other: &Self) -> bool {
        match (self.bounds, other.bounds) {
            (Some((left_min, left_max)), Some((right_min, right_max))) => {
                left_max < right_min || right_max < left_min
            }
            _ => true,
        }
    }

    pub(crate) fn is_disjoint(&self, other: &Self) -> bool {
        if self.separated(other) {
            return true;
        }
        match (&self.repr, &other.repr) {
            (Repr::Sorted(left), Repr::Sorted(right)) => set_ops::is_disjoint(left, right),
            (Repr::Dense(left), Repr::Dense(right)) => left.is_disjoint(right),
            (Repr::Roaring(left), Repr::Roaring(right)) => W::roaring_is_disjoint(left, right),
            (Repr::Sorted(left), Repr::Dense(right)) => {
                if self.cardinality <= other.cardinality {
                    left.iter().all(|&word| !right.contains(word))
                } else {
                    right.iter().all(|word| left.binary_search(&word).is_err())
                }
            }
            (Repr::Dense(left), Repr::Sorted(right)) => {
                if self.cardinality <= other.cardinality {
                    left.iter().all(|word| right.binary_search(&word).is_err())
                } else {
                    right.iter().all(|&word| !left.contains(word))
                }
            }
            (Repr::Sorted(left), Repr::Roaring(right)) => {
                if self.cardinality <= other.cardinality {
                    left.iter().all(|&word| !W::roaring_contains(right, word))
                } else {
                    W::roaring_iter(right).all(|word| left.binary_search(&word).is_err())
                }
            }
            (Repr::Roaring(left), Repr::Sorted(right)) => {
                if self.cardinality <= other.cardinality {
                    W::roaring_iter(left).all(|word| right.binary_search(&word).is_err())
                } else {
                    right.iter().all(|&word| !W::roaring_contains(left, word))
                }
            }
            (Repr::Dense(left), Repr::Roaring(right)) => {
                if self.cardinality <= other.cardinality {
                    left.iter().all(|word| !W::roaring_contains(right, word))
                } else {
                    W::roaring_iter(right).all(|word| !left.contains(word))
                }
            }
            (Repr::Roaring(left), Repr::Dense(right)) => {
                if self.cardinality <= other.cardinality {
                    W::roaring_iter(left).all(|word| !right.contains(word))
                } else {
                    right.iter().all(|word| !W::roaring_contains(left, word))
                }
            }
        }
    }

    pub(crate) fn is_subset(&self, other: &Self) -> bool {
        if self.cardinality == 0 || core::ptr::eq(self, other) {
            return true;
        }
        if self.cardinality > other.cardinality {
            return false;
        }
        let Some((left_min, left_max)) = self.bounds else {
            return true;
        };
        let Some((right_min, right_max)) = other.bounds else {
            return false;
        };
        if left_min < right_min || left_max > right_max {
            return false;
        }
        match (&self.repr, &other.repr) {
            (Repr::Sorted(left), Repr::Sorted(right)) => set_ops::is_subset(left, right),
            (Repr::Dense(left), Repr::Dense(right)) => left.is_subset(right),
            (Repr::Roaring(left), Repr::Roaring(right)) => W::roaring_is_subset(left, right),
            (Repr::Sorted(left), Repr::Dense(right)) => {
                left.iter().all(|&word| right.contains(word))
            }
            (Repr::Dense(left), Repr::Sorted(right)) => {
                left.iter().all(|word| right.binary_search(&word).is_ok())
            }
            (Repr::Sorted(left), Repr::Roaring(right)) => {
                left.iter().all(|&word| W::roaring_contains(right, word))
            }
            (Repr::Roaring(left), Repr::Sorted(right)) => {
                W::roaring_iter(left).all(|word| right.binary_search(&word).is_ok())
            }
            (Repr::Dense(left), Repr::Roaring(right)) => {
                left.iter().all(|word| W::roaring_contains(right, word))
            }
            (Repr::Roaring(left), Repr::Dense(right)) => {
                W::roaring_iter(left).all(|word| right.contains(word))
            }
        }
    }

    pub(crate) fn binary(&self, other: &Self, op: Operation) -> Self {
        if core::ptr::eq(self, other) {
            return match op {
                Operation::Union | Operation::Intersection => self.clone(),
                Operation::Difference | Operation::SymmetricDifference => Self::sorted(Vec::new()),
            };
        }
        match (&self.repr, &other.repr) {
            (Repr::Roaring(left), Repr::Roaring(right)) => {
                Self::roaring(W::roaring_binary(left, right, op))
            }
            (Repr::Dense(left), Repr::Dense(right)) => {
                if let Some(value) = left.binary(right, op) {
                    return Self::dense(value);
                }
                Self::sorted(merge(left.iter(), right.iter(), op))
            }
            (Repr::Sorted(left), Repr::Sorted(right)) => {
                let result = match op {
                    Operation::Union => set_ops::SetUnion::new(left, right).copied().collect(),
                    Operation::Intersection => set_ops::SetIntersection::new(left, right)
                        .copied()
                        .collect(),
                    Operation::Difference => {
                        set_ops::SetDifference::new(left, right).copied().collect()
                    }
                    Operation::SymmetricDifference => {
                        set_ops::SetSymmetricDifference::new(left, right)
                            .copied()
                            .collect()
                    }
                };
                Self::sorted(result)
            }
            (Repr::Sorted(left), Repr::Dense(right)) => Self::sorted(mixed(
                left.iter().copied(),
                right.iter(),
                |word| left.binary_search(&word).is_ok(),
                |word| right.contains(word),
                self.cardinality,
                other.cardinality,
                op,
            )),
            (Repr::Dense(left), Repr::Sorted(right)) => Self::sorted(mixed(
                left.iter(),
                right.iter().copied(),
                |word| left.contains(word),
                |word| right.binary_search(&word).is_ok(),
                self.cardinality,
                other.cardinality,
                op,
            )),
            (Repr::Sorted(left), Repr::Roaring(right)) => Self::sorted(mixed(
                left.iter().copied(),
                W::roaring_iter(right),
                |word| left.binary_search(&word).is_ok(),
                |word| W::roaring_contains(right, word),
                self.cardinality,
                other.cardinality,
                op,
            )),
            (Repr::Roaring(left), Repr::Sorted(right)) => Self::sorted(mixed(
                W::roaring_iter(left),
                right.iter().copied(),
                |word| W::roaring_contains(left, word),
                |word| right.binary_search(&word).is_ok(),
                self.cardinality,
                other.cardinality,
                op,
            )),
            (Repr::Dense(left), Repr::Roaring(right)) => Self::sorted(mixed(
                left.iter(),
                W::roaring_iter(right),
                |word| left.contains(word),
                |word| W::roaring_contains(right, word),
                self.cardinality,
                other.cardinality,
                op,
            )),
            (Repr::Roaring(left), Repr::Dense(right)) => Self::sorted(mixed(
                W::roaring_iter(left),
                right.iter(),
                |word| W::roaring_contains(left, word),
                |word| right.contains(word),
                self.cardinality,
                other.cardinality,
                op,
            )),
        }
    }
}

fn probe_count<W>(words: impl Iterator<Item = W>, mut contains: impl FnMut(W) -> bool) -> u64 {
    words.fold(0, |count, word| count + u64::from(contains(word)))
}

fn mixed<W: Word>(
    left: impl Iterator<Item = W>,
    right: impl Iterator<Item = W>,
    mut left_contains: impl FnMut(W) -> bool,
    mut right_contains: impl FnMut(W) -> bool,
    left_len: u64,
    right_len: u64,
    op: Operation,
) -> Vec<W> {
    match op {
        Operation::Intersection if left_len <= right_len => {
            left.filter(|&word| right_contains(word)).collect()
        }
        Operation::Intersection => right.filter(|&word| left_contains(word)).collect(),
        Operation::Difference => left.filter(|&word| !right_contains(word)).collect(),
        Operation::Union | Operation::SymmetricDifference => merge(left, right, op),
    }
}

fn merge<W: Word>(
    left: impl Iterator<Item = W>,
    right: impl Iterator<Item = W>,
    op: Operation,
) -> Vec<W> {
    let mut left = left.peekable();
    let mut right = right.peekable();
    let mut out = Vec::new();
    while let (Some(&a), Some(&b)) = (left.peek(), right.peek()) {
        match a.cmp(&b) {
            Ordering::Less => {
                if !matches!(op, Operation::Intersection) {
                    out.push(a);
                }
                left.next();
            }
            Ordering::Greater => {
                if matches!(op, Operation::Union | Operation::SymmetricDifference) {
                    out.push(b);
                }
                right.next();
            }
            Ordering::Equal => {
                if matches!(op, Operation::Union | Operation::Intersection) {
                    out.push(a);
                }
                left.next();
                right.next();
            }
        }
    }
    if !matches!(op, Operation::Intersection) {
        out.extend(left);
    }
    if matches!(op, Operation::Union | Operation::SymmetricDifference) {
        out.extend(right);
    }
    out
}

fn partition_count<W: Word>(words: &[W]) -> u64 {
    let mut count = 0;
    let mut previous = None;
    for &word in words {
        let prefix = word.to_u64() >> 32;
        if previous != Some(prefix) {
            count += 1;
        }
        previous = Some(prefix);
    }
    count
}

/// Deterministic planning proxy, not an allocator or resident-memory measurement.
/// It includes conservative container and treemap-entry overhead. Run structure
/// matters: the candidate is optimized into run containers after construction.
fn roaring_score<W: Word>(words: &[W]) -> (u64, u128) {
    if words.is_empty() {
        return (0, 32);
    }
    let mut partitions = 0_u64;
    let mut previous: Option<u64> = None;
    let mut container_members = 0_u128;
    let mut container_runs = 0_u128;
    let mut score = 32_u128;
    for &word in words {
        let value = word.to_u64();
        if previous.is_none_or(|previous| previous >> 32 != value >> 32) {
            partitions += 1;
        }
        if previous.is_some_and(|previous| previous >> 16 != value >> 16) {
            score += 32
                + (2 * container_members)
                    .min(8192)
                    .min(4 * container_runs + 2);
            container_members = 0;
            container_runs = 0;
        }
        if previous.and_then(|previous| previous.checked_add(1)) != Some(value)
            || container_members == 0
        {
            container_runs += 1;
        }
        container_members += 1;
        previous = Some(value);
    }
    score += 32
        + (2 * container_members)
            .min(8192)
            .min(4 * container_runs + 2);
    if size_of::<W>() == 8 {
        score += u128::from(partitions) * 80;
    }
    (partitions, score)
}

fn dense_bytes(low: u64, high: u64) -> u128 {
    let base = low & !63;
    ((u128::from(high) - u128::from(base)) / 64 + 1) * 8
}

#[derive(Clone)]
struct Dense<W: Word> {
    base: u64,
    words: Box<[u64]>,
    word: PhantomData<W>,
}

impl<W: Word> Dense<W> {
    fn empty() -> Self {
        Self {
            base: 0,
            words: Box::new([]),
            word: PhantomData,
        }
    }

    fn from_sorted(values: &[W]) -> Self {
        let Some(&first) = values.first() else {
            return Self::empty();
        };
        let base = first.to_u64() & !63;
        let last = values.last().expect("nonempty input").to_u64();
        let count = usize::try_from(dense_bytes(base, last) / 8)
            .expect("planner checked addressable dense span");
        let mut words = vec![0_u64; count];
        for &value in values {
            let offset = value.to_u64() - base;
            words[usize::try_from(offset / 64).expect("offset in allocated dense span")] |=
                1_u64 << (offset % 64);
        }
        Self {
            base,
            words: words.into_boxed_slice(),
            word: PhantomData,
        }
    }

    fn start_block(&self) -> u64 {
        self.base / 64
    }
    fn end_block(&self) -> u64 {
        self.start_block() + self.words.len() as u64
    }

    fn contains(&self, member: W) -> bool {
        let Some(offset) = member.to_u64().checked_sub(self.base) else {
            return false;
        };
        let Ok(index) = usize::try_from(offset / 64) else {
            return false;
        };
        self.words
            .get(index)
            .is_some_and(|word| word & (1_u64 << (offset % 64)) != 0)
    }

    fn at_block(&self, block: u64) -> u64 {
        let Some(offset) = block.checked_sub(self.start_block()) else {
            return 0;
        };
        usize::try_from(offset)
            .ok()
            .and_then(|index| self.words.get(index))
            .copied()
            .unwrap_or(0)
    }

    fn iter(&self) -> DenseIter<'_, W> {
        DenseIter {
            dense: self,
            index: 0,
            pending: 0,
        }
    }

    fn bounds(&self) -> Option<(W, W)> {
        let (low_index, low) = self
            .words
            .iter()
            .enumerate()
            .find(|(_, word)| **word != 0)?;
        let (high_index, high) = self
            .words
            .iter()
            .enumerate()
            .rfind(|(_, word)| **word != 0)?;
        let low = self.base + low_index as u64 * 64 + u64::from(low.trailing_zeros());
        let high = self.base + high_index as u64 * 64 + u64::from(63 - high.leading_zeros());
        Some((
            W::try_from_u64(low).expect("stored word width"),
            W::try_from_u64(high).expect("stored word width"),
        ))
    }

    fn overlap<'a>(&'a self, other: &'a Self) -> (&'a [u64], &'a [u64]) {
        let start = self.start_block().max(other.start_block());
        let end = self.end_block().min(other.end_block());
        if start >= end {
            return (&[], &[]);
        }
        let count = usize::try_from(end - start).expect("overlap is addressable");
        let a = usize::try_from(start - self.start_block()).expect("overlap index");
        let b = usize::try_from(start - other.start_block()).expect("overlap index");
        (&self.words[a..a + count], &other.words[b..b + count])
    }

    fn intersection_len(&self, other: &Self) -> u64 {
        let (left, right) = self.overlap(other);
        left.iter()
            .zip(right)
            .map(|(a, b)| u64::from((a & b).count_ones()))
            .sum()
    }
    fn is_disjoint(&self, other: &Self) -> bool {
        let (left, right) = self.overlap(other);
        left.iter().zip(right).all(|(a, b)| a & b == 0)
    }
    fn is_subset(&self, other: &Self) -> bool {
        self.words
            .iter()
            .enumerate()
            .all(|(index, word)| word & !other.at_block(self.start_block() + index as u64) == 0)
    }

    /// None means the union window exceeds the bounded materialization policy.
    fn binary(&self, other: &Self, op: Operation) -> Option<Self> {
        let (start, end) = match op {
            Operation::Union | Operation::SymmetricDifference => {
                if self.words.is_empty() {
                    return Some(other.clone());
                }
                if other.words.is_empty() {
                    return Some(self.clone());
                }
                (
                    self.start_block().min(other.start_block()),
                    self.end_block().max(other.end_block()),
                )
            }
            Operation::Intersection => (
                self.start_block().max(other.start_block()),
                self.end_block().min(other.end_block()),
            ),
            Operation::Difference => (self.start_block(), self.end_block()),
        };
        if start >= end {
            return Some(Self::empty());
        }
        let bytes = u128::from(end - start) * 8;
        let limit = IntSetPolicy::auto()
            .max_dense_bytes
            .max(self.words.len() * 8)
            .max(other.words.len() * 8);
        if bytes > limit as u128 || bytes > isize::MAX as u128 {
            return None;
        }
        let mut words =
            Vec::with_capacity(usize::try_from(end - start).expect("bounded output window"));
        for block in start..end {
            let a = self.at_block(block);
            let b = other.at_block(block);
            words.push(match op {
                Operation::Union => a | b,
                Operation::Intersection => a & b,
                Operation::Difference => a & !b,
                Operation::SymmetricDifference => a ^ b,
            });
        }
        let Some(first) = words.iter().position(|&word| word != 0) else {
            return Some(Self::empty());
        };
        let last = words
            .iter()
            .rposition(|&word| word != 0)
            .expect("nonempty result");
        words.truncate(last + 1);
        if first != 0 {
            words.drain(..first);
        }
        Some(Self {
            base: (start + first as u64) * 64,
            words: words.into_boxed_slice(),
            word: PhantomData,
        })
    }
}

#[derive(Clone)]
struct DenseIter<'a, W: Word> {
    dense: &'a Dense<W>,
    index: usize,
    pending: u64,
}

impl<W: Word> Iterator for DenseIter<'_, W> {
    type Item = W;
    fn next(&mut self) -> Option<W> {
        loop {
            if self.pending != 0 {
                let bit = self.pending.trailing_zeros();
                self.pending &= self.pending - 1;
                let code = self.dense.base + (self.index as u64 - 1) * 64 + u64::from(bit);
                return Some(W::try_from_u64(code).expect("dense members retain their word width"));
            }
            self.pending = *self.dense.words.get(self.index)?;
            self.index += 1;
        }
    }
}
impl<W: Word> FusedIterator for DenseIter<'_, W> {}

#[derive(Clone)]
enum WordIter<'a, W: Word> {
    Sorted(core::iter::Copied<core::slice::Iter<'a, W>>),
    Dense(DenseIter<'a, W>),
    Roaring(W::RoaringIter<'a>),
}

#[derive(Clone)]
pub(crate) struct Words<'a, W: Word> {
    inner: WordIter<'a, W>,
    remaining: u64,
}

impl<W: Word> Iterator for Words<'_, W> {
    type Item = W;
    fn next(&mut self) -> Option<W> {
        if self.remaining == 0 {
            return None;
        }
        let next = match &mut self.inner {
            WordIter::Sorted(iter) => iter.next(),
            WordIter::Dense(iter) => iter.next(),
            WordIter::Roaring(iter) => iter.next(),
        };
        if next.is_some() {
            self.remaining -= 1;
        } else {
            self.remaining = 0;
        }
        next
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match usize::try_from(self.remaining) {
            Ok(count) => (count, Some(count)),
            Err(_) => (usize::MAX, None),
        }
    }
}
impl<W: Word> FusedIterator for Words<'_, W> {}

//! Private word mapping, bounded representation planning, and encoded kernels.
//!
//! Dense bases are absolute multiples of 64. Operations align those coordinates
//! before combining words; equal local offsets alone never imply equal members.

use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::mem::size_of;
use core::ops::ControlFlow;

use super::{IntSetBackend, IntSetError, IntSetPolicy, IntSetRepr};
use crate::family::ordered::set_ops;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operation {
    Union,
    Intersection,
    Difference,
    SymmetricDifference,
}

mod word;
pub(crate) use word::Word;

/// Prefer Dense when Roaring would save at most this many estimated bytes. This
/// fixed allocation-scale allowance is not a measured CPU crossover threshold.
const DENSE_PREFERENCE_BYTES: u128 = 128;

/// Bound per-member native mutations. Larger sorted operands use the native
/// sorted builder and container algebra instead of repeated tree searches.
const SMALL_SORTED_UPDATES: usize = 64;

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
                    let allowance = if backend == IntSetBackend::Dense {
                        DENSE_PREFERENCE_BYTES
                    } else {
                        0
                    };
                    if partitions <= policy.max_roaring_partitions as u64
                        && roaring_score + allowance < score
                    {
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

    pub(crate) fn contiguous_payload_bytes(&self) -> Option<usize> {
        match &self.repr {
            Repr::Sorted(words) => Some(core::mem::size_of_val(words.as_ref())),
            Repr::Dense(dense) => Some(core::mem::size_of_val(dense.words.as_ref())),
            Repr::Roaring(_) => None,
        }
    }

    pub(crate) fn logical_eq(&self, other: &Self) -> bool {
        if core::ptr::eq(self, other) {
            return true;
        }
        if self.cardinality != other.cardinality || self.bounds != other.bounds {
            return false;
        }
        match (&self.repr, &other.repr) {
            (Repr::Sorted(left), Repr::Sorted(right)) => left == right,
            (Repr::Roaring(left), Repr::Roaring(right)) => left == right,
            (Repr::Dense(left), Repr::Dense(right)) => {
                if left.base == right.base && left.words.len() == right.words.len() {
                    left.words == right.words
                } else {
                    left.is_subset(right)
                }
            }
            _ => self.iter().eq(other.iter()),
        }
    }

    /// The format depends only on content: cardinality, then either a run tag
    /// and maximal endpoints or a member tag and u64-normalized codes (including
    /// 32-bit word representations). A short run scout avoids hashing two
    /// endpoints per isolated member. Physical storage never selects the stream;
    /// these bytes are not a persistent serialization format.
    pub(crate) fn hash<H: Hasher>(&self, state: &mut H) {
        const MEMBER_TAG: u8 = 0;
        const RUN_TAG: u8 = 1;
        const SMALL_SET: u64 = 64;
        const RUN_LIMIT: usize = 16;

        self.cardinality.hash(state);
        let Some((low, high)) = self.bounds else {
            return;
        };
        let low = low.to_u64();
        let high = high.to_u64();
        if u128::from(high) - u128::from(low) + 1 == u128::from(self.cardinality) {
            RUN_TAG.hash(state);
            low.hash(state);
            high.hash(state);
            return;
        }
        if self.cardinality <= SMALL_SET {
            MEMBER_TAG.hash(state);
            self.hash_members(state);
            return;
        }

        let mut runs = [(0_u64, 0_u64); RUN_LIMIT];
        let mut count = 0;
        let completed = self.try_visit_raw_runs(|start, end| {
            if count != 0 && runs[count - 1].1.checked_add(1) == Some(start) {
                runs[count - 1].1 = end;
            } else if count == RUN_LIMIT {
                // The 17th run has started. Its end is irrelevant: stop now,
                // even if this run would contain millions of members.
                return ControlFlow::Break(());
            } else {
                runs[count] = (start, end);
                count += 1;
            }
            ControlFlow::Continue(())
        });
        if completed.is_break() {
            MEMBER_TAG.hash(state);
            self.hash_members(state);
        } else {
            RUN_TAG.hash(state);
            for &(start, end) in &runs[..count] {
                start.hash(state);
                end.hash(state);
            }
        }
    }

    fn hash_members<H: Hasher>(&self, state: &mut H) {
        match &self.repr {
            Repr::Sorted(words) => {
                for &word in words {
                    word.to_u64().hash(state);
                }
            }
            Repr::Dense(dense) => {
                for word in dense.iter() {
                    word.to_u64().hash(state);
                }
            }
            Repr::Roaring(roaring) => {
                for word in W::roaring_iter(roaring) {
                    word.to_u64().hash(state);
                }
            }
        }
    }

    /// Raw pieces may stop at a physical word/container boundary. Callers
    /// coalesce before interpreting run counts. Break propagates through every
    /// representation. The scout bounds its run buffer, not prefix traversal
    /// work: a long initial run may still require many word/member visits.
    fn try_visit_raw_runs<B>(
        &self,
        mut visit: impl FnMut(u64, u64) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        match &self.repr {
            Repr::Sorted(words) => {
                for &word in words {
                    visit(word.to_u64(), word.to_u64())?;
                }
                ControlFlow::Continue(())
            }
            Repr::Dense(dense) => dense.try_visit_runs(visit),
            Repr::Roaring(roaring) => W::roaring_try_visit_runs(roaring, visit),
        }
    }

    fn visit_runs(&self, mut visit: impl FnMut(u64, u64)) {
        let mut pending: Option<(u64, u64)> = None;
        {
            let mut coalesce = |start, end| {
                if let Some((low, high)) = pending {
                    if high.checked_add(1) == Some(start) {
                        pending = Some((low, end));
                    } else {
                        visit(low, high);
                        pending = Some((start, end));
                    }
                } else {
                    pending = Some((start, end));
                }
            };
            match &self.repr {
                Repr::Sorted(words) => {
                    for &word in words {
                        coalesce(word.to_u64(), word.to_u64());
                    }
                }
                Repr::Dense(dense) => dense.visit_runs(&mut coalesce),
                Repr::Roaring(roaring) => W::roaring_visit_runs(roaring, &mut coalesce),
            }
        }
        if let Some((start, end)) = pending {
            visit(start, end);
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
                if prefer_sorted_probe(self.cardinality, other.cardinality, right.words.len()) {
                    probe_count(left.iter().copied(), |word| right.contains(word))
                } else {
                    probe_count(right.iter(), |word| left.binary_search(&word).is_ok())
                }
            }
            (Repr::Dense(left), Repr::Sorted(right)) => {
                if prefer_sorted_probe(other.cardinality, self.cardinality, left.words.len()) {
                    probe_count(right.iter().copied(), |word| left.contains(word))
                } else {
                    probe_count(left.iter(), |word| right.binary_search(&word).is_ok())
                }
            }
            (Repr::Sorted(left), Repr::Roaring(right)) => {
                if prefer_sorted_probe(self.cardinality, other.cardinality, 0) {
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
                if prefer_sorted_probe(other.cardinality, self.cardinality, 0) {
                    probe_count(right.iter().copied(), |word| {
                        W::roaring_contains(left, word)
                    })
                } else {
                    probe_count(W::roaring_iter(left), |word| {
                        right.binary_search(&word).is_ok()
                    })
                }
            }
            (Repr::Dense(left), Repr::Roaring(right)) => {
                if prefer_dense_promotion(left.words.len(), self.cardinality.min(other.cardinality))
                {
                    let promoted = W::roaring_from_dense(left.base, &left.words);
                    W::roaring_intersection_len(&promoted, right)
                } else if self.cardinality <= other.cardinality {
                    probe_count(left.iter(), |word| W::roaring_contains(right, word))
                } else {
                    probe_count(W::roaring_iter(right), |word| left.contains(word))
                }
            }
            (Repr::Roaring(left), Repr::Dense(right)) => {
                if prefer_dense_promotion(
                    right.words.len(),
                    self.cardinality.min(other.cardinality),
                ) {
                    let promoted = W::roaring_from_dense(right.base, &right.words);
                    W::roaring_intersection_len(left, &promoted)
                } else if self.cardinality <= other.cardinality {
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
                if prefer_sorted_probe(self.cardinality, other.cardinality, right.words.len()) {
                    left.iter().all(|&word| !right.contains(word))
                } else {
                    right.iter().all(|word| left.binary_search(&word).is_err())
                }
            }
            (Repr::Dense(left), Repr::Sorted(right)) => {
                if prefer_sorted_probe(other.cardinality, self.cardinality, left.words.len()) {
                    right.iter().all(|&word| !left.contains(word))
                } else {
                    left.iter().all(|word| right.binary_search(&word).is_err())
                }
            }
            (Repr::Sorted(left), Repr::Roaring(right)) => {
                if prefer_sorted_probe(self.cardinality, other.cardinality, 0) {
                    left.iter().all(|&word| !W::roaring_contains(right, word))
                } else {
                    W::roaring_iter(right).all(|word| left.binary_search(&word).is_err())
                }
            }
            (Repr::Roaring(left), Repr::Sorted(right)) => {
                if prefer_sorted_probe(other.cardinality, self.cardinality, 0) {
                    right.iter().all(|&word| !W::roaring_contains(left, word))
                } else {
                    W::roaring_iter(left).all(|word| right.binary_search(&word).is_err())
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

    pub(crate) fn replan(&self, policy: IntSetPolicy) -> Result<Self, IntSetError> {
        self.clone().finish(policy)
    }

    /// Every path, including identity and empty operations, honors the supplied
    /// result policy. Representation choices are never inherited as requirements.
    pub(crate) fn binary(
        &self,
        other: &Self,
        op: Operation,
        policy: IntSetPolicy,
    ) -> Result<Self, IntSetError> {
        if core::ptr::eq(self, other) {
            return match op {
                Operation::Union | Operation::Intersection => self.replan(policy),
                Operation::Difference | Operation::SymmetricDifference => {
                    Self::from_sorted(Vec::new(), policy)
                }
            };
        }
        if self.cardinality == 0 || other.cardinality == 0 {
            return match op {
                Operation::Union | Operation::SymmetricDifference => {
                    if self.cardinality == 0 {
                        other.replan(policy)
                    } else {
                        self.replan(policy)
                    }
                }
                Operation::Intersection => Self::from_sorted(Vec::new(), policy),
                Operation::Difference => self.replan(policy),
            };
        }
        if self.separated(other) {
            match op {
                Operation::Intersection => return Self::from_sorted(Vec::new(), policy),
                Operation::Difference => return self.replan(policy),
                Operation::Union | Operation::SymmetricDifference => {}
            }
        }
        if let (Repr::Sorted(left), Repr::Sorted(right)) = (&self.repr, &other.repr) {
            let result = match op {
                Operation::Union => set_ops::SetUnion::new(left, right).copied().collect(),
                Operation::Intersection => set_ops::SetIntersection::new(left, right)
                    .copied()
                    .collect(),
                Operation::Difference => {
                    set_ops::SetDifference::new(left, right).copied().collect()
                }
                Operation::SymmetricDifference => set_ops::SetSymmetricDifference::new(left, right)
                    .copied()
                    .collect(),
            };
            return Self::from_sorted(result, policy);
        }

        if let Some(result) = self.filtered_binary(other, op) {
            return Self::from_sorted(result, policy);
        }
        if policy.backend == IntSetBackend::Sorted {
            return Ok(Self::sorted(mixed(
                self.iter(),
                other.iter(),
                |word| self.contains(word),
                |word| other.contains(word),
                self.cardinality <= other.cardinality,
                op,
            )));
        }
        if let Some(result) = self.dense_binary(other, op, policy) {
            return result.finish(policy);
        }

        // A sparse sorted operand can itself rule out Roaring for union/XOR.
        // Avoid constructing a large treemap only to throw it away afterward.
        if matches!(op, Operation::Union | Operation::SymmetricDifference) {
            let excessive = [&self.repr, &other.repr].into_iter().any(|repr| {
                matches!(repr, Repr::Sorted(words) if partition_count(words) > policy.max_roaring_partitions as u64)
            });
            if excessive && policy.backend == IntSetBackend::Auto {
                return Self::from_sorted(merge(self.iter(), other.iter(), op), policy);
            }
        }

        // A small sorted operand updates a native clone without reconstructing
        // every member of the larger Roaring operand.
        match (&self.repr, &other.repr, op) {
            (Repr::Roaring(left), Repr::Sorted(right), _)
                if right.len() <= SMALL_SORTED_UPDATES
                    && matches!(
                        op,
                        Operation::Union | Operation::Difference | Operation::SymmetricDifference
                    ) =>
            {
                return Self::roaring(roaring_update::<W>(left, right, op)).finish(policy);
            }
            (
                Repr::Sorted(left),
                Repr::Roaring(right),
                Operation::Union | Operation::SymmetricDifference,
            ) if left.len() <= SMALL_SORTED_UPDATES => {
                return Self::roaring(roaring_update::<W>(right, left, op)).finish(policy);
            }
            _ => {}
        }

        // Dense promotion scans bitmap words, using bounded little-endian byte
        // chunks. In particular, two distant dense runs never require a sorted
        // staging array merely because their combined dense window is too wide.
        let left = self.as_roaring();
        let right = other.as_roaring();
        let mut result = W::roaring_binary(&left, &right, op);
        W::roaring_optimize(&mut result);
        Self::roaring(result).finish(policy)
    }

    fn dense_binary(&self, other: &Self, op: Operation, policy: IntSetPolicy) -> Option<Self> {
        if policy.backend == IntSetBackend::Roaring {
            return None;
        }
        if let (Repr::Dense(left), Repr::Dense(right)) = (&self.repr, &other.repr)
            && let Some(dense) = left.binary(right, op, policy.max_dense_bytes)
        {
            return Some(Self::dense(dense));
        }
        if !matches!(op, Operation::Intersection) {
            if let Some(result) = self.dense_update(other, op, policy) {
                return Some(result);
            }
            if !matches!(op, Operation::Difference)
                && let Some(result) = other.dense_update(self, op, policy)
            {
                return Some(result);
            }
        }
        None
    }

    /// Filter an already addressable sorted input or a small compressed input.
    /// Large compressed operands continue to the native container kernels.
    fn filtered_binary(&self, other: &Self, op: Operation) -> Option<Vec<W>> {
        match (&self.repr, &other.repr, op) {
            (Repr::Sorted(left), Repr::Dense(right), Operation::Difference) => Some(
                left.iter()
                    .copied()
                    .filter(|&word| !right.contains(word))
                    .collect(),
            ),
            (Repr::Sorted(left), Repr::Roaring(right), Operation::Difference) => Some(
                left.iter()
                    .copied()
                    .filter(|&word| !W::roaring_contains(right, word))
                    .collect(),
            ),
            (Repr::Sorted(left), Repr::Dense(right), Operation::Intersection) => {
                Some(sorted_compressed_intersection(
                    left,
                    right.iter(),
                    |word| right.contains(word),
                    other.cardinality,
                    right.words.len(),
                ))
            }
            (Repr::Sorted(left), Repr::Roaring(right), Operation::Intersection) => {
                Some(sorted_compressed_intersection(
                    left,
                    W::roaring_iter(right),
                    |word| W::roaring_contains(right, word),
                    other.cardinality,
                    0,
                ))
            }
            (Repr::Dense(left), Repr::Sorted(right), Operation::Intersection) => {
                Some(sorted_compressed_intersection(
                    right,
                    left.iter(),
                    |word| left.contains(word),
                    self.cardinality,
                    left.words.len(),
                ))
            }
            (Repr::Roaring(left), Repr::Sorted(right), Operation::Intersection) => {
                Some(sorted_compressed_intersection(
                    right,
                    W::roaring_iter(left),
                    |word| W::roaring_contains(left, word),
                    self.cardinality,
                    0,
                ))
            }
            (Repr::Dense(dense), Repr::Roaring(roaring), Operation::Intersection)
                if other.cardinality <= dense.words.len() as u64 + 64 =>
            {
                Some(
                    W::roaring_iter(roaring)
                        .filter(|&word| dense.contains(word))
                        .collect(),
                )
            }
            (Repr::Roaring(roaring), Repr::Dense(dense), Operation::Intersection)
                if self.cardinality <= dense.words.len() as u64 + 64 =>
            {
                Some(
                    W::roaring_iter(roaring)
                        .filter(|&word| dense.contains(word))
                        .collect(),
                )
            }
            _ => None,
        }
    }

    fn as_roaring(&self) -> Cow<'_, W::Roaring> {
        match &self.repr {
            Repr::Sorted(words) => Cow::Owned(W::roaring_from_sorted(words)),
            Repr::Dense(dense) => Cow::Owned(W::roaring_from_dense(dense.base, &dense.words)),
            Repr::Roaring(roaring) => Cow::Borrowed(roaring),
        }
    }

    fn partitions(&self) -> u64 {
        match &self.repr {
            Repr::Sorted(words) => partition_count(words),
            Repr::Dense(dense) => dense.partition_count(),
            Repr::Roaring(roaring) => W::roaring_estimate(roaring).0,
        }
    }

    fn finish(self, policy: IntSetPolicy) -> Result<Self, IntSetError> {
        let required_dense = self
            .bounds
            .map_or(0, |(low, high)| dense_bytes(low.to_u64(), high.to_u64()));
        let dense_limit = policy.max_dense_bytes.min(isize::MAX as usize);
        let sorted_score = u128::from(self.cardinality) * size_of::<W>() as u128 + 16;
        let backend = if policy.backend == IntSetBackend::Auto {
            match &self.repr {
                Repr::Sorted(_) => {
                    let Repr::Sorted(words) = self.repr else {
                        unreachable!()
                    };
                    return Self::from_sorted(words.into_vec(), policy);
                }
                Repr::Dense(dense) => {
                    // Preserve a competitive legal Dense result. This keeps a
                    // tiny update proportional to the clone plus changed bits,
                    // without rescanning every member or recomputing run scores.
                    let allocated = core::mem::size_of_val(dense.words.as_ref());
                    if allocated <= dense_limit && allocated as u128 + 24 <= sorted_score {
                        return Ok(self);
                    }
                    if self.cardinality == 0 {
                        return Ok(Self::empty());
                    }
                    if sorted_score <= required_dense + 24
                        || self.partitions() > policy.max_roaring_partitions as u64
                    {
                        return Self::from_sorted(self.iter().collect(), policy);
                    }
                    return Self::roaring(self.as_roaring().into_owned()).finish(policy);
                }
                Repr::Roaring(roaring) => {
                    let (partitions, roaring_score) = W::roaring_estimate(roaring);
                    // As with Dense, preserve a legal native result competitive
                    // with Sorted. Rebuilding alternating Roaring members into
                    // Dense can cost much more than the container operation.
                    // Result planning is history-aware, not a global minimum.
                    if partitions <= policy.max_roaring_partitions as u64
                        && roaring_score <= sorted_score
                    {
                        return Ok(self);
                    }
                    if required_dense <= dense_limit as u128 && required_dense + 24 < sorted_score {
                        IntSetBackend::Dense
                    } else {
                        IntSetBackend::Sorted
                    }
                }
            }
        } else {
            policy.backend
        };
        match backend {
            IntSetBackend::Sorted => {
                if matches!(self.repr, Repr::Sorted(_)) {
                    Ok(self)
                } else {
                    Ok(Self::sorted(self.iter().collect()))
                }
            }
            IntSetBackend::Dense => {
                if required_dense > dense_limit as u128 {
                    return Err(IntSetError::DenseBudgetExceeded {
                        required_bytes: required_dense,
                        max_bytes: dense_limit,
                    });
                }
                if let Repr::Dense(dense) = &self.repr
                    && core::mem::size_of_val(dense.words.as_ref()) <= dense_limit
                {
                    return Ok(self);
                }
                let mut dense = Dense::from_bounds(self.bounds);
                self.visit_runs(|start, end| dense.insert_run(start, end));
                Ok(Self {
                    repr: Repr::Dense(dense),
                    cardinality: self.cardinality,
                    bounds: self.bounds,
                })
            }
            IntSetBackend::Roaring => {
                let partitions = self.partitions();
                if partitions > policy.max_roaring_partitions as u64 {
                    return Err(IntSetError::RoaringPartitionLimit {
                        partitions,
                        max_partitions: policy.max_roaring_partitions,
                    });
                }
                if matches!(self.repr, Repr::Roaring(_)) {
                    Ok(self)
                } else {
                    Ok(Self::roaring(self.as_roaring().into_owned()))
                }
            }
            IntSetBackend::Auto => unreachable!("result planning chooses a concrete backend"),
        }
    }

    /// Clone and change individual bits for small updates. Cardinality changes
    /// are tracked from prior bit values; a full bitmap popcount is unnecessary.
    fn dense_update(&self, other: &Self, op: Operation, policy: IntSetPolicy) -> Option<Self> {
        let Repr::Dense(original) = &self.repr else {
            return None;
        };
        if other.cardinality > original.words.len() as u64 + 64 {
            return None;
        }
        let bounds = if matches!(op, Operation::Difference) {
            self.bounds
        } else {
            combined_bounds(self.bounds, other.bounds)
        };
        let bytes = bounds.map_or(0, |(low, high)| dense_bytes(low.to_u64(), high.to_u64()));
        if bytes > policy.max_dense_bytes.min(isize::MAX as usize) as u128 {
            return None;
        }
        let max_cardinality = if matches!(op, Operation::Difference) {
            u128::from(self.cardinality)
        } else {
            u128::from(self.cardinality) + u128::from(other.cardinality)
        };
        if policy.backend == IntSetBackend::Auto
            && bytes + 24 > max_cardinality * size_of::<W>() as u128 + 16
        {
            return None;
        }
        let mut dense = original.expanded(bounds);
        let mut cardinality = self.cardinality;
        let mut result_bounds = self.bounds;
        let mut removed_boundary = false;
        for word in other.iter() {
            let previous = dense.contains(word);
            let present = match op {
                Operation::Union => true,
                Operation::Difference => false,
                Operation::SymmetricDifference => !previous,
                Operation::Intersection => unreachable!("small updates only add or remove members"),
            };
            if previous == present {
                continue;
            }
            dense.set(word, present);
            if present {
                cardinality = cardinality
                    .checked_add(1)
                    .expect("addressable dense cardinality fits u64");
                result_bounds = combined_bounds(result_bounds, Some((word, word)));
            } else {
                cardinality -= 1;
                removed_boundary |=
                    result_bounds.is_some_and(|(low, high)| word == low || word == high);
            }
        }
        if cardinality == 0 {
            dense = Dense::empty();
            result_bounds = None;
        } else if removed_boundary {
            result_bounds = dense.bounds();
            dense.trim_to_bounds(result_bounds);
        }
        Some(Self {
            repr: Repr::Dense(dense),
            cardinality,
            bounds: result_bounds,
        })
    }
}

fn combined_bounds<W: Word>(left: Option<(W, W)>, right: Option<(W, W)>) -> Option<(W, W)> {
    match (left, right) {
        (Some((a, b)), Some((c, d))) => Some((a.min(c), b.max(d))),
        (Some(bounds), None) | (None, Some(bounds)) => Some(bounds),
        (None, None) => None,
    }
}

fn roaring_update<W: Word>(original: &W::Roaring, updates: &[W], op: Operation) -> W::Roaring {
    let mut result = original.clone();
    for &word in updates {
        match op {
            Operation::Union => {
                W::roaring_insert(&mut result, word);
            }
            Operation::Difference => {
                W::roaring_remove(&mut result, word);
            }
            Operation::SymmetricDifference => {
                if !W::roaring_remove(&mut result, word) {
                    W::roaring_insert(&mut result, word);
                }
            }
            Operation::Intersection => unreachable!("intersection uses membership filtering"),
        }
    }
    W::roaring_optimize(&mut result);
    result
}

/// Bound the byte-conversion work against the number of membership probes it
/// replaces. This conservative structural heuristic is not a CPU timing model.
fn prefer_dense_promotion(dense_words: usize, probes: u64) -> bool {
    dense_words as u128 * 8 < u128::from(probes)
}

/// Choose traversal before the filtering loop. Scanning the compressed side
/// is useful only when its cardinality is smaller than the addressable sorted
/// operand, so either direction has an addressable result without expansion of
/// a large compressed set. Both input iterators preserve ascending code order.
fn sorted_compressed_intersection<W: Word>(
    sorted: &[W],
    compressed: impl Iterator<Item = W>,
    mut contains: impl FnMut(W) -> bool,
    compressed_len: u64,
    traversal_words: usize,
) -> Vec<W> {
    let sorted_len = u64::try_from(sorted.len()).expect("addressable sorted length fits u64");
    if prefer_sorted_probe(sorted_len, compressed_len, traversal_words) {
        sorted
            .iter()
            .copied()
            .filter(|&word| contains(word))
            .collect()
    } else {
        compressed
            .filter(|word| sorted.binary_search(word).is_ok())
            .collect()
    }
}

/// Compare scanning sorted codes with repeated binary searches into them.
/// `bit_length(n)` equals ceil(log2(n + 1)) for n > 0, without overflowing at
/// `u64::MAX`. The wide product bounds comparison work rather than claiming a
/// machine-specific crossover. Near-balanced operands favor direct compressed
/// membership; genuinely tiny compressed operands still search the sorted side.
/// Dense iteration also scans zero words, so its word count contributes to the
/// estimated traversal work even when the bitmap has very few members.
fn prefer_sorted_probe(sorted_len: u64, compressed_len: u64, traversal_words: usize) -> bool {
    let comparisons = sorted_len.bit_width();
    u128::from(sorted_len)
        <= u128::from(compressed_len) * u128::from(comparisons) + traversal_words as u128
}

fn probe_count<W>(words: impl Iterator<Item = W>, mut contains: impl FnMut(W) -> bool) -> u64 {
    words.fold(0, |count, word| count + u64::from(contains(word)))
}

fn mixed<W: Word>(
    left: impl Iterator<Item = W>,
    right: impl Iterator<Item = W>,
    mut left_contains: impl FnMut(W) -> bool,
    mut right_contains: impl FnMut(W) -> bool,
    probe_left: bool,
    op: Operation,
) -> Vec<W> {
    match op {
        Operation::Intersection if probe_left => {
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

    /// Caller has already checked the payload budget with wide arithmetic.
    fn from_bounds(bounds: Option<(W, W)>) -> Self {
        let Some((low, high)) = bounds else {
            return Self::empty();
        };
        let count = usize::try_from(dense_bytes(low.to_u64(), high.to_u64()) / 8)
            .expect("planner checked addressable dense span");
        Self {
            base: low.to_u64() & !63,
            words: vec![0; count].into_boxed_slice(),
            word: PhantomData,
        }
    }

    fn expanded(&self, bounds: Option<(W, W)>) -> Self {
        if bounds.is_some_and(|(low, high)| {
            low.to_u64() & !63 == self.base
                && dense_bytes(low.to_u64(), high.to_u64()) == self.words.len() as u128 * 8
        }) {
            return self.clone();
        }
        let mut result = Self::from_bounds(bounds);
        if !self.words.is_empty() {
            let offset = usize::try_from((self.base - result.base) / 64)
                .expect("bounded dense expansion offset");
            result.words[offset..offset + self.words.len()].copy_from_slice(&self.words);
        }
        result
    }

    fn set(&mut self, member: W, present: bool) {
        let offset = member.to_u64() - self.base;
        let index = usize::try_from(offset / 64).expect("member lies in the allocated window");
        let mask = 1_u64 << (offset % 64);
        if present {
            self.words[index] |= mask;
        } else {
            self.words[index] &= !mask;
        }
    }

    /// These ranges come only from existing consecutive members. No domain
    /// gaps or invented numeric-universe members are introduced by conversion.
    fn insert_run(&mut self, start: u64, end: u64) {
        let first = usize::try_from((start - self.base) / 64).expect("bounded run start");
        let last = usize::try_from((end - self.base) / 64).expect("bounded run end");
        let first_mask = u64::MAX << (start % 64);
        let last_mask = u64::MAX >> (63 - end % 64);
        if first == last {
            self.words[first] |= first_mask & last_mask;
        } else {
            self.words[first] |= first_mask;
            self.words[first + 1..last].fill(u64::MAX);
            self.words[last] |= last_mask;
        }
    }

    fn trim_to_bounds(&mut self, bounds: Option<(W, W)>) {
        let Some((low, high)) = bounds else {
            *self = Self::empty();
            return;
        };
        let first = usize::try_from((low.to_u64() - self.base) / 64).expect("stored lower bound");
        let last = usize::try_from((high.to_u64() - self.base) / 64).expect("stored upper bound");
        if first == 0 && last + 1 == self.words.len() {
            return;
        }
        let mut words = core::mem::replace(&mut self.words, Box::new([])).into_vec();
        words.truncate(last + 1);
        if first != 0 {
            words.drain(..first);
        }
        self.base += first as u64 * 64;
        self.words = words.into_boxed_slice();
    }

    fn visit_runs(&self, mut visit: impl FnMut(u64, u64)) {
        let _: ControlFlow<()> = self.try_visit_runs(|start, end| {
            visit(start, end);
            ControlFlow::Continue(())
        });
    }

    fn try_visit_runs<B>(
        &self,
        mut visit: impl FnMut(u64, u64) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        for (index, &word) in self.words.iter().enumerate() {
            let base = self.base + index as u64 * 64;
            let mut pending = word;
            while pending != 0 {
                let start = pending.trailing_zeros();
                let count = (pending >> start).trailing_ones();
                visit(base + u64::from(start), base + u64::from(start + count - 1))?;
                let mask = (u64::MAX >> (64 - count)) << start;
                pending &= !mask;
            }
        }
        ControlFlow::Continue(())
    }

    fn partition_count(&self) -> u64 {
        let mut previous = None;
        let mut count = 0;
        for (index, &word) in self.words.iter().enumerate() {
            if word == 0 {
                continue;
            }
            let prefix = (self.base + index as u64 * 64) >> 32;
            if previous != Some(prefix) {
                count += 1;
                previous = Some(prefix);
            }
        }
        count
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
    fn binary(&self, other: &Self, op: Operation, max_dense_bytes: usize) -> Option<Self> {
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
        let limit = max_dense_bytes.min(isize::MAX as usize);
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

#[cfg(test)]
mod tests {
    use super::{Storage, sorted_compressed_intersection};
    use crate::integer::IntSetPolicy;
    use alloc::vec::Vec;
    use core::cell::Cell;
    use core::hash::Hasher;
    use core::ops::ControlFlow;

    #[derive(Default)]
    struct HashEvents(Vec<(u8, u64)>);

    impl Hasher for HashEvents {
        fn finish(&self) -> u64 {
            0
        }
        fn write(&mut self, _: &[u8]) {
            panic!("hash uses normalized integer events");
        }
        fn write_u8(&mut self, value: u8) {
            self.0.push((8, u64::from(value)));
        }
        fn write_u64(&mut self, value: u64) {
            self.0.push((64, value));
        }
    }

    #[test]
    fn hash_scout_selects_content_format_at_both_thresholds() {
        for (count, expected_tag) in [(64_u64, 0), (65, 1)] {
            let values: Vec<_> = (0..32).chain(64..64 + count - 32).collect();
            for policy in [
                IntSetPolicy::sorted(),
                IntSetPolicy::dense(),
                IntSetPolicy::roaring(),
            ] {
                let storage = Storage::from_sorted(values.clone(), policy).unwrap();
                let mut events = HashEvents::default();
                storage.hash(&mut events);
                assert_eq!(events.0[0], (64, count));
                assert_eq!(events.0[1], (8, expected_tag));
            }
        }
        for (runs, expected_tag) in [(16_u64, 1), (17, 0)] {
            let values: Vec<_> = (0..runs).flat_map(|run| run * 8..run * 8 + 5).collect();
            for policy in [
                IntSetPolicy::sorted(),
                IntSetPolicy::dense(),
                IntSetPolicy::roaring(),
            ] {
                let storage = Storage::from_sorted(values.clone(), policy).unwrap();
                let mut events = HashEvents::default();
                storage.hash(&mut events);
                assert_eq!(events.0[0], (64, runs * 5));
                assert_eq!(events.0[1], (8, expected_tag));
                let endpoints = if runs == 16 { 32 } else { 85 };
                assert_eq!(events.0.len(), 2 + endpoints);
            }
        }
    }

    #[test]
    fn run_scout_break_propagates_before_the_seventeenth_run_tail() {
        let values: Vec<_> = (0_u64..16)
            .flat_map(|run| run * 8..run * 8 + 5)
            .chain(1024..1152)
            .collect();
        for policy in [
            IntSetPolicy::sorted(),
            IntSetPolicy::dense(),
            IntSetPolicy::roaring(),
        ] {
            let storage = Storage::from_sorted(values.clone(), policy).unwrap();
            let mut runs = 0;
            let mut previous: Option<u64> = None;
            let mut stopped = false;
            let result = storage.try_visit_raw_runs(|start, end| {
                assert!(
                    !stopped,
                    "visitor must stop invoking the callback after Break"
                );
                if previous.and_then(|end| end.checked_add(1)) != Some(start) {
                    runs += 1;
                }
                if runs == 17 {
                    assert_eq!(start, 1024);
                    // Dense emits at most one physical word before observing
                    // Break; Sorted and Roaring offer just the next member.
                    assert!(end <= 1087, "must not traverse the long final run first");
                    stopped = true;
                    return ControlFlow::Break(());
                }
                previous = Some(end);
                ControlFlow::Continue(())
            });
            assert!(result.is_break());
            assert!(stopped);
        }
    }

    #[test]
    fn skewed_intersection_does_not_scan_the_large_operand() {
        let large: Vec<_> = (0_u64..4096).collect();
        let probes = Cell::new(0);
        let result = sorted_compressed_intersection(
            &large,
            [3, 4095].into_iter(),
            |word| {
                probes.set(probes.get() + 1);
                word == 3 || word == 4095
            },
            2,
            0,
        );
        assert_eq!(result, [3, 4095]);
        assert_eq!(
            probes.get(),
            0,
            "a tiny compressed input must search the sorted input"
        );

        let visits = Cell::new(0);
        let result = sorted_compressed_intersection(
            &[3_u64, 4095],
            (0..4096).inspect(|_| visits.set(visits.get() + 1)),
            |word| word < 4096,
            4096,
            64,
        );
        assert_eq!(result, [3, 4095]);
        assert_eq!(
            visits.get(),
            0,
            "a tiny sorted input must probe compressed membership"
        );
    }
}

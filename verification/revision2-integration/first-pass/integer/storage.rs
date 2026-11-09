//! Private word mapping, bounded representation planning, and encoded kernels.
//!
//! Dense bases are absolute multiples of 64. Operations align those coordinates
//! before combining words; equal local offsets alone never imply equal members.
//!
//! Algebra between different representations promotes one operand into the
//! other's representation (or edits a copy of the larger operand) instead of
//! degrading to sorted codes. A result that equals an operand shares that
//! operand's allocation.

use super::Shared as Arc;
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::iter::FusedIterator;
use core::mem::size_of;
use core::ops::ControlFlow;

use super::chunked::{Chunked, ChunkedIter};
use super::dense::{Dense, DenseIter, dense_bytes};
use super::{IntSetBackend, IntSetError, IntSetPolicy, IntSetRepr};
use crate::family::ordered::set_ops;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Union,
    Intersection,
    Difference,
    SymmetricDifference,
}

/// Allocation-scale preference retained from the reviewed main planner.
/// This is a deterministic memory allowance, not a measured CPU threshold.
const DENSE_PREFERENCE_BYTES: u128 = 128;
const SMALL_SORTED_UPDATES: u64 = 64;

/// A probe into a compressed operand costs roughly this many promoted words.
const PROBE_WORDS: u64 = 4;

/// Limits carried by a set and inherited by its results.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Budget {
    pub(crate) dense_bytes: usize,
    pub(crate) roaring_partitions: usize,
}

impl Budget {
    pub(crate) const fn from_policy(policy: IntSetPolicy) -> Self {
        Self {
            dense_bytes: policy.max_dense_bytes,
            roaring_partitions: policy.max_roaring_partitions,
        }
    }
    fn dense_limit(self) -> usize {
        self.dense_bytes.min(isize::MAX as usize)
    }
}

mod word;
pub(crate) use word::Word;

#[derive(Clone)]
enum Repr<W: Word> {
    Sorted(Box<[W]>),
    Dense(Dense<W>),
    Roaring(W::Roaring),
    Chunked(Chunked<W>),
}

/// Immutable cardinality and bounds are cached for every representation.
#[derive(Clone)]
pub(crate) struct Storage<W: Word> {
    repr: Repr<W>,
    cardinality: u64,
    bounds: Option<(W, W)>,
}

/// Planning statistics gathered in one pass over strictly ascending codes.
struct Survey {
    partitions: u64,
    roaring_score: u128,
}

/// Deterministic planning proxy, not an allocator or resident-memory
/// measurement. It includes conservative container and treemap-entry overhead.
fn survey<W: Word>(words: &[W]) -> Survey {
    if words.is_empty() {
        return Survey {
            partitions: 0,
            roaring_score: 32,
        };
    }
    let mut partitions = 0_u64;
    let mut previous: Option<u64> = None;
    let mut container_members = 0_u64;
    let mut container_runs = 0_u64;
    let mut score = 32_u128;
    let close = |members: u64, runs: u64| -> u128 {
        32 + u128::from((2 * members).min(8192).min(4 * runs + 2))
    };
    for &word in words {
        let value = word.to_u64();
        if previous.is_none_or(|previous| previous >> 32 != value >> 32) {
            partitions += 1;
        }
        if previous.is_some_and(|previous| previous >> 16 != value >> 16) {
            score += close(container_members, container_runs);
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
    score += close(container_members, container_runs);
    if size_of::<W>() == 8 {
        score += u128::from(partitions) * 80;
    }
    Survey {
        partitions,
        roaring_score: score,
    }
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
        let budget = Budget::from_policy(policy);
        let dense_bytes = bounds.map_or(0, |(low, high)| dense_bytes(low.to_u64(), high.to_u64()));
        let dense_limit = budget.dense_limit();
        let backend = match policy.backend {
            IntSetBackend::Auto => plan(&words, dense_bytes, budget),
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
                let partitions = partition_count(&words);
                if partitions > policy.max_roaring_partitions as u64 {
                    return Err(IntSetError::RoaringPartitionLimit {
                        partitions,
                        max_partitions: policy.max_roaring_partitions,
                    });
                }
                Repr::Roaring(W::roaring_from_sorted(&words))
            }
            IntSetBackend::Chunked => Repr::Chunked(Chunked::from_sorted(&words)),
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

    fn roaring(mut value: W::Roaring) -> Self {
        W::roaring_optimize(&mut value);
        let cardinality = W::roaring_len(&value);
        let bounds = W::roaring_bounds(&value);
        Self {
            repr: Repr::Roaring(value),
            cardinality,
            bounds,
        }
    }

    fn dense(value: Dense<W>) -> Self {
        let cardinality = value.cardinality();
        let bounds = value.bounds();
        Self {
            repr: Repr::Dense(value),
            cardinality,
            bounds,
        }
    }

    fn chunked(value: Chunked<W>) -> Self {
        let cardinality = value.cardinality();
        let bounds = value.bounds();
        Self {
            repr: Repr::Chunked(value),
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
            Repr::Chunked(_) => IntSetRepr::Chunked,
        }
    }

    pub(crate) fn contiguous_payload_bytes(&self) -> Option<usize> {
        match &self.repr {
            Repr::Sorted(words) => Some(core::mem::size_of_val(words.as_ref())),
            Repr::Dense(dense) => Some(dense.payload_bytes()),
            Repr::Roaring(_) | Repr::Chunked(_) => None,
        }
    }

    /// Estimated payload bytes of the stored representation.
    pub(crate) fn payload_bytes(&self) -> u64 {
        match &self.repr {
            Repr::Sorted(words) => words.len() as u64 * size_of::<W>() as u64,
            Repr::Dense(dense) => dense.payload_bytes() as u64,
            Repr::Roaring(value) => u64::try_from(W::roaring_estimate(value).1).unwrap_or(u64::MAX),
            Repr::Chunked(chunked) => chunked.payload_bytes(),
        }
    }

    pub(crate) fn chunk_count(&self) -> u64 {
        match &self.repr {
            Repr::Chunked(chunked) => chunked.chunk_count() as u64,
            _ => 0,
        }
    }

    pub(crate) fn shared_chunks(&self, other: &Self) -> u64 {
        match (&self.repr, &other.repr) {
            (Repr::Chunked(left), Repr::Chunked(right)) => left.shared_chunks(right),
            _ => 0,
        }
    }

    pub(crate) fn contains(&self, member: W) -> bool {
        match &self.repr {
            Repr::Sorted(words) => words.binary_search(&member).is_ok(),
            Repr::Dense(words) => words.contains(member),
            Repr::Roaring(words) => W::roaring_contains(words, member),
            Repr::Chunked(chunks) => chunks.contains(member),
        }
    }

    pub(crate) fn iter(&self) -> Words<'_, W> {
        Words {
            inner: match &self.repr {
                Repr::Sorted(words) => WordIter::Sorted(words.iter().copied()),
                Repr::Dense(words) => WordIter::Dense(words.iter()),
                Repr::Roaring(words) => WordIter::Roaring(W::roaring_iter(words)),
                Repr::Chunked(chunks) => WordIter::Chunked(chunks.iter()),
            },
            remaining: self.cardinality,
        }
    }

    /// Representation-independent equality with same-representation fast paths.
    pub(crate) fn equals(&self, other: &Self) -> bool {
        if self.cardinality != other.cardinality || self.bounds != other.bounds {
            return false;
        }
        match (&self.repr, &other.repr) {
            (Repr::Sorted(left), Repr::Sorted(right)) => left == right,
            (Repr::Dense(left), Repr::Dense(right)) => left == right,
            (Repr::Roaring(left), Repr::Roaring(right)) => left == right,
            (Repr::Chunked(left), Repr::Chunked(right)) => left == right,
            _ => self.iter().eq(other.iter()),
        }
    }

    pub(crate) fn compare(&self, other: &Self) -> Ordering {
        if self.equals(other) {
            return Ordering::Equal;
        }
        self.iter().cmp(other.iter())
    }

    /// The format depends only on content: cardinality, then either a run tag
    /// and maximal endpoints or a member tag and u64-normalized codes (including
    /// 32-bit word representations). A short run scout avoids hashing two
    /// endpoints per isolated member. Physical storage never selects the stream;
    /// these bytes are not a persistent serialization format.
    pub(crate) fn hash_into<H: Hasher>(&self, state: &mut H) {
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
            Repr::Chunked(chunks) => {
                for word in chunks.iter() {
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
            Repr::Chunked(chunks) => chunks.try_visit_runs(visit),
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
                Repr::Chunked(chunks) => chunks.visit_runs(&mut coalesce),
            }
        }
        if let Some((start, end)) = pending {
            visit(start, end);
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

    // ----------------------------------------------------------------------
    // Counting predicates
    // ----------------------------------------------------------------------

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
            (Repr::Chunked(left), Repr::Chunked(right)) => left.intersection_len(right),
            (Repr::Sorted(sorted), Repr::Dense(dense))
            | (Repr::Dense(dense), Repr::Sorted(sorted)) => {
                let sub = window_slice(sorted, dense);
                if prefer_sorted_probe(
                    sub.len() as u64,
                    dense_cardinality(self, other),
                    dense.words.len(),
                ) {
                    probe_count(sub.iter().copied(), |word| dense.contains(word))
                } else {
                    probe_count(dense.iter(), |word| sub.binary_search(&word).is_ok())
                }
            }
            (Repr::Sorted(sorted), Repr::Roaring(roaring))
            | (Repr::Roaring(roaring), Repr::Sorted(sorted)) => {
                let sorted_len = sorted.len() as u64;
                let roaring_len = W::roaring_len(roaring);
                if prefer_sorted_probe(sorted_len, roaring_len, 0) {
                    probe_count(sorted.iter().copied(), |word| {
                        W::roaring_contains(roaring, word)
                    })
                } else {
                    probe_count(W::roaring_iter(roaring), |word| {
                        sorted.binary_search(&word).is_ok()
                    })
                }
            }
            (Repr::Dense(dense), Repr::Roaring(roaring))
            | (Repr::Roaring(roaring), Repr::Dense(dense)) => {
                match dense_roaring_strategy(
                    dense.words.len() as u64,
                    dense_cardinality(self, other),
                    W::roaring_len(roaring),
                ) {
                    DenseRoaring::Promote => W::roaring_intersection_len(
                        &W::roaring_from_dense(dense.base, &dense.words),
                        roaring,
                    ),
                    DenseRoaring::IterateDense => {
                        probe_count(dense.iter(), |word| W::roaring_contains(roaring, word))
                    }
                    DenseRoaring::IterateRoaring => {
                        probe_count(W::roaring_iter(roaring), |word| dense.contains(word))
                    }
                }
            }
            (Repr::Chunked(chunks), Repr::Dense(dense))
            | (Repr::Dense(dense), Repr::Chunked(chunks)) => {
                chunks.intersection_len(&Chunked::from_dense(dense.base, &dense.words))
            }
            (Repr::Chunked(chunks), Repr::Sorted(sorted))
            | (Repr::Sorted(sorted), Repr::Chunked(chunks)) => {
                if prefer_sorted_probe(sorted.len() as u64, chunks.cardinality(), 0) {
                    probe_count(sorted.iter().copied(), |word| chunks.contains(word))
                } else {
                    probe_count(chunks.iter(), |word| sorted.binary_search(&word).is_ok())
                }
            }
            (Repr::Chunked(chunks), Repr::Roaring(roaring))
            | (Repr::Roaring(roaring), Repr::Chunked(chunks)) => {
                if chunks.cardinality() <= W::roaring_len(roaring) {
                    probe_count(chunks.iter(), |word| W::roaring_contains(roaring, word))
                } else {
                    probe_count(W::roaring_iter(roaring), |word| chunks.contains(word))
                }
            }
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
            (Repr::Chunked(left), Repr::Chunked(right)) => left.is_disjoint(right),
            (Repr::Sorted(sorted), Repr::Dense(dense))
            | (Repr::Dense(dense), Repr::Sorted(sorted)) => {
                let sub = window_slice(sorted, dense);
                if prefer_sorted_probe(
                    sub.len() as u64,
                    dense_cardinality(self, other),
                    dense.words.len(),
                ) {
                    sub.iter().all(|&word| !dense.contains(word))
                } else {
                    dense.iter().all(|word| sub.binary_search(&word).is_err())
                }
            }
            (Repr::Sorted(sorted), Repr::Roaring(roaring))
            | (Repr::Roaring(roaring), Repr::Sorted(sorted)) => {
                if prefer_sorted_probe(sorted.len() as u64, W::roaring_len(roaring), 0) {
                    sorted
                        .iter()
                        .all(|&word| !W::roaring_contains(roaring, word))
                } else {
                    W::roaring_iter(roaring).all(|word| sorted.binary_search(&word).is_err())
                }
            }
            (Repr::Dense(dense), Repr::Roaring(roaring))
            | (Repr::Roaring(roaring), Repr::Dense(dense)) => {
                match dense_roaring_strategy(
                    dense.words.len() as u64,
                    dense_cardinality(self, other),
                    W::roaring_len(roaring),
                ) {
                    DenseRoaring::Promote => W::roaring_is_disjoint(
                        &W::roaring_from_dense(dense.base, &dense.words),
                        roaring,
                    ),
                    DenseRoaring::IterateDense => {
                        dense.iter().all(|word| !W::roaring_contains(roaring, word))
                    }
                    DenseRoaring::IterateRoaring => {
                        W::roaring_iter(roaring).all(|word| !dense.contains(word))
                    }
                }
            }
            (Repr::Chunked(chunks), Repr::Dense(dense))
            | (Repr::Dense(dense), Repr::Chunked(chunks)) => {
                chunks.is_disjoint(&Chunked::from_dense(dense.base, &dense.words))
            }
            (Repr::Chunked(chunks), Repr::Sorted(sorted))
            | (Repr::Sorted(sorted), Repr::Chunked(chunks)) => {
                if prefer_sorted_probe(sorted.len() as u64, chunks.cardinality(), 0) {
                    sorted.iter().all(|&word| !chunks.contains(word))
                } else {
                    chunks
                        .iter()
                        .all(|word| sorted.binary_search(&word).is_err())
                }
            }
            (Repr::Chunked(chunks), Repr::Roaring(roaring))
            | (Repr::Roaring(roaring), Repr::Chunked(chunks)) => {
                if chunks.cardinality() <= W::roaring_len(roaring) {
                    chunks
                        .iter()
                        .all(|word| !W::roaring_contains(roaring, word))
                } else {
                    W::roaring_iter(roaring).all(|word| !chunks.contains(word))
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
            (Repr::Chunked(left), Repr::Chunked(right)) => left.is_subset(right),
            (Repr::Sorted(left), Repr::Dense(right)) => {
                left.iter().all(|&word| right.contains(word))
            }
            (Repr::Dense(left), Repr::Sorted(right)) => {
                let sub = window_slice(right, left);
                left.iter().all(|word| sub.binary_search(&word).is_ok())
            }
            (Repr::Sorted(left), Repr::Roaring(right)) => {
                left.iter().all(|&word| W::roaring_contains(right, word))
            }
            (Repr::Roaring(left), Repr::Sorted(right)) => {
                W::roaring_iter(left).all(|word| right.binary_search(&word).is_ok())
            }
            (Repr::Dense(left), Repr::Roaring(right)) => {
                if dense_roaring_strategy(
                    left.words.len() as u64,
                    self.cardinality,
                    W::roaring_len(right),
                ) == DenseRoaring::Promote
                {
                    W::roaring_is_subset(&W::roaring_from_dense(left.base, &left.words), right)
                } else {
                    left.iter().all(|word| W::roaring_contains(right, word))
                }
            }
            (Repr::Roaring(left), Repr::Dense(right)) => {
                W::roaring_iter(left).all(|word| right.contains(word))
            }
            (Repr::Chunked(left), Repr::Dense(right)) => {
                left.is_subset(&Chunked::from_dense(right.base, &right.words))
            }
            (Repr::Dense(left), Repr::Chunked(right)) => {
                Chunked::from_dense(left.base, &left.words).is_subset(right)
            }
            (Repr::Chunked(left), Repr::Sorted(right)) => {
                left.iter().all(|word| right.binary_search(&word).is_ok())
            }
            (Repr::Sorted(left), Repr::Chunked(right)) => {
                left.iter().all(|&word| right.contains(word))
            }
            (Repr::Chunked(left), Repr::Roaring(right)) => {
                left.iter().all(|word| W::roaring_contains(right, word))
            }
            (Repr::Roaring(left), Repr::Chunked(right)) => {
                W::roaring_iter(left).all(|word| right.contains(word))
            }
        }
    }

    // ----------------------------------------------------------------------
    // Materialized algebra
    // ----------------------------------------------------------------------

    /// Whether the current representation satisfies an output policy.
    fn accepts(&self, policy: IntSetPolicy) -> bool {
        match &self.repr {
            Repr::Sorted(_) => {
                matches!(policy.backend, IntSetBackend::Auto | IntSetBackend::Sorted)
            }
            Repr::Dense(dense) => {
                matches!(policy.backend, IntSetBackend::Auto | IntSetBackend::Dense)
                    && dense.payload_bytes() <= policy.max_dense_bytes.min(isize::MAX as usize)
            }
            Repr::Roaring(_) => {
                matches!(policy.backend, IntSetBackend::Auto | IntSetBackend::Roaring)
                    && self.partitions() <= policy.max_roaring_partitions as u64
            }
            Repr::Chunked(_) => {
                matches!(policy.backend, IntSetBackend::Auto | IntSetBackend::Chunked)
            }
        }
    }

    pub(crate) fn replan_shared(
        self: &Arc<Self>,
        policy: IntSetPolicy,
    ) -> Result<Arc<Self>, IntSetError> {
        if policy.backend != IntSetBackend::Auto && self.accepts(policy) {
            return Ok(Arc::clone(self));
        }
        self.as_ref().clone().finish(policy).map(Arc::new)
    }

    fn share_or_replan(self: &Arc<Self>, policy: IntSetPolicy) -> Result<Arc<Self>, IntSetError> {
        if self.accepts(policy) {
            Ok(Arc::clone(self))
        } else {
            self.replan_shared(policy)
        }
    }

    /// Every path, including empty and equal operands, enforces output limits.
    /// Reuse an eligible operand allocation when the logical result is identical.
    pub(crate) fn binary(
        self: &Arc<Self>,
        other: &Arc<Self>,
        op: Operation,
        policy: IntSetPolicy,
    ) -> Result<Arc<Self>, IntSetError> {
        let empty = || Self::from_sorted(Vec::new(), policy).map(Arc::new);
        if Arc::ptr_eq(self, other) {
            return match op {
                Operation::Union | Operation::Intersection => self.share_or_replan(policy),
                Operation::Difference | Operation::SymmetricDifference => empty(),
            };
        }
        if self.cardinality == 0 {
            return match op {
                Operation::Union | Operation::SymmetricDifference => other.share_or_replan(policy),
                Operation::Intersection | Operation::Difference => empty(),
            };
        }
        if other.cardinality == 0 {
            return match op {
                Operation::Intersection => empty(),
                _ => self.share_or_replan(policy),
            };
        }
        if self.separated(other) {
            match op {
                Operation::Intersection => return empty(),
                Operation::Difference => return self.share_or_replan(policy),
                Operation::Union | Operation::SymmetricDifference => {}
            }
        }
        // A scattered sorted operand can rule out the native partition budget.
        // Avoid constructing a large temporary treemap only to discard it.
        if policy.backend == IntSetBackend::Auto
            && matches!(op, Operation::Union | Operation::SymmetricDifference)
            && !matches!(&self.repr, Repr::Chunked(_))
            && !matches!(&other.repr, Repr::Chunked(_))
            && [&self.repr, &other.repr].into_iter().any(|repr| {
                matches!(repr, Repr::Sorted(words) if partition_count(words) > policy.max_roaring_partitions as u64)
            })
        {
            return Self::from_sorted(merge(self.iter(), other.iter(), op, 0), policy).map(Arc::new);
        }
        let combined = if policy.backend != IntSetBackend::Roaring
            && policy.backend != IntSetBackend::Chunked
            && !matches!(op, Operation::Intersection)
            && !matches!((&self.repr, &other.repr), (Repr::Dense(_), Repr::Dense(_)))
            && !matches!(&self.repr, Repr::Chunked(_))
            && !matches!(&other.repr, Repr::Chunked(_))
        {
            let fast = self.dense_update(other, op, policy).or_else(|| {
                if op == Operation::Difference {
                    None
                } else {
                    other.dense_update(self, op, policy)
                }
            });
            fast.map_or_else(
                || self.combine(other, op, Budget::from_policy(policy)),
                Combined::Stored,
            )
        } else {
            self.combine(other, op, Budget::from_policy(policy))
        };
        let cardinality = match &combined {
            Combined::Stored(storage) => storage.cardinality,
            Combined::Codes(codes) => codes.len() as u64,
        };
        let equals_left = cardinality == self.cardinality
            && matches!(
                op,
                Operation::Union | Operation::Intersection | Operation::Difference
            );
        let equals_right = cardinality == other.cardinality
            && matches!(op, Operation::Union | Operation::Intersection);
        // Try both operands: equal contents can have differently eligible layouts.
        if equals_left && self.accepts(policy) {
            return Ok(Arc::clone(self));
        }
        if equals_right && other.accepts(policy) {
            return Ok(Arc::clone(other));
        }
        match combined {
            Combined::Stored(storage) => storage.finish(policy),
            Combined::Codes(codes) => Self::from_sorted(codes, policy),
        }
        .map(Arc::new)
    }

    fn as_roaring(&self) -> Cow<'_, W::Roaring> {
        match &self.repr {
            Repr::Sorted(words) => Cow::Owned(W::roaring_from_sorted(words)),
            Repr::Dense(dense) => Cow::Owned(W::roaring_from_dense(dense.base, &dense.words)),
            Repr::Roaring(roaring) => Cow::Borrowed(roaring),
            Repr::Chunked(chunks) => Cow::Owned(W::roaring_from_ascending(chunks.iter())),
        }
    }

    fn partitions(&self) -> u64 {
        match &self.repr {
            Repr::Sorted(words) => partition_count(words),
            Repr::Dense(dense) => dense.partition_count(),
            Repr::Roaring(roaring) => W::roaring_estimate(roaring).0,
            Repr::Chunked(chunks) => chunks.partition_count(),
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
                Repr::Chunked(_) => return Ok(self),
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
            IntSetBackend::Chunked => match &self.repr {
                Repr::Chunked(_) => Ok(self),
                Repr::Dense(dense) => {
                    Ok(Self::chunked(Chunked::from_dense(dense.base, &dense.words)))
                }
                _ => Ok(Self::chunked(Chunked::from_ascending(
                    self.iter().map(Word::to_u64),
                ))),
            },
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

    fn combine(&self, other: &Self, op: Operation, budget: Budget) -> Combined<W> {
        Combined::Stored(match (&self.repr, &other.repr) {
            (Repr::Roaring(left), Repr::Roaring(right)) => {
                Self::roaring(W::roaring_binary(left, right, op))
            }
            (Repr::Chunked(left), Repr::Chunked(right)) => Self::chunked(left.binary(right, op)),
            (Repr::Dense(left), Repr::Dense(right)) => {
                let limit = budget.dense_limit();
                match left.binary(right, op, limit) {
                    Some(value) => Self::dense(value),
                    // Two dense islands too far apart for one window: Roaring
                    // is the representation built for exactly that shape.
                    None => Self::roaring(W::roaring_binary(
                        &W::roaring_from_dense(left.base, &left.words),
                        &W::roaring_from_dense(right.base, &right.words),
                        op,
                    )),
                }
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
            (Repr::Dense(dense), Repr::Sorted(sorted)) => {
                return dense_sorted(dense, self.cardinality, sorted, op, true, budget);
            }
            (Repr::Sorted(sorted), Repr::Dense(dense)) => {
                return dense_sorted(dense, other.cardinality, sorted, op, false, budget);
            }
            (Repr::Dense(dense), Repr::Roaring(roaring)) => {
                dense_roaring(dense, self.cardinality, roaring, op, true, budget)
            }
            (Repr::Roaring(roaring), Repr::Dense(dense)) => {
                dense_roaring(dense, other.cardinality, roaring, op, false, budget)
            }
            (Repr::Sorted(sorted), Repr::Roaring(roaring)) => {
                return sorted_roaring(sorted, roaring, op, true);
            }
            (Repr::Roaring(roaring), Repr::Sorted(sorted)) => {
                return sorted_roaring(sorted, roaring, op, false);
            }
            (Repr::Chunked(chunks), Repr::Dense(dense)) => {
                Self::chunked(chunks.binary(&Chunked::from_dense(dense.base, &dense.words), op))
            }
            (Repr::Dense(dense), Repr::Chunked(chunks)) => {
                Self::chunked(Chunked::from_dense(dense.base, &dense.words).binary(chunks, op))
            }
            (Repr::Chunked(chunks), Repr::Sorted(sorted)) => {
                chunked_sorted(chunks, sorted, op, true)
            }
            (Repr::Sorted(sorted), Repr::Chunked(chunks)) => {
                chunked_sorted(chunks, sorted, op, false)
            }
            (Repr::Chunked(chunks), Repr::Roaring(roaring)) => {
                chunked_roaring(chunks, roaring, op, true)
            }
            (Repr::Roaring(roaring), Repr::Chunked(chunks)) => {
                chunked_roaring(chunks, roaring, op, false)
            }
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

/// A materialized result, or merged codes finished only if the result is not
/// one of the operands. Fresh results are finished under the output policy.
enum Combined<W: Word> {
    Stored(Storage<W>),
    Codes(Vec<W>),
}

/// A Roaring set with fewer than this many members per container is treated as
/// scattered: copying its container tree costs more than merging its codes.
const WELL_FILLED_MEMBERS: u64 = 16;

/// Automatic planning over strictly ascending codes.
fn plan<W: Word>(words: &[W], dense_bytes: u128, budget: Budget) -> IntSetBackend {
    if words.is_empty() {
        return IntSetBackend::Sorted;
    }
    let dense_ok = dense_bytes <= budget.dense_limit() as u128;
    let sorted_score = words.len() as u128 * size_of::<W>() as u128 + 16;
    let mut backend = IntSetBackend::Sorted;
    let mut score = sorted_score;
    if dense_ok && dense_bytes + 24 < score {
        backend = IntSetBackend::Dense;
        score = dense_bytes + 24;
    }
    let survey = survey(words);
    let allowance = if backend == IntSetBackend::Dense {
        DENSE_PREFERENCE_BYTES
    } else {
        0
    };
    if survey.partitions <= budget.roaring_partitions as u64
        && survey.roaring_score + allowance < score
    {
        backend = IntSetBackend::Roaring;
    }
    backend
}

/// Compare scanning sorted codes with repeated binary searches into them.
/// `bit_length(n)` equals ceil(log2(n + 1)) for n > 0, without overflowing at
/// `u64::MAX`. Dense iteration also scans zero words, so its word count
/// contributes to the estimated traversal work even for few members.
fn prefer_sorted_probe(sorted_len: u64, compressed_len: u64, traversal_words: usize) -> bool {
    let comparisons = sorted_len.bit_width();
    u128::from(sorted_len)
        <= u128::from(compressed_len) * u128::from(comparisons) + traversal_words as u128
}

/// How to combine a dense window with a Roaring set, in word-cost units.
///
/// Promoting the window costs about two units per word plus the native kernel.
/// Iterating the window and probing Roaring costs one unit per word plus
/// [`PROBE_WORDS`] per member; iterating Roaring and probing the window costs
/// about [`ITERATE_WORDS`] per Roaring member. These are work estimates, not
/// cycle counts; the direction tests in the suite pin the resulting answers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DenseRoaring {
    Promote,
    IterateDense,
    IterateRoaring,
}

const ITERATE_WORDS: u64 = 5;

fn dense_roaring_strategy(words: u64, dense_members: u64, roaring_members: u64) -> DenseRoaring {
    let promote = words.saturating_mul(2);
    let iterate_dense = words.saturating_add(dense_members.saturating_mul(PROBE_WORDS));
    let iterate_roaring = roaring_members.saturating_mul(ITERATE_WORDS);
    if promote <= iterate_dense && promote <= iterate_roaring {
        DenseRoaring::Promote
    } else if iterate_dense <= iterate_roaring {
        DenseRoaring::IterateDense
    } else {
        DenseRoaring::IterateRoaring
    }
}

/// Cardinality of whichever operand is the dense one.
fn dense_cardinality<W: Word>(left: &Storage<W>, right: &Storage<W>) -> u64 {
    if matches!(left.repr, Repr::Dense(_)) {
        left.cardinality
    } else {
        right.cardinality
    }
}

/// The part of a sorted slice that can fall inside a dense window. Tiny slices
/// are returned whole: two binary searches would cost more than they save.
fn window_slice<'a, W: Word>(sorted: &'a [W], dense: &Dense<W>) -> &'a [W] {
    if dense.is_empty() {
        return &[];
    }
    if sorted.len() <= 128 {
        return sorted;
    }
    let start = sorted.partition_point(|word| word.to_u64() < dense.window_start());
    let end = sorted.partition_point(|word| word.to_u64() <= dense.window_end());
    &sorted[start..end.max(start)]
}

fn probe_count<W>(words: impl Iterator<Item = W>, mut contains: impl FnMut(W) -> bool) -> u64 {
    words.fold(0, |count, word| count + u64::from(contains(word)))
}

/// Whether a dense result window of `window_bytes` is an acceptable output for
/// at most `members` codes: within the limit and competitive with sorted storage.
fn accept_dense_result<W: Word>(window_bytes: u128, members: u64, limit: usize) -> bool {
    window_bytes <= limit as u128
        && window_bytes + 24 <= u128::from(members) * size_of::<W>() as u128 + 16
}

fn merge<W: Word>(
    left: impl Iterator<Item = W>,
    right: impl Iterator<Item = W>,
    op: Operation,
    capacity: usize,
) -> Vec<W> {
    let mut left = left.peekable();
    let mut right = right.peekable();
    let mut out = Vec::with_capacity(capacity);
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

fn dense_sorted<W: Word>(
    dense: &Dense<W>,
    dense_count: u64,
    sorted: &[W],
    op: Operation,
    dense_left: bool,
    budget: Budget,
) -> Combined<W> {
    let sub = window_slice(sorted, dense);
    let limit = budget.dense_limit();
    match op {
        // Dense membership is a single load, so the intersection probes the
        // sorted slice into the window (or searches the slice for each dense
        // member when the window is far sparser). The result is bounded by the
        // smaller operand and stays sorted.
        Operation::Intersection => {
            Combined::Stored(Storage::sorted(sorted_compressed_intersection(
                sub,
                dense.iter(),
                |word| dense.contains(word),
                dense_count,
                dense.words.len(),
            )))
        }
        Operation::Difference if dense_left => Combined::Stored(Storage::dense(
            dense.without_members(sub.iter().map(|word| word.to_u64())),
        )),
        Operation::Difference => Combined::Stored(Storage::sorted(
            sorted
                .iter()
                .copied()
                .filter(|&word| !dense.contains(word))
                .collect(),
        )),
        Operation::Union | Operation::SymmetricDifference => {
            let (low, high) = (
                sorted[0].to_u64().min(dense.window_start()),
                sorted[sorted.len() - 1].to_u64().max(dense.window_end()),
            );
            let window_bytes = dense_bytes(low, high);
            let members = dense_count + sorted.len() as u64;
            if accept_dense_result::<W>(window_bytes, members, limit)
                && let Some(value) = dense.with_members(
                    sorted.iter().map(|word| word.to_u64()),
                    sorted[0].to_u64(),
                    sorted[sorted.len() - 1].to_u64(),
                    matches!(op, Operation::SymmetricDifference),
                    limit,
                )
            {
                return Combined::Stored(Storage::dense(value));
            }
            let promoted_dense = W::roaring_from_dense(dense.base, &dense.words);
            let promoted_sorted = W::roaring_from_sorted(sorted);
            Combined::Stored(Storage::roaring(if dense_left {
                W::roaring_binary(&promoted_dense, &promoted_sorted, op)
            } else {
                W::roaring_binary(&promoted_sorted, &promoted_dense, op)
            }))
        }
    }
}

fn dense_roaring<W: Word>(
    dense: &Dense<W>,
    dense_count: u64,
    roaring: &W::Roaring,
    op: Operation,
    dense_left: bool,
    budget: Budget,
) -> Storage<W> {
    let roaring_len = W::roaring_len(roaring);
    let strategy = dense_roaring_strategy(dense.words.len() as u64, dense_count, roaring_len);
    let native = |op: Operation| {
        let promoted = W::roaring_from_dense(dense.base, &dense.words);
        Storage::roaring(if dense_left {
            W::roaring_binary(&promoted, roaring, op)
        } else {
            W::roaring_binary(roaring, &promoted, op)
        })
    };
    match op {
        Operation::Intersection => match strategy {
            DenseRoaring::Promote => native(op),
            DenseRoaring::IterateDense => Storage::sorted(
                dense
                    .iter()
                    .filter(|&word| W::roaring_contains(roaring, word))
                    .collect(),
            ),
            DenseRoaring::IterateRoaring => Storage::sorted(
                W::roaring_iter(roaring)
                    .filter(|&word| dense.contains(word))
                    .collect(),
            ),
        },
        Operation::Difference if dense_left => {
            if strategy == DenseRoaring::IterateRoaring {
                Storage::dense(dense.without_members(W::roaring_iter(roaring).map(Word::to_u64)))
            } else {
                native(op)
            }
        }
        Operation::Difference => {
            if dense_count.saturating_mul(8) <= roaring_len {
                let mut result = roaring.clone();
                for word in dense.iter() {
                    W::roaring_remove(&mut result, word);
                }
                Storage::roaring(result)
            } else {
                native(op)
            }
        }
        Operation::Union | Operation::SymmetricDifference => {
            if strategy == DenseRoaring::IterateRoaring
                && let Some((low, high)) = W::roaring_bounds(roaring)
            {
                let limit = budget.dense_limit();
                let window_bytes = dense_bytes(
                    low.to_u64().min(dense.window_start()),
                    high.to_u64().max(dense.window_end()),
                );
                if accept_dense_result::<W>(window_bytes, dense_count + roaring_len, limit)
                    && let Some(value) = dense.with_members(
                        W::roaring_iter(roaring).map(Word::to_u64),
                        low.to_u64(),
                        high.to_u64(),
                        matches!(op, Operation::SymmetricDifference),
                        limit,
                    )
                {
                    return Storage::dense(value);
                }
            }
            native(op)
        }
    }
}

fn sorted_roaring<W: Word>(
    sorted: &[W],
    roaring: &W::Roaring,
    op: Operation,
    sorted_left: bool,
) -> Combined<W> {
    let sorted_len = sorted.len() as u64;
    let roaring_len = W::roaring_len(roaring);
    // Copying a scattered container tree costs more than merging its codes,
    // and the planner would not keep such a set in Roaring either. Counting
    // containers is O(containers), so only the paths that copy ask.
    let well_filled =
        || roaring_len >= W::roaring_containers(roaring).saturating_mul(WELL_FILLED_MEMBERS);
    let small_sorted = sorted_len <= SMALL_SORTED_UPDATES;
    let roaring_dominant = sorted_len <= roaring_len.saturating_mul(4);
    let native = |op: Operation| {
        let promoted = W::roaring_from_sorted(sorted);
        Storage::roaring(if sorted_left {
            W::roaring_binary(&promoted, roaring, op)
        } else {
            W::roaring_binary(roaring, &promoted, op)
        })
    };
    let merged = |op: Operation| {
        let capacity = usize::try_from(sorted_len + roaring_len).unwrap_or(0);
        Combined::Codes(if sorted_left {
            merge(
                sorted.iter().copied(),
                W::roaring_iter(roaring),
                op,
                capacity,
            )
        } else {
            merge(
                W::roaring_iter(roaring),
                sorted.iter().copied(),
                op,
                capacity,
            )
        })
    };
    match op {
        Operation::Intersection => {
            Combined::Stored(Storage::sorted(sorted_compressed_intersection(
                sorted,
                W::roaring_iter(roaring),
                |word| W::roaring_contains(roaring, word),
                roaring_len,
                0,
            )))
        }
        Operation::Difference if sorted_left => Combined::Stored(Storage::sorted(
            sorted
                .iter()
                .copied()
                .filter(|&word| !W::roaring_contains(roaring, word))
                .collect(),
        )),
        Operation::Difference => {
            if !well_filled() {
                merged(op)
            } else if small_sorted {
                let mut result = roaring.clone();
                for &word in sorted {
                    W::roaring_remove(&mut result, word);
                }
                Combined::Stored(Storage::roaring(result))
            } else {
                Combined::Stored(native(op))
            }
        }
        Operation::Union | Operation::SymmetricDifference => {
            if !well_filled() {
                merged(op)
            } else if small_sorted {
                let mut result = roaring.clone();
                for &word in sorted {
                    // A symmetric difference toggles: remove when present.
                    let toggled_off = matches!(op, Operation::SymmetricDifference)
                        && W::roaring_remove(&mut result, word);
                    if !toggled_off {
                        W::roaring_insert(&mut result, word);
                    }
                }
                Combined::Stored(Storage::roaring(result))
            } else if roaring_dominant {
                Combined::Stored(native(op))
            } else {
                merged(op)
            }
        }
    }
}

fn chunked_sorted<W: Word>(
    chunks: &Chunked<W>,
    sorted: &[W],
    op: Operation,
    chunked_left: bool,
) -> Storage<W> {
    let chunked_len = chunks.cardinality();
    if (sorted.len() as u64) <= chunked_len.saturating_mul(4) {
        let promoted = Chunked::from_sorted(sorted);
        return Storage::chunked(if chunked_left {
            chunks.binary(&promoted, op)
        } else {
            promoted.binary(chunks, op)
        });
    }
    // A dominant sorted operand is probed, never promoted; results that are
    // subsets of the chunked operand stay chunked.
    match op {
        Operation::Intersection => Storage::chunked(Chunked::from_ascending(
            chunks
                .iter()
                .filter(|word| sorted.binary_search(word).is_ok())
                .map(Word::to_u64),
        )),
        Operation::Difference if chunked_left => Storage::chunked(Chunked::from_ascending(
            chunks
                .iter()
                .filter(|word| sorted.binary_search(word).is_err())
                .map(Word::to_u64),
        )),
        Operation::Difference => Storage::sorted(
            sorted
                .iter()
                .copied()
                .filter(|&word| !chunks.contains(word))
                .collect(),
        ),
        Operation::Union | Operation::SymmetricDifference => {
            let capacity = usize::try_from(sorted.len() as u64 + chunked_len).unwrap_or(0);
            Storage::sorted(if chunked_left {
                merge(chunks.iter(), sorted.iter().copied(), op, capacity)
            } else {
                merge(sorted.iter().copied(), chunks.iter(), op, capacity)
            })
        }
    }
}

fn chunked_roaring<W: Word>(
    chunks: &Chunked<W>,
    roaring: &W::Roaring,
    op: Operation,
    chunked_left: bool,
) -> Storage<W> {
    if W::roaring_len(roaring) <= chunks.cardinality().saturating_mul(4) {
        let promoted = Chunked::from_ascending(W::roaring_iter(roaring).map(Word::to_u64));
        Storage::chunked(if chunked_left {
            chunks.binary(&promoted, op)
        } else {
            promoted.binary(chunks, op)
        })
    } else {
        let promoted = W::roaring_from_ascending(chunks.iter());
        Storage::roaring(if chunked_left {
            W::roaring_binary(&promoted, roaring, op)
        } else {
            W::roaring_binary(roaring, &promoted, op)
        })
    }
}

#[derive(Clone)]
enum WordIter<'a, W: Word> {
    Sorted(core::iter::Copied<core::slice::Iter<'a, W>>),
    Dense(DenseIter<'a, W>),
    Roaring(W::RoaringIter<'a>),
    Chunked(ChunkedIter<'a, W>),
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
            WordIter::Chunked(iter) => iter.next(),
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
    /// Dispatch once per traversal rather than once per element.
    fn fold<B, F: FnMut(B, W) -> B>(self, init: B, f: F) -> B {
        if self.remaining == 0 {
            return init;
        }
        match self.inner {
            WordIter::Sorted(iter) => iter.fold(init, f),
            WordIter::Dense(iter) => iter.fold(init, f),
            WordIter::Roaring(iter) => iter.fold(init, f),
            WordIter::Chunked(iter) => iter.fold(init, f),
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
                IntSetPolicy::chunked(),
            ] {
                let storage = Storage::from_sorted(values.clone(), policy).unwrap();
                let mut events = HashEvents::default();
                storage.hash_into(&mut events);
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
                IntSetPolicy::chunked(),
            ] {
                let storage = Storage::from_sorted(values.clone(), policy).unwrap();
                let mut events = HashEvents::default();
                storage.hash_into(&mut events);
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
            IntSetPolicy::chunked(),
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

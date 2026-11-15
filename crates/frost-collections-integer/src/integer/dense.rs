//! Aligned dense bitmap windows and their word kernels.
//!
//! A window stores absolute 64-bit blocks starting at `base / 64`. Two windows
//! are combined by absolute block number; equal local indices never imply equal
//! members. Kernels operate on three slice regions (left-only prefix, overlap,
//! right-only suffix) so the compiler can vectorize them. Storage computes the
//! result cardinality from the normalized words after the kernel completes.

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::ops::ControlFlow;

use super::storage::{Operation, Word};

/// Payload bytes of the aligned window covering `[low, high]`, computed widely.
pub(crate) fn dense_bytes(low: u64, high: u64) -> u128 {
    let base = low & !63;
    ((u128::from(high) - u128::from(base)) / 64 + 1) * 8
}

#[derive(Clone)]
pub(crate) struct Dense<W: Word> {
    pub(crate) base: u64,
    pub(crate) words: Box<[u64]>,
    word: PhantomData<W>,
}

impl<W: Word> PartialEq for Dense<W> {
    /// Windows are always trimmed to their first and last set bit, so equal
    /// member sets have identical bases and words.
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base && self.words == other.words
    }
}

impl<W: Word> Dense<W> {
    pub(crate) fn empty() -> Self {
        Self {
            base: 0,
            words: Box::new([]),
            word: PhantomData,
        }
    }

    /// Caller supplies strictly ascending codes whose aligned span the planner
    /// has already checked against the allocation bound.
    pub(crate) fn from_sorted(values: &[W]) -> Self {
        let Some(&first) = values.first() else {
            return Self::empty();
        };
        let base = first.to_u64() & !63;
        let last = values.last().expect("nonempty input").to_u64();
        let count = usize::try_from(dense_bytes(base, last) / 8)
            .expect("planner checked addressable dense span");
        let mut words = vec![0_u64; count];
        for &value in values {
            set_bit(&mut words, base, value.to_u64());
        }
        Self::from_words(base, words)
    }

    /// Allocate an empty window whose bounds the planner has already checked.
    /// Callers fill the first and last set bits with `insert_run` before using
    /// operations that rely on normalized windows.
    pub(crate) fn from_bounds(bounds: Option<(W, W)>) -> Self {
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

    pub(crate) fn expanded(&self, bounds: Option<(W, W)>) -> Self {
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

    pub(crate) fn set(&mut self, member: W, present: bool) {
        let offset = member.to_u64() - self.base;
        let index = usize::try_from(offset / 64).expect("member lies in the allocated window");
        let mask = 1_u64 << (offset % 64);
        if present {
            self.words[index] |= mask;
        } else {
            self.words[index] &= !mask;
        }
    }

    pub(crate) fn trim_to_bounds(&mut self, bounds: Option<(W, W)>) {
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

    /// Insert an inclusive run of existing codes within the allocated window.
    /// Conversion must not fill gaps in the key codec's logical domain.
    pub(crate) fn insert_run(&mut self, start: u64, end: u64) {
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

    pub(crate) fn visit_runs(&self, mut visit: impl FnMut(u64, u64)) {
        let _: ControlFlow<()> = self.try_visit_runs(|start, end| {
            visit(start, end);
            ControlFlow::Continue(())
        });
    }

    pub(crate) fn try_visit_runs<B>(
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

    pub(crate) fn partition_count(&self) -> u64 {
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

    /// Build a trimmed window from raw words; `base` must be a multiple of 64.
    pub(crate) fn from_words(base: u64, mut words: Vec<u64>) -> Self {
        debug_assert_eq!(base % 64, 0);
        let Some(first) = words.iter().position(|&word| word != 0) else {
            return Self::empty();
        };
        let last = words
            .iter()
            .rposition(|&word| word != 0)
            .expect("nonempty result");
        words.truncate(last + 1);
        if first != 0 {
            words.drain(..first);
        }
        Self {
            base: base + first as u64 * 64,
            words: words.into_boxed_slice(),
            word: PhantomData,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
    pub(crate) fn start_block(&self) -> u64 {
        self.base / 64
    }
    pub(crate) fn end_block(&self) -> u64 {
        self.start_block() + self.words.len() as u64
    }
    /// First code not below the window (inclusive lower coordinate).
    pub(crate) fn window_start(&self) -> u64 {
        self.base
    }
    /// Last code inside the window (inclusive upper coordinate).
    pub(crate) fn window_end(&self) -> u64 {
        if self.is_empty() {
            return self.base;
        }
        // The inclusive last block coordinate fits even when the exclusive
        // end of the full u64 universe is 2^64.
        (self.end_block() - 1) * 64 + 63
    }
    pub(crate) fn payload_bytes(&self) -> usize {
        self.words.len() * 8
    }

    pub(crate) fn contains(&self, member: W) -> bool {
        let Some(offset) = member.to_u64().checked_sub(self.base) else {
            return false;
        };
        let Ok(index) = usize::try_from(offset >> 6) else {
            return false;
        };
        self.words
            .get(index)
            .is_some_and(|word| word & (1_u64 << (offset & 63)) != 0)
    }

    pub(crate) fn cardinality(&self) -> u64 {
        popcount(&self.words)
    }

    pub(crate) fn iter(&self) -> DenseIter<'_, W> {
        DenseIter {
            dense: self,
            index: 0,
            pending: 0,
        }
    }

    pub(crate) fn bounds(&self) -> Option<(W, W)> {
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
        Some((W::from_u64(low), W::from_u64(high)))
    }

    /// Overlapping word ranges of two windows, in absolute block coordinates.
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

    pub(crate) fn intersection_len(&self, other: &Self) -> u64 {
        let (left, right) = self.overlap(other);
        left.iter()
            .zip(right)
            .map(|(a, b)| u64::from((a & b).count_ones()))
            .sum()
    }
    pub(crate) fn is_disjoint(&self, other: &Self) -> bool {
        let (left, right) = self.overlap(other);
        left.iter().zip(right).all(|(a, b)| a & b == 0)
    }
    /// Every set bit of `self` is set in `other`. Windows are trimmed, so a
    /// window reaching outside the other one has a set bit outside it.
    pub(crate) fn is_subset(&self, other: &Self) -> bool {
        if self.is_empty() {
            return true;
        }
        if self.start_block() < other.start_block() || self.end_block() > other.end_block() {
            return false;
        }
        let (left, right) = self.overlap(other);
        left.iter().zip(right).all(|(a, b)| a & !b == 0)
    }

    /// Combine two windows into a window over `[start, end)` blocks.
    ///
    /// `None` means the result window exceeds `limit_bytes` or the target's
    /// allocation bound; callers choose a different representation then.
    pub(crate) fn binary(&self, other: &Self, op: Operation, limit_bytes: usize) -> Option<Self> {
        let (start, end) = match op {
            Operation::Union | Operation::SymmetricDifference => {
                if self.is_empty() {
                    return Some(other.clone());
                }
                if other.is_empty() {
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
        if bytes > limit_bytes as u128 || bytes > isize::MAX as u128 {
            return None;
        }
        let count = usize::try_from(end - start).expect("bounded output window");
        let mut words = vec![0_u64; count];
        let left = self.region(start, end);
        let right = other.region(start, end);
        kernel(&mut words, left, right, op);
        Some(Self::from_words(start * 64, words))
    }

    /// Where this window's words land inside a result window over `[start, end)`.
    fn region(&self, start: u64, end: u64) -> Region<'_> {
        let from = self.start_block().max(start);
        let to = self.end_block().min(end);
        if from >= to {
            return Region {
                offset: 0,
                words: &[],
            };
        }
        let a = usize::try_from(from - self.start_block()).expect("region index");
        let count = usize::try_from(to - from).expect("region length");
        Region {
            offset: usize::try_from(from - start).expect("region offset"),
            words: &self.words[a..a + count],
        }
    }

    /// Clone the window and set the given ascending codes, growing the window
    /// to the aligned span that covers them. `None` if that span exceeds
    /// `limit_bytes`.
    pub(crate) fn with_members(
        &self,
        members: impl Iterator<Item = u64> + Clone,
        low: u64,
        high: u64,
        toggle: bool,
        limit_bytes: usize,
    ) -> Option<Self> {
        let (base, mut words) = self.grown(low, high, limit_bytes)?;
        for member in members {
            if toggle {
                toggle_bit(&mut words, base, member);
            } else {
                set_bit(&mut words, base, member);
            }
        }
        Some(Self::from_words(base, words))
    }

    /// Clone the window and clear the given codes; codes outside the window are
    /// ignored because they were never members.
    pub(crate) fn without_members(&self, members: impl Iterator<Item = u64>) -> Self {
        if self.is_empty() {
            return Self::empty();
        }
        let mut words = self.words.to_vec();
        let end = self.window_end();
        for member in members {
            if member >= self.base && member <= end {
                clear_bit(&mut words, self.base, member);
            }
        }
        Self::from_words(self.base, words)
    }

    /// Copy the words into a window covering this one and `[low, high]`.
    fn grown(&self, low: u64, high: u64, limit_bytes: usize) -> Option<(u64, Vec<u64>)> {
        if self.is_empty() {
            let base = low & !63;
            let bytes = dense_bytes(low, high);
            if bytes > limit_bytes as u128 || bytes > isize::MAX as u128 {
                return None;
            }
            return Some((base, vec![0_u64; usize::try_from(bytes / 8).ok()?]));
        }
        let base = (low & !63).min(self.base);
        let top = high.max(self.window_end());
        let bytes = dense_bytes(base, top);
        if bytes > limit_bytes as u128 || bytes > isize::MAX as u128 {
            return None;
        }
        let mut words = vec![0_u64; usize::try_from(bytes / 8).ok()?];
        let offset = usize::try_from((self.base - base) / 64).ok()?;
        words[offset..offset + self.words.len()].copy_from_slice(&self.words);
        Some((base, words))
    }
}

#[derive(Clone, Copy)]
struct Region<'a> {
    offset: usize,
    words: &'a [u64],
}

/// Three-region kernel writing `op(left, right)` into `out`, which starts zeroed.
fn kernel(out: &mut [u64], left: Region<'_>, right: Region<'_>, op: Operation) {
    match op {
        Operation::Intersection => {
            // Both regions share the overlap coordinates by construction.
            let start = left.offset.max(right.offset);
            let end = (left.offset + left.words.len()).min(right.offset + right.words.len());
            if start < end {
                let a = &left.words[start - left.offset..end - left.offset];
                let b = &right.words[start - right.offset..end - right.offset];
                for ((o, x), y) in out[start..end].iter_mut().zip(a).zip(b) {
                    *o = x & y;
                }
            }
        }
        Operation::Difference => {
            out[left.offset..left.offset + left.words.len()].copy_from_slice(left.words);
            let start = left.offset.max(right.offset);
            let end = (left.offset + left.words.len()).min(right.offset + right.words.len());
            if start < end {
                let b = &right.words[start - right.offset..end - right.offset];
                for (o, y) in out[start..end].iter_mut().zip(b) {
                    *o &= !y;
                }
            }
        }
        Operation::Union => {
            out[left.offset..left.offset + left.words.len()].copy_from_slice(left.words);
            for (o, y) in out[right.offset..right.offset + right.words.len()]
                .iter_mut()
                .zip(right.words)
            {
                *o |= y;
            }
        }
        Operation::SymmetricDifference => {
            out[left.offset..left.offset + left.words.len()].copy_from_slice(left.words);
            for (o, y) in out[right.offset..right.offset + right.words.len()]
                .iter_mut()
                .zip(right.words)
            {
                *o ^= y;
            }
        }
    }
}

pub(crate) fn popcount(words: &[u64]) -> u64 {
    words.iter().map(|word| u64::from(word.count_ones())).sum()
}

fn set_bit(words: &mut [u64], base: u64, value: u64) {
    let offset = value - base;
    words[usize::try_from(offset >> 6).expect("offset in allocated dense span")] |=
        1_u64 << (offset & 63);
}
fn clear_bit(words: &mut [u64], base: u64, value: u64) {
    let offset = value - base;
    words[usize::try_from(offset >> 6).expect("offset in allocated dense span")] &=
        !(1_u64 << (offset & 63));
}
fn toggle_bit(words: &mut [u64], base: u64, value: u64) {
    let offset = value - base;
    words[usize::try_from(offset >> 6).expect("offset in allocated dense span")] ^=
        1_u64 << (offset & 63);
}

#[derive(Clone)]
pub(crate) struct DenseIter<'a, W: Word> {
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
                return Some(W::from_u64(code));
            }
            self.pending = *self.dense.words.get(self.index)?;
            self.index += 1;
        }
    }
}

impl<W: Word> FusedIterator for DenseIter<'_, W> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleting_from_an_empty_window_is_a_no_op() {
        let empty = Dense::<u64>::empty().without_members([0, u64::MAX].into_iter());
        assert!(empty.is_empty());
        assert_eq!(empty.cardinality(), 0);
    }

    #[test]
    fn dense_runs_preserve_word_edges_and_the_maximum_code() {
        let base = u64::MAX - 255;
        let members: Vec<_> = (base..=u64::MAX).filter(|code| code % 13 != 0).collect();
        let source = Dense::from_sorted(&members);
        let mut rebuilt = Dense::from_bounds(source.bounds());
        source.visit_runs(|start, end| rebuilt.insert_run(start, end));
        assert_eq!(rebuilt.iter().collect::<Vec<_>>(), members);
        assert_eq!(rebuilt.partition_count(), 1);
        assert_eq!(rebuilt.window_end(), u64::MAX);
        let mut visited = 0;
        let stopped = source.try_visit_runs(|_, _| {
            visited += 1;
            ControlFlow::Break(())
        });
        assert_eq!(stopped, ControlFlow::Break(()));
        assert_eq!(visited, 1);
    }
}

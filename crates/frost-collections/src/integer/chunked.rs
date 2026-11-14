//! Chunked storage with structurally shared containers.
//!
//! Codes are split into a high key (`code >> 16`) and a 16-bit low part. Each
//! key owns one immutable container behind an `Arc`, so algebra results share
//! every untouched container with their operands and a repeated
//! `snapshot.union(&small)` copies one container plus the key index rather
//! than the whole set. Containers whose `Arc`s are the same allocation are
//! combined without reading them (`A ∪ A = A`, `A ∖ A = ∅`).
//!
//! Containers are `Array` (≤ 4,096 sorted low parts), `Bitmap` (1,024 words
//! with a cached count), or `Full` (all 65,536 members, no payload).

// Low parts, word indices, bit offsets and container counts are bounded by the
// 2^16 span, so narrowing casts in this module cannot truncate.
#![allow(clippy::cast_possible_truncation)]

use super::Shared as Arc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::ops::ControlFlow;

use super::storage::{Operation, Word};
use frost_collections_core::__private::set_ops;

const LOW_BITS: u32 = 16;
const SPAN: u32 = 1 << LOW_BITS;
const ARRAY_MAX: usize = 4096;
const WORDS: usize = 1024;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Container {
    /// Strictly ascending low parts; never empty, at most [`ARRAY_MAX`].
    Array(Box<[u16]>),
    /// Bitmap words with the cached cardinality, strictly between the array
    /// limit and the full span.
    Bitmap(Box<[u64; WORDS]>, u32),
    /// Every low part is present.
    Full,
}

impl Container {
    fn cardinality(&self) -> u32 {
        match self {
            Self::Array(lows) => lows.len() as u32,
            Self::Bitmap(_, count) => *count,
            Self::Full => SPAN,
        }
    }

    fn contains(&self, low: u16) -> bool {
        match self {
            Self::Array(lows) => lows.binary_search(&low).is_ok(),
            Self::Bitmap(words, _) => words[usize::from(low >> 6)] & (1_u64 << (low & 63)) != 0,
            Self::Full => true,
        }
    }

    fn min(&self) -> u16 {
        match self {
            Self::Array(lows) => lows[0],
            Self::Bitmap(words, _) => {
                let (index, word) = words
                    .iter()
                    .enumerate()
                    .find(|(_, word)| **word != 0)
                    .expect("bitmap container is never empty");
                (index as u16) * 64 + word.trailing_zeros() as u16
            }
            Self::Full => 0,
        }
    }

    fn max(&self) -> u16 {
        match self {
            Self::Array(lows) => lows[lows.len() - 1],
            Self::Bitmap(words, _) => {
                let (index, word) = words
                    .iter()
                    .enumerate()
                    .rfind(|(_, word)| **word != 0)
                    .expect("bitmap container is never empty");
                (index as u16) * 64 + (63 - word.leading_zeros()) as u16
            }
            Self::Full => u16::MAX,
        }
    }

    /// Build from strictly ascending low parts.
    fn from_lows(lows: Vec<u16>) -> Option<Self> {
        if lows.is_empty() {
            return None;
        }
        if lows.len() == SPAN as usize {
            return Some(Self::Full);
        }
        if lows.len() <= ARRAY_MAX {
            return Some(Self::Array(lows.into_boxed_slice()));
        }
        let mut words = Box::new([0_u64; WORDS]);
        for low in &lows {
            words[usize::from(low >> 6)] |= 1_u64 << (low & 63);
        }
        Some(Self::Bitmap(words, lows.len() as u32))
    }

    /// Normalize raw words with a known count into the canonical container.
    fn from_words(words: Box<[u64; WORDS]>, count: u32) -> Option<Self> {
        match count {
            0 => None,
            SPAN => Some(Self::Full),
            count if count as usize <= ARRAY_MAX => {
                let mut lows = Vec::with_capacity(count as usize);
                for (index, word) in words.iter().enumerate() {
                    let mut word = *word;
                    while word != 0 {
                        lows.push((index as u16) * 64 + word.trailing_zeros() as u16);
                        word &= word - 1;
                    }
                }
                Some(Self::Array(lows.into_boxed_slice()))
            }
            count => Some(Self::Bitmap(words, count)),
        }
    }

    fn complement(&self) -> Option<Self> {
        match self {
            Self::Full => None,
            Self::Array(lows) => {
                let mut words = Box::new([u64::MAX; WORDS]);
                for low in lows {
                    words[usize::from(low >> 6)] &= !(1_u64 << (low & 63));
                }
                Self::from_words(words, SPAN - lows.len() as u32)
            }
            Self::Bitmap(words, count) => {
                let mut inverted = words.clone();
                for word in &mut *inverted {
                    *word = !*word;
                }
                Self::from_words(inverted, SPAN - count)
            }
        }
    }

    fn binary(&self, other: &Self, op: Operation) -> Option<Self> {
        use Container::{Array, Bitmap, Full};
        match (self, other, op) {
            (Full, Full, Operation::Union | Operation::Intersection) => Some(Full),
            (Full, Full, _) => None,
            (Full, _, Operation::Union) | (_, Full, Operation::Union) => Some(Full),
            (Full, x, Operation::Intersection) | (x, Full, Operation::Intersection) => {
                Some(x.clone_container())
            }
            (Full, x, Operation::Difference | Operation::SymmetricDifference)
            | (x, Full, Operation::SymmetricDifference) => x.complement(),
            (_, Full, Operation::Difference) => None,
            (Array(a), Array(b), op) => {
                let lows: Vec<u16> = match op {
                    Operation::Union => set_ops::union(&a[..], &b[..]).copied().collect(),
                    Operation::Intersection => {
                        set_ops::intersection(&a[..], &b[..]).copied().collect()
                    }
                    Operation::Difference => set_ops::difference(&a[..], &b[..]).copied().collect(),
                    Operation::SymmetricDifference => set_ops::symmetric_difference(&a[..], &b[..])
                        .copied()
                        .collect(),
                };
                Self::from_lows(lows)
            }
            (Array(a), Bitmap(words, _), Operation::Intersection)
            | (Bitmap(words, _), Array(a), Operation::Intersection) => {
                let lows: Vec<u16> = a
                    .iter()
                    .copied()
                    .filter(|low| words[usize::from(low >> 6)] & (1_u64 << (low & 63)) != 0)
                    .collect();
                Self::from_lows(lows)
            }
            (Array(a), Bitmap(words, count), Operation::Union)
            | (Bitmap(words, count), Array(a), Operation::Union) => {
                let mut out = words.clone();
                let mut added = 0_u32;
                for low in a {
                    let slot = &mut out[usize::from(low >> 6)];
                    let bit = 1_u64 << (low & 63);
                    added += u32::from(*slot & bit == 0);
                    *slot |= bit;
                }
                Self::from_words(out, count + added)
            }
            (Array(a), Bitmap(words, _), Operation::Difference) => {
                let lows: Vec<u16> = a
                    .iter()
                    .copied()
                    .filter(|low| words[usize::from(low >> 6)] & (1_u64 << (low & 63)) == 0)
                    .collect();
                Self::from_lows(lows)
            }
            (Bitmap(words, count), Array(a), Operation::Difference) => {
                let mut out = words.clone();
                let mut removed = 0_u32;
                for low in a {
                    let slot = &mut out[usize::from(low >> 6)];
                    let bit = 1_u64 << (low & 63);
                    removed += u32::from(*slot & bit != 0);
                    *slot &= !bit;
                }
                Self::from_words(out, count - removed)
            }
            (Array(a), Bitmap(words, count), Operation::SymmetricDifference)
            | (Bitmap(words, count), Array(a), Operation::SymmetricDifference) => {
                let mut out = words.clone();
                let mut delta = 0_i64;
                for low in a {
                    let slot = &mut out[usize::from(low >> 6)];
                    let bit = 1_u64 << (low & 63);
                    delta += if *slot & bit == 0 { 1 } else { -1 };
                    *slot ^= bit;
                }
                let count = u32::try_from(i64::from(*count) + delta).expect("count in span");
                Self::from_words(out, count)
            }
            (Bitmap(a, _), Bitmap(b, _), op) => {
                let mut out = Box::new([0_u64; WORDS]);
                let mut count = 0_u32;
                for ((o, x), y) in out.iter_mut().zip(a.iter()).zip(b.iter()) {
                    *o = match op {
                        Operation::Union => x | y,
                        Operation::Intersection => x & y,
                        Operation::Difference => x & !y,
                        Operation::SymmetricDifference => x ^ y,
                    };
                    count += o.count_ones();
                }
                Self::from_words(out, count)
            }
        }
    }

    fn clone_container(&self) -> Self {
        match self {
            Self::Array(lows) => Self::Array(lows.clone()),
            Self::Bitmap(words, count) => Self::Bitmap(words.clone(), *count),
            Self::Full => Self::Full,
        }
    }

    fn intersection_len(&self, other: &Self) -> u64 {
        use Container::{Array, Bitmap, Full};
        match (self, other) {
            (Full, x) | (x, Full) => u64::from(x.cardinality()),
            (Array(a), Array(b)) => {
                u64::try_from(set_ops::intersection_len(&a[..], &b[..])).expect("container count")
            }
            (Array(a), Bitmap(words, _)) | (Bitmap(words, _), Array(a)) => {
                a.iter()
                    .filter(|low| words[usize::from(*low >> 6)] & (1_u64 << (*low & 63)) != 0)
                    .count() as u64
            }
            (Bitmap(a, _), Bitmap(b, _)) => a
                .iter()
                .zip(b.iter())
                .map(|(x, y)| u64::from((x & y).count_ones()))
                .sum(),
        }
    }

    fn is_subset(&self, other: &Self) -> bool {
        use Container::{Array, Bitmap, Full};
        match (self, other) {
            (_, Full) => true,
            // A bitmap holds more members than any array can.
            (Full, _) | (Bitmap(..), Array(_)) => false,
            (Array(a), Array(b)) => set_ops::is_subset(&a[..], &b[..]),
            (Array(a), Bitmap(words, _)) => a
                .iter()
                .all(|low| words[usize::from(low >> 6)] & (1_u64 << (low & 63)) != 0),
            (Bitmap(a, ca), Bitmap(b, cb)) => {
                ca <= cb && a.iter().zip(b.iter()).all(|(x, y)| x & !y == 0)
            }
        }
    }

    fn is_disjoint(&self, other: &Self) -> bool {
        use Container::{Array, Bitmap, Full};
        match (self, other) {
            (Full, _) | (_, Full) => false,
            (Array(a), Array(b)) => set_ops::is_disjoint(&a[..], &b[..]),
            (Array(a), Bitmap(words, _)) | (Bitmap(words, _), Array(a)) => a
                .iter()
                .all(|low| words[usize::from(low >> 6)] & (1_u64 << (low & 63)) == 0),
            (Bitmap(a, _), Bitmap(b, _)) => a.iter().zip(b.iter()).all(|(x, y)| x & y == 0),
        }
    }
}

/// Immutable chunk index: sorted high keys and their shared containers.
pub(crate) struct Chunked<W: Word> {
    keys: Box<[u64]>,
    chunks: Box<[Arc<Container>]>,
    word: PhantomData<W>,
}

impl<W: Word> Clone for Chunked<W> {
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.clone(),
            chunks: self.chunks.clone(),
            word: PhantomData,
        }
    }
}

impl<W: Word> PartialEq for Chunked<W> {
    fn eq(&self, other: &Self) -> bool {
        self.keys == other.keys
            && self
                .chunks
                .iter()
                .zip(other.chunks.iter())
                .all(|(a, b)| Arc::ptr_eq(a, b) || **a == **b)
    }
}

struct Builder {
    keys: Vec<u64>,
    chunks: Vec<Arc<Container>>,
}

impl Builder {
    fn new() -> Self {
        Self {
            keys: Vec::new(),
            chunks: Vec::new(),
        }
    }
    fn with_capacity(capacity: usize) -> Self {
        Self {
            keys: Vec::with_capacity(capacity),
            chunks: Vec::with_capacity(capacity),
        }
    }
    fn push(&mut self, key: u64, container: Option<Container>) {
        if let Some(container) = container {
            self.keys.push(key);
            self.chunks.push(Arc::new(container));
        }
    }
    fn share(&mut self, key: u64, container: &Arc<Container>) {
        self.keys.push(key);
        self.chunks.push(Arc::clone(container));
    }
    fn finish<W: Word>(self) -> Chunked<W> {
        Chunked {
            keys: self.keys.into_boxed_slice(),
            chunks: self.chunks.into_boxed_slice(),
            word: PhantomData,
        }
    }
}

impl<W: Word> Chunked<W> {
    pub(crate) fn empty() -> Self {
        Builder::new().finish()
    }

    /// Build from strictly ascending codes.
    pub(crate) fn from_ascending(codes: impl Iterator<Item = u64>) -> Self {
        let mut builder = Builder::new();
        let mut current: Option<u64> = None;
        let mut lows = Vec::new();
        for code in codes {
            let key = code >> LOW_BITS;
            if current != Some(key) {
                if let Some(previous) = current {
                    builder.push(previous, Container::from_lows(core::mem::take(&mut lows)));
                }
                current = Some(key);
            }
            lows.push((code & u64::from(u16::MAX)) as u16);
        }
        if let Some(previous) = current {
            builder.push(previous, Container::from_lows(lows));
        }
        builder.finish()
    }

    pub(crate) fn from_sorted(words: &[W]) -> Self {
        Self::from_ascending(words.iter().map(|word| word.to_u64()))
    }

    /// Promote an aligned dense window (`base` is a multiple of 64).
    pub(crate) fn from_dense(base: u64, words: &[u64]) -> Self {
        if words.is_empty() {
            return Self::empty();
        }
        // Exclusive ends are computed widely: a window may reach `u64::MAX`.
        let end = u128::from(base) + words.len() as u128 * 64;
        let first_key = base >> LOW_BITS;
        let last_key = u64::try_from((end - 1) >> LOW_BITS).expect("key fits");
        let mut builder =
            Builder::with_capacity(usize::try_from(last_key - first_key + 1).unwrap_or(0));
        for key in first_key..=last_key {
            let window_start = u128::from(key) << LOW_BITS;
            let from = window_start.max(u128::from(base));
            let to = (window_start + u128::from(SPAN)).min(end);
            let source = &words[usize::try_from((from - u128::from(base)) / 64)
                .expect("window index")
                ..usize::try_from((to - u128::from(base)) / 64).expect("window index")];
            let offset = usize::try_from((from - window_start) / 64).expect("container offset");
            let mut container = Box::new([0_u64; WORDS]);
            container[offset..offset + source.len()].copy_from_slice(source);
            let count = super::dense::popcount(source);
            builder.push(
                key,
                Container::from_words(container, u32::try_from(count).expect("span count")),
            );
        }
        builder.finish()
    }

    pub(crate) fn chunk_count(&self) -> usize {
        self.keys.len()
    }

    pub(crate) fn cardinality(&self) -> u64 {
        self.chunks
            .iter()
            .map(|chunk| u64::from(chunk.cardinality()))
            .sum()
    }

    pub(crate) fn bounds(&self) -> Option<(W, W)> {
        let first = self.chunks.first()?;
        let last = self.chunks.last()?;
        let low = (self.keys[0] << LOW_BITS) | u64::from(first.min());
        let high = (self.keys[self.keys.len() - 1] << LOW_BITS) | u64::from(last.max());
        Some((W::from_u64(low), W::from_u64(high)))
    }

    /// Visit ascending runs of existing codes without expanding full chunks.
    /// Adjacent pieces may be emitted separately; callers may coalesce them.
    pub(crate) fn try_visit_runs<B>(
        &self,
        mut visit: impl FnMut(u64, u64) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        for (&key, container) in self.keys.iter().zip(&self.chunks) {
            let high = key << LOW_BITS;
            match &**container {
                Container::Array(lows) => {
                    for &low in lows {
                        let code = high | u64::from(low);
                        visit(code, code)?;
                    }
                }
                Container::Bitmap(words, _) => {
                    for (index, &word) in words.iter().enumerate() {
                        let base = high | (index as u64 * 64);
                        let mut pending = word;
                        while pending != 0 {
                            let start = pending.trailing_zeros();
                            let count = (pending >> start).trailing_ones();
                            visit(base + u64::from(start), base + u64::from(start + count - 1))?;
                            pending &= !((u64::MAX >> (64 - count)) << start);
                        }
                    }
                }
                Container::Full => visit(high, high | u64::from(u16::MAX))?,
            }
        }
        ControlFlow::Continue(())
    }

    pub(crate) fn visit_runs(&self, mut visit: impl FnMut(u64, u64)) {
        let _: ControlFlow<()> = self.try_visit_runs(|start, end| {
            visit(start, end);
            ControlFlow::Continue(())
        });
    }

    /// Number of nonempty upper-32-bit partitions, counted from the directory.
    pub(crate) fn partition_count(&self) -> u64 {
        let mut previous = None;
        let mut count = 0;
        for &key in &self.keys {
            let prefix = key >> (32 - LOW_BITS);
            if previous != Some(prefix) {
                count += 1;
                previous = Some(prefix);
            }
        }
        count
    }

    /// Estimated payload bytes: index entries plus container payloads.
    pub(crate) fn payload_bytes(&self) -> u64 {
        self.chunks
            .iter()
            .map(|chunk| match &**chunk {
                Container::Array(lows) => 16 + 2 * lows.len() as u64,
                Container::Bitmap(..) => 16 + 8 * WORDS as u64,
                Container::Full => 16,
            })
            .sum()
    }

    pub(crate) fn contains(&self, member: W) -> bool {
        let code = member.to_u64();
        self.keys
            .binary_search(&(code >> LOW_BITS))
            .is_ok_and(|index| self.chunks[index].contains((code & u64::from(u16::MAX)) as u16))
    }

    pub(crate) fn iter(&self) -> ChunkedIter<'_, W> {
        ChunkedIter {
            set: self,
            chunk: 0,
            cursor: Cursor::Start,
        }
    }

    /// Number of containers physically shared with `other`.
    pub(crate) fn shared_chunks(&self, other: &Self) -> u64 {
        let mut shared = 0;
        let (mut i, mut j) = (0, 0);
        while i < self.keys.len() && j < other.keys.len() {
            match self.keys[i].cmp(&other.keys[j]) {
                Ordering::Less => i += 1,
                Ordering::Greater => j += 1,
                Ordering::Equal => {
                    shared += u64::from(Arc::ptr_eq(&self.chunks[i], &other.chunks[j]));
                    i += 1;
                    j += 1;
                }
            }
        }
        shared
    }

    pub(crate) fn binary(&self, other: &Self, op: Operation) -> Self {
        let mut builder = Builder::with_capacity(match op {
            Operation::Intersection => self.keys.len().min(other.keys.len()),
            Operation::Difference => self.keys.len(),
            Operation::Union | Operation::SymmetricDifference => self.keys.len() + other.keys.len(),
        });
        let (mut i, mut j) = (0, 0);
        while i < self.keys.len() && j < other.keys.len() {
            match self.keys[i].cmp(&other.keys[j]) {
                Ordering::Less => {
                    if !matches!(op, Operation::Intersection) {
                        builder.share(self.keys[i], &self.chunks[i]);
                    }
                    i += 1;
                }
                Ordering::Greater => {
                    if matches!(op, Operation::Union | Operation::SymmetricDifference) {
                        builder.share(other.keys[j], &other.chunks[j]);
                    }
                    j += 1;
                }
                Ordering::Equal => {
                    let key = self.keys[i];
                    let (a, b) = (&self.chunks[i], &other.chunks[j]);
                    i += 1;
                    j += 1;
                    if Arc::ptr_eq(a, b) {
                        if matches!(op, Operation::Union | Operation::Intersection) {
                            builder.share(key, a);
                        }
                        continue;
                    }
                    // A full container decides unions and intersections
                    // without a new allocation.
                    match (op, &**a, &**b) {
                        (Operation::Union, Container::Full, _)
                        | (Operation::Intersection, _, Container::Full) => builder.share(key, a),
                        (Operation::Union, _, Container::Full)
                        | (Operation::Intersection, Container::Full, _) => builder.share(key, b),
                        _ => {
                            if let Some(result) = a.binary(b, op) {
                                let count = result.cardinality();
                                // For these operations equal cardinality proves
                                // equality with that operand. Reuse unchanged
                                // containers even when the inputs are distinct
                                // allocations; no additional member scan is needed.
                                if count == a.cardinality()
                                    && matches!(
                                        op,
                                        Operation::Union
                                            | Operation::Intersection
                                            | Operation::Difference
                                    )
                                {
                                    builder.share(key, a);
                                } else if count == b.cardinality()
                                    && matches!(op, Operation::Union | Operation::Intersection)
                                {
                                    builder.share(key, b);
                                } else {
                                    builder.push(key, Some(result));
                                }
                            }
                        }
                    }
                }
            }
        }
        if !matches!(op, Operation::Intersection) {
            for index in i..self.keys.len() {
                builder.share(self.keys[index], &self.chunks[index]);
            }
        }
        if matches!(op, Operation::Union | Operation::SymmetricDifference) {
            for index in j..other.keys.len() {
                builder.share(other.keys[index], &other.chunks[index]);
            }
        }
        builder.finish()
    }

    pub(crate) fn intersection_len(&self, other: &Self) -> u64 {
        let mut count = 0;
        self.for_each_common(other, |a, b| {
            count += if Arc::ptr_eq(a, b) {
                u64::from(a.cardinality())
            } else {
                a.intersection_len(b)
            };
            true
        });
        count
    }

    pub(crate) fn is_disjoint(&self, other: &Self) -> bool {
        let mut disjoint = true;
        self.for_each_common(other, |a, b| {
            disjoint = !Arc::ptr_eq(a, b) && a.is_disjoint(b);
            disjoint
        });
        disjoint
    }

    pub(crate) fn is_subset(&self, other: &Self) -> bool {
        // Every key of self must exist in other with a container superset.
        let (mut i, mut j) = (0, 0);
        while i < self.keys.len() {
            while j < other.keys.len() && other.keys[j] < self.keys[i] {
                j += 1;
            }
            if j >= other.keys.len() || other.keys[j] != self.keys[i] {
                return false;
            }
            let (a, b) = (&self.chunks[i], &other.chunks[j]);
            if !(Arc::ptr_eq(a, b) || a.is_subset(b)) {
                return false;
            }
            i += 1;
            j += 1;
        }
        true
    }

    /// Visit container pairs with equal keys until the callback returns false.
    fn for_each_common(
        &self,
        other: &Self,
        mut visit: impl FnMut(&Arc<Container>, &Arc<Container>) -> bool,
    ) {
        let (mut i, mut j) = (0, 0);
        while i < self.keys.len() && j < other.keys.len() {
            match self.keys[i].cmp(&other.keys[j]) {
                Ordering::Less => i += 1,
                Ordering::Greater => j += 1,
                Ordering::Equal => {
                    if !visit(&self.chunks[i], &other.chunks[j]) {
                        return;
                    }
                    i += 1;
                    j += 1;
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Cursor {
    Start,
    Array(usize),
    Bitmap { index: usize, pending: u64 },
    Full(u32),
}

#[derive(Clone)]
pub(crate) struct ChunkedIter<'a, W: Word> {
    set: &'a Chunked<W>,
    chunk: usize,
    cursor: Cursor,
}

impl<W: Word> Iterator for ChunkedIter<'_, W> {
    type Item = W;
    fn next(&mut self) -> Option<W> {
        loop {
            let container = self.set.chunks.get(self.chunk)?;
            let high = self.set.keys[self.chunk] << LOW_BITS;
            let low = match (&mut self.cursor, &**container) {
                (Cursor::Start, Container::Array(_)) => {
                    self.cursor = Cursor::Array(0);
                    continue;
                }
                (Cursor::Start, Container::Bitmap(..)) => {
                    self.cursor = Cursor::Bitmap {
                        index: 0,
                        pending: 0,
                    };
                    continue;
                }
                (Cursor::Start, Container::Full) => {
                    self.cursor = Cursor::Full(0);
                    continue;
                }
                (Cursor::Array(index), Container::Array(lows)) => {
                    if let Some(&low) = lows.get(*index) {
                        *index += 1;
                        Some(u64::from(low))
                    } else {
                        None
                    }
                }
                (Cursor::Bitmap { index, pending }, Container::Bitmap(words, _)) => loop {
                    if *pending != 0 {
                        let bit = pending.trailing_zeros();
                        *pending &= *pending - 1;
                        break Some((*index as u64 - 1) * 64 + u64::from(bit));
                    }
                    match words.get(*index) {
                        Some(&word) => {
                            *pending = word;
                            *index += 1;
                        }
                        None => break None,
                    }
                },
                (Cursor::Full(next), Container::Full) => {
                    if *next < SPAN {
                        let low = u64::from(*next);
                        *next += 1;
                        Some(low)
                    } else {
                        None
                    }
                }
                _ => unreachable!("cursor matches its container"),
            };
            if let Some(low) = low {
                return Some(W::from_u64(high | low));
            }
            self.chunk += 1;
            self.cursor = Cursor::Start;
        }
    }

    fn fold<B, F: FnMut(B, W) -> B>(self, init: B, mut f: F) -> B {
        let mut accum = init;
        let mut cursor = self.cursor;
        for (key, container) in self.set.keys[self.chunk..]
            .iter()
            .zip(&self.set.chunks[self.chunk..])
        {
            let high = *key << LOW_BITS;
            // Apply the saved cursor only to the first container. Calling
            // `next` to finish it would also consume the next container's first
            // member before revealing that the container boundary was crossed.
            match (&**container, cursor) {
                (Container::Array(lows), Cursor::Start | Cursor::Array(_)) => {
                    let start = match cursor {
                        Cursor::Array(index) => index,
                        _ => 0,
                    };
                    for &low in &lows[start..] {
                        accum = f(accum, W::from_u64(high | u64::from(low)));
                    }
                }
                (Container::Bitmap(words, _), Cursor::Start | Cursor::Bitmap { .. }) => {
                    let (next_word, pending) = match cursor {
                        Cursor::Bitmap { index, pending } => (index, pending),
                        _ => (0, 0),
                    };
                    let first = core::iter::once((next_word.saturating_sub(1), pending));
                    let rest = words[next_word..]
                        .iter()
                        .copied()
                        .enumerate()
                        .map(|(index, word)| (next_word + index, word));
                    for (index, mut word) in first.chain(rest) {
                        while word != 0 {
                            let bit = word.trailing_zeros();
                            word &= word - 1;
                            accum = f(
                                accum,
                                W::from_u64(high | (index as u64 * 64 + u64::from(bit))),
                            );
                        }
                    }
                }
                (Container::Full, Cursor::Start | Cursor::Full(_)) => {
                    let start = match cursor {
                        Cursor::Full(next) => next,
                        _ => 0,
                    };
                    for low in u64::from(start)..u64::from(SPAN) {
                        accum = f(accum, W::from_u64(high | low));
                    }
                }
                _ => unreachable!("cursor matches its container"),
            }
            cursor = Cursor::Start;
        }
        accum
    }
}

impl<W: Word> FusedIterator for ChunkedIter<'_, W> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn containers_normalize_at_both_thresholds() {
        let array = Container::from_lows((0..4096).collect()).unwrap();
        assert!(matches!(array, Container::Array(_)));
        let bitmap = Container::from_lows((0..4097).collect()).unwrap();
        assert!(matches!(bitmap, Container::Bitmap(_, 4097)));
        let full = Container::from_lows((0..=u16::MAX).collect()).unwrap();
        assert!(matches!(full, Container::Full));
        assert!(full.complement().is_none());
        assert!(matches!(
            bitmap.complement(),
            Some(Container::Bitmap(_, 61439))
        ));
        let nearly_full = Container::from_lows((1..=u16::MAX).collect()).unwrap();
        assert!(matches!(
            nearly_full.complement(),
            Some(Container::Array(ref lows)) if **lows == [0]
        ));
    }

    #[test]
    fn dense_promotion_splits_windows_on_container_boundaries() {
        // 64-bit words from 65_472 to 65_600 straddle keys 0 and 1.
        let words = [u64::MAX, 1 | (1 << 63)];
        let chunked = Chunked::<u32>::from_dense(65_472, &words);
        assert_eq!(chunked.chunk_count(), 2);
        let members: Vec<u32> = chunked.iter().collect();
        let mut expected: Vec<u32> = (65_472..65_536).collect();
        expected.push(65_536);
        expected.push(65_536 + 63);
        assert_eq!(members, expected);
        assert_eq!(chunked.cardinality(), 66);
    }

    #[test]
    fn partial_fold_preserves_members_across_every_container_kind() {
        let mut members = Vec::from([0_u64, 7, 63, 64, 65_535]);
        members.extend((0..4097).map(|low| 65_536 + low));
        members.extend(131_072..196_608);
        members.extend([196_609, 262_143]);
        // Repeat near the maximum code, where the final chunk reaches u64::MAX.
        for offset in [0, u64::MAX - 262_143] {
            let members: Vec<_> = members.iter().map(|member| offset + member).collect();
            let chunks = Chunked::<u64>::from_sorted(&members);
            for consumed in [
                0, 1, 4, 5, 6, 68, 69, 4101, 4102, 4103, 4166, 69_637, 69_638, 69_639, 69_640,
            ] {
                let mut iter = chunks.iter();
                for _ in 0..consumed {
                    assert!(iter.next().is_some());
                }
                let remaining = &members[consumed..];
                let folded = iter.clone().fold(Vec::new(), |mut out, member| {
                    out.push(member);
                    out
                });
                assert_eq!(
                    folded.len(),
                    remaining.len(),
                    "offset {offset}, consumed {consumed}"
                );
                assert_eq!(folded, remaining, "offset {offset}, consumed {consumed}");
                assert_eq!(iter.clone().count(), remaining.len());
                assert_eq!(iter.collect::<Vec<_>>(), remaining);
            }
        }
    }

    #[test]
    fn run_visitors_preserve_extreme_codes_and_stop_at_break() {
        let top = u64::MAX - 65_535;
        let members: Vec<_> = [0, 1, (1_u64 << 32) + 5]
            .into_iter()
            .chain(top..=u64::MAX)
            .collect();
        let chunks = Chunked::<u64>::from_sorted(&members);
        assert_eq!(chunks.partition_count(), 3);
        let mut runs = Vec::new();
        chunks.visit_runs(|start, end| runs.push((start, end)));
        assert_eq!(
            runs,
            [
                (0, 0),
                (1, 1),
                ((1_u64 << 32) + 5, (1_u64 << 32) + 5),
                (top, u64::MAX)
            ]
        );
        let mut visited = 0;
        let stopped = chunks.try_visit_runs(|start, end| {
            visited += 1;
            if start == 1 {
                ControlFlow::Break(end)
            } else {
                ControlFlow::Continue(())
            }
        });
        assert_eq!(stopped, ControlFlow::Break(1));
        assert_eq!(visited, 2);
        assert_eq!(Chunked::<u64>::empty().partition_count(), 0);

        let bitmap_members: Vec<_> = (0_u64..10_000).filter(|code| code % 7 != 0).collect();
        let bitmap = Chunked::<u64>::from_sorted(&bitmap_members);
        let mut expanded = Vec::new();
        bitmap.visit_runs(|start, end| expanded.extend(start..=end));
        assert_eq!(expanded, bitmap_members);
    }

    #[test]
    fn unchanged_overlapping_containers_keep_their_allocations() {
        let source = Chunked::<u32>::from_sorted(&[1, 3, 65_537, 65_538]);
        for (values, op, expected) in [
            (
                Vec::from([1, 65_539]),
                Operation::Union,
                Vec::from([1, 3, 65_537, 65_538, 65_539]),
            ),
            (
                Vec::from([1, 3, 5, 65_537]),
                Operation::Intersection,
                Vec::from([1, 3, 65_537]),
            ),
            (
                Vec::from([2, 65_537]),
                Operation::Difference,
                Vec::from([1, 3, 65_538]),
            ),
        ] {
            let result = source.binary(&Chunked::from_sorted(&values), op);
            assert_eq!(result.iter().collect::<Vec<_>>(), expected);
            assert_eq!(result.shared_chunks(&source), 1, "{op:?}");
        }
    }

    #[test]
    fn partial_fold_resumes_near_chunk_ends_without_repeating_next_chunk() {
        // Valid containers with a cursor on their last member. Constructing
        // the private cursor avoids visiting 65,535 members merely to test
        // the transition, keeping this ownership regression bounded in Miri.
        let mut words = Box::new([0_u64; WORDS]);
        words[WORDS - 65] = 1_u64 << 63;
        words[WORDS - 64..].fill(u64::MAX);
        let cases = [
            (Container::Array(Box::new([1, u16::MAX])), Cursor::Array(1)),
            (
                Container::from_words(words, 4097).unwrap(),
                Cursor::Bitmap {
                    index: WORDS,
                    pending: 1_u64 << 63,
                },
            ),
            (Container::Full, Cursor::Full(SPAN - 1)),
        ];
        let final_key = u64::MAX >> LOW_BITS;
        for (container, cursor) in cases {
            let set = Chunked::<u64> {
                keys: Box::new([final_key - 1, final_key]),
                chunks: Box::new([
                    Arc::new(container),
                    Arc::new(Container::Array(Box::new([0, u16::MAX]))),
                ]),
                word: PhantomData,
            };
            let iter = ChunkedIter {
                set: &set,
                chunk: 0,
                cursor,
            };
            let expected = [u64::MAX - 65_536, u64::MAX - 65_535, u64::MAX];
            assert_eq!(iter.clone().count(), expected.len());
            assert_eq!(
                iter.fold(Vec::new(), |mut out, code| {
                    out.push(code);
                    out
                }),
                expected
            );
        }
    }

    #[test]
    fn full_chunk_runs_and_early_break_are_bounded() {
        let mut builder = Builder::new();
        builder.push(0, Some(Container::Full));
        builder.push(u64::MAX >> LOW_BITS, Some(Container::Full));
        let chunks = builder.finish::<u64>();
        assert_eq!(chunks.partition_count(), 2);
        let mut runs = Vec::new();
        chunks.visit_runs(|start, end| runs.push((start, end)));
        assert_eq!(runs, [(0, 65_535), (u64::MAX - 65_535, u64::MAX)]);
        let mut calls = 0;
        let stopped = chunks.try_visit_runs(|start, end| {
            calls += 1;
            ControlFlow::Break((start, end))
        });
        assert_eq!(stopped, ControlFlow::Break((0, 65_535)));
        assert_eq!(calls, 1);
    }
}

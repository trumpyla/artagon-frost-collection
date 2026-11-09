//! Borrowed set algebra over strictly ascending slices.

use core::cmp::Ordering;
use core::fmt;
use core::iter::FusedIterator;

// Select the seeking kernel only for markedly different input cardinalities.
// This is a comparison-work heuristic, not a machine-specific crossover claim.
// The linear merge remains the kernel for similarly sized inputs.
const SEEK_RATIO: usize = 32;

#[derive(Clone, Copy)]
enum Strategy {
    Start,
    Merge,
    SeekLeft,
    SeekRight,
    Identical,
}

const fn seek_strategy(left: usize, right: usize) -> Strategy {
    if left == 0 || right == 0 {
        Strategy::Merge
    } else if left / SEEK_RATIO >= right {
        Strategy::SeekLeft
    } else if right / SEEK_RATIO >= left {
        Strategy::SeekRight
    } else {
        Strategy::Merge
    }
}

macro_rules! set_iterator {
    ($(#[$meta:meta])* $name:ident $(, $strategy:ident)?) => {
        $(#[$meta])*
        #[must_use = "iterators are lazy and do nothing unless consumed"]
        pub struct $name<'a, T> {
            left: &'a [T],
            right: &'a [T],
            $($strategy: Strategy,)?
        }

        impl<'a, T> $name<'a, T> {
            pub(crate) const fn new(left: &'a [T], right: &'a [T]) -> Self {
                Self {
                    left,
                    right,
                    $($strategy: Strategy::Start,)?
                }
            }
        }

        // The cursors borrow their elements; cloning a cursor never clones T.
        impl<T> Clone for $name<'_, T> {
            fn clone(&self) -> Self {
                Self {
                    left: self.left,
                    right: self.right,
                    $($strategy: self.$strategy,)?
                }
            }
        }

        impl<T> fmt::Debug for $name<'_, T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("left_remaining", &self.left.len())
                    .field("right_remaining", &self.right.len())
                    .finish()
            }
        }

        impl<T: Ord> FusedIterator for $name<'_, T> {}
    };
}

set_iterator! {
    /// Lazy ascending union of two frozen ordered sets.
    ///
    /// Equivalent members yield the left set's stored representative. The
    /// iterator allocates nothing and does not clone elements. Traversing all
    /// results uses O(n + m) comparisons and O(1) auxiliary storage.
    /// Its size hint is a bound, not an exact cardinality.
    SetUnion
}

set_iterator! {
    /// Lazy ascending intersection of two frozen ordered sets.
    ///
    /// Every result borrows the left set's stored representative. The iterator
    /// allocates nothing and does not clone elements. Traversing all results
    /// uses O(n + m) comparisons and O(1) auxiliary storage. Markedly skewed
    /// inputs use monotonic exponential searches through the larger slice.
    /// Its size hint is a bound, not an exact cardinality.
    SetIntersection, strategy
}

set_iterator! {
    /// Lazy ascending members present only in the left frozen ordered set.
    ///
    /// Results borrow the left set's stored representatives. The iterator
    /// allocates nothing and does not clone elements. Traversing all results
    /// uses O(n + m) comparisons and O(1) auxiliary storage. A much larger
    /// right operand is searched with monotonic exponential skips.
    /// Its size hint is a bound, not an exact cardinality.
    SetDifference, strategy
}

set_iterator! {
    /// Lazy ascending members present in exactly one of two frozen ordered sets.
    ///
    /// Results borrow the representative from the set containing that member.
    /// The iterator allocates nothing and does not clone elements. Traversing
    /// all results uses O(n + m) comparisons and O(1) auxiliary storage.
    /// Its size hint is a bound, not an exact cardinality.
    SetSymmetricDifference
}

fn take_first<'a, T>(items: &mut &'a [T]) -> Option<&'a T> {
    let (first, rest) = items.split_first()?;
    *items = rest;
    Some(first)
}

/// Skip a sorted prefix known to start below `target`.
///
/// Exponential bracketing makes the work logarithmic in the skipped distance;
/// the final binary search stays inside that bracket. The last-element check
/// handles an exhausted or entirely earlier slice without traversing it.
#[inline(never)]
fn skip_below<'a, T: Ord>(items: &'a [T], target: &T) -> &'a [T] {
    debug_assert!(!items.is_empty());
    if items
        .last()
        .is_none_or(|last| last.cmp(target) == Ordering::Less)
    {
        return &[];
    }
    let mut lower = 1;
    let mut upper = 1;
    while upper < items.len() && items[upper].cmp(target) == Ordering::Less {
        lower = upper + 1;
        upper = upper.saturating_mul(2).min(items.len());
    }
    let offset = items[lower..upper].partition_point(|item| item.cmp(target) == Ordering::Less);
    &items[lower + offset..]
}

// Consume the smaller input monotonically, searching only the still-unvisited
// suffix of the larger input. RETURN_LARGE preserves the left representative
// when the larger operand happens to be the left one.
#[inline]
fn next_seeking<'a, T: Ord, const RETURN_LARGE: bool>(
    small: &mut &'a [T],
    large: &mut &'a [T],
) -> Option<&'a T> {
    while let (Some(needle), Some(first)) = (small.first(), large.first()) {
        match first.cmp(needle) {
            Ordering::Less => *large = skip_below(large, needle),
            Ordering::Greater => *small = &small[1..],
            Ordering::Equal => {
                *small = &small[1..];
                let matched = take_first(large);
                return if RETURN_LARGE { matched } else { Some(needle) };
            }
        }
    }
    None
}

#[inline(never)]
fn intersection_len_seeking<T: Ord>(mut small: &[T], mut large: &[T]) -> usize {
    let mut count = 0;
    // Keep consecutive matches as cheap as the merge kernel: no per-match
    // iterator/helper call. Only a gap invokes the separate seeking routine.
    while let (Some(needle), Some(first)) = (small.first(), large.first()) {
        match needle.cmp(first) {
            Ordering::Less => small = &small[1..],
            Ordering::Greater => large = skip_below(large, needle),
            Ordering::Equal => {
                count += 1;
                small = &small[1..];
                large = &large[1..];
            }
        }
    }
    count
}

// Keep bulk merge traversal independent of the larger seeking branches. The
// callback is still statically specialized inside this one-call-per-fold
// kernel; strategy dispatch never enters its inner loop.
#[inline(never)]
fn intersection_fold_merge<'a, T: Ord, B, F>(
    mut left: &'a [T],
    mut right: &'a [T],
    mut accum: B,
    mut f: F,
) -> B
where
    F: FnMut(B, &'a T) -> B,
{
    while let (Some(a), Some(b)) = (left.first(), right.first()) {
        match a.cmp(b) {
            Ordering::Less => left = &left[1..],
            Ordering::Greater => right = &right[1..],
            Ordering::Equal => {
                accum = f(accum, a);
                left = &left[1..];
                right = &right[1..];
            }
        }
    }
    accum
}

impl<'a, T: Ord> Iterator for SetUnion<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.left.first(), self.right.first()) {
            (Some(left), Some(right)) => match left.cmp(right) {
                Ordering::Less => take_first(&mut self.left),
                Ordering::Greater => take_first(&mut self.right),
                Ordering::Equal => {
                    self.right = &self.right[1..];
                    take_first(&mut self.left)
                }
            },
            (Some(_), None) => take_first(&mut self.left),
            (None, Some(_)) => take_first(&mut self.right),
            (None, None) => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.left.len().max(self.right.len()),
            self.left.len().checked_add(self.right.len()),
        )
    }

    fn count(self) -> usize {
        union_len(self.left, self.right)
    }
}

impl<'a, T: Ord> Iterator for SetIntersection<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if matches!(self.strategy, Strategy::Start) {
            if core::ptr::eq(self.left, self.right) {
                self.strategy = Strategy::Identical;
            } else if separated(self.left, self.right) {
                self.left = &[];
                self.right = &[];
                self.strategy = Strategy::Merge;
                return None;
            } else {
                self.strategy = seek_strategy(self.left.len(), self.right.len());
            }
        }
        match self.strategy {
            Strategy::Identical => {
                let next = take_first(&mut self.left);
                self.right = self.left;
                return next;
            }
            Strategy::SeekLeft => {
                return next_seeking::<T, true>(&mut self.right, &mut self.left);
            }
            Strategy::SeekRight => {
                return next_seeking::<T, false>(&mut self.left, &mut self.right);
            }
            Strategy::Start | Strategy::Merge => {}
        }
        while let (Some(left), Some(right)) = (self.left.first(), self.right.first()) {
            match left.cmp(right) {
                Ordering::Less => self.left = &self.left[1..],
                Ordering::Greater => self.right = &self.right[1..],
                Ordering::Equal => {
                    self.right = &self.right[1..];
                    return take_first(&mut self.left);
                }
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.left.len().min(self.right.len())))
    }

    fn count(self) -> usize {
        intersection_len(self.left, self.right)
    }

    // Adapters such as map(...).sum() delegate to fold. Select the kernel once
    // for the whole traversal, so they do not redispatch at every common item.
    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        let mut left = self.left;
        let mut right = self.right;
        if core::ptr::eq(left, right) {
            return left.iter().fold(init, f);
        }
        if separated(left, right) {
            return init;
        }
        let mut accum = init;
        match seek_strategy(left.len(), right.len()) {
            Strategy::SeekLeft => {
                while let Some(item) = next_seeking::<T, true>(&mut right, &mut left) {
                    accum = f(accum, item);
                }
            }
            Strategy::SeekRight => {
                while let Some(item) = next_seeking::<T, false>(&mut left, &mut right) {
                    accum = f(accum, item);
                }
            }
            _ => return intersection_fold_merge(left, right, accum, f),
        }
        accum
    }
}

impl<'a, T: Ord> Iterator for SetDifference<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if matches!(self.strategy, Strategy::Start) {
            if core::ptr::eq(self.left, self.right) {
                self.left = &[];
                self.right = &[];
            } else if separated(self.left, self.right) {
                self.right = &[];
            }
            self.strategy = seek_strategy(self.left.len(), self.right.len());
        }
        if matches!(self.strategy, Strategy::SeekRight) {
            while let (Some(left), Some(right)) = (self.left.first(), self.right.first()) {
                match left.cmp(right) {
                    Ordering::Less => return take_first(&mut self.left),
                    Ordering::Greater => self.right = skip_below(self.right, left),
                    Ordering::Equal => {
                        self.left = &self.left[1..];
                        self.right = &self.right[1..];
                    }
                }
            }
            return take_first(&mut self.left);
        }
        while let (Some(left), Some(right)) = (self.left.first(), self.right.first()) {
            match left.cmp(right) {
                Ordering::Less => return take_first(&mut self.left),
                Ordering::Greater => self.right = &self.right[1..],
                Ordering::Equal => {
                    self.left = &self.left[1..];
                    self.right = &self.right[1..];
                }
            }
        }
        take_first(&mut self.left)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.left.len().saturating_sub(self.right.len()),
            Some(self.left.len()),
        )
    }

    fn count(self) -> usize {
        difference_len(self.left, self.right)
    }
}

impl<'a, T: Ord> Iterator for SetSymmetricDifference<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        while let (Some(left), Some(right)) = (self.left.first(), self.right.first()) {
            match left.cmp(right) {
                Ordering::Less => return take_first(&mut self.left),
                Ordering::Greater => return take_first(&mut self.right),
                Ordering::Equal => {
                    self.left = &self.left[1..];
                    self.right = &self.right[1..];
                }
            }
        }
        take_first(&mut self.left).or_else(|| take_first(&mut self.right))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.left.len().abs_diff(self.right.len()),
            self.left.len().checked_add(self.right.len()),
        )
    }

    fn count(self) -> usize {
        symmetric_difference_len(self.left, self.right)
    }
}

/// Empty sets or nonoverlapping endpoint ranges cannot intersect.
fn separated<T: Ord>(left: &[T], right: &[T]) -> bool {
    match (left.first(), left.last(), right.first(), right.last()) {
        (Some(left_first), Some(left_last), Some(right_first), Some(right_last)) => {
            left_last.cmp(right_first) == Ordering::Less
                || right_last.cmp(left_first) == Ordering::Less
        }
        _ => true,
    }
}

pub(crate) fn intersection_len<T: Ord>(left: &[T], right: &[T]) -> usize {
    if core::ptr::eq(left, right) {
        return left.len();
    }
    if separated(left, right) {
        return 0;
    }
    match seek_strategy(left.len(), right.len()) {
        Strategy::SeekLeft => intersection_len_seeking(right, left),
        Strategy::SeekRight => intersection_len_seeking(left, right),
        _ => intersection_len_merge(left, right),
    }
}

// Keep the balanced loop simple and available for caller optimization. The
// separate non-inlined seeking kernel keeps search machinery out of this path.
#[inline]
fn intersection_len_merge<T: Ord>(mut left: &[T], mut right: &[T]) -> usize {
    let mut common = 0;
    while let (Some(a), Some(b)) = (left.first(), right.first()) {
        match a.cmp(b) {
            Ordering::Less => left = &left[1..],
            Ordering::Greater => right = &right[1..],
            Ordering::Equal => {
                // At most min(original_left.len(), original_right.len()) hits.
                common += 1;
                left = &left[1..];
                right = &right[1..];
            }
        }
    }
    common
}

pub(super) fn union_len<T: Ord>(left: &[T], right: &[T]) -> usize {
    let common = intersection_len(left, right);
    left.len()
        .checked_add(right.len() - common)
        .expect("set cardinality exceeds usize::MAX")
}

pub(super) fn difference_len<T: Ord>(left: &[T], right: &[T]) -> usize {
    left.len() - intersection_len(left, right)
}

pub(super) fn symmetric_difference_len<T: Ord>(left: &[T], right: &[T]) -> usize {
    let common = intersection_len(left, right);
    (left.len() - common)
        .checked_add(right.len() - common)
        .expect("set cardinality exceeds usize::MAX")
}

pub(crate) fn is_disjoint<'a, T: Ord>(mut left: &'a [T], mut right: &'a [T]) -> bool {
    if core::ptr::eq(left, right) {
        return left.is_empty();
    }
    if separated(left, right) {
        return true;
    }
    match seek_strategy(left.len(), right.len()) {
        Strategy::SeekLeft => return next_seeking::<T, false>(&mut right, &mut left).is_none(),
        Strategy::SeekRight => return next_seeking::<T, false>(&mut left, &mut right).is_none(),
        _ => {}
    }
    while let (Some(a), Some(b)) = (left.first(), right.first()) {
        match a.cmp(b) {
            Ordering::Less => left = &left[1..],
            Ordering::Greater => right = &right[1..],
            Ordering::Equal => return false,
        }
    }
    true
}

pub(crate) fn is_subset<T: Ord>(mut left: &[T], mut right: &[T]) -> bool {
    if left.is_empty() {
        return true;
    }
    if core::ptr::eq(left, right) {
        return true;
    }
    if left.len() > right.len()
        || left.first().cmp(&right.first()) == Ordering::Less
        || left.last().cmp(&right.last()) == Ordering::Greater
    {
        return false;
    }
    if matches!(seek_strategy(left.len(), right.len()), Strategy::SeekRight) {
        for needle in left {
            let Some(first) = right.first() else {
                return false;
            };
            match first.cmp(needle) {
                Ordering::Less => {
                    right = skip_below(right, needle);
                    if right
                        .first()
                        .is_none_or(|candidate| candidate.cmp(needle) != Ordering::Equal)
                    {
                        return false;
                    }
                }
                Ordering::Greater => return false,
                Ordering::Equal => {}
            }
            right = &right[1..];
        }
        return true;
    }
    while let (Some(a), Some(b)) = (left.first(), right.first()) {
        match a.cmp(b) {
            Ordering::Less => return false,
            Ordering::Greater => right = &right[1..],
            Ordering::Equal => {
                left = &left[1..];
                right = &right[1..];
            }
        }
        // Every remaining left element needs a distinct remaining right match.
        if left.len() > right.len() {
            return false;
        }
    }
    left.is_empty()
}

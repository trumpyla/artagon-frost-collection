//! Borrowed set algebra over strictly ascending slices.
//!
//! # Execution strategy
//!
//! Every kernel is an *adaptive merge* in the style of `TimSort`-style galloping
//! mode. While both cursors keep taking turns, the kernel advances one cursor
//! per comparison (the classic linear merge, `O(n + m)`). When one cursor
//! has won [`MIN_GALLOP`] consecutive comparisons, that side is instead
//! advanced with an exponential search followed by a bounded binary search
//! ([`lower_bound_gallop`]), whose first probe is placed at the expected gap
//! between matches. The kernel stays in galloping mode while jumps keep
//! paying off and drops back to merging when they stop.
//!
//! A skewed input therefore costs `O(n · log(m / n))` comparisons instead
//! of `O(n + m)`, matching the bound of `BTreeSet`'s size-aware algorithms,
//! while a balanced input performs the same comparisons as a plain merge plus
//! one counter update per step. The decision is based on what the data does,
//! not on the inputs' initial lengths, so inputs that start balanced and
//! become skewed (or vice versa) switch strategy without a restart.
//!
//! Union and symmetric difference must emit (almost) every member of both
//! inputs, so their traversal stays a plain merge; their scalar counts reuse
//! the adaptive intersection kernel.

use core::cmp::Ordering;
use core::fmt;
use core::iter::FusedIterator;

/// Initial number of consecutive wins by one cursor before the kernel
/// switches to galloping on that side, and the jump length a gallop must
/// achieve to count as worthwhile. `TimSort` uses the same initial threshold.
const MIN_GALLOP: u32 = 7;

/// Upper bound for the adaptive threshold, so a long balanced stretch after a
/// few failed gallops cannot disable galloping for the rest of a traversal.
const MAX_GALLOP_THRESHOLD: u32 = 64;

/// Index of the first element of the ascending slice `items` that is `>= key`
/// (its lower bound). Probes `first_probe - 1`, then doubles the distance
/// until a probe is `>= key` or leaves the slice, then binary-searches the
/// final interval.
///
/// With `first_probe == 1` this is classic exponential search, costing at most
/// `2·⌈log2(r + 1)⌉ + 1` comparisons for a result `r`. Passing the expected
/// gap as `first_probe` halves that when the guess is good and never costs
/// more than a plain binary search when it is not.
fn lower_bound_gallop<T: Ord>(items: &[T], key: &T, first_probe: usize) -> usize {
    let len = items.len();
    // Invariant: `items[..low] < key`; the next probe index is `bound - 1`.
    let mut low = 0_usize;
    let mut bound = first_probe.max(1);
    while bound <= len && items[bound - 1].cmp(key) == Ordering::Less {
        low = bound;
        bound = bound.saturating_mul(2);
    }
    let high = bound.min(len);
    low + items[low..high].partition_point(|x| x.cmp(key) == Ordering::Less)
}

/// Galloping-mode bookkeeping for one pair of cursors.
///
/// `threshold` adapts as `TimSort`'s `min_gallop` does: every gallop that skips
/// at least [`MIN_GALLOP`] elements lowers it (down to 1), so a persistently
/// skewed input soon gallops after every single win; a gallop that does not
/// pay off raises it again, so noisy balanced data stays in merge mode.
#[derive(Clone, Copy)]
struct Gallop {
    left_wins: u32,
    right_wins: u32,
    threshold: u32,
}

impl Gallop {
    const fn new() -> Self {
        Self {
            left_wins: 0,
            right_wins: 0,
            threshold: MIN_GALLOP,
        }
    }

    /// Record that `left` advanced alone. Returns `true` when the left cursor
    /// should now be galloped.
    #[inline]
    fn left_won(&mut self) -> bool {
        self.right_wins = 0;
        self.left_wins = self.left_wins.saturating_add(1);
        self.left_wins >= self.threshold
    }

    /// Record that `right` advanced alone. Returns `true` when the right
    /// cursor should now be galloped.
    #[inline]
    fn right_won(&mut self) -> bool {
        self.left_wins = 0;
        self.right_wins = self.right_wins.saturating_add(1);
        self.right_wins >= self.threshold
    }

    /// Both cursors advanced: neither side is winning.
    #[inline]
    fn tie(&mut self) {
        self.left_wins = 0;
        self.right_wins = 0;
    }

    /// The left cursor galloped over `jumped` elements.
    #[inline]
    fn left_galloped(&mut self, jumped: usize) {
        self.left_wins = if self.adapt(jumped) {
            self.threshold
        } else {
            0
        };
    }

    /// The right cursor galloped over `jumped` elements.
    #[inline]
    fn right_galloped(&mut self, jumped: usize) {
        self.right_wins = if self.adapt(jumped) {
            self.threshold
        } else {
            0
        };
    }

    /// Lower the threshold after a worthwhile jump, raise it after a wasted
    /// one; returns whether the jump was worthwhile.
    #[inline]
    fn adapt(&mut self, jumped: usize) -> bool {
        let worthwhile = jumped >= MIN_GALLOP as usize;
        self.threshold = if worthwhile {
            self.threshold.saturating_sub(1).max(1)
        } else {
            (self.threshold + 1).min(MAX_GALLOP_THRESHOLD)
        };
        worthwhile
    }
}

/// Advance `long` to the lower bound of `key`, guessing the gap from the two
/// remaining lengths. Returns the number of elements skipped.
#[inline]
fn gallop_to<T: Ord>(long: &mut &[T], key: &T, short_remaining: usize) -> usize {
    let guess = long.len() / short_remaining.max(1);
    let at = lower_bound_gallop(long, key, guess);
    *long = &long[at..];
    at
}

macro_rules! set_iterator {
    ($(#[$meta:meta])* $name:ident $(, $gallop:ident)?) => {
        $(#[$meta])*
        #[must_use = "iterators are lazy and do nothing unless consumed"]
        pub struct $name<'a, T> {
            left: &'a [T],
            right: &'a [T],
            $($gallop: Gallop,)?
        }

        impl<'a, T> $name<'a, T> {
            pub(super) const fn new(left: &'a [T], right: &'a [T]) -> Self {
                Self {
                    left,
                    right,
                    $($gallop: Gallop::new(),)?
                }
            }
        }

        // The cursors borrow their elements; cloning a cursor never clones T.
        impl<T> Clone for $name<'_, T> {
            fn clone(&self) -> Self {
                Self {
                    left: self.left,
                    right: self.right,
                    $($gallop: self.$gallop,)?
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
    /// uses O(n + m) comparisons for balanced inputs and O(n · log(m / n))
    /// comparisons when one input is much larger (adaptive galloping), with
    /// O(1) auxiliary storage. Its size hint is a bound, not an exact
    /// cardinality.
    SetIntersection, gallop
}

set_iterator! {
    /// Lazy ascending members present only in the left frozen ordered set.
    ///
    /// Results borrow the left set's stored representatives. The iterator
    /// allocates nothing and does not clone elements. Traversing all results
    /// uses O(n + m) comparisons for balanced inputs and O(n · log(m / n))
    /// comparisons when the right input is much larger (adaptive galloping),
    /// with O(1) auxiliary storage. Its size hint is a bound, not an exact
    /// cardinality.
    SetDifference, gallop
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

    fn next(&mut self) -> Option<Self::Item> {
        while let (Some(left), Some(right)) = (self.left.first(), self.right.first()) {
            match left.cmp(right) {
                Ordering::Less => {
                    self.left = &self.left[1..];
                    if self.gallop.left_won() {
                        let jumped = gallop_to(&mut self.left, right, self.right.len());
                        self.gallop.left_galloped(jumped);
                    }
                }
                Ordering::Greater => {
                    self.right = &self.right[1..];
                    if self.gallop.right_won() {
                        let jumped = gallop_to(&mut self.right, left, self.left.len());
                        self.gallop.right_galloped(jumped);
                    }
                }
                Ordering::Equal => {
                    self.gallop.tie();
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
}

impl<'a, T: Ord> Iterator for SetDifference<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        while let (Some(left), Some(right)) = (self.left.first(), self.right.first()) {
            match left.cmp(right) {
                // Left-only members are results, so the left cursor is never
                // galloped: every element it passes is emitted.
                Ordering::Less => {
                    self.gallop.left_won();
                    return take_first(&mut self.left);
                }
                Ordering::Greater => {
                    self.right = &self.right[1..];
                    if self.gallop.right_won() {
                        let jumped = gallop_to(&mut self.right, left, self.left.len());
                        self.gallop.right_galloped(jumped);
                    }
                }
                Ordering::Equal => {
                    self.gallop.tie();
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

/// What an adaptive merge should do when it finds equivalent members.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OnMatch {
    /// Count it and continue.
    Count,
    /// Report it: the inputs are not disjoint.
    Stop,
}

/// Shared adaptive merge for the symmetric kernels. Returns the number of
/// common members, or `None` as soon as one is found when `on_match` is
/// [`OnMatch::Stop`].
fn adaptive_common<T: Ord>(mut left: &[T], mut right: &[T], on_match: OnMatch) -> Option<usize> {
    let mut common = 0;
    let mut gallop = Gallop::new();
    while let (Some(a), Some(b)) = (left.first(), right.first()) {
        match a.cmp(b) {
            Ordering::Less => {
                left = &left[1..];
                if gallop.left_won() {
                    let jumped = gallop_to(&mut left, b, right.len());
                    gallop.left_galloped(jumped);
                }
            }
            Ordering::Greater => {
                right = &right[1..];
                if gallop.right_won() {
                    let jumped = gallop_to(&mut right, a, left.len());
                    gallop.right_galloped(jumped);
                }
            }
            Ordering::Equal => {
                if on_match == OnMatch::Stop {
                    return None;
                }
                gallop.tie();
                // At most min(left.len(), right.len()) hits.
                common += 1;
                left = &left[1..];
                right = &right[1..];
            }
        }
    }
    Some(common)
}

pub(super) fn intersection_len<T: Ord>(left: &[T], right: &[T]) -> usize {
    if separated(left, right) {
        return 0;
    }
    adaptive_common(left, right, OnMatch::Count).unwrap_or(0)
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

pub(super) fn is_disjoint<T: Ord>(left: &[T], right: &[T]) -> bool {
    if separated(left, right) {
        return true;
    }
    adaptive_common(left, right, OnMatch::Stop).is_some()
}

pub(super) fn is_subset<T: Ord>(mut left: &[T], mut right: &[T]) -> bool {
    if left.is_empty() {
        return true;
    }
    if left.len() > right.len()
        || left.first().cmp(&right.first()) == Ordering::Less
        || left.last().cmp(&right.last()) == Ordering::Greater
    {
        return false;
    }
    let mut gallop = Gallop::new();
    while let (Some(a), Some(b)) = (left.first(), right.first()) {
        match a.cmp(b) {
            Ordering::Less => return false,
            Ordering::Greater => {
                right = &right[1..];
                if gallop.right_won() {
                    let jumped = gallop_to(&mut right, a, left.len());
                    gallop.right_galloped(jumped);
                }
            }
            Ordering::Equal => {
                gallop.tie();
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

#[cfg(test)]
mod gallop_tests {
    use super::lower_bound_gallop;

    #[test]
    fn lower_bound_matches_partition_point_for_every_first_probe() {
        for len in 0..48_u32 {
            let items: alloc::vec::Vec<u32> = (0..len).map(|x| x * 3 + 1).collect();
            for key in 0..=(3 * len + 3) {
                let expected = items.partition_point(|&x| x < key);
                for first_probe in 0..(len as usize + 3) {
                    assert_eq!(
                        lower_bound_gallop(&items, &key, first_probe),
                        expected,
                        "len {len} key {key} first_probe {first_probe}"
                    );
                }
                assert_eq!(lower_bound_gallop(&items, &key, usize::MAX), expected);
            }
        }
    }
}

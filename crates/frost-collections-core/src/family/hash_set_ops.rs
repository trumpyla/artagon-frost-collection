//! Borrowed hash-set algebra with explicit representative retention.

use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::iter::FusedIterator;
use std::hash::RandomState;

use super::{FrozenHashSet, HashSetIter};

/// Borrowed members of the left set that are absent from the right set.
///
/// Created by [`FrozenHashSet::difference`]. Traversal is lazy, unordered, and
/// allocation-free. Cloning this iterator clones only its traversal state.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct SetDifference<'a, T, S = RandomState> {
    iter: HashSetIter<'a, T>,
    other: &'a FrozenHashSet<T, S>,
}

impl<'a, T, S> SetDifference<'a, T, S> {
    pub(super) fn new<S1>(left: &'a FrozenHashSet<T, S1>, right: &'a FrozenHashSet<T, S>) -> Self {
        Self {
            iter: left.iter(),
            other: right,
        }
    }
}

impl<T, S> Clone for SetDifference<'_, T, S> {
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            other: self.other,
        }
    }
}

impl<T, S> fmt::Debug for SetDifference<'_, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetDifference")
            .field("remaining_candidates", &self.iter.len())
            .finish_non_exhaustive()
    }
}

impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for SetDifference<'a, T, S> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find(|item| !self.other.contains(*item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.iter.len().saturating_sub(self.other.len()),
            Some(self.iter.len()),
        )
    }

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        self.iter.fold(init, |acc, item| {
            if self.other.contains(item) {
                acc
            } else {
                f(acc, item)
            }
        })
    }
}

impl<T: Eq + Hash, S: BuildHasher> FusedIterator for SetDifference<'_, T, S> {}

enum IntersectionRepr<'a, T, S1, S2> {
    Left {
        iter: HashSetIter<'a, T>,
        right: &'a FrozenHashSet<T, S2>,
    },
    Right {
        iter: HashSetIter<'a, T>,
        left: &'a FrozenHashSet<T, S1>,
    },
}

/// Borrowed common members, retaining the left set's representatives.
///
/// Created by [`FrozenHashSet::intersection`]. Scans the operand with fewer
/// members and looks up left representatives when scanning the right operand.
/// Traversal is lazy, unordered, and allocation-free. Cloning this iterator
/// clones only its traversal state.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct SetIntersection<'a, T, S1 = RandomState, S2 = RandomState> {
    repr: IntersectionRepr<'a, T, S1, S2>,
}

impl<'a, T, S1, S2> SetIntersection<'a, T, S1, S2> {
    pub(super) fn new(left: &'a FrozenHashSet<T, S1>, right: &'a FrozenHashSet<T, S2>) -> Self {
        let repr = if left.len() <= right.len() {
            IntersectionRepr::Left {
                iter: left.iter(),
                right,
            }
        } else {
            IntersectionRepr::Right {
                iter: right.iter(),
                left,
            }
        };
        Self { repr }
    }

    fn remaining_candidates(&self) -> usize {
        match &self.repr {
            IntersectionRepr::Left { iter, .. } | IntersectionRepr::Right { iter, .. } => {
                iter.len()
            }
        }
    }
}

impl<T, S1, S2> Clone for SetIntersection<'_, T, S1, S2> {
    fn clone(&self) -> Self {
        let repr = match &self.repr {
            IntersectionRepr::Left { iter, right } => IntersectionRepr::Left {
                iter: iter.clone(),
                right,
            },
            IntersectionRepr::Right { iter, left } => IntersectionRepr::Right {
                iter: iter.clone(),
                left,
            },
        };
        Self { repr }
    }
}

impl<T, S1, S2> fmt::Debug for SetIntersection<'_, T, S1, S2> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetIntersection")
            .field("remaining_candidates", &self.remaining_candidates())
            .finish_non_exhaustive()
    }
}

impl<'a, T: Eq + Hash, S1: BuildHasher, S2: BuildHasher> Iterator
    for SetIntersection<'a, T, S1, S2>
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.repr {
            IntersectionRepr::Left { iter, right } => iter.find(|item| right.contains(*item)),
            IntersectionRepr::Right { iter, left } => iter.find_map(|item| (*left).get(item)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.remaining_candidates()))
    }

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        match self.repr {
            IntersectionRepr::Left { iter, right } => iter.fold(init, |acc, item| {
                if right.contains(item) {
                    f(acc, item)
                } else {
                    acc
                }
            }),
            IntersectionRepr::Right { iter, left } => iter.fold(init, |acc, item| {
                if let Some(stored) = left.get(item) {
                    f(acc, stored)
                } else {
                    acc
                }
            }),
        }
    }
}

impl<T: Eq + Hash, S1: BuildHasher, S2: BuildHasher> FusedIterator
    for SetIntersection<'_, T, S1, S2>
{
}

/// Borrowed union retaining the left set's representative on overlap.
///
/// Created by [`FrozenHashSet::union`]. Traversal is lazy, unordered, and
/// allocation-free. Cloning this iterator clones only its traversal state.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct SetUnion<'a, T, S = RandomState> {
    left: HashSetIter<'a, T>,
    right: SetDifference<'a, T, S>,
}

impl<'a, T, S> SetUnion<'a, T, S> {
    pub(super) fn new<S2>(left: &'a FrozenHashSet<T, S>, right: &'a FrozenHashSet<T, S2>) -> Self {
        Self {
            left: left.iter(),
            right: SetDifference::new(right, left),
        }
    }
}

impl<T, S> Clone for SetUnion<'_, T, S> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
        }
    }
}

impl<T, S> fmt::Debug for SetUnion<'_, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetUnion")
            .field("remaining_left", &self.left.len())
            .field("right", &self.right)
            .finish_non_exhaustive()
    }
}

impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for SetUnion<'a, T, S> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.left.next().or_else(|| self.right.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.right.size_hint();
        (
            self.left.len().saturating_add(lower),
            upper.and_then(|upper| self.left.len().checked_add(upper)),
        )
    }

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        self.right.fold(self.left.fold(init, &mut f), f)
    }
}

impl<T: Eq + Hash, S: BuildHasher> FusedIterator for SetUnion<'_, T, S> {}

/// Borrowed members present in exactly one of two hash sets.
///
/// Created by [`FrozenHashSet::symmetric_difference`]. Traversal is lazy,
/// unordered, and allocation-free. Each representative comes from the operand
/// containing that member. Cloning this iterator clones only its traversal state.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct SetSymmetricDifference<'a, T, S1 = RandomState, S2 = RandomState> {
    left: SetDifference<'a, T, S2>,
    right: SetDifference<'a, T, S1>,
}

impl<'a, T, S1, S2> SetSymmetricDifference<'a, T, S1, S2> {
    pub(super) fn new(left: &'a FrozenHashSet<T, S1>, right: &'a FrozenHashSet<T, S2>) -> Self {
        Self {
            left: SetDifference::new(left, right),
            right: SetDifference::new(right, left),
        }
    }
}

impl<T, S1, S2> Clone for SetSymmetricDifference<'_, T, S1, S2> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
        }
    }
}

impl<T, S1, S2> fmt::Debug for SetSymmetricDifference<'_, T, S1, S2> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetSymmetricDifference")
            .field("left", &self.left)
            .field("right", &self.right)
            .finish_non_exhaustive()
    }
}

impl<'a, T: Eq + Hash, S1: BuildHasher, S2: BuildHasher> Iterator
    for SetSymmetricDifference<'a, T, S1, S2>
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.left.next().or_else(|| self.right.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (left_lower, left_upper) = self.left.size_hint();
        let (right_lower, right_upper) = self.right.size_hint();
        (
            left_lower.saturating_add(right_lower),
            left_upper.and_then(|left| right_upper.and_then(|right| left.checked_add(right))),
        )
    }

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        self.right.fold(self.left.fold(init, &mut f), f)
    }
}

impl<T: Eq + Hash, S1: BuildHasher, S2: BuildHasher> FusedIterator
    for SetSymmetricDifference<'_, T, S1, S2>
{
}

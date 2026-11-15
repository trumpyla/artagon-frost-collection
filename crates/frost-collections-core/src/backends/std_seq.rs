//! `Vec`, `Box<[T]>`, `VecDeque`, `LinkedList`, `BinaryHeap` (all `alloc`).

use alloc::boxed::Box;
use alloc::collections::{BinaryHeap, LinkedList, VecDeque, binary_heap, linked_list, vec_deque};
use alloc::vec::Vec;
use core::ops::{Deref, Index};

use crate::family::FrozenVec;
use crate::frozen::sealed::Sealed;
use crate::plan::{LengthMismatch, Sorted};
use crate::read::{DequeRead, ElementType, Elements, Len, PriorityRead, TwoSlices};
use crate::{Compact, Eligible, ExactLen, FreezeWith, Frozen, Rejected, Retain, TryFreezeWith};

macro_rules! eligible {
    ($($ty:ty => [$($g:tt)*]),* $(,)?) => {$(
        impl<$($g)*> Sealed for $ty {}
        impl<$($g)*> Eligible for $ty {}
        impl<$($g)*> FreezeWith<Retain> for $ty {
            type Output = Frozen<$ty>;
            #[inline]
            fn freeze_with(self, _: Retain) -> Self::Output {
                Frozen::new_unchecked(self)
            }
        }
    )*};
}

eligible! {
    Vec<T> => [T],
    Box<[T]> => [T],
    VecDeque<T> => [T],
    LinkedList<T> => [T],
    BinaryHeap<T> => [T],
}

// ---------------------------------------------------------------------------
// Len / Elements
// ---------------------------------------------------------------------------

macro_rules! len_elements {
    ($($ty:ty => $base:ty, $iter:ty);* $(;)?) => {$(
        impl<T> Len for $ty {
            #[inline]
            fn len(&self) -> usize {
                <$base>::len(self)
            }
        }
        impl<T> ElementType for $ty {
            type Item = T;
        }

        impl<T> Elements for $ty {
            type Iter<'a> = $iter where Self: 'a;
            #[inline]
            fn elements(&self) -> Self::Iter<'_> {
                <$base>::iter(self)
            }
        }
    )*};
}

len_elements! {
    Vec<T> => [T], core::slice::Iter<'a, T>;
    Box<[T]> => [T], core::slice::Iter<'a, T>;
    VecDeque<T> => VecDeque<T>, vec_deque::Iter<'a, T>;
    LinkedList<T> => LinkedList<T>, linked_list::Iter<'a, T>;
    // Storage order — *not* priority order.
    BinaryHeap<T> => BinaryHeap<T>, binary_heap::Iter<'a, T>;
}

impl<T> DequeRead for VecDeque<T> {
    fn front(&self) -> Option<&T> {
        VecDeque::front(self)
    }
    fn back(&self) -> Option<&T> {
        VecDeque::back(self)
    }
}

impl<T> TwoSlices for VecDeque<T> {
    fn as_slices(&self) -> (&[T], &[T]) {
        VecDeque::as_slices(self)
    }
}

impl<T> DequeRead for LinkedList<T> {
    fn front(&self) -> Option<&T> {
        LinkedList::front(self)
    }
    fn back(&self) -> Option<&T> {
        LinkedList::back(self)
    }
}

impl<T: Ord> PriorityRead for BinaryHeap<T> {
    fn peek(&self) -> Option<&T> {
        BinaryHeap::peek(self)
    }
}

// ---------------------------------------------------------------------------
// Plans
// ---------------------------------------------------------------------------

/// Drops excess capacity. Reuses the allocation when `len == capacity`.
impl<T> FreezeWith<Compact> for Vec<T> {
    type Output = FrozenVec<T>;
    fn freeze_with(self, _: Compact) -> FrozenVec<T> {
        FrozenVec::from(self)
    }
}

/// O(1): already exact-length.
impl<T> FreezeWith<Compact> for Box<[T]> {
    type Output = FrozenVec<T>;
    fn freeze_with(self, _: Compact) -> FrozenVec<T> {
        FrozenVec::from(self)
    }
}

/// `Vec::from(VecDeque)` reuses the buffer but may move elements to make the
/// ring contiguous; `into_boxed_slice` may then reallocate.
impl<T> FreezeWith<Compact> for VecDeque<T> {
    type Output = FrozenVec<T>;
    fn freeze_with(self, _: Compact) -> FrozenVec<T> {
        FrozenVec::from(Vec::from(self))
    }
}

/// O(n): allocates one contiguous buffer and moves every element.
impl<T> FreezeWith<Compact> for LinkedList<T> {
    type Output = FrozenVec<T>;
    fn freeze_with(self, _: Compact) -> FrozenVec<T> {
        self.into_iter().collect()
    }
}

/// Ascending priority order (`BinaryHeap::into_sorted_vec`), in place.
impl<T: Ord> FreezeWith<Sorted> for BinaryHeap<T> {
    type Output = FrozenVec<T>;
    fn freeze_with(self, _: Sorted) -> FrozenVec<T> {
        FrozenVec::from(self.into_sorted_vec())
    }
}

/// Recoverable: on length mismatch the vector is returned unchanged.
impl<T, const N: usize> TryFreezeWith<ExactLen<N>> for Vec<T> {
    type Output = Frozen<[T; N]>;
    type Error = Rejected<Vec<T>, LengthMismatch>;
    fn try_freeze_with(self, _: ExactLen<N>) -> Result<Self::Output, Self::Error> {
        let actual = self.len();
        <[T; N]>::try_from(self)
            .map(Frozen::new_unchecked)
            .map_err(|source| Rejected {
                input: source,
                error: LengthMismatch {
                    expected: N,
                    actual,
                },
            })
    }
}

// ---------------------------------------------------------------------------
// Inherent API and std traits on the frozen wrappers
// ---------------------------------------------------------------------------

macro_rules! slice_like {
    ($($ty:ty),*) => {$(
        impl<T> Deref for Frozen<$ty> {
            type Target = [T];
            #[inline]
            fn deref(&self) -> &[T] {
                self.backend()
            }
        }
        impl<T> AsRef<[T]> for Frozen<$ty> {
            #[inline]
            fn as_ref(&self) -> &[T] {
                self.backend()
            }
        }
    )*};
}
slice_like!(Vec<T>, Box<[T]>);

impl<T> Frozen<Vec<T>> {
    /// Retained capacity (sealing does not shrink).
    pub fn capacity(&self) -> usize {
        self.backend().capacity()
    }
}

/// Frozen → more compact frozen; never passes through a mutable state the
/// caller can observe.
impl<T> FreezeWith<Compact> for Frozen<Vec<T>> {
    type Output = FrozenVec<T>;
    fn freeze_with(self, _: Compact) -> FrozenVec<T> {
        FrozenVec::from(self.into_inner())
    }
}

impl<T> Frozen<VecDeque<T>> {
    /// Element at logical position `index`.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.backend().get(index)
    }
    /// First element.
    pub fn front(&self) -> Option<&T> {
        self.backend().front()
    }
    /// Last element.
    pub fn back(&self) -> Option<&T> {
        self.backend().back()
    }
    /// Ring contents as `(head, tail)` slices.
    pub fn as_slices(&self) -> (&[T], &[T]) {
        self.backend().as_slices()
    }
    /// Logical front-to-back iteration.
    pub fn iter(&self) -> vec_deque::Iter<'_, T> {
        self.backend().iter()
    }
}

impl<T> Index<usize> for Frozen<VecDeque<T>> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        &self.backend()[index]
    }
}

/// No `Index`/`get(usize)`: a linked list does not pretend to random access.
impl<T> Frozen<LinkedList<T>> {
    /// First element.
    pub fn front(&self) -> Option<&T> {
        self.backend().front()
    }
    /// Last element.
    pub fn back(&self) -> Option<&T> {
        self.backend().back()
    }
    /// Front-to-back traversal.
    pub fn iter(&self) -> linked_list::Iter<'_, T> {
        self.backend().iter()
    }
}

impl<T> Frozen<BinaryHeap<T>> {
    /// Elements in **storage** (heap) order.
    pub fn iter(&self) -> binary_heap::Iter<'_, T> {
        self.backend().iter()
    }
    /// Underlying storage-order slice.
    pub fn as_slice(&self) -> &[T] {
        self.backend().as_slice()
    }
}

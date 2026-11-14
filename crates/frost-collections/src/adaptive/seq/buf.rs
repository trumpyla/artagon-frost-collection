//! Fixed-capacity inline storage shared by [`InlineVec`](super::InlineVec) and
//! [`InlineFrozenVec`](super::InlineFrozenVec). The only module in the crate
//! with `unsafe`; every block has a `SAFETY` comment and one operation.
#![allow(unsafe_code)]

use alloc::vec::Vec;
use core::iter::FusedIterator;
use core::mem::MaybeUninit;

/// Initialized prefix of an inline buffer.
///
/// Invariant: `len <= N` and `buf[..len]` are initialized; `buf[len..]` are
/// not. Every method preserves it; `Drop` relies on it.
pub(crate) struct InlineBuf<T, const N: usize> {
    len: usize,
    buf: [MaybeUninit<T>; N],
}

const fn uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    [const { MaybeUninit::uninit() }; N]
}

impl<T, const N: usize> InlineBuf<T, N> {
    pub(crate) const fn new() -> Self {
        Self {
            len: 0,
            buf: uninit_array(),
        }
    }

    #[inline]
    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub(crate) const fn is_full(&self) -> bool {
        self.len == N
    }

    /// Safe append. Panics (index check) if full, leaving the invariant intact.
    #[inline]
    pub(crate) fn push(&mut self, value: T) {
        self.buf[self.len].write(value);
        self.len += 1;
    }

    /// Append, or hand the value back when full.
    #[inline]
    pub(crate) fn try_push(&mut self, value: T) -> Result<(), T> {
        if self.is_full() {
            Err(value)
        } else {
            self.push(value);
            Ok(())
        }
    }

    #[inline]
    pub(crate) fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        // Shrink first: after this the slot is logically uninitialized and
        // owned by the value we read out.
        self.len -= 1;
        // SAFETY: `buf[len]` was initialized (it was inside the old prefix) and
        // is now outside the prefix, so it is read exactly once.
        Some(unsafe { self.buf[self.len].assume_init_read() })
    }

    #[inline]
    pub(crate) fn as_slice(&self) -> &[T] {
        let init = &self.buf[..self.len];
        // SAFETY: `buf[..len]` is initialized; `MaybeUninit<T>` has `T`'s layout.
        unsafe { &*(core::ptr::from_ref::<[MaybeUninit<T>]>(init) as *const [T]) }
    }

    #[inline]
    pub(crate) fn as_mut_slice(&mut self) -> &mut [T] {
        let init = &mut self.buf[..self.len];
        // SAFETY: as `as_slice`; the unique borrow of `self` makes it unique.
        unsafe { &mut *(core::ptr::from_mut::<[MaybeUninit<T>]>(init) as *mut [T]) }
    }

    /// Drop elements from the back until `len <= new_len`. A panicking `Drop`
    /// leaves the remaining prefix owned and consistent.
    pub(crate) fn truncate(&mut self, new_len: usize) {
        while self.len > new_len {
            drop(self.pop());
        }
    }

    /// Move the elements into a new `Vec` with at least `capacity`, leaving
    /// `self` empty.
    pub(crate) fn take_into_vec(&mut self, capacity: usize) -> Vec<T> {
        let len = self.len;
        // May panic before any element moves: invariant intact.
        let mut v = Vec::with_capacity(capacity.max(len));
        self.len = 0; // ownership moves below; never drop these again here
        let src = self.buf.as_ptr().cast::<T>();
        // SAFETY: `src` holds `len` initialized `T`s; `v` has room for `len`
        // and does not overlap the inline buffer.
        unsafe { core::ptr::copy_nonoverlapping(src, v.as_mut_ptr(), len) };
        // SAFETY: the first `len` slots of `v` were initialized just above.
        unsafe { v.set_len(len) };
        v
    }

    /// Owning iterator; `self` is left empty.
    pub(crate) fn take_iter(&mut self) -> InlineIntoIter<T, N> {
        let back = self.len;
        self.len = 0;
        InlineIntoIter {
            front: 0,
            back,
            buf: core::mem::replace(&mut self.buf, uninit_array()),
        }
    }
}

impl<T, const N: usize> Drop for InlineBuf<T, N> {
    fn drop(&mut self) {
        let init: *mut [T] =
            core::ptr::slice_from_raw_parts_mut(self.buf.as_mut_ptr().cast::<T>(), self.len);
        // SAFETY: `buf[..len]` is initialized and owned by `self`; dropped once.
        unsafe { core::ptr::drop_in_place(init) };
    }
}

impl<T: Clone, const N: usize> Clone for InlineBuf<T, N> {
    fn clone(&self) -> Self {
        // A panicking `T::clone` drops exactly the clones made so far.
        let mut out = Self::new();
        for x in self.as_slice() {
            out.push(x.clone());
        }
        out
    }
}

impl<T, const N: usize> FromIterator<T> for InlineBuf<T, N> {
    /// Panics if the iterator yields more than `N` items (callers check).
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut out = Self::new();
        for x in iter {
            out.push(x);
        }
        out
    }
}

/// Owning iterator over an inline buffer.
///
/// Invariant: `buf[front..back]` are initialized and owned by the iterator.
pub(crate) struct InlineIntoIter<T, const N: usize> {
    front: usize,
    back: usize,
    buf: [MaybeUninit<T>; N],
}

impl<T, const N: usize> InlineIntoIter<T, N> {
    /// Mutable reference to the next element, without consuming it.
    pub(crate) fn first_mut(&mut self) -> Option<&mut T> {
        if self.front == self.back {
            return None;
        }
        // SAFETY: `front < back`, so `buf[front]` is initialized and owned by
        // the iterator; the unique borrow of `self` makes the reference unique.
        Some(unsafe { self.buf[self.front].assume_init_mut() })
    }

    /// Remaining elements as a slice.
    pub(crate) fn as_slice(&self) -> &[T] {
        let rest = &self.buf[self.front..self.back];
        // SAFETY: `buf[front..back]` is initialized by the struct invariant.
        unsafe { &*(core::ptr::from_ref::<[MaybeUninit<T>]>(rest) as *const [T]) }
    }
}

impl<T, const N: usize> Iterator for InlineIntoIter<T, N> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        if self.front == self.back {
            return None;
        }
        let i = self.front;
        self.front += 1; // the slot leaves the owned range before it is read
        // SAFETY: `i` was in `front..back`, hence initialized, and is read once.
        Some(unsafe { self.buf[i].assume_init_read() })
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

impl<T, const N: usize> DoubleEndedIterator for InlineIntoIter<T, N> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        // SAFETY: `back` was inside `front..back`, hence initialized; read once.
        Some(unsafe { self.buf[self.back].assume_init_read() })
    }
}

impl<T, const N: usize> ExactSizeIterator for InlineIntoIter<T, N> {}
impl<T, const N: usize> FusedIterator for InlineIntoIter<T, N> {}

impl<T, const N: usize> Drop for InlineIntoIter<T, N> {
    fn drop(&mut self) {
        let rest: *mut [T] = core::ptr::slice_from_raw_parts_mut(
            self.buf[self.front..self.back].as_mut_ptr().cast::<T>(),
            self.back - self.front,
        );
        // SAFETY: `buf[front..back]` is initialized and owned; dropped once.
        unsafe { core::ptr::drop_in_place(rest) };
    }
}

impl<T: core::fmt::Debug, const N: usize> core::fmt::Debug for InlineIntoIter<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("InlineIntoIter")
            .field(&self.as_slice())
            .finish()
    }
}

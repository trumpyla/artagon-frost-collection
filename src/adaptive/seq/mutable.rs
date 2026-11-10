//! `InlineVec<T, N>`: mutable sequence that stores up to `N` elements inline
//! and spills to the heap. Safe code over [`InlineBuf`].

use alloc::vec::{self, Vec};
use core::fmt;
use core::hash::{Hash, Hasher};
use core::iter::FusedIterator;
use core::ops::{Deref, DerefMut};

use super::buf::{InlineBuf, InlineIntoIter};
use super::frozen::{InlineFrozenVec, Repr as FrozenRepr};
use crate::family::FrozenVec;
use crate::read::{ElementType, Elements, Len};
use crate::write::Push;
use crate::{Compact, FreezeWith};

pub(crate) enum Repr<T, const N: usize> {
    Inline(InlineBuf<T, N>),
    Heap(Vec<T>),
}

/// Mutable sequence profile of the *sequence* sub-category: inline storage for
/// up to `N` elements, spilling to a heap vector on growth.
///
/// * Spill is one-way during mutation (no oscillation at the boundary);
///   [`shrink_to_fit`](Self::shrink_to_fit) moves a short heap sequence back
///   inline explicitly.
/// * Equality, ordering and hashing are those of `[T]`.
/// * Paired frozen profile: [`InlineFrozenVec<T, N>`] via
///   [`freeze`](Self::freeze); moving between them never allocates for the
///   inline variant.
///
/// ```
/// use frost::prelude::*;
/// let mut v: InlineVec<u8, 2> = InlineVec::new();
/// v.push(1);
/// v.push(2);
/// assert!(v.is_inline());
/// v.push(3);                      // spills
/// assert!(!v.is_inline());
/// v.truncate(2);
/// v.shrink_to_fit();              // explicit move back inline
/// assert!(v.is_inline());
/// let f = v.freeze();
/// assert_eq!(f, [1, 2]);
/// ```
pub struct InlineVec<T, const N: usize> {
    pub(crate) repr: Repr<T, N>,
}

impl<T, const N: usize> InlineVec<T, N> {
    /// Empty, inline; does not allocate.
    pub const fn new() -> Self {
        Self {
            repr: Repr::Inline(InlineBuf::new()),
        }
    }

    /// Empty; heap-allocated up front only if `capacity > N`.
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity > N {
            Self::from_heap(Vec::with_capacity(capacity))
        } else {
            Self::new()
        }
    }

    pub(super) fn from_buf(buf: InlineBuf<T, N>) -> Self {
        Self {
            repr: Repr::Inline(buf),
        }
    }

    pub(crate) fn from_heap(v: Vec<T>) -> Self {
        Self {
            repr: Repr::Heap(v),
        }
    }

    /// Adopt a vector: inline if `len <= N` (frees the vector), else keeps it.
    pub fn from_vec(v: Vec<T>) -> Self {
        if v.len() <= N {
            Self::from_buf(v.into_iter().collect())
        } else {
            Self::from_heap(v)
        }
    }

    /// `true` if stored inline.
    pub fn is_inline(&self) -> bool {
        matches!(self.repr, Repr::Inline(_))
    }

    /// Element count.
    pub fn len(&self) -> usize {
        match &self.repr {
            Repr::Inline(b) => b.len(),
            Repr::Heap(v) => v.len(),
        }
    }

    /// `true` when empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `N` while inline, otherwise the heap capacity.
    pub fn capacity(&self) -> usize {
        match &self.repr {
            Repr::Inline(_) => N,
            Repr::Heap(v) => v.capacity(),
        }
    }

    /// Elements as a slice (one match, then a plain slice).
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        match &self.repr {
            Repr::Inline(b) => b.as_slice(),
            Repr::Heap(v) => v,
        }
    }

    /// Elements as a mutable slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match &mut self.repr {
            Repr::Inline(b) => b.as_mut_slice(),
            Repr::Heap(v) => v,
        }
    }

    fn spill(&mut self, extra: usize) -> &mut Vec<T> {
        if let Repr::Inline(b) = &mut self.repr {
            // Check the logical length separately: Vec accepts usize::MAX
            // capacity for ZSTs, but len + additional must still fit usize.
            let required = b.len().checked_add(extra).expect("capacity overflow");
            let v = b.take_into_vec(required.max(N.saturating_mul(2)).max(4));
            self.repr = Repr::Heap(v);
        }
        match &mut self.repr {
            Repr::Heap(v) => v,
            Repr::Inline(_) => unreachable!("spilled above"),
        }
    }

    /// Append; spills to the heap when the inline buffer is full.
    pub fn push(&mut self, value: T) {
        match &mut self.repr {
            Repr::Inline(b) => {
                if let Err(value) = b.try_push(value) {
                    self.spill(1).push(value);
                }
            }
            Repr::Heap(v) => v.push(value),
        }
    }

    /// Remove the last element.
    pub fn pop(&mut self) -> Option<T> {
        match &mut self.repr {
            Repr::Inline(b) => b.pop(),
            Repr::Heap(v) => v.pop(),
        }
    }

    /// Insert at `index`, shifting later elements (safe: push + rotate).
    ///
    /// # Panics
    /// If `index > len`.
    pub fn insert(&mut self, index: usize, value: T) {
        let len = self.len();
        assert!(
            index <= len,
            "insertion index {index} out of bounds (len {len})"
        );
        self.push(value);
        self.as_mut_slice()[index..].rotate_right(1);
    }

    /// Remove at `index`, shifting later elements.
    ///
    /// # Panics
    /// If `index >= len`.
    pub fn remove(&mut self, index: usize) -> T {
        let len = self.len();
        assert!(
            index < len,
            "removal index {index} out of bounds (len {len})"
        );
        self.as_mut_slice()[index..].rotate_left(1);
        self.pop().expect("non-empty")
    }

    /// Remove at `index`, moving the last element into its place.
    ///
    /// # Panics
    /// If `index >= len`.
    pub fn swap_remove(&mut self, index: usize) -> T {
        let len = self.len();
        assert!(
            index < len,
            "swap_remove index {index} out of bounds (len {len})"
        );
        self.as_mut_slice().swap(index, len - 1);
        self.pop().expect("non-empty")
    }

    /// Keep the first `len` elements.
    pub fn truncate(&mut self, len: usize) {
        match &mut self.repr {
            Repr::Inline(b) => b.truncate(len),
            Repr::Heap(v) => v.truncate(len),
        }
    }

    /// Remove all elements; keeps the current representation.
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// Keep elements for which `f` returns `true`, in order. Never allocates
    /// for the inline variant. If `f` panics, every element not yet rejected
    /// is kept (as `Vec::retain`).
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut f: F) {
        self.retain_mut(|x| f(x));
    }

    /// [`retain`](Self::retain) with mutable access.
    pub fn retain_mut<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
        match &mut self.repr {
            Repr::Inline(b) => {
                let src = b.take_iter();
                // On unwind the guard moves the unvisited suffix (including
                // the element being tested) back behind the kept prefix.
                let mut g = RetainGuard { dst: b, src };
                while let Some(first) = g.src.first_mut() {
                    let keep = f(first);
                    if let Some(x) = g.src.next() {
                        if keep {
                            g.dst.push(x);
                        } else {
                            drop(x);
                        }
                    }
                }
            }
            Repr::Heap(v) => v.retain_mut(f),
        }
    }

    /// Ensure room for `additional` more elements, spilling if needed.
    ///
    /// # Panics
    /// If the requested length overflows, including for zero-sized elements.
    pub fn reserve(&mut self, additional: usize) {
        let required = self
            .len()
            .checked_add(additional)
            .expect("capacity overflow");
        match &mut self.repr {
            Repr::Inline(_) if required > N => {
                self.spill(additional);
            }
            Repr::Inline(_) => {}
            Repr::Heap(v) => v.reserve(additional),
        }
    }

    /// Release excess capacity; a heap sequence with `len <= N` moves back
    /// inline (the only demotion path).
    pub fn shrink_to_fit(&mut self) {
        if let Repr::Heap(v) = &mut self.repr {
            if v.len() <= N {
                let buf: InlineBuf<T, N> = core::mem::take(v).into_iter().collect();
                self.repr = Repr::Inline(buf);
            } else {
                v.shrink_to_fit();
            }
        }
    }

    /// Freeze into [`InlineFrozenVec<T, N>`]. Inline: moves the buffer, no
    /// allocation. Heap: re-selected canonically (`len <= N` ⇒ inline).
    #[must_use]
    pub fn freeze(self) -> InlineFrozenVec<T, N> {
        match self.repr {
            Repr::Inline(b) => InlineFrozenVec {
                repr: FrozenRepr::Inline(b),
            },
            Repr::Heap(v) => InlineFrozenVec::from_vec(v),
        }
    }

    /// Interop: the elements as a `Vec<T>` (allocates when inline).
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        match self.repr {
            Repr::Inline(mut b) => b.take_into_vec(0),
            Repr::Heap(v) => v,
        }
    }
}

impl<T: Clone, const N: usize> InlineVec<T, N> {
    /// Append clones of `other`.
    pub fn extend_from_slice(&mut self, other: &[T]) {
        self.reserve(other.len());
        match &mut self.repr {
            // Spilled storage is a `Vec`: one bulk append, not a push per element.
            Repr::Heap(v) => v.extend_from_slice(other),
            Repr::Inline(_) => {
                for x in other {
                    self.push(x.clone());
                }
            }
        }
    }
}

impl<T, const N: usize> Default for InlineVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone, const N: usize> Clone for InlineVec<T, N> {
    fn clone(&self) -> Self {
        Self {
            repr: match &self.repr {
                Repr::Inline(b) => Repr::Inline(b.clone()),
                Repr::Heap(v) => Repr::Heap(v.clone()),
            },
        }
    }
}

impl<T, const N: usize> From<Vec<T>> for InlineVec<T, N> {
    fn from(v: Vec<T>) -> Self {
        Self::from_vec(v)
    }
}

impl<T, const N: usize> FromIterator<T> for InlineVec<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut out = Self::new();
        out.extend(iter);
        out
    }
}

impl<T, const N: usize> Extend<T> for InlineVec<T, N> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut iter = iter.into_iter();
        match &mut self.repr {
            Repr::Inline(buffer) => {
                while let Some(value) = iter.next() {
                    if let Err(value) = buffer.try_push(value) {
                        let heap = self.spill(1);
                        heap.push(value);
                        // Only a result that actually exceeds N reaches Vec's
                        // reservation logic or consults its remaining hint.
                        heap.extend(iter);
                        return;
                    }
                }
            }
            Repr::Heap(values) => values.extend(iter),
        }
    }
}

impl<'a, T: Copy + 'a, const N: usize> Extend<&'a T> for InlineVec<T, N> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().copied());
    }
}

impl<T, const N: usize> Deref for InlineVec<T, N> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T, const N: usize> DerefMut for InlineVec<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}
impl<T, const N: usize> AsRef<[T]> for InlineVec<T, N> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T, const N: usize> AsMut<[T]> for InlineVec<T, N> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: PartialEq<U>, U, const N: usize, const M: usize> PartialEq<InlineVec<U, M>>
    for InlineVec<T, N>
{
    fn eq(&self, other: &InlineVec<U, M>) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl<T: Eq, const N: usize> Eq for InlineVec<T, N> {}
impl<T: PartialEq<U>, U, const N: usize, const M: usize> PartialEq<InlineFrozenVec<U, M>>
    for InlineVec<T, N>
{
    fn eq(&self, other: &InlineFrozenVec<U, M>) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl<T: PartialEq<U>, U, const N: usize, const M: usize> PartialEq<InlineVec<U, M>>
    for InlineFrozenVec<T, N>
{
    fn eq(&self, other: &InlineVec<U, M>) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U]> for InlineVec<T, N> {
    fn eq(&self, other: &[U]) -> bool {
        self.as_slice() == other
    }
}
impl<T: PartialEq<U>, U, const N: usize, const M: usize> PartialEq<[U; M]> for InlineVec<T, N> {
    fn eq(&self, other: &[U; M]) -> bool {
        self.as_slice() == &other[..]
    }
}
impl<T: PartialEq<U>, U, const N: usize> PartialEq<Vec<U>> for InlineVec<T, N> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.as_slice() == &other[..]
    }
}
impl<T: PartialOrd, const N: usize> PartialOrd for InlineVec<T, N> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}
impl<T: Ord, const N: usize> Ord for InlineVec<T, N> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}
impl<T: Hash, const N: usize> Hash for InlineVec<T, N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}
impl<T: fmt::Debug, const N: usize> fmt::Debug for InlineVec<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

struct RetainGuard<'a, T, const N: usize> {
    dst: &'a mut InlineBuf<T, N>,
    src: InlineIntoIter<T, N>,
}

impl<T, const N: usize> Drop for RetainGuard<'_, T, N> {
    fn drop(&mut self) {
        // Capacity cannot be exceeded: kept + remaining <= original <= N.
        for x in self.src.by_ref() {
            self.dst.push(x);
        }
    }
}

// --- owning iterator shared by both inline profiles ------------------------

enum IntoIterRepr<T, const N: usize> {
    Inline(InlineIntoIter<T, N>),
    Heap(vec::IntoIter<T>),
}

/// Owning iterator of [`InlineVec`] and [`InlineFrozenVec`].
pub struct IntoIter<T, const N: usize>(IntoIterRepr<T, N>);

impl<T, const N: usize> IntoIter<T, N> {
    pub(super) fn from_frozen(repr: FrozenRepr<T, N>) -> Self {
        Self(match repr {
            FrozenRepr::Inline(mut b) => IntoIterRepr::Inline(b.take_iter()),
            FrozenRepr::Heap(b) => IntoIterRepr::Heap(b.into_vec().into_iter()),
        })
    }
    /// Remaining elements.
    pub fn as_slice(&self) -> &[T] {
        match &self.0 {
            IntoIterRepr::Inline(it) => it.as_slice(),
            IntoIterRepr::Heap(it) => it.as_slice(),
        }
    }
}

impl<T, const N: usize> Iterator for IntoIter<T, N> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        match &mut self.0 {
            IntoIterRepr::Inline(it) => it.next(),
            IntoIterRepr::Heap(it) => it.next(),
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            IntoIterRepr::Inline(it) => it.size_hint(),
            IntoIterRepr::Heap(it) => it.size_hint(),
        }
    }
    #[inline]
    fn fold<B, F: FnMut(B, T) -> B>(self, init: B, f: F) -> B {
        match self.0 {
            IntoIterRepr::Inline(it) => it.fold(init, f),
            IntoIterRepr::Heap(it) => it.fold(init, f),
        }
    }
}
impl<T, const N: usize> DoubleEndedIterator for IntoIter<T, N> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        match &mut self.0 {
            IntoIterRepr::Inline(it) => it.next_back(),
            IntoIterRepr::Heap(it) => it.next_back(),
        }
    }
}
impl<T, const N: usize> ExactSizeIterator for IntoIter<T, N> {}
impl<T, const N: usize> FusedIterator for IntoIter<T, N> {}
impl<T: fmt::Debug, const N: usize> fmt::Debug for IntoIter<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}

impl<T, const N: usize> IntoIterator for InlineVec<T, N> {
    type Item = T;
    type IntoIter = IntoIter<T, N>;
    fn into_iter(self) -> IntoIter<T, N> {
        IntoIter(match self.repr {
            Repr::Inline(mut b) => IntoIterRepr::Inline(b.take_iter()),
            Repr::Heap(v) => IntoIterRepr::Heap(v.into_iter()),
        })
    }
}
impl<'a, T, const N: usize> IntoIterator for &'a InlineVec<T, N> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}
impl<'a, T, const N: usize> IntoIterator for &'a mut InlineVec<T, N> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

// --- plans and capabilities ------------------------------------------------

impl<T, const N: usize> FreezeWith<Compact> for InlineVec<T, N> {
    type Output = FrozenVec<T>;
    fn freeze_with(self, _: Compact) -> FrozenVec<T> {
        FrozenVec::from(self.into_vec())
    }
}

impl<T, const N: usize> Len for InlineVec<T, N> {
    fn len(&self) -> usize {
        Self::len(self)
    }
}
impl<T, const N: usize> ElementType for InlineVec<T, N> {
    type Item = T;
}
impl<T, const N: usize> Elements for InlineVec<T, N> {
    type Iter<'a>
        = core::slice::Iter<'a, T>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.as_slice().iter()
    }
}
impl<T, const N: usize> Push for InlineVec<T, N> {
    type Item = T;
    fn push(&mut self, value: T) {
        Self::push(self, value);
    }
}

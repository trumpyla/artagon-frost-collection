//! `InlineFrozenVec<T, N>`: up to `N` elements stored inline, otherwise an
//! exact-length boxed slice. Safe code over [`InlineBuf`].

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::Deref;

use super::buf::InlineBuf;
use super::mutable::{InlineVec, IntoIter};
use crate::read::{ElementType, Elements, Len};
use crate::{FreezeWith, Frozen};

/// Plan: freeze a sequence into [`InlineFrozenVec<T, N>`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Inline<const N: usize>;

pub(crate) enum Repr<T, const N: usize> {
    Inline(InlineBuf<T, N>),
    Heap(Box<[T]>),
}

/// Frozen sequence that stores up to `N` elements inline.
///
/// * Canonical: `len <= N` ⇒ inline, otherwise boxed. Equality, ordering and
///   hashing are those of `[T]` and never observe the variant.
/// * Size: every value is at least as large as the inline variant
///   (`N * size_of::<T>()` plus a length and discriminant), including values
///   that hold the boxed variant. Measure arrays of these, not only one.
/// * Element addresses change when the value moves (inline variant).
/// * Drop check is stricter than `Vec`'s (no `#[may_dangle]` on stable):
///   elements may not borrow data declared after the collection.
///
/// ```
/// # extern crate frost_collections_core as frost;
/// use frost::prelude::*;
/// let small: InlineFrozenVec<u32, 4> = vec![1, 2, 3].freeze_with(Inline::<4>);
/// let large: InlineFrozenVec<u32, 4> = (0..10).collect::<Vec<_>>().freeze_with(Inline::<4>);
/// assert!(small.is_inline() && !large.is_inline());
/// assert_eq!(small.iter().sum::<u32>(), 6);    // slice API via Deref
/// assert_eq!(large.thaw().len(), 10);
/// ```
pub struct InlineFrozenVec<T, const N: usize> {
    pub(crate) repr: Repr<T, N>,
}

impl<T, const N: usize> InlineFrozenVec<T, N> {
    /// Build from a vector: moves ≤ `N` elements inline and frees the vector's
    /// buffer; otherwise `into_boxed_slice` (drops excess capacity).
    pub fn from_vec(v: Vec<T>) -> Self {
        let repr = if v.len() <= N {
            Repr::Inline(v.into_iter().collect())
        } else {
            Repr::Heap(v.into_boxed_slice())
        };
        Self { repr }
    }

    /// Elements as a slice. The variant is matched once here; loops over the
    /// returned slice do not branch on it.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        match &self.repr {
            Repr::Inline(b) => b.as_slice(),
            Repr::Heap(b) => b,
        }
    }

    /// `true` if stored inline.
    pub fn is_inline(&self) -> bool {
        matches!(self.repr, Repr::Inline(_))
    }

    /// Back to the paired mutable profile [`InlineVec<T, N>`]. Inline: moves
    /// ≤ `N` elements, no allocation. Boxed: O(1).
    #[must_use]
    pub fn thaw(self) -> InlineVec<T, N> {
        match self.repr {
            Repr::Inline(b) => InlineVec::from_buf(b),
            Repr::Heap(b) => InlineVec::from_heap(b.into_vec()),
        }
    }

    /// Interop: the elements as a `Vec<T>` (allocates when inline).
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        match self.repr {
            Repr::Inline(mut b) => b.take_into_vec(0),
            Repr::Heap(b) => b.into_vec(),
        }
    }
}

impl<T, const N: usize> FreezeWith<Inline<N>> for Vec<T> {
    type Output = InlineFrozenVec<T, N>;
    fn freeze_with(self, _: Inline<N>) -> Self::Output {
        InlineFrozenVec::from_vec(self)
    }
}

impl<T, const N: usize> FreezeWith<Inline<N>> for Frozen<Vec<T>> {
    type Output = InlineFrozenVec<T, N>;
    fn freeze_with(self, _: Inline<N>) -> Self::Output {
        InlineFrozenVec::from_vec(self.into_inner())
    }
}

impl<T, const N: usize> FromIterator<T> for InlineFrozenVec<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        InlineVec::<T, N>::from_iter(iter).freeze()
    }
}

impl<T, const N: usize> Default for InlineFrozenVec<T, N> {
    fn default() -> Self {
        Self {
            repr: Repr::Inline(InlineBuf::new()),
        }
    }
}

impl<T: Clone, const N: usize> Clone for InlineFrozenVec<T, N> {
    fn clone(&self) -> Self {
        Self {
            repr: match &self.repr {
                Repr::Inline(b) => Repr::Inline(b.clone()),
                Repr::Heap(b) => Repr::Heap(b.clone()),
            },
        }
    }
}

impl<T, const N: usize> Deref for InlineFrozenVec<T, N> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const N: usize> AsRef<[T]> for InlineFrozenVec<T, N> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: PartialEq<U>, U, const N: usize, const M: usize> PartialEq<InlineFrozenVec<U, M>>
    for InlineFrozenVec<T, N>
{
    fn eq(&self, other: &InlineFrozenVec<U, M>) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl<T: Eq, const N: usize> Eq for InlineFrozenVec<T, N> {}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U]> for InlineFrozenVec<T, N> {
    fn eq(&self, other: &[U]) -> bool {
        self.as_slice() == other
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<Vec<U>> for InlineFrozenVec<T, N> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.as_slice() == &other[..]
    }
}

impl<T: PartialEq<U>, U, const N: usize, const M: usize> PartialEq<[U; M]>
    for InlineFrozenVec<T, N>
{
    fn eq(&self, other: &[U; M]) -> bool {
        self.as_slice() == &other[..]
    }
}

impl<T: PartialOrd, const N: usize> PartialOrd for InlineFrozenVec<T, N> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}
impl<T: Ord, const N: usize> Ord for InlineFrozenVec<T, N> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

/// Hashes as `[T]`, consistent with `PartialEq`.
impl<T: Hash, const N: usize> Hash for InlineFrozenVec<T, N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for InlineFrozenVec<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

impl<T, const N: usize> IntoIterator for InlineFrozenVec<T, N> {
    type Item = T;
    type IntoIter = IntoIter<T, N>;
    fn into_iter(self) -> IntoIter<T, N> {
        IntoIter::from_frozen(self.repr)
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a InlineFrozenVec<T, N> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<T, const N: usize> Len for InlineFrozenVec<T, N> {
    fn len(&self) -> usize {
        self.as_slice().len()
    }
}

impl<T, const N: usize> ElementType for InlineFrozenVec<T, N> {
    type Item = T;
}

impl<T, const N: usize> Elements for InlineFrozenVec<T, N> {
    type Iter<'a>
        = core::slice::Iter<'a, T>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.as_slice().iter()
    }
}

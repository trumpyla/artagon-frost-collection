use alloc::boxed::Box;
use alloc::collections::TryReserveError;
use alloc::vec::{self, Vec};
use core::borrow::{Borrow, BorrowMut};
use core::fmt;
use core::ops::{Deref, DerefMut, RangeBounds};

use super::FrozenVec;
use crate::adaptive::{Inline, InlineFrozenVec};
use crate::iter::wrap_iter;
use crate::read::{ElementType, Elements, Len};
use crate::write::{Push, PushRejected, TryPush};
use crate::{Compact, FreezeWith};

/// Mutable contiguous sequence with a private representation.
///
/// Representation today: `Vec<T>`. It is not part of the API: signatures use
/// crate-owned iterator types, and `From<Vec<T>>` / `Into<Vec<T>>` are
/// interop conversions whose cost may change (O(1) today).
///
/// Paired frozen family: [`FrozenVec`] via [`freeze`](Self::freeze).
///
/// ```
/// # extern crate frost_collections_core as frost;
/// use frost::prelude::*;
/// let mut v = MutableVec::new();
/// v.push(3);
/// v.extend([1, 2]);
/// v.sort();                         // slice API via DerefMut
/// let f: FrozenVec<i32> = v.freeze();
/// assert_eq!(f, [1, 2, 3]);
/// let mut back: MutableVec<i32> = f.thaw();
/// back.pop();
/// assert_eq!(back, [1, 2]);
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MutableVec<T> {
    pub(crate) v: Vec<T>,
}

wrap_iter! {
    /// Owning iterator of [`MutableVec`] / [`FrozenVec`].
    pub struct IntoIter<T>(vec::IntoIter<T>) -> T; double, exact
}

wrap_iter! {
    /// Draining iterator of [`MutableVec::drain`].
    pub struct Drain<'a, T>(vec::Drain<'a, T>) -> T; double, exact
}

impl<T> Default for MutableVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> MutableVec<T> {
    /// Empty; does not allocate.
    pub const fn new() -> Self {
        Self { v: Vec::new() }
    }
    /// Empty with room for `capacity` elements.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            v: Vec::with_capacity(capacity),
        }
    }
    /// Element count.
    pub fn len(&self) -> usize {
        self.v.len()
    }
    /// `true` when empty.
    pub fn is_empty(&self) -> bool {
        self.v.is_empty()
    }
    /// Allocated capacity.
    pub fn capacity(&self) -> usize {
        self.v.capacity()
    }
    /// Reserve room for `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        self.v.reserve(additional);
    }
    /// Fallible [`reserve`](Self::reserve).
    ///
    /// # Errors
    /// Capacity overflow or allocator failure; nothing changes.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.v.try_reserve(additional)
    }
    /// Release excess capacity.
    pub fn shrink_to_fit(&mut self) {
        self.v.shrink_to_fit();
    }
    /// Append.
    pub fn push(&mut self, value: T) {
        self.v.push(value);
    }
    /// Remove the last element.
    pub fn pop(&mut self) -> Option<T> {
        self.v.pop()
    }
    /// Insert at `index`, shifting later elements.
    ///
    /// # Panics
    /// If `index > len`.
    pub fn insert(&mut self, index: usize, value: T) {
        self.v.insert(index, value);
    }
    /// Remove at `index`, shifting later elements.
    ///
    /// # Panics
    /// If `index >= len`.
    pub fn remove(&mut self, index: usize) -> T {
        self.v.remove(index)
    }
    /// Remove at `index`, moving the last element into its place.
    ///
    /// # Panics
    /// If `index >= len`.
    pub fn swap_remove(&mut self, index: usize) -> T {
        self.v.swap_remove(index)
    }
    /// Keep the first `len` elements.
    pub fn truncate(&mut self, len: usize) {
        self.v.truncate(len);
    }
    /// Remove all elements.
    pub fn clear(&mut self) {
        self.v.clear();
    }
    /// Keep elements for which `f` returns `true`, in order.
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, f: F) {
        self.v.retain(f);
    }
    /// [`retain`](Self::retain) with mutable access.
    pub fn retain_mut<F: FnMut(&mut T) -> bool>(&mut self, f: F) {
        self.v.retain_mut(f);
    }
    /// Remove consecutive elements for which `same` holds.
    pub fn dedup_by<F: FnMut(&mut T, &mut T) -> bool>(&mut self, same: F) {
        self.v.dedup_by(same);
    }
    /// Remove consecutive elements with equal keys.
    pub fn dedup_by_key<K: PartialEq, F: FnMut(&mut T) -> K>(&mut self, key: F) {
        self.v.dedup_by_key(key);
    }
    /// Move all elements of `other` to the end of `self`.
    pub fn append(&mut self, other: &mut Self) {
        self.v.append(&mut other.v);
    }
    /// Split off `[at, len)` into a new sequence.
    ///
    /// # Panics
    /// If `at > len`.
    #[must_use]
    pub fn split_off(&mut self, at: usize) -> Self {
        Self {
            v: self.v.split_off(at),
        }
    }
    /// Remove and yield a range.
    ///
    /// # Panics
    /// On an out-of-bounds or inverted range.
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, T> {
        Drain(self.v.drain(range))
    }
    /// Resize with values produced by `f`.
    pub fn resize_with<F: FnMut() -> T>(&mut self, new_len: usize, f: F) {
        self.v.resize_with(new_len, f);
    }
    /// The elements.
    pub fn as_slice(&self) -> &[T] {
        &self.v
    }
    /// The elements, mutably (length cannot change through it).
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.v
    }

    /// Freeze into [`FrozenVec`]; drops excess capacity (no reallocation when
    /// `len == capacity`).
    #[must_use]
    pub fn freeze(self) -> FrozenVec<T> {
        FrozenVec::from(self.v)
    }
}

impl<T: Clone> MutableVec<T> {
    /// Append clones of `other`.
    pub fn extend_from_slice(&mut self, other: &[T]) {
        self.v.extend_from_slice(other);
    }
    /// Resize, filling with clones of `value`.
    pub fn resize(&mut self, new_len: usize, value: T) {
        self.v.resize(new_len, value);
    }
}

impl<T: PartialEq> MutableVec<T> {
    /// Remove consecutive duplicates.
    pub fn dedup(&mut self) {
        self.v.dedup();
    }
}

impl<T> MutableVec<T> {
    /// Adopt a native vector without reallocating or cloning elements.
    pub fn from_vec(vector: Vec<T>) -> Self {
        Self { v: vector }
    }

    /// Extract the native vector without reallocating or cloning elements.
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.v
    }
}

// --- explicit native conversions ------------------------------------------

impl<T> From<Vec<T>> for MutableVec<T> {
    fn from(v: Vec<T>) -> Self {
        Self { v }
    }
}
impl<T> From<MutableVec<T>> for Vec<T> {
    fn from(m: MutableVec<T>) -> Self {
        m.v
    }
}
impl<T> From<Box<[T]>> for MutableVec<T> {
    fn from(b: Box<[T]>) -> Self {
        Self { v: b.into_vec() }
    }
}
impl<T, const N: usize> From<[T; N]> for MutableVec<T> {
    fn from(a: [T; N]) -> Self {
        Self { v: Vec::from(a) }
    }
}
impl<T: Clone> From<&[T]> for MutableVec<T> {
    fn from(s: &[T]) -> Self {
        Self { v: s.to_vec() }
    }
}

impl<T> FromIterator<T> for MutableVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            v: iter.into_iter().collect(),
        }
    }
}
impl<T> Extend<T> for MutableVec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.v.extend(iter);
    }
}
impl<'a, T: Copy + 'a> Extend<&'a T> for MutableVec<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.v.extend(iter);
    }
}

impl<T> IntoIterator for MutableVec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter(self.v.into_iter())
    }
}
impl<'a, T> IntoIterator for &'a MutableVec<T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.v.iter()
    }
}
impl<'a, T> IntoIterator for &'a mut MutableVec<T> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.v.iter_mut()
    }
}

impl<T> Deref for MutableVec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.v
    }
}
impl<T> DerefMut for MutableVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.v
    }
}
impl<T> AsRef<[T]> for MutableVec<T> {
    fn as_ref(&self) -> &[T] {
        &self.v
    }
}
impl<T> AsMut<[T]> for MutableVec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        &mut self.v
    }
}
impl<T> Borrow<[T]> for MutableVec<T> {
    fn borrow(&self) -> &[T] {
        &self.v
    }
}
impl<T> BorrowMut<[T]> for MutableVec<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        &mut self.v
    }
}

impl<T: PartialEq<U>, U> PartialEq<[U]> for MutableVec<T> {
    fn eq(&self, other: &[U]) -> bool {
        self.v[..] == *other
    }
}
impl<T: PartialEq<U>, U> PartialEq<Vec<U>> for MutableVec<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.v[..] == other[..]
    }
}
impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for MutableVec<T> {
    fn eq(&self, other: &[U; N]) -> bool {
        self.v[..] == other[..]
    }
}

impl<T: fmt::Debug> fmt::Debug for MutableVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.v).finish()
    }
}

// --- plans and capabilities ------------------------------------------------

impl<T> FreezeWith<Compact> for MutableVec<T> {
    type Output = FrozenVec<T>;
    fn freeze_with(self, _: Compact) -> FrozenVec<T> {
        self.freeze()
    }
}

impl<T, const N: usize> FreezeWith<Inline<N>> for MutableVec<T> {
    type Output = InlineFrozenVec<T, N>;
    fn freeze_with(self, _: Inline<N>) -> Self::Output {
        InlineFrozenVec::from_vec(self.v)
    }
}

impl<T> Len for MutableVec<T> {
    fn len(&self) -> usize {
        self.v.len()
    }
}
impl<T> ElementType for MutableVec<T> {
    type Item = T;
}
impl<T> Elements for MutableVec<T> {
    type Iter<'a>
        = core::slice::Iter<'a, T>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.v.iter()
    }
}
impl<T> Push for MutableVec<T> {
    type Item = T;
    fn push(&mut self, value: T) {
        self.v.push(value);
    }
}
impl<T> TryPush for MutableVec<T> {
    type Item = T;
    type Error = TryReserveError;
    fn try_push(&mut self, value: T) -> Result<(), PushRejected<T, TryReserveError>> {
        TryPush::try_push(&mut self.v, value)
    }
}

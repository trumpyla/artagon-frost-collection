use alloc::boxed::Box;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::ops::Deref;

use super::{IntoIter, MutableVec};
use crate::read::{ElementType, Elements, Len};

/// Frozen contiguous sequence; representation is an exact-length `Box<[T]>`.
///
/// Contract: logical element order; equality, ordering and hashing are those
/// of `[T]`. Read access is `Deref<Target = [T]>`.
///
/// ```
/// # extern crate frost_collections_core as frost;
/// use frost::prelude::*;
/// let f: FrozenVec<_> = vec![1, 2, 3].compact();
/// assert_eq!(f.iter().sum::<i32>(), 6);
/// assert_eq!(f.get(1), Some(&2));          // slice API via Deref
/// let v: MutableVec<i32> = f.thaw();       // O(1): reuses the allocation
/// assert_eq!(v, [1, 2, 3]);
/// ```
///
/// ```compile_fail,E0594
/// # extern crate frost_collections_core as frost;
/// use frost::prelude::*;
/// let f: FrozenVec<_> = vec![1].compact();
/// f[0] = 2; // no DerefMut / IndexMut
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrozenVec<T> {
    pub(crate) items: Box<[T]>,
}

// Manual: `derive(Default)` would demand `T: Default`.
impl<T> Default for FrozenVec<T> {
    fn default() -> Self {
        Self {
            items: Box::new([]),
        }
    }
}

impl<T> FrozenVec<T> {
    /// Back to the paired mutable family. O(1); reuses the boxed allocation.
    /// Capacity equals length for non-zero-sized elements. Zero-sized elements
    /// follow Vec's `usize::MAX` capacity convention.
    #[must_use]
    pub fn thaw(self) -> MutableVec<T> {
        MutableVec::from(self.items.into_vec())
    }

    /// Interop: the elements as a `Vec<T>`. O(1).
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.items.into_vec()
    }

    /// The underlying boxed slice.
    #[must_use]
    pub fn into_boxed_slice(self) -> Box<[T]> {
        self.items
    }

    /// The elements.
    pub fn as_slice(&self) -> &[T] {
        &self.items
    }
}

/// Discards excess capacity (`Vec::into_boxed_slice`): no reallocation when
/// `len == capacity`, otherwise the allocator may shrink or move.
impl<T> From<Vec<T>> for FrozenVec<T> {
    fn from(v: Vec<T>) -> Self {
        Self {
            items: v.into_boxed_slice(),
        }
    }
}

impl<T> From<Box<[T]>> for FrozenVec<T> {
    fn from(items: Box<[T]>) -> Self {
        Self { items }
    }
}

impl<T, const N: usize> From<[T; N]> for FrozenVec<T> {
    fn from(a: [T; N]) -> Self {
        Self {
            items: Box::new(a) as Box<[T]>,
        }
    }
}

/// Collects into a `Vec` and then boxes: may reallocate once at the end.
impl<T> FromIterator<T> for FrozenVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            items: iter.into_iter().collect(),
        }
    }
}

impl<T> Deref for FrozenVec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.items
    }
}

impl<T> AsRef<[T]> for FrozenVec<T> {
    fn as_ref(&self) -> &[T] {
        &self.items
    }
}

impl<T: PartialEq<U>, U> PartialEq<[U]> for FrozenVec<T> {
    fn eq(&self, other: &[U]) -> bool {
        self.items[..] == *other
    }
}

impl<T: PartialEq<U>, U> PartialEq<Vec<U>> for FrozenVec<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.items[..] == other[..]
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for FrozenVec<T> {
    fn eq(&self, other: &[U; N]) -> bool {
        self.items[..] == other[..]
    }
}

/// Sound: `Eq`/`Ord`/`Hash` are exactly `[T]`'s.
impl<T> Borrow<[T]> for FrozenVec<T> {
    fn borrow(&self) -> &[T] {
        &self.items
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for FrozenVec<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.items.iter()).finish()
    }
}

impl<T> IntoIterator for FrozenVec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.thaw().into_iter()
    }
}

impl<'a, T> IntoIterator for &'a FrozenVec<T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<T> Len for FrozenVec<T> {
    fn len(&self) -> usize {
        self.items.len()
    }
}

impl<T> ElementType for FrozenVec<T> {
    type Item = T;
}

impl<T> Elements for FrozenVec<T> {
    type Iter<'a>
        = core::slice::Iter<'a, T>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.items.iter()
    }
}

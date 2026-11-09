//! `[T; N]` — available in `core`-only builds.

use core::ops::Deref;

use crate::frozen::sealed::Sealed;
use crate::{Eligible, FreezeWith, Frozen, Retain};

impl<T, const N: usize> Sealed for [T; N] {}
impl<T, const N: usize> Eligible for [T; N] {}

impl<T, const N: usize> FreezeWith<Retain> for [T; N] {
    type Output = Frozen<[T; N]>;
    #[inline]
    fn freeze_with(self, _: Retain) -> Self::Output {
        Frozen::new_unchecked(self)
    }
}

impl<T, const N: usize> Deref for Frozen<[T; N]> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &[T] {
        self.backend()
    }
}

impl<T, const N: usize> AsRef<[T]> for Frozen<[T; N]> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.backend()
    }
}

impl<T, const N: usize> Frozen<[T; N]> {
    /// The array by shared reference (arrays expose no `&self` mutation).
    #[inline]
    pub const fn as_array(&self) -> &[T; N] {
        self.backend()
    }
}

//! Fixed-capacity sequences work without `alloc`; overflow returns its value.

use arrayvec::ArrayVec;
use core::ops::Deref;

use crate::frozen::sealed::Sealed;
use crate::plan::LengthMismatch;
use crate::read::{ElementType, Elements, Len};
use crate::write::{PushRejected, TryPush};
use crate::{Eligible, ExactLen, FreezeWith, Frozen, Rejected, Retain, TryFreezeWith};

impl<T, const N: usize> Sealed for ArrayVec<T, N> {}
impl<T, const N: usize> Eligible for ArrayVec<T, N> {}
impl<T, const N: usize> Len for ArrayVec<T, N> {
    fn len(&self) -> usize {
        ArrayVec::len(self)
    }
}
impl<T, const N: usize> ElementType for ArrayVec<T, N> {
    type Item = T;
}
impl<T, const N: usize> Elements for ArrayVec<T, N> {
    type Iter<'a>
        = core::slice::Iter<'a, T>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.as_slice().iter()
    }
}
impl<T, const N: usize> FreezeWith<Retain> for ArrayVec<T, N> {
    type Output = Frozen<Self>;
    fn freeze_with(self, _: Retain) -> Self::Output {
        Frozen::new_unchecked(self)
    }
}
#[cfg(feature = "alloc")]
impl<T, const N: usize> FreezeWith<crate::Compact> for ArrayVec<T, N> {
    type Output = crate::family::FrozenVec<T>;
    fn freeze_with(self, _: crate::Compact) -> Self::Output {
        self.into_iter().collect()
    }
}
impl<T, const CAP: usize, const N: usize> TryFreezeWith<ExactLen<N>> for ArrayVec<T, CAP> {
    type Output = Frozen<[T; N]>;
    type Error = Rejected<Self, LengthMismatch>;
    fn try_freeze_with(self, _: ExactLen<N>) -> Result<Self::Output, Self::Error> {
        let actual = self.len();
        if actual != N {
            return Err(Rejected {
                input: self,
                error: LengthMismatch {
                    expected: N,
                    actual,
                },
            });
        }
        let mut values = self.into_iter();
        let array = core::array::from_fn(|_| {
            values
                .next()
                .expect("ArrayVec length was checked before moving elements")
        });
        Ok(Frozen::new_unchecked(array))
    }
}
impl<T, const N: usize> Deref for Frozen<ArrayVec<T, N>> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.backend().as_slice()
    }
}
impl<T, const N: usize> AsRef<[T]> for Frozen<ArrayVec<T, N>> {
    fn as_ref(&self) -> &[T] {
        self
    }
}
impl<T, const N: usize> Frozen<ArrayVec<T, N>> {
    /// Fixed capacity, including currently unused inline slots.
    pub const fn capacity(&self) -> usize {
        N
    }
}
impl<T, const N: usize> TryPush for ArrayVec<T, N> {
    type Item = T;
    type Error = arrayvec::CapacityError;
    fn try_push(&mut self, value: T) -> Result<(), PushRejected<T, Self::Error>> {
        ArrayVec::try_push(self, value).map_err(|error| PushRejected {
            value: error.element(),
            error: arrayvec::CapacityError::new(()),
        })
    }
}

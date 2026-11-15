//! Small vectors preserve their inline/spilled representation when sealed.

use core::ops::Deref;
use smallvec::{Array, CollectionAllocErr, SmallVec};

use crate::family::FrozenVec;
use crate::frozen::sealed::Sealed;
use crate::read::{ElementType, Elements, Len};
use crate::write::{Push, PushRejected, TryPush};
use crate::{Compact, Eligible, FreezeWith, Frozen, Retain};

impl<A: Array> Sealed for SmallVec<A> {}
impl<A: Array> Eligible for SmallVec<A> {}

impl<A: Array> Len for SmallVec<A> {
    fn len(&self) -> usize {
        SmallVec::len(self)
    }
}
impl<A: Array> ElementType for SmallVec<A> {
    type Item = A::Item;
}
impl<A: Array> Elements for SmallVec<A> {
    type Iter<'a>
        = core::slice::Iter<'a, A::Item>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.as_slice().iter()
    }
}
impl<A: Array> FreezeWith<Retain> for SmallVec<A> {
    type Output = Frozen<Self>;
    fn freeze_with(self, _: Retain) -> Self::Output {
        Frozen::new_unchecked(self)
    }
}
impl<A: Array> FreezeWith<Compact> for SmallVec<A> {
    type Output = FrozenVec<A::Item>;
    fn freeze_with(self, _: Compact) -> Self::Output {
        FrozenVec::from(self.into_boxed_slice())
    }
}
impl<A: Array> Deref for Frozen<SmallVec<A>> {
    type Target = [A::Item];
    fn deref(&self) -> &Self::Target {
        self.backend().as_slice()
    }
}
impl<A: Array> AsRef<[A::Item]> for Frozen<SmallVec<A>> {
    fn as_ref(&self) -> &[A::Item] {
        self
    }
}
impl<A: Array> Frozen<SmallVec<A>> {
    /// Whether the retained backend currently uses heap storage.
    pub fn spilled(&self) -> bool {
        self.backend().spilled()
    }
    /// Capacity of the retained inline buffer or allocation.
    pub fn capacity(&self) -> usize {
        self.backend().capacity()
    }
}
impl<A: Array> Push for SmallVec<A> {
    type Item = A::Item;
    fn push(&mut self, value: Self::Item) {
        SmallVec::push(self, value);
    }
}
impl<A: Array> TryPush for SmallVec<A> {
    type Item = A::Item;
    type Error = CollectionAllocErr;
    fn try_push(&mut self, value: Self::Item) -> Result<(), PushRejected<Self::Item, Self::Error>> {
        if let Err(error) = self.try_reserve(1) {
            return Err(PushRejected { value, error });
        }
        self.push(value);
        Ok(())
    }
}

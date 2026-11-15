//! Compressed byte-string maps and sets. Lookups decode values; streams lend keys.
//!
//! A stream's key buffer is valid only until its next advance. It cannot be
//! exposed as `Entries` or `Elements`. Use the native `Streamer` API instead.
//! Retaining freeze is restricted to audited immutable byte storage, such as
//! `Vec<u8>`, `Box<[u8]>`, or arrays, not arbitrary user-defined `AsRef` types.

use crate::frozen::sealed::Sealed;
use crate::read::{Len, LookupValue, SetContains};
use crate::{Eligible, FreezeWith, Frozen, Retain};

impl<D: AsRef<[u8]>> Len for fst::Map<D> {
    fn len(&self) -> usize {
        fst::Map::len(self)
    }
}
impl<D: AsRef<[u8]>, Q: ?Sized + AsRef<[u8]>> LookupValue<Q> for fst::Map<D> {
    type Value = u64;
    fn get_value(&self, query: &Q) -> Option<u64> {
        self.get(query)
    }
}
impl<D: AsRef<[u8]>> Len for fst::Set<D> {
    fn len(&self) -> usize {
        fst::Set::len(self)
    }
}
impl<D: AsRef<[u8]>, Q: ?Sized + AsRef<[u8]>> SetContains<Q> for fst::Set<D> {
    fn contains(&self, query: &Q) -> bool {
        fst::Set::contains(self, query)
    }
}

macro_rules! eligible {
    ($($name:ident);* $(;)?) => {$(
        impl<D: AsRef<[u8]> + Eligible> Sealed for fst::$name<D> {}
        impl<D: AsRef<[u8]> + Eligible> Eligible for fst::$name<D> {}
        impl<D: AsRef<[u8]> + Eligible> FreezeWith<Retain> for fst::$name<D> {
            type Output = Frozen<Self>;
            fn freeze_with(self, _: Retain) -> Self::Output { Frozen::new_unchecked(self) }
        }
    )*};
}
eligible! { Map; Set; }

impl<D: AsRef<[u8]>> Frozen<fst::Map<D>> {
    /// Decode the associated `u64` without allocating a key.
    pub fn get<Q: ?Sized + AsRef<[u8]>>(&self, query: &Q) -> Option<u64> {
        self.backend().get(query)
    }
    /// Borrow a lexicographically ordered, lending key/value stream.
    pub fn stream(&self) -> fst::map::Stream<'_> {
        self.backend().stream()
    }
}
impl<D: AsRef<[u8]>> Frozen<fst::Set<D>> {
    /// Test byte-string membership.
    pub fn contains<Q: ?Sized + AsRef<[u8]>>(&self, query: &Q) -> bool {
        self.backend().contains(query)
    }
    /// Borrow a lexicographically ordered, lending key stream.
    pub fn stream(&self) -> fst::set::Stream<'_> {
        self.backend().stream()
    }
}

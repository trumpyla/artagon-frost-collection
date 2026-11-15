//! `borsh` 1.x (feature `borsh-1`): canonical binary encoding.
//!
//! * Sequences: `u32` length + elements (identical to `Vec<T>`).
//! * Maps: `u32` length + entries in **strictly ascending key order** for every
//!   family, hash families included (identical bytes to borsh's own
//!   `BTreeMap`/`HashMap` encoding). Hash families therefore need `K: Ord` to
//!   encode.
//! * Family decoding is **always strict**: non-ascending or duplicate keys
//!   are rejected with `InvalidData`, independent of `de_strict_order`.
//!   Exact `Frozen<B>` wrappers retain the backend's decoding rules instead.
//! * Canonical ordering does not make user-defined encodings canonical.
//!   Equal keys may carry differently encoded metadata, and custom element
//!   serializers remain responsible for their own determinism.
//! * Family decoding stages input before checking ordering and uniqueness.
//!   It does not impose a total allocation or input-byte budget.
//! * Zero-sized key types are rejected, as borsh does for maps.

use alloc::vec::Vec;
#[cfg(feature = "std")]
use core::hash::{BuildHasher, Hash};

use ::borsh::io::{Error, ErrorKind, Read, Result, Write};
use ::borsh::{BorshDeserialize, BorshSerialize};

use crate::adaptive::{AdaptiveOrdered, AdaptiveOrderedMap, InlineFrozenVec, InlineVec};
use crate::family::{
    FrozenOrderedMap, FrozenOrderedSet, FrozenVec, MutableOrderedMap, MutableOrderedSet, MutableVec,
};
use crate::{Eligible, Frozen};

const NOT_CANONICAL: &str = "keys or members are not unique and strictly ascending";
const ZST_KEY: &str = "zero-sized key types are not supported";

fn check_zst<K>() -> Result<()> {
    if core::mem::size_of::<K>() == 0 {
        return Err(Error::new(ErrorKind::InvalidData, ZST_KEY));
    }
    Ok(())
}

fn write_len<W: Write>(len: usize, w: &mut W) -> Result<()> {
    u32::try_from(len)
        .map_err(|_| Error::new(ErrorKind::InvalidData, "length exceeds u32::MAX"))?
        .serialize(w)
}

/// Write `len` + entries already in ascending key order.
fn write_ascending<'a, K, V, I, W>(len: usize, entries: I, w: &mut W) -> Result<()>
where
    K: BorshSerialize + 'a,
    V: BorshSerialize + 'a,
    I: Iterator<Item = (&'a K, &'a V)>,
    W: Write,
{
    check_zst::<K>()?;
    write_len(len, w)?;
    for (k, v) in entries {
        k.serialize(w)?;
        v.serialize(w)?;
    }
    Ok(())
}

/// Read entries and require strictly ascending keys.
fn read_ascending<K: BorshDeserialize + Ord, V: BorshDeserialize, R: Read>(
    r: &mut R,
) -> Result<Vec<(K, V)>> {
    check_zst::<K>()?;
    let v = Vec::<(K, V)>::deserialize_reader(r)?;
    if v.windows(2).any(|w| w[0].0 >= w[1].0) {
        return Err(Error::new(ErrorKind::InvalidData, NOT_CANONICAL));
    }
    Ok(v)
}

impl<B: BorshSerialize> BorshSerialize for Frozen<B> {
    fn serialize<W: Write>(&self, w: &mut W) -> Result<()> {
        self.backend().serialize(w)
    }
}
/// Backend semantics (exact-wrapper rule); eligibility-gated.
impl<B: Eligible + BorshDeserialize> BorshDeserialize for Frozen<B> {
    fn deserialize_reader<R: Read>(r: &mut R) -> Result<Self> {
        B::deserialize_reader(r).map(Frozen::new_unchecked)
    }
}

// --- sequences --------------------------------------------------------------

macro_rules! seq_borsh {
    ($([$($g:tt)*] $ty:ty => |$v:ident| $build:expr),* $(,)?) => {$(
        impl<$($g)*> BorshSerialize for $ty
        where
            T: BorshSerialize,
        {
            fn serialize<W: Write>(&self, w: &mut W) -> Result<()> {
                self.as_slice().serialize(w)
            }
        }
        impl<$($g)*> BorshDeserialize for $ty
        where
            T: BorshDeserialize,
        {
            fn deserialize_reader<R: Read>(r: &mut R) -> Result<Self> {
                let $v = Vec::<T>::deserialize_reader(r)?;
                Ok($build)
            }
        }
    )*};
}

seq_borsh! {
    [T] MutableVec<T> => |v| MutableVec::from(v),
    [T] FrozenVec<T> => |v| FrozenVec::from(v),
    [T, const N: usize] InlineVec<T, N> => |v| InlineVec::from_vec(v),
    [T, const N: usize] InlineFrozenVec<T, N> => |v| InlineFrozenVec::from_vec(v),
}

impl<T: BorshSerialize> BorshSerialize for FrozenOrderedSet<T> {
    fn serialize<W: Write>(&self, w: &mut W) -> Result<()> {
        check_zst::<T>()?;
        self.as_slice().serialize(w)
    }
}
impl<T: BorshDeserialize + Ord> BorshDeserialize for FrozenOrderedSet<T> {
    fn deserialize_reader<R: Read>(r: &mut R) -> Result<Self> {
        check_zst::<T>()?;
        FrozenOrderedSet::try_from_sorted_unique(Vec::<T>::deserialize_reader(r)?)
            .map_err(|_| Error::new(ErrorKind::InvalidData, NOT_CANONICAL))
    }
}

impl<T: BorshSerialize> BorshSerialize for MutableOrderedSet<T> {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        check_zst::<T>()?;
        write_len(self.len(), writer)?;
        for member in self {
            member.serialize(writer)?;
        }
        Ok(())
    }
}

impl<T: BorshDeserialize + Ord> BorshDeserialize for MutableOrderedSet<T> {
    fn deserialize_reader<R: Read>(reader: &mut R) -> Result<Self> {
        FrozenOrderedSet::<T>::deserialize_reader(reader).map(FrozenOrderedSet::thaw)
    }
}

// --- ordered maps -----------------------------------------------------------

macro_rules! ordered_borsh {
    ($($ty:ident => |$v:ident| $build:expr),* $(,)?) => {$(
        impl<K: BorshSerialize, V: BorshSerialize> BorshSerialize for $ty<K, V> {
            fn serialize<W: Write>(&self, w: &mut W) -> Result<()> {
                write_ascending(self.len(), self.iter(), w)
            }
        }
        impl<K: BorshDeserialize + Ord, V: BorshDeserialize> BorshDeserialize for $ty<K, V> {
            fn deserialize_reader<R: Read>(r: &mut R) -> Result<Self> {
                let $v = read_ascending::<K, V, R>(r)?;
                Ok($build)
            }
        }
    )*};
}

ordered_borsh! {
    FrozenOrderedMap => |v| FrozenOrderedMap::from_sorted_unique_unchecked(v),
    MutableOrderedMap => |v| FrozenOrderedMap::from_sorted_unique_unchecked(v).thaw(),
    AdaptiveOrderedMap => |v| AdaptiveOrderedMap::from_frozen(
        FrozenOrderedMap::from_sorted_unique_unchecked(v),
        AdaptiveOrdered::default(),
    ),
}

// --- hash maps: sorted on encode ---------------------------------------------

#[cfg(feature = "std")]
mod hash_impls {
    use super::{
        BorshDeserialize, BorshSerialize, BuildHasher, Hash, Read, Result, Vec, Write,
        read_ascending, write_ascending,
    };
    use crate::adaptive::{AdaptiveFrozenHashMap, AdaptiveHash, AdaptiveHashMap};
    use crate::family::{FrozenHashMap, FrozenHashSet, MutableHashMap, MutableHashSet};

    /// Sort for canonical order. Keys distinct under `Eq` but equal under
    /// `Ord` would encode bytes that strict decoding rejects, so they are
    /// refused here instead.
    fn sorted<'a, K: Ord, V>(
        it: impl Iterator<Item = (&'a K, &'a V)>,
    ) -> Result<Vec<(&'a K, &'a V)>> {
        let mut v: Vec<_> = it.collect();
        v.sort_unstable_by(|a, b| a.0.cmp(b.0));
        if v.windows(2).any(|w| w[0].0 >= w[1].0) {
            return Err(super::Error::new(
                super::ErrorKind::InvalidData,
                "keys are not totally ordered by `Ord` (Eq and Ord disagree)",
            ));
        }
        Ok(v)
    }

    macro_rules! hash_borsh {
        ($($ty:ident where [$($b:tt)*] => |$v:ident| $build:expr),* $(,)?) => {$(
            impl<K: BorshSerialize + Ord, V: BorshSerialize, S> BorshSerialize for $ty<K, V, S> {
                fn serialize<W: Write>(&self, w: &mut W) -> Result<()> {
                    write_ascending(self.len(), sorted(self.iter())?.into_iter(), w)
                }
            }
            impl<K, V, S> BorshDeserialize for $ty<K, V, S>
            where
                K: BorshDeserialize + Ord + Eq + Hash,
                V: BorshDeserialize,
                $($b)*
            {
                fn deserialize_reader<R: Read>(r: &mut R) -> Result<Self> {
                    let $v = read_ascending::<K, V, R>(r)?;
                    let expected_len = $v.len();
                    let decoded: Self = $build;
                    if decoded.len() != expected_len {
                        return Err(super::Error::new(super::ErrorKind::InvalidData, super::NOT_CANONICAL));
                    }
                    Ok(decoded)
                }
            }
        )*};
    }

    hash_borsh! {
        MutableHashMap where [S: BuildHasher + Default] => |v| v.into_iter().collect(),
        FrozenHashMap where [S: BuildHasher + Default] => |v| v.into_iter().collect(),
        AdaptiveHashMap where [S: BuildHasher + Clone + Default] => |v| v.into_iter().collect(),
        AdaptiveFrozenHashMap where [S: BuildHasher + Clone + Default]
            => |v| AdaptiveFrozenHashMap::from_map(v.into_iter().collect(), AdaptiveHash::default()),
    }

    macro_rules! hash_set_borsh {
        ($($ty:ident),* $(,)?) => {$(
            impl<T: BorshSerialize + Ord, S> BorshSerialize for $ty<T, S> {
                fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
                    super::check_zst::<T>()?;
                    let mut members: Vec<_> = self.iter().collect();
                    members.sort_unstable();
                    if members.windows(2).any(|pair| pair[0] >= pair[1]) {
                        return Err(super::Error::new(super::ErrorKind::InvalidData, super::NOT_CANONICAL));
                    }
                    super::write_len(members.len(), writer)?;
                    for member in members {
                        member.serialize(writer)?;
                    }
                    Ok(())
                }
            }

            impl<T, S> BorshDeserialize for $ty<T, S>
            where
                T: BorshDeserialize + Ord + Eq + Hash,
                S: BuildHasher + Default,
            {
                fn deserialize_reader<R: Read>(reader: &mut R) -> Result<Self> {
                    super::check_zst::<T>()?;
                    let members = Vec::<T>::deserialize_reader(reader)?;
                    if members.windows(2).any(|pair| pair[0] >= pair[1]) {
                        return Err(super::Error::new(super::ErrorKind::InvalidData, super::NOT_CANONICAL));
                    }
                    let expected_len = members.len();
                    let decoded: Self = members.into_iter().collect();
                    if decoded.len() != expected_len {
                        return Err(super::Error::new(super::ErrorKind::InvalidData, super::NOT_CANONICAL));
                    }
                    Ok(decoded)
                }
            }
        )*};
    }

    hash_set_borsh!(MutableHashSet, FrozenHashSet);
}

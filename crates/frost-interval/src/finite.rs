//! `FiniteSet<V>`: subsets of a small finite domain as one `u128` bitset.

use crate::domain::FiniteDomain;
use core::marker::PhantomData;

/// A subset of a [`FiniteDomain`] with at most 128 values.
/// Complement is relative to the domain's universe; no order is implied.
pub struct FiniteSet<V: FiniteDomain> {
    bits: u128,
    _v: PhantomData<V>,
}

#[allow(
    clippy::expl_impl_clone_on_copy,
    reason = "a derive would require `V: Clone`"
)]
impl<V: FiniteDomain> Clone for FiniteSet<V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<V: FiniteDomain> Copy for FiniteSet<V> {}
impl<V: FiniteDomain> PartialEq for FiniteSet<V> {
    fn eq(&self, o: &Self) -> bool {
        self.bits == o.bits
    }
}
impl<V: FiniteDomain> Eq for FiniteSet<V> {}
impl<V: FiniteDomain + core::fmt::Debug> core::fmt::Debug for FiniteSet<V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<V: FiniteDomain> FiniteSet<V> {
    const MASK: u128 = {
        assert!(
            V::CARDINALITY <= 128,
            "FiniteSet supports at most 128 values"
        );
        if V::CARDINALITY == 128 {
            u128::MAX
        } else {
            (1u128 << V::CARDINALITY) - 1
        }
    };

    /// The empty set.
    pub const fn empty() -> Self {
        let _ = Self::MASK; // force the cardinality check
        FiniteSet {
            bits: 0,
            _v: PhantomData,
        }
    }
    /// The set of every value of the domain.
    pub const fn all() -> Self {
        FiniteSet {
            bits: Self::MASK,
            _v: PhantomData,
        }
    }
    /// Bit of `v`. An index outside the universe is a law violation of the
    /// `FiniteDomain` impl; it panics here and never corrupts the set.
    #[inline]
    fn bit(v: &V) -> u128 {
        let i = v.index();
        assert!(
            i < V::CARDINALITY,
            "FiniteDomain law: index {i} is outside 0..{}",
            V::CARDINALITY
        );
        1u128 << i
    }
    #[must_use]
    /// This set with `v` added.
    pub fn with(self, v: &V) -> Self {
        FiniteSet {
            bits: self.bits | Self::bit(v),
            _v: PhantomData,
        }
    }
    #[inline]
    /// `true` if `v` is a member.
    pub fn contains(&self, v: &V) -> bool {
        self.bits & Self::bit(v) != 0
    }
    #[must_use]
    /// The union of `self` and `o`.
    pub fn union(self, o: Self) -> Self {
        FiniteSet {
            bits: self.bits | o.bits,
            _v: PhantomData,
        }
    }
    #[must_use]
    /// The intersection of `self` and `o`.
    pub fn intersection(self, o: Self) -> Self {
        FiniteSet {
            bits: self.bits & o.bits,
            _v: PhantomData,
        }
    }
    #[must_use]
    /// The members of `self` that are not members of `o`.
    pub fn difference(self, o: Self) -> Self {
        FiniteSet {
            bits: self.bits & !o.bits,
            _v: PhantomData,
        }
    }
    #[must_use]
    /// The members of exactly one of `self` and `o`.
    pub fn symmetric_difference(self, o: Self) -> Self {
        FiniteSet {
            bits: self.bits ^ o.bits,
            _v: PhantomData,
        }
    }
    /// Complement within the domain's universe.
    #[must_use]
    pub fn complement(self) -> Self {
        FiniteSet {
            bits: !self.bits & Self::MASK,
            _v: PhantomData,
        }
    }
    /// The number of members. The type is `u128`, as for
    /// `FrozenIntervalSet::count`.
    pub fn count(&self) -> u128 {
        u128::from(self.bits.count_ones())
    }
    /// `true` if the set has no members.
    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }
    /// The members in index order.
    pub fn iter(&self) -> impl Iterator<Item = V> + '_ {
        let mut b = self.bits;
        core::iter::from_fn(move || {
            if b == 0 {
                return None;
            }
            let i = b.trailing_zeros() as usize;
            b &= b - 1;
            // `None` here is a law violation; iteration then stops early.
            V::from_index(i)
        })
    }
}

impl<V: FiniteDomain> FromIterator<V> for FiniteSet<V> {
    fn from_iter<I: IntoIterator<Item = V>>(it: I) -> Self {
        it.into_iter().fold(Self::empty(), |s, v| s.with(&v))
    }
}

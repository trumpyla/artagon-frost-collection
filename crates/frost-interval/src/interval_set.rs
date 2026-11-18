//! `IntervalSet<V>`: a mutable set of values of an [`OrderedDomain`].
//!
//! It is the mutable counterpart of [`FrozenIntervalSet`]. It stores the same
//! canonical boundaries, but in a [`BoundaryTree`](crate::tree::BoundaryTree),
//! so one insert or remove costs O(log n) instead of a rebuild. Like the
//! frozen set, it is a thin typed wrapper over a domain-free kernel.
//!
//! Insert and remove of one value are two toggles: the value `v` is the
//! interval `[Below(v), Above(v))`, and the symmetric difference with an
//! interval toggles its two cuts. The result is canonical with no extra step:
//! adjacent intervals join, and a removed value splits its interval.

use crate::cut::Cut;
use crate::domain::{BoundariesOf, Density, OrderedDomain};
use crate::kernel::Boundaries;
use crate::set::FrozenIntervalSet;
use crate::tree::RawTreeSet;
use core::marker::PhantomData;

/// The owned boundary type of a domain: its word, or its word and a side bit.
pub type BoundaryOf<V> = <BoundariesOf<V> as Boundaries>::Owned;

/// A mutable set of values of a totally ordered domain, stored as disjoint intervals.
///
/// It keeps the canonical boundaries of `FrozenIntervalSet` in a B+tree, so a
/// single-value insert or remove costs `O(log n)` in the number of boundaries.
pub struct IntervalSet<V: OrderedDomain> {
    raw: RawTreeSet<BoundaryOf<V>>,
    _v: PhantomData<V>,
}

impl<V: OrderedDomain> Clone for IntervalSet<V> {
    fn clone(&self) -> Self {
        IntervalSet {
            raw: self.raw.clone(),
            _v: PhantomData,
        }
    }
}
impl<V: OrderedDomain> Default for IntervalSet<V> {
    fn default() -> Self {
        Self::empty()
    }
}
impl<V: OrderedDomain> PartialEq for IntervalSet<V> {
    fn eq(&self, o: &Self) -> bool {
        self.raw.starts_inside == o.raw.starts_inside
            && self.raw.tree.to_vec() == o.raw.tree.to_vec()
    }
}
impl<V: OrderedDomain + core::fmt::Debug> core::fmt::Debug for IntervalSet<V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.freeze().fmt(f)
    }
}

impl<V: OrderedDomain> IntervalSet<V> {
    /// The empty set.
    pub fn empty() -> Self {
        IntervalSet {
            raw: RawTreeSet::empty(),
            _v: PhantomData,
        }
    }
    /// The whole declared domain.
    pub fn all() -> Self {
        IntervalSet {
            raw: RawTreeSet::all(),
            _v: PhantomData,
        }
    }

    /// One descent of the tree.
    #[inline]
    pub fn contains(&self, x: &V) -> bool {
        self.raw.inside(<V::Density as Density<V>>::tree_segment_of(
            &self.raw.tree,
            x,
        ))
    }

    /// Adds `v`. Returns `false` if it was already a member.
    pub fn insert(&mut self, v: V) -> bool {
        if self.contains(&v) {
            return false;
        }
        self.toggle_value(v);
        true
    }

    /// Removes `v`. Returns `false` if it was not a member.
    pub fn remove(&mut self, v: &V) -> bool {
        if !self.contains(v) {
            return false;
        }
        self.toggle_value(v.clone());
        true
    }

    /// Symmetric difference with `{v}`: toggles the cuts below and above `v`.
    fn toggle_value(&mut self, v: V) {
        let below = <V::Density as Density<V>>::boundary(Cut::Below(v.clone()).canonical());
        let above = <V::Density as Density<V>>::boundary(Cut::Above(v).canonical());
        self.raw.toggle(below);
        self.raw.toggle(above);
    }

    /// The complement in the declared domain, in O(1).
    #[must_use]
    pub fn complement(mut self) -> Self {
        self.raw.starts_inside = !self.raw.starts_inside;
        self
    }

    /// `true` if the set has no members.
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }
    /// `true` if every value of the domain is a member.
    pub fn is_all(&self) -> bool {
        self.raw.is_all()
    }
    /// The number of stored boundaries.
    pub fn boundary_count(&self) -> usize {
        self.raw.tree.len()
    }
    /// The number of maximal disjoint intervals.
    pub fn interval_count(&self) -> usize {
        self.raw.interval_count()
    }
    /// The heap memory that the tree uses, in bytes.
    pub fn heap_bytes(&self) -> usize {
        self.raw.tree.heap_bytes()
    }

    /// The same set as a [`FrozenIntervalSet`], in O(n).
    pub fn freeze(&self) -> FrozenIntervalSet<V> {
        FrozenIntervalSet::from_raw(self.raw.to_raw_set())
    }
}

impl<V: OrderedDomain> From<&FrozenIntervalSet<V>> for IntervalSet<V> {
    /// O(n): the boundaries are already sorted.
    fn from(s: &FrozenIntervalSet<V>) -> Self {
        IntervalSet {
            raw: RawTreeSet::from_raw_set(&s.raw),
            _v: PhantomData,
        }
    }
}
impl<V: OrderedDomain> From<FrozenIntervalSet<V>> for IntervalSet<V> {
    fn from(s: FrozenIntervalSet<V>) -> Self {
        Self::from(&s)
    }
}

impl<V: OrderedDomain> Extend<V> for IntervalSet<V> {
    fn extend<I: IntoIterator<Item = V>>(&mut self, it: I) {
        for v in it {
            self.insert(v);
        }
    }
}
impl<V: OrderedDomain> FromIterator<V> for IntervalSet<V> {
    fn from_iter<I: IntoIterator<Item = V>>(it: I) -> Self {
        let mut s = Self::empty();
        s.extend(it);
        s
    }
}

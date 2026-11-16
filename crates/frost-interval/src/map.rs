//! `FrozenIntervalMap<V, T>`: an immutable piecewise-constant map from an ordered
//! domain to values.
//!
//! A thin typed wrapper over the domain-free kernel [`RawMap`]: `n` sorted
//! boundaries cut the domain into `n + 1` segments, and each segment has one
//! value or none. Adjacent segments always differ, so equal neighbors are
//! coalesced and each map has one stored form.
//!
//! - `get(x)` is one search for the segment of `x`.
//! - Every binary operation is one linear pass of [`FrozenIntervalMap::merge_with`]
//!   with a caller-chosen combine function.

use crate::cut::Cut;
use crate::domain::{BoundariesOf, Density, OrderedDomain};
use crate::interval::{Interval, IntoInterval};
use crate::kernel::{Boundaries, RawMap};
use crate::set::FrozenIntervalSet;
use core::marker::PhantomData;

/// An immutable map from the disjoint intervals of a totally ordered domain to values.
///
/// Adjacent intervals always have different values, so the representation is canonical.
pub struct FrozenIntervalMap<V: OrderedDomain, T> {
    raw: RawMap<BoundariesOf<V>, T>,
    _v: PhantomData<V>,
}

impl<V: OrderedDomain, T: Clone> Clone for FrozenIntervalMap<V, T> {
    fn clone(&self) -> Self {
        Self::from_raw(self.raw.clone())
    }
}
impl<V: OrderedDomain, T: PartialEq> PartialEq for FrozenIntervalMap<V, T> {
    fn eq(&self, o: &Self) -> bool {
        self.raw == o.raw
    }
}
impl<V: OrderedDomain + core::fmt::Debug, T: core::fmt::Debug> core::fmt::Debug
    for FrozenIntervalMap<V, T>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_map().entries(self.entries()).finish()
    }
}

impl<V: OrderedDomain, T> FrozenIntervalMap<V, T> {
    #[inline]
    fn from_raw(raw: RawMap<BoundariesOf<V>, T>) -> Self {
        FrozenIntervalMap {
            raw,
            _v: PhantomData,
        }
    }
    /// The empty map.
    pub fn empty() -> Self {
        Self::from_raw(RawMap::empty())
    }
    #[inline]
    fn segment_of(&self, x: &V) -> usize {
        <V::Density as Density<V>>::segment_of(&self.raw.bounds, x)
    }
    /// The value at `x`.
    #[inline]
    pub fn get(&self, x: &V) -> Option<&T> {
        self.raw.value(self.segment_of(x))
    }
    /// `true` if `x` has a value.
    pub fn contains(&self, x: &V) -> bool {
        self.get(x).is_some()
    }
    /// `true` if the map has no entries.
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }
    /// Number of entries.
    pub fn entry_count(&self) -> usize {
        self.raw.entry_count()
    }
    /// The number of stored boundaries.
    pub fn boundary_count(&self) -> usize {
        self.raw.bounds.len()
    }

    fn segment_interval(&self, i: usize) -> Interval<V> {
        let cut = |j: usize| <V::Density as Density<V>>::cut(self.raw.bounds.get(j));
        let lo = if i == 0 { Cut::BelowAll } else { cut(i - 1) };
        let hi = if i == self.raw.bounds.len() {
            Cut::AboveAll
        } else {
            cut(i)
        };
        Interval::new(lo, hi)
    }

    /// Entries in order. Neighbors with equal values are already one entry.
    pub fn entries(&self) -> impl Iterator<Item = (Interval<V>, &T)> + '_ {
        (0..=self.raw.bounds.len())
            .filter_map(move |i| self.raw.value(i).map(|t| (self.segment_interval(i), t)))
    }
    /// The entry that holds `x`.
    pub fn get_entry(&self, x: &V) -> Option<(Interval<V>, &T)> {
        let i = self.segment_of(x);
        self.raw.value(i).map(|t| (self.segment_interval(i), t))
    }
    /// The set of mapped values of the domain.
    pub fn key_set(&self) -> FrozenIntervalSet<V> {
        FrozenIntervalSet::from_raw(self.raw.key_set())
    }
    /// The unmapped part of the domain.
    pub fn gaps(&self) -> FrozenIntervalSet<V> {
        self.key_set().complement()
    }
}

impl<V: OrderedDomain, T: Clone + PartialEq> FrozenIntervalMap<V, T> {
    /// Builds from `(interval, value)` entries. Where entries overlap, the later
    /// entry wins (Guava `put` order). O(n log n).
    pub fn from_entries<I, R>(it: I) -> Self
    where
        I: IntoIterator<Item = (R, T)>,
        R: IntoInterval<V>,
    {
        Self::from_raw(RawMap::from_entries(it.into_iter().map(|(r, t)| {
            let (lo, hi) = r.into_interval().into_cuts();
            (
                <V::Density as Density<V>>::boundary(lo),
                <V::Density as Density<V>>::boundary(hi),
                t,
            )
        })))
    }
    /// A set as a map: every member maps to `value`.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "the caller gives the value away; the map stores clones of it"
    )]
    pub fn from_set(set: &FrozenIntervalSet<V>, value: T) -> Self {
        Self::from_raw(RawMap::from_set(&set.raw, &value))
    }

    /// The general binary operation: one linear pass over both maps. `f` gets
    /// the two values of each segment and returns the value of the result.
    /// `f(None, None)` should be `None`, or the result maps the whole domain.
    pub fn merge_with<U, W: PartialEq>(
        &self,
        o: &FrozenIntervalMap<V, U>,
        f: impl FnMut(Option<&T>, Option<&U>) -> Option<W>,
    ) -> FrozenIntervalMap<V, W> {
        FrozenIntervalMap::from_raw(self.raw.merge_with(&o.raw, f))
    }
    /// `o` on top of `self`: where both map a value, `o` wins.
    #[must_use]
    pub fn overlay(&self, o: &Self) -> Self {
        Self::from_raw(self.raw.overlay(&o.raw))
    }
    /// Union with aggregation on overlap (Boost.ICL style).
    #[must_use]
    pub fn union_with(&self, o: &Self, f: impl FnMut(&T, &T) -> T) -> Self {
        Self::from_raw(self.raw.union_with(&o.raw, f))
    }
    /// The part where both maps have a value, combined with `f`.
    pub fn intersection_with<U, W>(
        &self,
        o: &FrozenIntervalMap<V, U>,
        f: impl FnMut(&T, &U) -> W,
    ) -> FrozenIntervalMap<V, W>
    where
        W: PartialEq,
    {
        FrozenIntervalMap::from_raw(self.raw.intersection_with(&o.raw, f))
    }
    /// The entries cut down to `set`.
    #[must_use]
    pub fn restrict(&self, set: &FrozenIntervalSet<V>) -> Self {
        Self::from_raw(self.raw.restrict(&set.raw))
    }
    /// The entries with `set` removed.
    #[must_use]
    pub fn remove(&self, set: &FrozenIntervalSet<V>) -> Self {
        Self::from_raw(self.raw.remove(&set.raw))
    }
    /// The entries cut down to one interval.
    #[must_use]
    pub fn overlapping(&self, r: impl IntoInterval<V>) -> Self {
        self.restrict(&FrozenIntervalSet::of(r))
    }
    /// Maps every value; neighbors that become equal are coalesced.
    pub fn map_values<W: PartialEq>(&self, f: impl FnMut(&T) -> W) -> FrozenIntervalMap<V, W> {
        FrozenIntervalMap::from_raw(self.raw.map_values(f))
    }
}

impl<V: OrderedDomain, T: Clone + PartialEq, R: IntoInterval<V>> FromIterator<(R, T)>
    for FrozenIntervalMap<V, T>
{
    fn from_iter<I: IntoIterator<Item = (R, T)>>(it: I) -> Self {
        Self::from_entries(it)
    }
}

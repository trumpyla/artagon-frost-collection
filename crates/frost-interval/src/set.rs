//! `FrozenIntervalSet<V>`: an immutable set of values of an [`OrderedDomain`],
//! stored as sorted boundaries of words.
//!
//! This type is a thin typed wrapper. It converts cuts of `V` to boundaries
//! and calls the domain-free kernel [`RawSet`]. The domain is type-level, so
//! the set stores only the kernel and `PhantomData<V>`. Complement is relative
//! to the declared domain.

use crate::cut::Cut;
use crate::domain::{
    BoundariesOf, CoverPreserving, CyclicDomain, Density, DiscreteDomain, Extrema, MachineWord,
    OrderedDomain, OrderedDomainExt, Ranked,
};
use crate::interval::{Interval, IntoInterval};
use crate::kernel::{Boundaries, RawSet};
use alloc::vec::Vec;
use core::marker::PhantomData;

/// An immutable set of values of a totally ordered domain, stored as disjoint intervals.
///
/// The set stores canonical boundaries as words, so two sets with the same
/// members are equal.
pub struct FrozenIntervalSet<V: OrderedDomain> {
    pub(crate) raw: RawSet<BoundariesOf<V>>,
    _v: PhantomData<V>,
}

impl<V: OrderedDomain> Clone for FrozenIntervalSet<V> {
    fn clone(&self) -> Self {
        Self::from_raw(self.raw.clone())
    }
}
impl<V: OrderedDomain> PartialEq for FrozenIntervalSet<V> {
    fn eq(&self, o: &Self) -> bool {
        self.raw == o.raw
    }
}
impl<V: OrderedDomain> Eq for FrozenIntervalSet<V> where BoundariesOf<V>: Eq {}
impl<V: OrderedDomain + core::fmt::Debug> core::fmt::Debug for FrozenIntervalSet<V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.intervals()).finish()
    }
}

impl<V: OrderedDomain> FrozenIntervalSet<V> {
    #[inline]
    pub(crate) fn from_raw(raw: RawSet<BoundariesOf<V>>) -> Self {
        FrozenIntervalSet {
            raw,
            _v: PhantomData,
        }
    }
    /// The empty set.
    pub fn empty() -> Self {
        Self::from_raw(RawSet::empty())
    }
    /// The whole declared domain.
    pub fn all() -> Self {
        Self::from_raw(RawSet::all())
    }
    /// One interval as a set.
    pub fn of(r: impl IntoInterval<V>) -> Self {
        Self::from_intervals([r.into_interval()])
    }
    /// Builds from intervals in any order. Accepts `Interval<V>` and std ranges.
    pub fn from_intervals<I>(it: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoInterval<V>,
    {
        let spans = it
            .into_iter()
            .map(|r| {
                let (lo, hi) = r.into_interval().into_cuts();
                (
                    <V::Density as Density<V>>::boundary(lo),
                    <V::Density as Density<V>>::boundary(hi),
                )
            })
            .collect();
        Self::from_raw(RawSet::from_spans(spans))
    }

    #[inline]
    /// `true` if `x` is a member. One binary search over the boundaries.
    pub fn contains(&self, x: &V) -> bool {
        self.raw
            .inside(<V::Density as Density<V>>::segment_of(&self.raw.bounds, x))
    }

    /// Complement within the declared domain `V`.
    #[must_use]
    pub fn complement(&self) -> Self {
        Self::from_raw(self.raw.complement())
    }
    #[must_use]
    /// The union of `self` and `o`. One linear pass over both sets.
    pub fn union(&self, o: &Self) -> Self {
        Self::from_raw(self.raw.union(&o.raw))
    }
    #[must_use]
    /// The intersection of `self` and `o`. One linear pass over both sets.
    pub fn intersection(&self, o: &Self) -> Self {
        Self::from_raw(self.raw.intersection(&o.raw))
    }
    #[must_use]
    /// The members of `self` that are not members of `o`. One linear pass over both sets.
    pub fn difference(&self, o: &Self) -> Self {
        Self::from_raw(self.raw.difference(&o.raw))
    }
    #[must_use]
    /// The members of exactly one of `self` and `o`. One linear pass over both sets.
    pub fn symmetric_difference(&self, o: &Self) -> Self {
        Self::from_raw(self.raw.symmetric_difference(&o.raw))
    }

    /// Every value of `self` is in `o`. One pass, no allocation.
    pub fn is_subset(&self, o: &Self) -> bool {
        self.raw.is_subset(&o.raw)
    }
    /// `true` if every member of `o` is a member of `self`. One pass, no allocation.
    pub fn is_superset(&self, o: &Self) -> bool {
        o.is_subset(self)
    }
    /// No value is in both. One pass, no allocation.
    pub fn is_disjoint(&self, o: &Self) -> bool {
        self.raw.is_disjoint(&o.raw)
    }
    /// The whole interval `r` is in the set.
    pub fn is_superset_of_interval(&self, r: impl IntoInterval<V>) -> bool {
        Self::of(r).is_subset(self)
    }
    /// No value of `r` is in the set.
    pub fn is_disjoint_from_interval(&self, r: impl IntoInterval<V>) -> bool {
        Self::of(r).is_disjoint(self)
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
        self.raw.bounds.len()
    }
    /// The number of maximal disjoint intervals.
    pub fn interval_count(&self) -> usize {
        self.raw.interval_count()
    }
    /// The heap memory that the set uses, in bytes.
    pub fn heap_bytes(&self) -> usize {
        self.raw.bounds.heap_bytes()
    }

    fn cuts(&self) -> impl Iterator<Item = Cut<V>> + '_ {
        (0..self.raw.bounds.len()).map(|i| <V::Density as Density<V>>::cut(self.raw.bounds.get(i)))
    }

    /// Canonical intervals in order. Domain edges appear as `BelowAll`/`AboveAll`.
    pub fn intervals(&self) -> Vec<Interval<V>> {
        let mut out = Vec::with_capacity(self.interval_count());
        let mut start: Option<Cut<V>> = self.raw.starts_inside.then_some(Cut::BelowAll);
        for c in self.cuts() {
            match start.take() {
                None => start = Some(c),
                Some(s) => out.push(Interval::new(s, c)),
            }
        }
        if let Some(s) = start {
            out.push(Interval::new(s, Cut::AboveAll));
        }
        out
    }

    /// Intervals with the domain's own edge cuts (`min_cut`/`max_cut`) in place
    /// of `BelowAll`/`AboveAll`, for display.
    pub fn intervals_with_edges(&self) -> Vec<(Cut<V>, Cut<V>)> {
        self.intervals()
            .into_iter()
            .map(|r| {
                let (lo, hi) = r.into_cuts();
                let lo = if lo == Cut::BelowAll {
                    V::min_cut()
                } else {
                    lo
                };
                let hi = if hi == Cut::AboveAll {
                    V::max_cut()
                } else {
                    hi
                };
                (lo, hi)
            })
            .collect()
    }
}

/// Discrete domains store one word per boundary.
impl<V: DiscreteDomain> FrozenIntervalSet<V> {
    /// The stored boundaries: the words of the canonical `Below` cuts.
    pub fn words(&self) -> &[V::Word] {
        &self.raw.bounds
    }
}

/// Member-level queries for bounded discrete domains.
impl<V: DiscreteDomain + Copy> FrozenIntervalSet<V> {
    /// Intervals as `(first member, end)`. `end` is the first value above the
    /// interval, or `None` when the interval reaches the end of the domain.
    fn spans(&self) -> impl Iterator<Item = (V, Option<V>)> + '_ {
        let mut cuts = self.cuts();
        let mut start = self.raw.starts_inside.then_some(Extrema::<V>::MIN);
        core::iter::from_fn(move || {
            loop {
                let Some(c) = cuts.next() else {
                    return start.take().map(|s| (s, None));
                };
                let Cut::Below(v) = c else { continue };
                match start.take() {
                    Some(s) => return Some((s, Some(v))),
                    None => start = Some(v),
                }
            }
        })
    }

    /// Members in order. It steps with `succ`, so it yields only values of
    /// the domain.
    pub fn iter(&self) -> impl Iterator<Item = V> + '_ {
        self.spans().flat_map(|(first, end)| {
            core::iter::successors(Some(first), move |v| {
                v.succ().filter(|s| end.is_none_or(|e| *s < e))
            })
        })
    }

    /// Closed `(first, last)` member pairs, one per interval.
    pub fn closed_intervals(&self) -> Vec<(V, V)> {
        self.spans()
            .map(|(first, end)| {
                (
                    first,
                    end.map_or(Extrema::<V>::MAX, |e| e.pred().unwrap_or(first)),
                )
            })
            .collect()
    }

    /// Like `closed_intervals`, but an interval that passes the end of the cycle is
    /// one pair `(first, last)` with `first > last`.
    pub fn cyclic_intervals(&self) -> Vec<(V, V)>
    where
        V: CyclicDomain,
    {
        let mut intervals = self.closed_intervals();
        let wraps = intervals.len() >= 2
            && intervals.first().is_some_and(|r| r.0 == Extrema::<V>::MIN)
            && intervals.last().is_some_and(|r| r.1 == Extrema::<V>::MAX);
        if wraps {
            let (_, first_end) = intervals.remove(0);
            if let Some(last) = intervals.last_mut() {
                last.1 = first_end;
            }
        }
        intervals
    }
}

/// Counting needs a rank, so its cost is one step per interval, never per member.
impl<V: Ranked> FrozenIntervalSet<V> {
    /// Exact member count in `u128`.
    pub fn count(&self) -> u128 {
        self.spans()
            .map(|(first, end)| {
                end.map_or(Extrema::<V>::MAX.rank() + 1, |e| e.rank()) - first.rank()
            })
            .sum()
    }
}

/// The encoding lifts from values to sets. For a cover-preserving encoding
/// the word of an interval is an interval, so the word of a set has the same number
/// of intervals.
impl<V> FrozenIntervalSet<V>
where
    V: CoverPreserving,
    V::Word: MachineWord + DiscreteDomain,
{
    /// The set of words: `{ encode(v) | v in self }`.
    pub fn to_words(&self) -> FrozenIntervalSet<V::Word> {
        self.closed_intervals()
            .into_iter()
            .map(|(a, b)| Interval::closed(a.encode(), b.encode()))
            .collect()
    }
    /// The set of values whose word is in `words`. Words that
    /// encode no value are dropped.
    pub fn from_words(words: &FrozenIntervalSet<V::Word>) -> Self {
        let domain = FrozenIntervalSet::<V::Word>::of(Interval::closed(
            Extrema::<V>::MIN.encode(),
            Extrema::<V>::MAX.encode(),
        ));
        words
            .intersection(&domain)
            .intervals()
            .into_iter()
            .filter_map(|r| {
                Some(Interval::closed(
                    V::decode(&r.first()?)?,
                    V::decode(&r.last()?)?,
                ))
            })
            .collect()
    }
}

macro_rules! bitop {
    ($tr:ident, $m:ident, $f:ident) => {
        impl<V: OrderedDomain> core::ops::$tr<&FrozenIntervalSet<V>> for &FrozenIntervalSet<V> {
            type Output = FrozenIntervalSet<V>;
            fn $m(self, o: &FrozenIntervalSet<V>) -> FrozenIntervalSet<V> {
                self.$f(o)
            }
        }
    };
}
bitop!(BitOr, bitor, union);
bitop!(BitAnd, bitand, intersection);
bitop!(Sub, sub, difference);
bitop!(BitXor, bitxor, symmetric_difference);

impl<V: OrderedDomain> core::ops::Not for &FrozenIntervalSet<V> {
    type Output = FrozenIntervalSet<V>;
    fn not(self) -> FrozenIntervalSet<V> {
        self.complement()
    }
}

impl<V: OrderedDomain, R: IntoInterval<V>> FromIterator<R> for FrozenIntervalSet<V> {
    fn from_iter<I: IntoIterator<Item = R>>(it: I) -> Self {
        Self::from_intervals(it)
    }
}

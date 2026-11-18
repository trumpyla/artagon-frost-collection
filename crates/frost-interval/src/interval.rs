//! `Interval<V>`: a pair of canonical cuts of the domain `V`.
//!
//! Invariant: both cuts are canonical for `V`, and the empty interval is the one
//! value `(AboveAll, AboveAll)`. There is no raw constructor, so two intervals are
//! equal exactly when they hold the same values, and `is_connected`, `gap`,
//! and `is_empty` are correct in discrete domains (`(3..4)` is empty for
//! integers; `[1..5]` and `[6..9]` are connected).

use crate::cut::Cut;
use crate::domain::{DiscreteDomain, OrderedDomain};
use core::ops::{Bound, RangeBounds};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// An interval of a totally ordered domain: two canonical cuts.
///
/// The empty interval has one representation, so two intervals are equal
/// exactly when they hold the same values.
pub struct Interval<V> {
    lo: Cut<V>,
    hi: Cut<V>,
}

impl<V: OrderedDomain> Interval<V> {
    /// Builds an interval from two cuts. The cuts are canonicalized for `V`.
    /// `lo >= hi` gives the empty interval.
    #[inline]
    pub fn new(lo: Cut<V>, hi: Cut<V>) -> Self {
        Self::from_canonical(lo.canonical(), hi.canonical())
    }
    #[inline]
    fn from_canonical(lo: Cut<V>, hi: Cut<V>) -> Self {
        if lo < hi {
            Interval { lo, hi }
        } else {
            Self::empty()
        }
    }
    #[inline]
    /// The empty interval.
    pub const fn empty() -> Self {
        Interval {
            lo: Cut::AboveAll,
            hi: Cut::AboveAll,
        }
    }
    /// The whole domain.
    #[inline]
    pub const fn all() -> Self {
        Interval {
            lo: Cut::BelowAll,
            hi: Cut::AboveAll,
        }
    }
    /// The interval `[a, b]`.
    pub fn closed(a: V, b: V) -> Self {
        Self::new(Cut::Below(a), Cut::Above(b))
    }
    /// The interval `(a, b)`.
    pub fn open(a: V, b: V) -> Self {
        Self::new(Cut::Above(a), Cut::Below(b))
    }
    /// The interval `[a, b)`.
    pub fn closed_open(a: V, b: V) -> Self {
        Self::new(Cut::Below(a), Cut::Below(b))
    }
    /// The interval `(a, b]`.
    pub fn open_closed(a: V, b: V) -> Self {
        Self::new(Cut::Above(a), Cut::Above(b))
    }
    /// The interval `[a, ∞)`.
    pub fn at_least(a: V) -> Self {
        Self::new(Cut::Below(a), Cut::AboveAll)
    }
    /// The interval `(a, ∞)`.
    pub fn greater_than(a: V) -> Self {
        Self::new(Cut::Above(a), Cut::AboveAll)
    }
    /// The interval `(-∞, b]`.
    pub fn at_most(b: V) -> Self {
        Self::new(Cut::BelowAll, Cut::Above(b))
    }
    /// The interval `(-∞, b)`.
    pub fn less_than(b: V) -> Self {
        Self::new(Cut::BelowAll, Cut::Below(b))
    }
    /// The interval `[v, v]` that holds only `v`.
    pub fn singleton(v: V) -> Self {
        Self::closed(v.clone(), v)
    }
    /// Converts any std range (`a..b`, `a..=b`, `..`, `(Bound, Bound)`).
    pub fn from_bounds<R: RangeBounds<V>>(r: R) -> Self {
        let lo = match r.start_bound() {
            Bound::Included(a) => Cut::Below(a.clone()),
            Bound::Excluded(a) => Cut::Above(a.clone()),
            Bound::Unbounded => Cut::BelowAll,
        };
        let hi = match r.end_bound() {
            Bound::Included(b) => Cut::Above(b.clone()),
            Bound::Excluded(b) => Cut::Below(b.clone()),
            Bound::Unbounded => Cut::AboveAll,
        };
        Self::new(lo, hi)
    }

    /// Canonical lower cut.
    pub fn lo(&self) -> &Cut<V> {
        &self.lo
    }
    /// Canonical upper cut.
    pub fn hi(&self) -> &Cut<V> {
        &self.hi
    }
    /// The lower and upper cuts.
    pub fn into_cuts(self) -> (Cut<V>, Cut<V>) {
        (self.lo, self.hi)
    }
    #[inline]
    /// `true` if the interval holds no value.
    pub fn is_empty(&self) -> bool {
        matches!(self.lo, Cut::AboveAll)
    }
    #[inline]
    /// `true` if `x` is in the interval.
    pub fn contains(&self, x: &V) -> bool {
        self.lo.le_below(x) && self.hi.ge_above(x)
    }
    /// True when the union of the two intervals is one interval (touching counts).
    /// The empty interval is connected to every interval. Equivalent to
    /// `self.gap(o).is_empty()`.
    pub fn is_connected(&self, o: &Self) -> bool {
        self.is_empty() || o.is_empty() || max_ref(&self.lo, &o.lo) <= min_ref(&self.hi, &o.hi)
    }
    /// Every value of `o` is in `self`.
    pub fn is_superset(&self, o: &Self) -> bool {
        o.is_empty() || (!self.is_empty() && self.lo <= o.lo && o.hi <= self.hi)
    }
    /// `true` if every value of `self` is in `o`.
    pub fn is_subset(&self, o: &Self) -> bool {
        o.is_superset(self)
    }
    /// No value is in both.
    pub fn is_disjoint(&self, o: &Self) -> bool {
        self.is_empty() || o.is_empty() || max_ref(&self.lo, &o.lo) >= min_ref(&self.hi, &o.hi)
    }
    #[must_use]
    /// The values in both intervals.
    pub fn intersection(&self, o: &Self) -> Self {
        if self.is_empty() || o.is_empty() {
            return Self::empty();
        }
        Self::from_canonical(
            max_ref(&self.lo, &o.lo).clone(),
            min_ref(&self.hi, &o.hi).clone(),
        )
    }
    /// The hull: the smallest interval that holds both. Empty inputs are ignored.
    #[must_use]
    pub fn hull(&self, o: &Self) -> Self {
        if self.is_empty() {
            return o.clone();
        }
        if o.is_empty() {
            return self.clone();
        }
        Self::from_canonical(
            min_ref(&self.lo, &o.lo).clone(),
            max_ref(&self.hi, &o.hi).clone(),
        )
    }
    /// The interval strictly between two intervals; empty if they are connected or
    /// either is empty.
    #[must_use]
    pub fn gap(&self, o: &Self) -> Self {
        if self.is_empty() || o.is_empty() {
            return Self::empty();
        }
        Self::from_canonical(
            min_ref(&self.hi, &o.hi).clone(),
            max_ref(&self.lo, &o.lo).clone(),
        )
    }
}

impl<V: DiscreteDomain> Interval<V> {
    /// First member, if the interval is non-empty and bounded below.
    pub fn first(&self) -> Option<V> {
        match &self.lo {
            Cut::BelowAll => V::MIN,
            Cut::Below(v) => Some(v.clone()),
            Cut::Above(_) | Cut::AboveAll => None,
        }
    }
    /// Last member, if the interval is non-empty and bounded above.
    pub fn last(&self) -> Option<V> {
        if self.is_empty() {
            return None;
        }
        match &self.hi {
            Cut::AboveAll => V::MAX,
            Cut::Below(v) => v.pred(),
            Cut::Above(_) | Cut::BelowAll => None,
        }
    }
}

impl<V: OrderedDomain> Interval<V> {
    /// The interval as std bounds; `None` for the empty interval, which std bounds
    /// cannot express without a value.
    pub fn to_bounds(&self) -> Option<(Bound<&V>, Bound<&V>)> {
        if self.is_empty() {
            return None;
        }
        let lo = match &self.lo {
            Cut::Below(v) => Bound::Included(v),
            Cut::Above(v) => Bound::Excluded(v),
            Cut::BelowAll | Cut::AboveAll => Bound::Unbounded,
        };
        let hi = match &self.hi {
            Cut::Below(v) => Bound::Excluded(v),
            Cut::Above(v) => Bound::Included(v),
            Cut::BelowAll | Cut::AboveAll => Bound::Unbounded,
        };
        Some((lo, hi))
    }
}

/// Anything that names one interval of the domain `V`: an [`Interval`], or any std
/// interval (`a..b`, `a..=b`, `a..`, `..b`, `..=b`, `..`, `(Bound, Bound)`, and
/// the `core::range` types). APIs take `impl IntoInterval<V>`.
pub trait IntoInterval<V: OrderedDomain> {
    /// Converts `self` to a canonical interval.
    fn into_interval(self) -> Interval<V>;
}
impl<V: OrderedDomain> IntoInterval<V> for Interval<V> {
    #[inline]
    fn into_interval(self) -> Interval<V> {
        self
    }
}
impl<V: OrderedDomain> IntoInterval<V> for &Interval<V> {
    #[inline]
    fn into_interval(self) -> Interval<V> {
        self.clone()
    }
}
macro_rules! into_interval_std {
    ($($t:ty),* $(,)?) => {$(
        impl<V: OrderedDomain> IntoInterval<V> for $t {
            #[inline]
            fn into_interval(self) -> Interval<V> {
                Interval::from_bounds(self)
            }
        }
    )*};
}
into_interval_std!(
    core::ops::Range<V>,
    core::ops::RangeInclusive<V>,
    core::ops::RangeFrom<V>,
    core::ops::RangeTo<V>,
    core::ops::RangeToInclusive<V>,
    (Bound<V>, Bound<V>),
    core::range::Range<V>,
    core::range::RangeInclusive<V>,
    core::range::RangeFrom<V>,
);
impl<V: OrderedDomain> IntoInterval<V> for core::ops::RangeFull {
    #[inline]
    fn into_interval(self) -> Interval<V> {
        Interval::all()
    }
}

fn min_ref<'a, T: Ord>(a: &'a T, b: &'a T) -> &'a T {
    if a <= b { a } else { b }
}
fn max_ref<'a, T: Ord>(a: &'a T, b: &'a T) -> &'a T {
    if a >= b { a } else { b }
}

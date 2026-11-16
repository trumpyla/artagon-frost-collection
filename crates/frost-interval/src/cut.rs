//! `Cut<T>`: a position on the line between values, after Guava's `Cut`.
//!
//! Order: `BelowAll < Below(v) < Above(v) < Below(w) < ... < AboveAll` for `v < w`.
//! A point `v` occupies the gap between `Below(v)` and `Above(v)`; no cut lies
//! strictly between those two.

use core::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// A cut: a position between values of a totally ordered domain (a Dedekind cut).
pub enum Cut<T> {
    /// Below every value of the domain.
    BelowAll,
    /// Just below the value: the value is above the cut.
    Below(T),
    /// Just above the value: the value is below the cut.
    Above(T),
    /// Above every value of the domain.
    AboveAll,
}

impl<T: Ord> Ord for Cut<T> {
    // Do NOT derive: a derived Ord compares the discriminant first and puts
    // every `Below(_)` before every `Above(_)`.
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        use Cut::{Above, AboveAll, Below, BelowAll};
        match (self, other) {
            (BelowAll, BelowAll) | (AboveAll, AboveAll) => Ordering::Equal,
            (BelowAll, _) | (_, AboveAll) => Ordering::Less,
            (_, BelowAll) | (AboveAll, _) => Ordering::Greater,
            (Below(a), Below(b)) | (Above(a), Above(b)) => a.cmp(b),
            (Below(a), Above(b)) => a.cmp(b).then(Ordering::Less),
            (Above(a), Below(b)) => a.cmp(b).then(Ordering::Greater),
        }
    }
}

impl<T: Ord> PartialOrd for Cut<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> Cut<T> {
    /// `self <= Below(x)` without building `Below(x)` (no clone of `x`).
    #[inline]
    pub fn le_below(&self, x: &T) -> bool {
        match self {
            Cut::BelowAll => true,
            Cut::Below(a) => a <= x,
            Cut::Above(a) => a < x,
            Cut::AboveAll => false,
        }
    }

    /// `Above(x) <= self` without building `Above(x)`.
    #[inline]
    pub fn ge_above(&self, x: &T) -> bool {
        match self {
            Cut::BelowAll => false,
            Cut::Below(a) => x < a,
            Cut::Above(a) => x <= a,
            Cut::AboveAll => true,
        }
    }
}

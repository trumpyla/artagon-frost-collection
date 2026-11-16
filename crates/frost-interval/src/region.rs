//! `Region`: the interval algebra that every domain kind shares.
//!
//! A region is a describable subset of a domain that is closed under
//! intersection. A [`Convex`] region family also has a hull. A
//! [`BooleanRegion`] also has union, difference, and a domain-relative
//! complement. Generic code is written once against these traits.
//!
//! | Domain kind | Region type | `Convex` | `BooleanRegion` |
//! |---|---|---|---|
//! | Total order | [`Interval<V>`] (two canonical cuts) | yes | no |
//! | Total order, many intervals | `FrozenIntervalSet<V>` | no | yes |
//! | Finite | `FiniteSet<V>` (a subset) | no | yes |
//! | Lattice | [`LatticeInterval<V>`]: `{ x | lo <= x <= hi }` | yes | no |
//! | ProductRegion of domains | [`ProductRegion<A, B>`] | if both sides are | no |
//! | Cyclic | [`CyclicInterval<V>`] is *not* a `Region`: two cyclic intervals can intersect in two parts. Its intersection is a `FrozenIntervalSet<V>`. | — | — |
//!
//! Every implementation keeps one canonical form per set of values, so
//! `a.is_superset(b)` equals `a.intersection(b) == b`.

use crate::cut::Cut;
use crate::domain::{BoundedLattice, CyclicDomain, FiniteDomain, OrderedDomain};
use crate::finite::FiniteSet;
use crate::interval::Interval;
use crate::set::FrozenIntervalSet;

/// A subset of a domain, closed under intersection.
pub trait Region: Clone + PartialEq {
    /// The type of the values in the region.
    type Value;
    /// The empty region.
    fn empty() -> Self;
    /// The whole domain.
    fn all() -> Self;
    /// `true` if the region holds no value.
    fn is_empty(&self) -> bool;
    /// `true` if `v` is in the region.
    fn contains(&self, v: &Self::Value) -> bool;
    /// Every value of `o` is in `self`.
    fn is_superset(&self, o: &Self) -> bool;
    /// `true` if every value of `self` is in `o`.
    fn is_subset(&self, o: &Self) -> bool {
        o.is_superset(self)
    }
    #[must_use]
    /// The values in both regions.
    fn intersection(&self, o: &Self) -> Self;
    /// No value is in both.
    fn is_disjoint(&self, o: &Self) -> bool {
        self.intersection(o).is_empty()
    }
}

/// A region family with a hull: the smallest region that holds two regions.
pub trait Convex: Region {
    #[must_use]
    /// The smallest region of this type that holds both regions.
    fn hull(&self, o: &Self) -> Self;
}

/// A region family that is a Boolean algebra: it also has union, difference,
/// and a complement within the declared domain.
pub trait BooleanRegion: Region {
    #[must_use]
    /// The values in either region.
    fn union(&self, o: &Self) -> Self;
    #[must_use]
    /// The values of `self` that are not in `o`.
    fn difference(&self, o: &Self) -> Self;
    #[must_use]
    /// The values of the domain that are not in `self`.
    fn complement(&self) -> Self;
}

// -------------------------------------------------------------- total order

impl<V: OrderedDomain> Region for Interval<V> {
    type Value = V;
    fn empty() -> Self {
        Interval::empty()
    }
    fn all() -> Self {
        Interval::all()
    }
    fn is_empty(&self) -> bool {
        Interval::is_empty(self)
    }
    fn contains(&self, v: &V) -> bool {
        Interval::contains(self, v)
    }
    fn is_superset(&self, o: &Self) -> bool {
        Interval::is_superset(self, o)
    }
    fn intersection(&self, o: &Self) -> Self {
        Interval::intersection(self, o)
    }
    fn is_disjoint(&self, o: &Self) -> bool {
        Interval::is_disjoint(self, o)
    }
}
impl<V: OrderedDomain> Convex for Interval<V> {
    fn hull(&self, o: &Self) -> Self {
        Interval::hull(self, o)
    }
}

impl<V: OrderedDomain> Region for FrozenIntervalSet<V> {
    type Value = V;
    fn empty() -> Self {
        FrozenIntervalSet::empty()
    }
    fn all() -> Self {
        FrozenIntervalSet::all()
    }
    fn is_empty(&self) -> bool {
        FrozenIntervalSet::is_empty(self)
    }
    fn contains(&self, v: &V) -> bool {
        FrozenIntervalSet::contains(self, v)
    }
    fn is_superset(&self, o: &Self) -> bool {
        FrozenIntervalSet::is_superset(self, o)
    }
    fn intersection(&self, o: &Self) -> Self {
        FrozenIntervalSet::intersection(self, o)
    }
    fn is_disjoint(&self, o: &Self) -> bool {
        FrozenIntervalSet::is_disjoint(self, o)
    }
}
impl<V: OrderedDomain> BooleanRegion for FrozenIntervalSet<V> {
    fn union(&self, o: &Self) -> Self {
        FrozenIntervalSet::union(self, o)
    }
    fn difference(&self, o: &Self) -> Self {
        FrozenIntervalSet::difference(self, o)
    }
    fn complement(&self) -> Self {
        FrozenIntervalSet::complement(self)
    }
}

// ------------------------------------------------------------------ finite

impl<V: FiniteDomain> Region for FiniteSet<V> {
    type Value = V;
    fn empty() -> Self {
        FiniteSet::empty()
    }
    fn all() -> Self {
        FiniteSet::all()
    }
    fn is_empty(&self) -> bool {
        FiniteSet::is_empty(self)
    }
    fn contains(&self, v: &V) -> bool {
        FiniteSet::contains(self, v)
    }
    fn is_superset(&self, o: &Self) -> bool {
        FiniteSet::difference(*o, *self).is_empty()
    }
    fn intersection(&self, o: &Self) -> Self {
        FiniteSet::intersection(*self, *o)
    }
}
impl<V: FiniteDomain> BooleanRegion for FiniteSet<V> {
    fn union(&self, o: &Self) -> Self {
        FiniteSet::union(*self, *o)
    }
    fn difference(&self, o: &Self) -> Self {
        FiniteSet::difference(*self, *o)
    }
    fn complement(&self) -> Self {
        FiniteSet::complement(*self)
    }
}

// ----------------------------------------------------------------- lattice

/// A lattice interval `{ x | lo <= x <= hi }`. `Cut` has no meaning in a
/// partial order, so the bounds are values and both are inclusive.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LatticeInterval<V>(Option<(V, V)>);

impl<V: BoundedLattice> LatticeInterval<V> {
    /// `[lo, hi]`; empty unless `lo <= hi` in the domain order.
    pub fn new(lo: V, hi: V) -> Self {
        LatticeInterval(lo.leq(&hi).then_some((lo, hi)))
    }
    /// Everything at or above `lo`.
    pub fn up_from(lo: V) -> Self {
        Self::new(lo, V::top())
    }
    /// Everything at or below `hi`.
    pub fn down_from(hi: V) -> Self {
        Self::new(V::bottom(), hi)
    }
    /// `(lo, hi)`, or `None` for the empty interval.
    pub fn bounds(&self) -> Option<(&V, &V)> {
        self.0.as_ref().map(|(a, b)| (a, b))
    }
}

impl<V: BoundedLattice> Region for LatticeInterval<V> {
    type Value = V;
    fn empty() -> Self {
        LatticeInterval(None)
    }
    fn all() -> Self {
        Self::new(V::bottom(), V::top())
    }
    fn is_empty(&self) -> bool {
        self.0.is_none()
    }
    fn contains(&self, v: &V) -> bool {
        self.0
            .as_ref()
            .is_some_and(|(lo, hi)| lo.leq(v) && v.leq(hi))
    }
    fn is_superset(&self, o: &Self) -> bool {
        match (&self.0, &o.0) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some((lo, hi)), Some((a, b))) => lo.leq(a) && b.leq(hi),
        }
    }
    fn intersection(&self, o: &Self) -> Self {
        match (&self.0, &o.0) {
            (Some((lo, hi)), Some((a, b))) => Self::new(lo.join(a), hi.meet(b)),
            _ => LatticeInterval(None),
        }
    }
}
impl<V: BoundedLattice> Convex for LatticeInterval<V> {
    fn hull(&self, o: &Self) -> Self {
        match (&self.0, &o.0) {
            (Some((lo, hi)), Some((a, b))) => LatticeInterval(Some((lo.meet(a), hi.join(b)))),
            (Some(_), None) => self.clone(),
            _ => o.clone(),
        }
    }
}

// ----------------------------------------------------------------- product

/// The product of two regions. Its values are pairs. Nest it for more
/// dimensions: `ProductRegion<A, ProductRegion<B, C>>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProductRegion<A, B>(A, B);

impl<A: Region, B: Region> ProductRegion<A, B> {
    /// `a x b`. If one side is empty, the product is the one empty product.
    pub fn new(a: A, b: B) -> Self {
        if a.is_empty() || b.is_empty() {
            ProductRegion(A::empty(), B::empty())
        } else {
            ProductRegion(a, b)
        }
    }
    /// The two factors of the product.
    pub fn sides(&self) -> (&A, &B) {
        (&self.0, &self.1)
    }
}

impl<A: Region, B: Region> Region for ProductRegion<A, B> {
    type Value = (A::Value, B::Value);
    fn empty() -> Self {
        ProductRegion(A::empty(), B::empty())
    }
    fn all() -> Self {
        ProductRegion(A::all(), B::all())
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    fn contains(&self, v: &Self::Value) -> bool {
        self.0.contains(&v.0) && self.1.contains(&v.1)
    }
    fn is_superset(&self, o: &Self) -> bool {
        o.is_empty() || (self.0.is_superset(&o.0) && self.1.is_superset(&o.1))
    }
    fn intersection(&self, o: &Self) -> Self {
        Self::new(self.0.intersection(&o.0), self.1.intersection(&o.1))
    }
}
impl<A: Convex, B: Convex> Convex for ProductRegion<A, B> {
    fn hull(&self, o: &Self) -> Self {
        if self.is_empty() {
            return o.clone();
        }
        if o.is_empty() {
            return self.clone();
        }
        ProductRegion(self.0.hull(&o.0), self.1.hull(&o.1))
    }
}

// ------------------------------------------------------------------ cyclic

/// An interval of a cyclic domain: it may pass the end of the cycle.
///
/// Stored as at most two linear intervals. Invariant: if `tail` is not empty,
/// `head` reaches `AboveAll`, `tail` starts at `BelowAll`, and they do not
/// meet. So each cyclic interval has one form, and the whole cycle is `(all, empty)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CyclicInterval<V> {
    head: Interval<V>,
    tail: Interval<V>,
}

impl<V: CyclicDomain> CyclicInterval<V> {
    fn from_pieces([head, tail]: [Interval<V>; 2]) -> Self {
        if tail.is_empty() {
            return CyclicInterval { head, tail };
        }
        if head.is_empty() {
            return CyclicInterval {
                head: tail,
                tail: head,
            };
        }
        // The two parts meet around the cycle: this is the whole cycle.
        if tail.hi() >= head.lo() {
            return Self::all();
        }
        CyclicInterval { head, tail }
    }
    /// The empty cyclic interval.
    pub fn empty() -> Self {
        CyclicInterval {
            head: Interval::empty(),
            tail: Interval::empty(),
        }
    }
    /// The whole cycle.
    pub fn all() -> Self {
        CyclicInterval {
            head: Interval::all(),
            tail: Interval::empty(),
        }
    }
    /// `[a, b]` going forward from `a`; wraps when `a > b`.
    pub fn closed(a: V, b: V) -> Self {
        Self::from_pieces(V::wrap_closed(a, b))
    }
    /// `[a, b)` going forward from `a`; wraps when `a > b`; empty when `a == b`.
    pub fn closed_open(a: V, b: V) -> Self {
        Self::from_pieces(V::wrap_closed_open(a, b))
    }
    /// `true` if the cyclic interval holds no value.
    pub fn is_empty(&self) -> bool {
        self.head.is_empty()
    }
    /// `true` if the cyclic interval holds every value.
    pub fn is_all(&self) -> bool {
        *self.head.lo() == Cut::BelowAll && *self.head.hi() == Cut::AboveAll
    }
    /// True when the interval passes the end of the cycle.
    pub fn wraps(&self) -> bool {
        !self.tail.is_empty()
    }
    /// `true` if `v` is in the cyclic interval.
    pub fn contains(&self, v: &V) -> bool {
        self.head.contains(v) || self.tail.contains(v)
    }
    /// The linear intervals; the second is empty unless the interval wraps.
    pub fn linear_intervals(&self) -> [&Interval<V>; 2] {
        [&self.head, &self.tail]
    }
    /// The rest of the cycle. The complement of a cyclic interval is a cyclic interval.
    #[must_use]
    pub fn complement(&self) -> Self {
        if self.is_empty() {
            return Self::all();
        }
        if self.wraps() {
            let piece = Interval::new(self.tail.hi().clone(), self.head.lo().clone());
            return CyclicInterval {
                head: piece,
                tail: Interval::empty(),
            };
        }
        Self::from_pieces([
            Interval::new(self.head.hi().clone(), Cut::AboveAll),
            Interval::new(Cut::BelowAll, self.head.lo().clone()),
        ])
    }
    /// The values as a set of at most two intervals.
    pub fn to_set(&self) -> FrozenIntervalSet<V> {
        FrozenIntervalSet::from_intervals([self.head.clone(), self.tail.clone()])
    }
    /// Two cyclic intervals can share two separate parts, so the result is a set.
    pub fn intersection(&self, o: &Self) -> FrozenIntervalSet<V> {
        self.to_set().intersection(&o.to_set())
    }
}

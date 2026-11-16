//! [`Domain`]: a value type that declares its logical domain at the type level.
//!
//! The domain is never a runtime object. Every capability is an associated
//! constant, function, or type, so collections store only `PhantomData<V>` and
//! every domain decision is resolved at compile time.
//!
//! # The encoding
//!
//! Every ordered domain `V` has an **encoding** into a word type. The
//! encoding is an order embedding: `encode: V -> V::Word` is strictly
//! monotone. `decode` is its partial inverse: it gives `None` for a word
//! that encodes no value. Collections store and compare words only, never
//! values.
//!
//! - `u64`, `String`, `i128`: the identity encoding (`Word = Self`).
//! - `i64`, `NotNanF64`, `UserId`, a time of day: `Word = u64`.
//! - `u8`, `i32`, `char`, `NotNanF32`, a date: `Word = u32`.
//!
//! All domains with the same word type and density share one kernel
//! instantiation (see [`crate::kernel`]). The storage layout is a function of
//! `(Density, Word)`, so an implementor never names a layout.
//!
//! | Trait | Claim | Gives |
//! |---|---|---|
//! | [`Domain`] | One logical identity. | — |
//! | [`OrderedDomain`] | Total order, `MIN`/`MAX`, density, encoding. | `FrozenIntervalSet`, `FrozenIntervalMap` |
//! | [`DiscreteDomain`] | `succ`/`pred`. | canonical half-open cuts, members |
//! | [`CoverPreserving`] | `encode(succ v) == encode(v) + 1`. | `rank` for free, word sets |
//! | [`Ranked`] | `rank(v)` = number of values below `v`. | `count()` in one step per interval |
//! | [`CyclicDomain`] | The order wraps. | cyclic intervals |
//! | [`FiniteDomain`] | Finite universe with a dense index; no order claim. | `FiniteSet` |
//! | [`PrefixDomain`] | Each value names an interval of a key domain. | prefix sets |
//! | [`Poset`], [`Lattice`], [`BoundedLattice`] | Partial order, meet, join. | `DownSet`, `LatticeInterval` |
//!
//! Capabilities are traits, not runtime flags: a method that needs a
//! capability has it as a bound, so its cost is known at compile time.
//!
//! These traits are safe. An unlawful impl can give wrong sets or panic, but
//! never undefined behavior. The law checks in this package's `tests/support/laws.rs`
//! test an impl; a shipped law kit is an open item of the design record.

use crate::cut::Cut;
use crate::interval::Interval;
use crate::kernel::{self, Boundaries, RawCut, Side, SideBitVec};
use crate::tree::BoundaryTree;
use alloc::vec::Vec;
use core::fmt::Debug;
use core::hash::Hash;
use core::ops::RangeBounds;

/// Base capability: the type is a value of one declared logical domain.
pub trait Domain: Clone + Eq {}

mod sealed {
    pub trait Density {}
    pub trait MachineWord {}
    impl Density for super::Dense {}
    impl Density for super::Discrete {}
    impl MachineWord for u32 {}
    impl MachineWord for u64 {}
}

// ------------------------------------------------------------------ ordered

/// A domain with a semantic total order and an order-preserving encoding.
///
/// `Ord` alone is a storage order. Only this trait claims that intervals have
/// meaning, so a categorical enum may derive `Ord` without getting intervals.
///
/// # Laws
///
/// - `Ord` is the domain order. `MIN`/`MAX` are the least/greatest values of
///   the type, or `None`. Every value of the type is admissible.
/// - `encode` is strictly monotone: `a.cmp(&b) == a.encode().cmp(&b.encode())`.
/// - `decode(&v.encode()) == Some(v)`, and if `decode(&i) == Some(v)` then
///   `v.encode() == i`.
/// - `into_word` and `with_word` agree with `encode`.
pub trait OrderedDomain: Domain + Ord {
    /// [`Dense`] or [`Discrete`].
    type Density: Density<Self>;
    /// The word type. `Self` for the identity encoding
    /// (see [`identity_encoding!`](crate::identity_encoding)).
    type Word: Ord + Clone;
    /// The least value of the domain, or `None` if it has none or does not declare one.
    const MIN: Option<Self> = None;
    /// The greatest value of the domain, or `None` if it has none or does not declare one.
    const MAX: Option<Self> = None;

    /// The word of this value.
    fn encode(&self) -> Self::Word;
    /// The value that this word encodes, or `None` if it encodes no value.
    fn decode(word: &Self::Word) -> Option<Self>;
    /// `encode` for an owned value. The identity encoding moves it.
    #[inline]
    fn into_word(self) -> Self::Word {
        self.encode()
    }
    /// Runs `f` on the word without building it when the encoding is the
    /// identity. Queries use it, so a `String` probe is never cloned.
    #[inline]
    fn with_word<R>(&self, f: impl FnOnce(&Self::Word) -> R) -> R {
        f(&self.encode())
    }
}

/// Items of the identity encoding, for use inside an `OrderedDomain` impl.
#[macro_export]
macro_rules! identity_encoding {
    () => {
        type Word = Self;
        #[inline]
        fn encode(&self) -> Self {
            ::core::clone::Clone::clone(self)
        }
        #[inline]
        fn decode(word: &Self) -> ::core::option::Option<Self> {
            ::core::option::Option::Some(::core::clone::Clone::clone(word))
        }
        #[inline]
        fn into_word(self) -> Self {
            self
        }
        #[inline]
        fn with_word<R>(&self, f: impl ::core::ops::FnOnce(&Self) -> R) -> R {
            f(self)
        }
    };
}

/// Boundary layout of the domain `V`: a function of its density and word.
pub type BoundariesOf<V> = <<V as OrderedDomain>::Density as Density<V>>::Boundaries;

// ------------------------------------------------------------------ density

/// The density of a domain. Sealed: [`Dense`] or [`Discrete`]. It selects the
/// canonical form of cuts and the boundary layout.
pub trait Density<V: OrderedDomain>: sealed::Density {
    /// `true` for a discrete domain, `false` for a dense one.
    const DISCRETE: bool;
    /// `Vec<V::Word>` for discrete domains, `SideBitVec<V::Word>` for dense.
    type Boundaries: Boundaries;
    #[doc(hidden)]
    fn canonical_above(v: V) -> Cut<V>;
    /// The boundary of a cut. Total: it never panics.
    #[doc(hidden)]
    fn boundary(c: Cut<V>) -> RawCut<<Self::Boundaries as Boundaries>::Owned>;
    #[doc(hidden)]
    fn cut(b: <Self::Boundaries as Boundaries>::Ref<'_>) -> Cut<V>;
    /// The segment that holds `x`: the number of boundaries at or below it.
    #[doc(hidden)]
    fn segment_of(bounds: &Self::Boundaries, x: &V) -> usize;
    /// The same for a boundary tree.
    #[doc(hidden)]
    fn tree_segment_of(
        tree: &BoundaryTree<<Self::Boundaries as Boundaries>::Owned>,
        x: &V,
    ) -> usize;
}

/// Between two values there can be a third. `Above(v)` stays `Above(v)`.
#[derive(Clone, Copy, Debug)]
pub struct Dense;

/// Every value but the last has a next value. `Above(v)` becomes `Below(succ v)`.
/// Selecting it requires a [`DiscreteDomain`] impl, so `succ` cannot be forgotten.
#[derive(Clone, Copy, Debug)]
pub struct Discrete;

/// Stored words come from `encode`, so a lawful encoding decodes them.
#[inline]
fn value_of<V: OrderedDomain>(word: &V::Word) -> V {
    V::decode(word).expect("encoding law: decode(encode(v)) == Some(v)")
}

impl<V: OrderedDomain> Density<V> for Dense {
    const DISCRETE: bool = false;
    type Boundaries = SideBitVec<V::Word>;
    #[inline]
    fn canonical_above(v: V) -> Cut<V> {
        if matches!(V::MAX, Some(m) if v >= m) {
            Cut::AboveAll
        } else {
            Cut::Above(v)
        }
    }
    #[inline]
    fn boundary(c: Cut<V>) -> RawCut<(V::Word, Side)> {
        match c {
            Cut::BelowAll => RawCut::BelowAll,
            Cut::Below(v) => RawCut::At((v.into_word(), Side::Below)),
            Cut::Above(v) => RawCut::At((v.into_word(), Side::Above)),
            Cut::AboveAll => RawCut::AboveAll,
        }
    }
    #[inline]
    fn cut((word, side): (&V::Word, Side)) -> Cut<V> {
        match side {
            Side::Below => Cut::Below(value_of::<V>(word)),
            Side::Above => Cut::Above(value_of::<V>(word)),
        }
    }
    #[inline]
    fn segment_of(bounds: &SideBitVec<V::Word>, x: &V) -> usize {
        x.with_word(|word| bounds.segment_of(word))
    }
    #[inline]
    fn tree_segment_of(tree: &BoundaryTree<(V::Word, Side)>, x: &V) -> usize {
        x.with_word(|word| tree.rank_below(word))
    }
}

impl<V: DiscreteDomain> Density<V> for Discrete {
    const DISCRETE: bool = true;
    type Boundaries = Vec<V::Word>;
    #[inline]
    fn canonical_above(v: V) -> Cut<V> {
        v.succ().map_or(Cut::AboveAll, Cut::Below)
    }
    #[inline]
    fn boundary(c: Cut<V>) -> RawCut<V::Word> {
        match c {
            Cut::BelowAll => RawCut::BelowAll,
            Cut::Below(v) => RawCut::At(v.into_word()),
            // Not produced by canonical cuts of a discrete domain; kept total.
            Cut::Above(v) => v
                .succ()
                .map_or(RawCut::AboveAll, |s| RawCut::At(s.into_word())),
            Cut::AboveAll => RawCut::AboveAll,
        }
    }
    #[inline]
    fn cut(word: &V::Word) -> Cut<V> {
        Cut::Below(value_of::<V>(word))
    }
    #[inline]
    fn segment_of(bounds: &Vec<V::Word>, x: &V) -> usize {
        x.with_word(|word| kernel::segment_of(bounds, word))
    }
    #[inline]
    fn tree_segment_of(tree: &BoundaryTree<V::Word>, x: &V) -> usize {
        x.with_word(|word| tree.rank_le(word))
    }
}

// ----------------------------------------------------------------- discrete

/// A discrete ordered domain.
///
/// # Laws
///
/// - `succ(v)` is the least value greater than `v` (the value that covers
///   `v`); `None` exactly when `Some(v) == MAX`. `pred` mirrors it with `MIN`.
/// - `pred(succ(v)) == v` and `succ(pred(v)) == v` where defined.
pub trait DiscreteDomain: OrderedDomain<Density = Discrete> {
    /// The next value, or `None` at the greatest value.
    fn succ(&self) -> Option<Self>;
    /// The previous value, or `None` at the least value.
    fn pred(&self) -> Option<Self>;
}

/// Sealed machine word: `u32` or `u64`. It is the word
/// type of every domain with machine words.
pub trait MachineWord:
    sealed::MachineWord + Copy + Ord + Hash + Debug + Send + Sync + TryFrom<u128> + 'static
{
    /// The word as a `u128`, without loss.
    fn to_u128(self) -> u128;
}
impl MachineWord for u32 {
    #[inline]
    fn to_u128(self) -> u128 {
        u128::from(self)
    }
}
impl MachineWord for u64 {
    #[inline]
    fn to_u128(self) -> u128 {
        u128::from(self)
    }
}

/// The encoding is cover-preserving: `encode(succ v) == encode(v) + 1`.
/// So the words of the domain are an interval of integers, and the encoding
/// is an order isomorphism onto it. A marker: it has no methods.
///
/// With an [`MachineWord`] word type, it gives [`Ranked`] for free.
pub trait CoverPreserving: DiscreteDomain + Copy {}

/// `rank(v)` is the number of values below `v`. It makes member counts cost
/// one step per interval.
///
/// # Laws
///
/// - `rank(MIN) == 0` and `rank(succ v) == rank(v) + 1`.
pub trait Ranked: DiscreteDomain + Copy {
    /// The number of values of the domain below `self`.
    fn rank(&self) -> u128;
}

impl<V> Ranked for V
where
    V: CoverPreserving,
    V::Word: MachineWord,
{
    #[inline]
    fn rank(&self) -> u128 {
        self.encode().to_u128() - Extrema::<V>::MIN.encode().to_u128()
    }
}

/// Compile-time extrema of a bounded domain. A missing extremum is a build error
/// at the first use.
pub struct Extrema<V>(core::marker::PhantomData<V>);

impl<V> core::fmt::Debug for Extrema<V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Extrema")
    }
}
impl<V: OrderedDomain + Copy> Extrema<V> {
    /// The least value of the domain.
    ///
    /// The constant fails to evaluate, so the build fails, if the domain declares no `MIN`.
    pub const MIN: V = match V::MIN {
        Some(v) => v,
        None => panic!("this operation needs a domain that declares MIN"),
    };
    /// The greatest value of the domain.
    ///
    /// The constant fails to evaluate, so the build fails, if the domain declares no `MAX`.
    pub const MAX: V = match V::MAX {
        Some(v) => v,
        None => panic!("this operation needs a domain that declares MAX"),
    };
}

impl<V: OrderedDomain> Cut<V> {
    /// Canonical cut: one representation per position in the domain `V`.
    ///
    /// 1. `Below(v)` with `v == MIN` becomes `BelowAll`.
    /// 2. `Above(v)` with `v == MAX` becomes `AboveAll`.
    /// 3. In a discrete domain, any other `Above(v)` becomes `Below(succ v)`.
    #[inline]
    #[must_use]
    pub fn canonical(self) -> Self {
        match self {
            Cut::Below(v) => {
                if matches!(V::MIN, Some(m) if v <= m) {
                    Cut::BelowAll
                } else {
                    Cut::Below(v)
                }
            }
            Cut::Above(v) => <V::Density as Density<V>>::canonical_above(v),
            edge => edge,
        }
    }
}

/// Derived domain API. Blanket-implemented for every [`OrderedDomain`], so an
/// implementor cannot override it and it cannot disagree with `MIN`/`MAX`.
pub trait OrderedDomainExt: OrderedDomain {
    /// `true` for a discrete domain, `false` for a dense one.
    const DISCRETE: bool = <Self::Density as Density<Self>>::DISCRETE;

    /// The domain's least cut: `Below(MIN)`, or `BelowAll` if there is no minimum.
    /// Both denote the same position; canonical storage uses `BelowAll`.
    #[inline]
    fn min_cut() -> Cut<Self> {
        Self::MIN.map_or(Cut::BelowAll, Cut::Below)
    }
    /// The domain's greatest cut: `Above(MAX)`, or `AboveAll` if there is no maximum.
    #[inline]
    fn max_cut() -> Cut<Self> {
        Self::MAX.map_or(Cut::AboveAll, Cut::Above)
    }
    #[inline]
    /// The interval of all values of the domain.
    fn all() -> Interval<Self> {
        Interval::all()
    }
    #[inline]
    /// The interval `[a, b]`.
    fn closed(a: Self, b: Self) -> Interval<Self> {
        Interval::closed(a, b)
    }
    #[inline]
    /// The interval `[a, b)`.
    fn closed_open(a: Self, b: Self) -> Interval<Self> {
        Interval::closed_open(a, b)
    }
    #[inline]
    /// The interval `(a, b)`.
    fn open(a: Self, b: Self) -> Interval<Self> {
        Interval::open(a, b)
    }
    #[inline]
    /// The interval `(a, b]`.
    fn open_closed(a: Self, b: Self) -> Interval<Self> {
        Interval::open_closed(a, b)
    }
    #[inline]
    /// The interval `[a, ∞)`.
    fn at_least(a: Self) -> Interval<Self> {
        Interval::at_least(a)
    }
    #[inline]
    /// The interval `(a, ∞)`.
    fn greater_than(a: Self) -> Interval<Self> {
        Interval::greater_than(a)
    }
    #[inline]
    /// The interval `(-∞, b]`.
    fn at_most(b: Self) -> Interval<Self> {
        Interval::at_most(b)
    }
    #[inline]
    /// The interval `(-∞, b)`.
    fn less_than(b: Self) -> Interval<Self> {
        Interval::less_than(b)
    }
    #[inline]
    /// The interval `[v, v]` that holds only `v`.
    fn singleton(v: Self) -> Interval<Self> {
        Interval::singleton(v)
    }
    /// Converts any std range (`a..b`, `a..=b`, `..`, `(Bound, Bound)`).
    #[inline]
    fn from_bounds<R: RangeBounds<Self>>(r: R) -> Interval<Self> {
        Interval::from_bounds(r)
    }
}
impl<V: OrderedDomain> OrderedDomainExt for V {}

/// Implements the domain traits for a tuple newtype by delegating to its
/// field. The newtype is a separate logical domain with the same word.
///
/// Forms: `Name(Inner): dense`, `Name(Inner): discrete`, and
/// `Name(Inner): discrete, cover_preserving`. The extrema come from the field
/// type. If the field is not `Copy`, give them: `Name(String): dense,
/// extrema(Some(Name(String::new())), None)`.
///
/// ```
/// use frost_interval::*;
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// struct OrderId(u64);
/// newtype_domain!(OrderId(u64): discrete, cover_preserving);
/// let s: FrozenIntervalSet<OrderId> = FrozenIntervalSet::of(OrderId(5)..OrderId(9));
/// assert_eq!(s.count(), 4);
/// ```
#[macro_export]
macro_rules! newtype_domain {
    ($name:ident($inner:ty): discrete, cover_preserving) => {
        $crate::newtype_domain!($name($inner): discrete);
        impl $crate::CoverPreserving for $name {}
    };
    ($name:ident($inner:ty): discrete $(, extrema($min:expr, $max:expr))?) => {
        $crate::newtype_domain!(@ordered $name($inner), $crate::Discrete $(, $min, $max)?);
        impl $crate::DiscreteDomain for $name {
            #[inline]
            fn succ(&self) -> Option<Self> {
                $crate::DiscreteDomain::succ(&self.0).map($name)
            }
            #[inline]
            fn pred(&self) -> Option<Self> {
                $crate::DiscreteDomain::pred(&self.0).map($name)
            }
        }
    };
    ($name:ident($inner:ty): dense $(, extrema($min:expr, $max:expr))?) => {
        $crate::newtype_domain!(@ordered $name($inner), $crate::Dense $(, $min, $max)?);
    };
    // Extrema copied from the field type. This needs a `Copy` field, because a
    // constant cannot take apart a value that has a destructor.
    (@ordered $name:ident($inner:ty), $density:ty) => {
        $crate::newtype_domain!(@ordered $name($inner), $density,
            match <$inner as $crate::OrderedDomain>::MIN {
                Some(v) => Some($name(v)),
                None => None,
            },
            match <$inner as $crate::OrderedDomain>::MAX {
                Some(v) => Some($name(v)),
                None => None,
            });
    };
    (@ordered $name:ident($inner:ty), $density:ty, $min:expr, $max:expr) => {
        impl $crate::Domain for $name {}
        impl $crate::OrderedDomain for $name {
            type Density = $density;
            type Word = <$inner as $crate::OrderedDomain>::Word;
            const MIN: Option<Self> = $min;
            const MAX: Option<Self> = $max;
            #[inline]
            fn encode(&self) -> Self::Word {
                $crate::OrderedDomain::encode(&self.0)
            }
            #[inline]
            fn decode(word: &Self::Word) -> Option<Self> {
                <$inner as $crate::OrderedDomain>::decode(word).map($name)
            }
            #[inline]
            fn into_word(self) -> Self::Word {
                $crate::OrderedDomain::into_word(self.0)
            }
            #[inline]
            fn with_word<R>(&self, f: impl FnOnce(&Self::Word) -> R) -> R {
                $crate::OrderedDomain::with_word(&self.0, f)
            }
        }
    };
}

// ------------------------------------------------------------------- cyclic

/// An ordered domain whose order wraps (weekday, hour of day, angle).
///
/// A wrapping interval splits into at most two linear intervals, so `FrozenIntervalSet<V>`
/// stores it unchanged and complement stays inside the period.
///
/// # Laws
///
/// - `MIN` is `Some` (the origin of the cycle).
pub trait CyclicDomain: OrderedDomain {
    /// `[a, b]` going forward from `a`; wraps when `a > b`.
    /// The second interval is empty when no wrap happens.
    fn wrap_closed(a: Self, b: Self) -> [Interval<Self>; 2] {
        if a <= b {
            [Interval::closed(a, b), Interval::empty()]
        } else {
            [Interval::at_least(a), Interval::at_most(b)]
        }
    }
    /// `[a, b)` going forward from `a`; wraps when `a > b`.
    /// `a == b` is the empty interval. Use `all()` for the full circle.
    fn wrap_closed_open(a: Self, b: Self) -> [Interval<Self>; 2] {
        if a <= b {
            [Interval::closed_open(a, b), Interval::empty()]
        } else {
            [Interval::at_least(a), Interval::less_than(b)]
        }
    }
    #[must_use]
    /// The next value; after the greatest value, the least value.
    fn wrapping_succ(self) -> Self
    where
        Self: DiscreteDomain + Copy,
    {
        self.succ().unwrap_or(Extrema::<Self>::MIN)
    }
    #[must_use]
    /// The previous value; before the least value, the greatest value.
    fn wrapping_pred(self) -> Self
    where
        Self: DiscreteDomain + Copy,
    {
        self.pred().unwrap_or(Extrema::<Self>::MAX)
    }
}

// ------------------------------------------------------------------- finite

/// A finite universe with a dense index. It makes no order claim, so it gives
/// subsets and a universe-relative complement, but no intervals.
///
/// # Laws
///
/// - `index` is a bijection from the domain onto `0..CARDINALITY`.
/// - `from_index(i)` is `None` exactly when `i >= CARDINALITY`.
pub trait FiniteDomain: Domain {
    /// The number of values of the domain.
    const CARDINALITY: usize;
    /// The position of `self` in `0..CARDINALITY`.
    fn index(&self) -> usize;
    /// The value at position `i`, or `None` if `i >= CARDINALITY`.
    fn from_index(i: usize) -> Option<Self>;
}

// ------------------------------------------------------------------- prefix

/// A hierarchical value (CIDR, string prefix) that names one contiguous interval
/// of an ordered key domain. Sets of prefixes are `FrozenIntervalSet<Self::Key>`.
/// Longest-prefix-match maps need a trie; this encoding covers sets only.
pub trait PrefixDomain: Domain {
    /// The ordered domain that a prefix maps into.
    type Key: OrderedDomain;
    /// The interval of keys that start with this prefix.
    fn to_interval(&self) -> Interval<Self::Key>;
}

// ------------------------------------------------------------ partial order

/// A semantic partial order, separate from `PartialOrd`/`Ord`: a type may be
/// `Ord` for storage and partially ordered in meaning. `Cut` does not
/// generalize here; sets of such values are [`crate::DownSet`]s.
pub trait Poset: Domain {
    /// `true` if `self` is at or below `other` in the partial order.
    fn leq(&self, other: &Self) -> bool;
}

/// A lattice: every pair has a least upper bound and a greatest lower bound.
pub trait Lattice: Poset {
    #[must_use]
    /// The least upper bound of `self` and `other`.
    fn join(&self, other: &Self) -> Self;
    #[must_use]
    /// The greatest lower bound of `self` and `other`.
    fn meet(&self, other: &Self) -> Self;
}

/// A lattice with a least and a greatest element.
pub trait BoundedLattice: Lattice {
    /// The least element.
    fn bottom() -> Self;
    /// The greatest element.
    fn top() -> Self;
}

//! Example application domains. Each one is a distinct type, so sets of
//! different domains never mix, even when their words are equal.

use core::cmp::Ordering;
use frost_interval::domain::{
    BoundedLattice, CoverPreserving, CyclicDomain, Dense, Discrete, DiscreteDomain, Domain,
    FiniteDomain, Lattice, OrderedDomain, Poset, PrefixDomain, Ranked,
};
use frost_interval::interval::Interval;
use std::net::Ipv4Addr;

// ------------------------------------------------------------------ UserId

/// A logical identifier with the same words as `u64`. One macro line gives
/// it the domain traits of its field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(pub u64);
frost_interval::newtype_domain!(UserId(u64): discrete, cover_preserving);

// ----------------------------------------------------- BoundedU8 (Month)

/// A restricted integer domain `LO..=HI`. The extrema are const generics, so
/// the domain stays zero-sized and each pair of extrema is a distinct type.
/// Values outside the domain are unrepresentable: `new` rejects them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundedU8<const LO: u8, const HI: u8>(u8);

impl<const LO: u8, const HI: u8> BoundedU8<LO, HI> {
    pub const fn new(v: u8) -> Option<Self> {
        const { assert!(LO <= HI, "empty bounded domain") };
        if v >= LO && v <= HI {
            Some(Self(v))
        } else {
            None
        }
    }
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl<const LO: u8, const HI: u8> Domain for BoundedU8<LO, HI> {}
impl<const LO: u8, const HI: u8> OrderedDomain for BoundedU8<LO, HI> {
    type Density = Discrete;
    type Word = u32;
    const MIN: Option<Self> = Some(Self(LO));
    const MAX: Option<Self> = Some(Self(HI));
    #[inline]
    fn encode(&self) -> u32 {
        u32::from(self.0)
    }
    #[inline]
    fn decode(word: &u32) -> Option<Self> {
        u8::try_from(*word).ok().and_then(Self::new)
    }
}
impl<const LO: u8, const HI: u8> DiscreteDomain for BoundedU8<LO, HI> {
    #[inline]
    fn succ(&self) -> Option<Self> {
        (self.0 < HI).then(|| Self(self.0 + 1))
    }
    #[inline]
    fn pred(&self) -> Option<Self> {
        (self.0 > LO).then(|| Self(self.0 - 1))
    }
}
impl<const LO: u8, const HI: u8> CoverPreserving for BoundedU8<LO, HI> {}

/// Months 1..=12.
pub type Month = BoundedU8<1, 12>;

// ----------------------------------------------------------- Tier (sparse)

/// A sparse enum with words {1, 4, 7}. Words 2, 3, 5, 6 encode no value,
/// so the encoding is not cover-preserving and `rank` is written by hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Tier {
    Bronze = 1,
    Silver = 4,
    Gold = 7,
}
impl Domain for Tier {}
impl OrderedDomain for Tier {
    type Density = Discrete;
    type Word = u32;
    const MIN: Option<Self> = Some(Tier::Bronze);
    const MAX: Option<Self> = Some(Tier::Gold);
    fn encode(&self) -> u32 {
        *self as u32
    }
    fn decode(word: &u32) -> Option<Self> {
        match word {
            1 => Some(Tier::Bronze),
            4 => Some(Tier::Silver),
            7 => Some(Tier::Gold),
            _ => None,
        }
    }
}
impl DiscreteDomain for Tier {
    fn succ(&self) -> Option<Self> {
        match self {
            Tier::Bronze => Some(Tier::Silver),
            Tier::Silver => Some(Tier::Gold),
            Tier::Gold => None,
        }
    }
    fn pred(&self) -> Option<Self> {
        match self {
            Tier::Bronze => None,
            Tier::Silver => Some(Tier::Bronze),
            Tier::Gold => Some(Tier::Silver),
        }
    }
}
impl Ranked for Tier {
    fn rank(&self) -> u128 {
        match self {
            Tier::Bronze => 0,
            Tier::Silver => 1,
            Tier::Gold => 2,
        }
    }
}

// ------------------------------------------------------- Weekday (cyclic)

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    pub const ALL: [Weekday; 7] = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];
}
impl Domain for Weekday {}
impl OrderedDomain for Weekday {
    type Density = Discrete;
    type Word = u32;
    const MIN: Option<Self> = Some(Weekday::Mon);
    const MAX: Option<Self> = Some(Weekday::Sun);
    fn encode(&self) -> u32 {
        *self as u32
    }
    fn decode(word: &u32) -> Option<Self> {
        Weekday::ALL.get(*word as usize).copied()
    }
}
impl DiscreteDomain for Weekday {
    fn succ(&self) -> Option<Self> {
        Weekday::ALL.get(*self as usize + 1).copied()
    }
    fn pred(&self) -> Option<Self> {
        (*self as usize).checked_sub(1).map(|i| Weekday::ALL[i])
    }
}
impl CoverPreserving for Weekday {}
impl CyclicDomain for Weekday {}

// ------------------------------------------------- Color (finite, no order)

/// A finite domain with no order claim (it does not derive `Ord`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Color {
    Red,
    Green,
    Blue,
    Yellow,
}

impl Domain for Color {}
impl FiniteDomain for Color {
    const CARDINALITY: usize = 4;
    fn index(&self) -> usize {
        *self as usize
    }
    fn from_index(i: usize) -> Option<Self> {
        [Color::Red, Color::Green, Color::Blue, Color::Yellow]
            .get(i)
            .copied()
    }
}

// -------------------------------------------------- Ipv4Prefix (prefix)

/// A canonical IPv4 CIDR prefix (host bits must be zero).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ipv4Prefix {
    net: u32,
    len: u8,
}

impl Ipv4Prefix {
    pub fn new(addr: Ipv4Addr, len: u8) -> Option<Self> {
        let net = u32::from(addr);
        (len <= 32 && net & !Self::mask(len) == 0).then_some(Ipv4Prefix { net, len })
    }
    fn mask(len: u8) -> u32 {
        if len == 0 { 0 } else { u32::MAX << (32 - len) }
    }
}
impl Domain for Ipv4Prefix {}
impl PrefixDomain for Ipv4Prefix {
    type Key = Ipv4Addr;
    fn to_interval(&self) -> Interval<Ipv4Addr> {
        let last = self.net | !Self::mask(self.len);
        Interval::closed(Ipv4Addr::from(self.net), Ipv4Addr::from(last))
    }
}

// ---------------------------------------------------- Perms (lattice)

/// Permission bits under subset order: a finite lattice (join = OR, meet = AND).
/// It derives `Ord` for storage, but its domain order is the partial subset order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Perms(pub u8);

impl Domain for Perms {}
impl Poset for Perms {
    fn leq(&self, o: &Self) -> bool {
        self.0 & !o.0 == 0
    }
}
impl Lattice for Perms {
    fn join(&self, o: &Self) -> Self {
        Perms(self.0 | o.0)
    }
    fn meet(&self, o: &Self) -> Self {
        Perms(self.0 & o.0)
    }
}
impl BoundedLattice for Perms {
    fn bottom() -> Self {
        Perms(0)
    }
    fn top() -> Self {
        Perms(u8::MAX)
    }
}

// ------------------------------------------------------ Unit (dense float)

/// A real number in `[0, 1]`: a dense domain with both extrema.
/// The constructor rejects NaN and out-of-interval input and stores `-0.0` as
/// `+0.0`, so every value has one representation and `total_cmp` is the
/// numeric order.
#[derive(Clone, Copy, Debug)]
pub struct Unit(f64);

impl Unit {
    pub fn new(x: f64) -> Option<Self> {
        (0.0..=1.0).contains(&x).then_some(Unit(x + 0.0))
    }
    pub const fn get(self) -> f64 {
        self.0
    }
}
impl PartialEq for Unit {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Unit {}
impl PartialOrd for Unit {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Unit {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0)
    }
}
impl Domain for Unit {}
impl OrderedDomain for Unit {
    type Density = Dense;
    const MIN: Option<Self> = Some(Unit(0.0));
    const MAX: Option<Self> = Some(Unit(1.0));
    frost_interval::identity_encoding!();
}

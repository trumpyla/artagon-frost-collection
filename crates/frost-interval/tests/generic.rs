//! Generic interval layer: `IntoInterval`, `Region`, `CyclicInterval`, `FrozenIntervalMap`, and
//! the numeric, float, and std temporal domains.
#![allow(
    clippy::many_single_char_names,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_bitwise_bool,
    clippy::items_after_statements,
    unreachable_pub,
    reason = "test code"
)]
mod support;
use support::laws;

use core::num::NonZero;
use core::ops::Bound;
use frost_interval::*;
use proptest::prelude::*;
use std::mem::size_of;
#[cfg(feature = "std")]
use std::time::{Duration, SystemTime};
use support::demo::*;

fn m(v: u8) -> Month {
    Month::new(v).unwrap()
}
fn months() -> Vec<Month> {
    (1..=12).map(m).collect()
}
fn f(x: f64) -> NotNanF64 {
    NotNanF64::new(x).unwrap()
}

// ------------------------------------------------------- numeric domains

#[test]
fn integer_domains_obey_the_laws() {
    laws::check_cover_preserving_sampled(&[0u32, 1, 2, 1 << 31, u32::MAX - 1, u32::MAX], 40);
    laws::check_cover_preserving_sampled(&[0u64, 1, 2, 1 << 63, u64::MAX - 1, u64::MAX], 40);
    laws::check_cover_preserving_sampled(&[0usize, 1, usize::MAX / 2, usize::MAX], 40);
    laws::check_cover_preserving_sampled(&[i32::MIN, -1, 0, 1, i32::MAX], 40);
    laws::check_cover_preserving_sampled(&[i64::MIN, -1, 0, 1, i64::MAX], 40);
    laws::check_cover_preserving_sampled(&[isize::MIN, -1, 0, 1, isize::MAX], 40);
    laws::check_cover_preserving_exhaustive::<NonZero<u8>>(1 << 9);
    laws::check_cover_preserving_exhaustive::<NonZero<u16>>(1 << 17);
    let nz = |v: u64| NonZero::new(v).unwrap();
    laws::check_cover_preserving_sampled(&[nz(1), nz(2), nz(u64::MAX - 1), nz(u64::MAX)], 40);
    let nz32 = |v: u32| NonZero::new(v).unwrap();
    laws::check_cover_preserving_sampled(&[nz32(1), nz32(2), nz32(u32::MAX)], 40);
    let u = [0u128, 1, 2, u128::MAX / 2, u128::MAX - 1, u128::MAX];
    laws::check_ordered(&u);
    laws::check_discrete(&u);
    let w = [i128::MIN, i128::MIN + 1, -1, 0, 1, i128::MAX - 1, i128::MAX];
    laws::check_ordered(&w);
    laws::check_discrete(&w);
}

#[test]
fn integer_sets_cost_one_word_per_boundary() {
    // Narrow types use u32 words, wide types u64, 128-bit types their own size.
    macro_rules! same_size { ($($a:ty = $b:ty),*) => {$(
        assert_eq!(size_of::<FrozenIntervalSet<$a>>(), size_of::<(bool, Vec<$b>)>());
    )*}; }
    same_size!(u8 = u32, i8 = u32, u16 = u32, i16 = u32, u32 = u32, i32 = u32,
               u64 = u64, i64 = u64, usize = u64, isize = u64, u128 = u128, i128 = i128,
               NonZero<u8> = u32, NonZero<u64> = u64, NotNanF32 = u32, NotNanF64 = u64, char = u32);
    let s: FrozenIntervalSet<i64> = [-5..=-1, 3..=9].into_iter().collect();
    assert_eq!(s.words().len(), 4);
    assert_eq!(s.heap_bytes(), 4 * 8);
    let n: FrozenIntervalSet<NonZero<u8>> = FrozenIntervalSet::all();
    assert_eq!(n.count(), 255);
}

// ------------------------------------------------------------- floats

#[test]
fn float_domains_obey_the_laws() {
    let s64 = [
        f64::NEG_INFINITY,
        f64::MIN,
        -1.0,
        -f64::MIN_POSITIVE,
        -5e-324,
        0.0,
        5e-324,
        f64::MIN_POSITIVE,
        1.0,
        f64::MAX,
        f64::INFINITY,
    ]
    .map(f);
    laws::check_cover_preserving_sampled(&s64, 64);
    let s32 = [
        f32::NEG_INFINITY,
        f32::MIN,
        -1.0,
        -f32::MIN_POSITIVE,
        -1e-45,
        0.0,
        1e-45,
        f32::MIN_POSITIVE,
        1.0,
        f32::MAX,
        f32::INFINITY,
    ]
    .map(|x| NotNanF32::new(x).unwrap());
    laws::check_cover_preserving_sampled(&s32, 64);
    assert!(NotNanF64::new(f64::NAN).is_none() && NotNanF32::new(f32::NAN).is_none());
    assert_eq!(f(-0.0), f(0.0));
    assert!(f(-0.0).get().is_sign_positive());
    assert_eq!(size_of::<NotNanF64>(), 8);
    assert_eq!(size_of::<NotNanF32>(), 4);
    assert_eq!(NotNanF64::try_from(1.5).map(f64::from), Ok(1.5));
    assert_eq!(NotNanF64::try_from(f64::NAN), Err(NanError));
}

/// Every `f32` value: 4,278,190,081 steps. Run with `--ignored`.
#[test]
#[ignore = "walks all 2^32 - 2^24 + 1 f32 values (about 2 minutes in release)"]
fn f32_exhaustive_walk() {
    laws::check_cover_preserving_exhaustive::<NotNanF32>(usize::MAX);
}

#[test]
fn float_sets_are_sets_of_floats() {
    // Discrete at one ULP: [0, 1) is [0, next_down(1)].
    assert_eq!(
        NotNanF64::closed_open(f(0.0), f(1.0)),
        NotNanF64::closed(f(0.0), f(1.0f64.next_down()))
    );
    assert!(NotNanF64::open(f(1.0), f(1.0f64.next_up())).is_empty());
    assert_eq!(
        NotNanF64::closed(NotNanF64::NEG_INFINITY, NotNanF64::INFINITY),
        NotNanF64::all()
    );
    // Exact counts of representable values.
    let s: FrozenIntervalSet<NotNanF64> = [NotNanF64::interval(1.0..2.0).unwrap()]
        .into_iter()
        .collect();
    assert_eq!(s.count(), 1 << 52);
    assert_eq!(
        FrozenIntervalSet::<NotNanF32>::all().count(),
        (1u128 << 32) - (1 << 24) + 1
    );
    assert!(s.contains(&f(1.5)) && !s.contains(&f(2.0)) && !s.contains(&f(0.999)));
    assert!(NotNanF64::interval(0.0..f64::NAN).is_none());
    // Adjacent float intervals coalesce: [0, 1) and [1, 2] are one interval.
    let t: FrozenIntervalSet<NotNanF64> = [
        NotNanF64::interval(0.0..1.0).unwrap(),
        NotNanF64::interval(1.0..=2.0).unwrap(),
    ]
    .into_iter()
    .collect();
    assert_eq!(t.interval_count(), 1);
    assert_eq!(t.closed_intervals(), [(f(0.0), f(2.0))]);
    // Negative and positive zero are one value.
    let z: FrozenIntervalSet<NotNanF64> = [NotNanF64::singleton(f(-0.0))].into_iter().collect();
    assert!(z.contains(&f(0.0)) && z.count() == 1);
}

// ------------------------------------------------------- std temporal

#[test]
#[cfg(feature = "std")]
fn std_time_domains() {
    let ns = Duration::from_nanos(1);
    let d = [
        Duration::ZERO,
        ns,
        Duration::new(0, 999_999_999),
        Duration::from_secs(1),
        Duration::MAX.checked_sub(ns).unwrap(),
        Duration::MAX,
    ];
    laws::check_ordered(&d);
    laws::check_discrete(&d);
    let s = Duration::from_secs;
    // Discrete at 1 ns: [0 s, 1 s] and (1 s, 2 s] are one interval.
    let set: FrozenIntervalSet<Duration> = [
        Duration::closed(s(0), s(1)),
        Duration::open_closed(s(1), s(2)),
    ]
    .into_iter()
    .collect();
    assert_eq!(set.interval_count(), 1);
    assert_eq!(set.closed_intervals(), [(s(0), s(2))]);
    assert_eq!(
        Duration::closed(Duration::ZERO, Duration::MAX),
        Duration::all()
    );

    let t = |secs| SystemTime::UNIX_EPOCH + s(secs);
    laws::check_ordered(&[t(0), t(1), t(2), t(1_000_000)]);
    let st: FrozenIntervalSet<SystemTime> = [t(10)..t(20), t(20)..t(30)].into_iter().collect();
    assert_eq!(st.interval_count(), 1);
    assert!(st.contains(&t(20)) && !st.contains(&t(30)));
    assert_eq!(SystemTime::min_cut(), Cut::BelowAll);
    let now = std::time::Instant::now();
    let i = |secs| now + s(secs);
    laws::check_ordered(&[i(0), i(1), i(5)]);
    let si: FrozenIntervalSet<std::time::Instant> = FrozenIntervalSet::of(i(1)..=i(5));
    assert!(si.contains(&i(5)) && !si.contains(&i(0)));
}

// ------------------------------------------------- order embedding

#[test]
fn encoding_lifts_to_sets() {
    // word: FrozenIntervalSet<Month> -> FrozenIntervalSet<u32>, for every subset.
    for mask in 0u32..(1 << 12) {
        let s: FrozenIntervalSet<Month> = months()
            .iter()
            .filter(|v| mask >> (v.get() - 1) & 1 == 1)
            .map(|&v| Month::singleton(v))
            .collect();
        let img = s.to_words();
        for w in 0u32..=14 {
            assert_eq!(
                img.contains(&w),
                (1..=12).contains(&w) && mask >> (w - 1) & 1 == 1,
                "mask {mask:#b} word {w}"
            );
        }
        assert_eq!(img.interval_count(), s.interval_count());
        assert_eq!(FrozenIntervalSet::<Month>::from_words(&img), s);
    }
    // from_words drops words that encode no value.
    let words: FrozenIntervalSet<u32> = FrozenIntervalSet::from_intervals([0..=2, 11..=40]);
    assert_eq!(
        FrozenIntervalSet::<Month>::from_words(&words).closed_intervals(),
        [(m(1), m(2)), (m(11), m(12))]
    );
    assert!(FrozenIntervalSet::<Month>::from_words(&FrozenIntervalSet::all()).is_all());
    // A set of ids and a set of plain numbers can meet in word space.
    let ids: FrozenIntervalSet<UserId> = FrozenIntervalSet::of(UserId(10)..UserId(20));
    let even_block: FrozenIntervalSet<u64> = FrozenIntervalSet::of(15..100);
    let both = FrozenIntervalSet::<UserId>::from_words(&ids.to_words().intersection(&even_block));
    assert_eq!(both.closed_intervals(), [(UserId(15), UserId(19))]);
    // Signed and float domains: the word set has the same shape.
    let neg: FrozenIntervalSet<i64> = FrozenIntervalSet::from_intervals([-5..=-1, 3..=9]);
    assert_eq!(neg.to_words().count(), neg.count());
    assert_eq!(FrozenIntervalSet::<i64>::from_words(&neg.to_words()), neg);
    let fl: FrozenIntervalSet<NotNanF64> = [NotNanF64::interval(-1.0..=1.0).unwrap()]
        .into_iter()
        .collect();
    assert_eq!(
        FrozenIntervalSet::<NotNanF64>::from_words(&fl.to_words()),
        fl
    );
}

#[test]
fn newtype_domains_take_one_line() {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct OrderId(u64);
    newtype_domain!(OrderId(u64): discrete, cover_preserving);
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct Celsius(NotNanF64);
    newtype_domain!(Celsius(NotNanF64): discrete, cover_preserving);
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct Name(String);
    newtype_domain!(Name(String): dense, extrema(Some(Name(String::new())), None));
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct Wide(i128);
    newtype_domain!(Wide(i128): discrete);

    laws::check_cover_preserving_sampled(&[OrderId(0), OrderId(7), OrderId(u64::MAX)], 20);
    let c = |x| Celsius(f(x));
    laws::check_cover_preserving_sampled(&[c(-273.15), c(0.0), c(100.0)], 20);
    laws::check_ordered(&[Name(String::new()), Name("a".into()), Name("b".into())]);
    laws::check_ordered(&[Wide(i128::MIN), Wide(0), Wide(i128::MAX)]);
    laws::check_discrete(&[Wide(i128::MIN), Wide(0), Wide(i128::MAX)]);

    // Same word and layout as the field type.
    let s: FrozenIntervalSet<OrderId> = FrozenIntervalSet::of(OrderId(5)..OrderId(9));
    assert_eq!(s.words(), FrozenIntervalSet::<u64>::of(5..9).words());
    assert_eq!(s.count(), 4);
    assert_eq!(OrderId::min_cut(), Cut::Below(OrderId(0)));
    let liquid: FrozenIntervalSet<Celsius> = FrozenIntervalSet::of(c(0.0)..c(100.0));
    assert!(liquid.contains(&c(36.6)) && !liquid.contains(&c(100.0)));
    let names: FrozenIntervalSet<Name> = FrozenIntervalSet::of(Name("a".into())..Name("m".into()));
    assert!(names.contains(&Name("frost".into())) && !names.contains(&Name("m".into())));
    assert_eq!(Name::min_cut(), Cut::Below(Name(String::new())));
}

/// A dense domain that counts its clones.
mod counted {
    use frost_interval::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    pub static CLONES: AtomicUsize = AtomicUsize::new(0);
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Counted(pub u32);
    impl Clone for Counted {
        fn clone(&self) -> Self {
            CLONES.fetch_add(1, Ordering::Relaxed);
            Counted(self.0)
        }
    }
    impl Domain for Counted {}
    impl OrderedDomain for Counted {
        type Density = Dense;
        identity_encoding!();
    }
}

#[test]
fn identity_encoding_never_clones_a_probe() {
    use counted::{CLONES, Counted};
    use std::sync::atomic::Ordering;
    let set: FrozenIntervalSet<Counted> =
        FrozenIntervalSet::from_intervals([Counted(1)..Counted(5), Counted(8)..Counted(9)]);
    let map: FrozenIntervalMap<Counted, u8> = [(Counted(1)..Counted(5), 7u8)].into_iter().collect();
    let before = CLONES.load(Ordering::Relaxed);
    let hits = (0..10).filter(|&x| set.contains(&Counted(x))).count();
    let found = (0..10).filter(|&x| map.get(&Counted(x)).is_some()).count();
    assert_eq!((hits, found), (5, 4));
    assert!(!set.is_disjoint(&set) && set.is_subset(&set));
    assert_eq!(
        CLONES.load(Ordering::Relaxed),
        before,
        "a query cloned a value"
    );
}

// ---------------------------------------------------------- IntoInterval

#[test]
#[allow(clippy::reversed_empty_ranges)] // `5..3` is the empty-interval case under test
fn any_std_range_is_an_interval() {
    let a: FrozenIntervalSet<u64> = FrozenIntervalSet::from_intervals([1..5, 10..20]);
    assert_eq!(a.closed_intervals(), [(1, 4), (10, 19)]);
    assert_eq!(
        FrozenIntervalSet::<u64>::of(..=3).closed_intervals(),
        [(0, 3)]
    );
    assert_eq!(
        FrozenIntervalSet::<u64>::of(7..).closed_intervals(),
        [(7, u64::MAX)]
    );
    assert!(FrozenIntervalSet::<u64>::of(..).is_all());
    assert_eq!(
        FrozenIntervalSet::<u64>::of((Bound::Excluded(1), Bound::Included(4))).closed_intervals(),
        [(2, 4)]
    );
    assert_eq!(
        FrozenIntervalSet::<u64>::of(core::range::Range::from(2..6)),
        FrozenIntervalSet::of(2..6)
    );
    assert_eq!(
        FrozenIntervalSet::<u64>::of(core::range::RangeInclusive::from(2..=5)),
        FrozenIntervalSet::of(2..6)
    );
    assert!(FrozenIntervalSet::<u64>::of(5..3).is_empty());
    assert!(
        a.is_superset_of_interval(2..=3)
            && !a.is_superset_of_interval(3..=5)
            && a.is_superset_of_interval(5..5)
    );
    assert!(!a.is_disjoint_from_interval(4..11) && a.is_disjoint_from_interval(5..10));
    // A domain type works the same way.
    let q: FrozenIntervalSet<Month> =
        FrozenIntervalSet::from_intervals([m(1)..=m(3), m(10)..=m(12)]);
    assert_eq!(q.count(), 6);
    // Round trip through std bounds.
    let r = Interval::closed(2u64, 9);
    assert_eq!(
        r.to_bounds(),
        Some((Bound::Included(&2), Bound::Excluded(&10)))
    );
    assert_eq!(
        Interval::<u64>::from_bounds(
            r.to_bounds()
                .map(|(a, b)| (a.cloned(), b.cloned()))
                .unwrap()
        ),
        r
    );
    assert_eq!(Interval::<u64>::empty().to_bounds(), None);
    // Dense domains keep both bound kinds.
    let (a, b) = ("a".to_string(), "b".to_string());
    assert_eq!(
        Interval::closed(a.clone(), b.clone()).to_bounds(),
        Some((Bound::Included(&a), Bound::Included(&b)))
    );
    assert_eq!(
        Interval::open(a.clone(), b.clone()).to_bounds(),
        Some((Bound::Excluded(&a), Bound::Excluded(&b)))
    );
    assert_eq!(
        Interval::<u64>::all().to_bounds(),
        Some((Bound::Unbounded, Bound::Unbounded))
    );
}

// ------------------------------------------------------------ regions

fn month_intervals() -> Vec<Interval<Month>> {
    let mut v = vec![Interval::empty()];
    for a in 1..=12 {
        for b in a..=12 {
            v.push(Interval::closed(m(a), m(b)));
        }
    }
    v
}

#[test]
fn region_laws_for_every_family() {
    // Total order: all 79 distinct intervals of Month.
    let intervals = month_intervals();
    assert_eq!(intervals.len(), 79);
    laws::check_region(&intervals, &months());
    laws::check_convex(&intervals);

    // Interval sets.
    let sets: Vec<FrozenIntervalSet<Month>> =
        [0u16, 0xFFF, 1, 0x800, 0x555, 0x0F0, 0xE07, 0x3C, 0x030]
            .iter()
            .map(|mask| {
                months()
                    .iter()
                    .filter(|v| mask >> (v.get() - 1) & 1 == 1)
                    .map(|&v| Month::singleton(v))
                    .collect()
            })
            .collect();
    laws::check_region(&sets, &months());

    // Categorical: all 16 subsets.
    let colors = [Color::Red, Color::Green, Color::Blue, Color::Yellow];
    let csets: Vec<FiniteSet<Color>> = (0u8..16)
        .map(|k| {
            colors
                .iter()
                .filter(|c| k >> c.index() & 1 == 1)
                .copied()
                .collect()
        })
        .collect();
    laws::check_region(&csets, &colors);

    // Lattice intervals over permission bits.
    let pts: Vec<Perms> = [0u8, 1, 2, 3, 4, 5, 6, 7, 255].map(Perms).to_vec();
    let mut ivs = vec![LatticeInterval::empty(), LatticeInterval::all()];
    for a in &pts {
        for b in &pts {
            ivs.push(LatticeInterval::new(*a, *b));
        }
    }
    ivs.dedup();
    laws::check_region(&ivs, &pts);
    laws::check_convex(&ivs[..40]);
    assert!(LatticeInterval::new(Perms(1), Perms(7)).contains(&Perms(5)));
    assert!(!LatticeInterval::new(Perms(1), Perms(7)).contains(&Perms(4)));
    assert!(LatticeInterval::new(Perms(1), Perms(2)).is_empty()); // 1 is not below 2
    assert_eq!(
        LatticeInterval::up_from(Perms(3)).intersection(&LatticeInterval::down_from(Perms(7))),
        LatticeInterval::new(Perms(3), Perms(7))
    );

    // Products: (month interval) x (color subset), and a convex box.
    let some_intervals = [
        Interval::empty(),
        Interval::all(),
        Interval::closed(m(1), m(3)),
        Interval::closed(m(3), m(8)),
        Interval::singleton(m(12)),
    ];
    let some_colors = [csets[0], csets[15], csets[5], csets[6]];
    let boxes: Vec<ProductRegion<Interval<Month>, FiniteSet<Color>>> = some_intervals
        .iter()
        .flat_map(|r| {
            some_colors
                .iter()
                .map(move |c| ProductRegion::new(r.clone(), *c))
        })
        .collect();
    let pairs: Vec<(Month, Color)> = months()
        .into_iter()
        .flat_map(|mo| colors.map(move |c| (mo, c)))
        .collect();
    laws::check_region(&boxes, &pairs);
    let cboxes: Vec<ProductRegion<Interval<Month>, LatticeInterval<Perms>>> = some_intervals
        .iter()
        .flat_map(|r| {
            ivs[..12]
                .iter()
                .map(move |i| ProductRegion::new(r.clone(), i.clone()))
        })
        .collect();
    let cpairs: Vec<(Month, Perms)> = months()
        .into_iter()
        .flat_map(|mo| [0u8, 1, 3, 7, 255].map(move |p| (mo, Perms(p))))
        .collect();
    laws::check_region(&cboxes, &cpairs);
    laws::check_convex(&cboxes);
}

/// Generic code written once against `Region` works for every family.
fn common_part<R: Region>(regions: &[R]) -> R {
    regions.iter().fold(R::all(), |acc, r| acc.intersection(r))
}

#[test]
fn generic_code_over_regions() {
    assert_eq!(
        common_part(&[
            Interval::closed(1u64, 9),
            Interval::closed(5, 20),
            Interval::at_least(7)
        ]),
        Interval::closed(7, 9)
    );
    assert_eq!(
        common_part(&[
            NotNanF64::interval(0.0..=1.0).unwrap(),
            NotNanF64::interval(0.5..).unwrap()
        ]),
        NotNanF64::interval(0.5..=1.0).unwrap()
    );
    let sets: [FrozenIntervalSet<u64>; 2] = [
        FrozenIntervalSet::from_intervals([0..10, 20..30]),
        FrozenIntervalSet::of(5..25),
    ];
    assert_eq!(
        common_part(&sets),
        FrozenIntervalSet::from_intervals([5..10, 20..25])
    );
    assert_eq!(common_part::<Interval<u64>>(&[]), Interval::all());
}

// ---------------------------------------------------- cyclic intervals

#[test]
fn cyclic_intervals_exhaustive() {
    let days = Weekday::ALL;
    let mask = |r: &CyclicInterval<Weekday>| {
        days.iter()
            .filter(|d| r.contains(d))
            .fold(0u8, |b, &d| b | 1 << d as u8)
    };
    let mut arcs = vec![(CyclicInterval::empty(), 0u8), (CyclicInterval::all(), 127)];
    for &a in &days {
        for &b in &days {
            // Model: walk forward from a.
            let (mut closed, mut d) = (0u8, a);
            loop {
                closed |= 1 << d as u8;
                if d == b {
                    break;
                }
                d = d.wrapping_succ();
            }
            let half = if a == b { 0 } else { closed & !(1 << b as u8) };
            arcs.push((CyclicInterval::closed(a, b), closed));
            arcs.push((CyclicInterval::closed_open(a, b), half));
            assert_eq!(CyclicInterval::closed(a, b).wraps(), a > b && closed != 127);
        }
    }
    for (r, want) in &arcs {
        assert_eq!(mask(r), *want, "{r:?}");
        assert_eq!(r.is_empty(), *want == 0);
        assert_eq!(r.is_all(), *want == 127);
        let c = r.complement();
        assert_eq!(mask(&c), !want & 127, "complement of {r:?}");
        assert_eq!(&c.complement(), r);
        let set: FrozenIntervalSet<Weekday> = r.to_set();
        assert_eq!(set.count(), u128::from(want.count_ones()));
        for (q, other) in &arcs {
            // One canonical form per arc.
            assert_eq!(r == q, want == other, "{r:?} vs {q:?}");
            let i: FrozenIntervalSet<Weekday> = r.intersection(q);
            let got = days
                .iter()
                .filter(|d| i.contains(d))
                .fold(0u8, |b, &d| b | 1 << d as u8);
            assert_eq!(got, want & other);
        }
    }
    // Two arcs can share two pieces: Fri..Tue and Mon..Sat.
    use Weekday::*;
    let two: FrozenIntervalSet<Weekday> =
        CyclicInterval::closed(Fri, Tue).intersection(&CyclicInterval::closed(Mon, Sat));
    assert_eq!(two.closed_intervals(), [(Mon, Tue), (Fri, Sat)]);
}

// ---------------------------------------------------------- interval maps

#[test]
fn map_examples() {
    // Guava: put([1, 10], a) then put([3, 6], b) splits the first entry.
    let g: FrozenIntervalMap<u64, char> = [(1..=10, 'a'), (3..=6, 'b')].into_iter().collect();
    let e: Vec<_> = g.entries().map(|(r, v)| (r, *v)).collect();
    assert_eq!(
        e,
        [
            (Interval::closed(1, 2), 'a'),
            (Interval::closed(3, 6), 'b'),
            (Interval::closed(7, 10), 'a')
        ]
    );
    assert_eq!(g.get(&6), Some(&'b'));
    assert_eq!(g.get(&11), None);
    assert_eq!(g.get_entry(&8), Some((Interval::closed(7, 10), &'a')));
    // Equal neighbors coalesce (discrete adjacency included).
    let c: FrozenIntervalMap<u64, char> = [(1..=4, 'x'), (5..=9, 'x')].into_iter().collect();
    assert_eq!(c.entry_count(), 1);
    assert_eq!(c.boundary_count(), 2);
    // Boost.ICL: aggregate on overlap.
    let party = |r, names: &[&str]| -> FrozenIntervalMap<u64, Vec<String>> {
        [(r, names.iter().map(ToString::to_string).collect::<Vec<_>>())]
            .into_iter()
            .collect()
    };
    let guests = party(8..11, &["John", "Denis"]).union_with(&party(9..12, &["Katie"]), |a, b| {
        [a.clone(), b.clone()].concat()
    });
    let e: Vec<_> = guests.entries().map(|(r, v)| (r, v.join("+"))).collect();
    assert_eq!(
        e,
        [
            (Interval::closed(8, 8), "John+Denis".to_string()),
            (Interval::closed(9, 10), "John+Denis+Katie".to_string()),
            (Interval::closed(11, 11), "Katie".to_string())
        ]
    );
    // Dense keys: the value changes exactly at the cut.
    let d: FrozenIntervalMap<String, u8> = [
        (Interval::closed_open("a".to_string(), "m".to_string()), 1),
        (Interval::at_least("m".to_string()), 2),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        (
            d.get(&"lzz".into()),
            d.get(&"m".into()),
            d.get(&String::new())
        ),
        (Some(&1), Some(&2), None)
    );
    // Floats as keys: a tax-bracket style lookup.
    let rate: FrozenIntervalMap<NotNanF64, u8> = [
        (NotNanF64::interval(..10_000.0).unwrap(), 0),
        (NotNanF64::interval(10_000.0..).unwrap(), 20),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        (rate.get(&f(9_999.99)), rate.get(&f(10_000.0))),
        (Some(&0), Some(&20))
    );
    assert!(rate.gaps().is_empty());
    assert_eq!(
        size_of::<FrozenIntervalMap<u64, u32>>(),
        size_of::<(Vec<u64>, Vec<Option<u32>>)>()
    );
}

type Spec = Vec<(u8, u8, u8, u8)>;

fn spec() -> impl Strategy<Value = Spec> {
    prop::collection::vec((1u8..=12, 1u8..=12, 0u8..6, 0u8..3), 0..7)
}

fn spec_interval(a: u8, b: u8, kind: u8) -> Interval<Month> {
    match kind {
        0 => Interval::closed(m(a), m(b)),
        1 => Interval::open(m(a), m(b)),
        2 => Interval::closed_open(m(a), m(b)),
        3 => Interval::at_least(m(a)),
        4 => Interval::at_most(m(b)),
        _ => Interval::open_closed(m(a), m(b)),
    }
}

fn build_map(s: &Spec) -> (FrozenIntervalMap<Month, u8>, [Option<u8>; 12]) {
    let mut model = [None; 12];
    for &(a, b, k, v) in s {
        let r = spec_interval(a, b, k);
        for p in 1..=12 {
            if r.contains(&m(p)) {
                model[p as usize - 1] = Some(v);
            }
        }
    }
    (
        s.iter()
            .map(|&(a, b, k, v)| (spec_interval(a, b, k), v))
            .collect(),
        model,
    )
}

fn check_map<T: Clone + PartialEq + core::fmt::Debug>(
    map: &FrozenIntervalMap<Month, T>,
    model: &[Option<T>; 12],
) -> Result<(), TestCaseError> {
    for p in 1..=12u8 {
        prop_assert_eq!(map.get(&m(p)), model[p as usize - 1].as_ref(), "at {}", p);
    }
    // One stored form: entries tile the model and equal neighbors are merged.
    let runs = model
        .chunk_by(|a, b| a == b)
        .filter(|c| c[0].is_some())
        .count();
    prop_assert_eq!(map.entry_count(), runs);
    let rebuilt: FrozenIntervalMap<Month, T> = map.entries().map(|(r, v)| (r, v.clone())).collect();
    prop_assert_eq!(&rebuilt, map);
    let keys = map.key_set();
    for p in 1..=12u8 {
        prop_assert_eq!(keys.contains(&m(p)), model[p as usize - 1].is_some());
        prop_assert_eq!(map.gaps().contains(&m(p)), model[p as usize - 1].is_none());
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4000))]

    #[test]
    fn map_matches_point_model(sa in spec(), sb in spec(), smask in 0u16..4096) {
        let (a, ma) = build_map(&sa);
        let (b, mb) = build_map(&sb);
        check_map(&a, &ma)?;
        // The sweep build equals a fold of single-entry overlays (merge kernel).
        let folded = sa.iter().fold(FrozenIntervalMap::<Month, u8>::empty(), |acc, &(x, y, k, v)| {
            acc.overlay(&[(spec_interval(x, y, k), v)].into_iter().collect())
        });
        prop_assert_eq!(&folded, &a);

        let zip = |f: &dyn Fn(Option<u8>, Option<u8>) -> Option<u8>| -> [Option<u8>; 12] {
            core::array::from_fn(|i| f(ma[i], mb[i]))
        };
        check_map(&a.overlay(&b), &zip(&|x, y| y.or(x)))?;
        check_map(&a.union_with(&b, |x, y| x + y), &zip(&|x, y| match (x, y) {
            (Some(x), Some(y)) => Some(x + y),
            (x, y) => x.or(y),
        }))?;
        check_map(&a.intersection_with(&b, |x, y| x * 3 + y), &zip(&|x, y| Some(x? * 3 + y?)))?;
        check_map(&a.map_values(|v| v / 2), &core::array::from_fn(|i| ma[i].map(|v| v / 2)))?;

        let set: FrozenIntervalSet<Month> = months().iter().filter(|v| smask >> (v.get() - 1) & 1 == 1).map(|&v| Month::singleton(v)).collect();
        let inside = |i: usize| smask >> i & 1 == 1;
        check_map(&a.restrict(&set), &core::array::from_fn(|i| ma[i].filter(|_| inside(i))))?;
        check_map(&a.remove(&set), &core::array::from_fn(|i| ma[i].filter(|_| !inside(i))))?;
        check_map(&a.overlapping(m(4)..=m(9)), &core::array::from_fn(|i| ma[i].filter(|_| (3..9).contains(&i))))?;
        check_map(&FrozenIntervalMap::from_set(&set, 9u8), &core::array::from_fn(|i| inside(i).then_some(9)))?;

        // Set kernels against the same model.
        let (ka, kb) = (a.key_set(), b.key_set());
        prop_assert_eq!(ka.is_subset(&kb), (0..12).all(|i| ma[i].is_none() || mb[i].is_some()));
        prop_assert_eq!(ka.is_disjoint(&kb), (0..12).all(|i| ma[i].is_none() || mb[i].is_none()));
    }

    #[test]
    fn map_over_u64_edges(entries in prop::collection::vec((prop_oneof![0u64..8, (u64::MAX - 3)..=u64::MAX],
                                                            prop_oneof![0u64..8, (u64::MAX - 3)..=u64::MAX], 0u8..5, 0u8..3), 0..7)) {
        let interval = |a: u64, b: u64, k: u8| match k {
            0 => Interval::closed(a, b), 1 => Interval::closed_open(a, b), 2 => Interval::at_least(a),
            3 => Interval::at_most(b), _ => Interval::all(),
        };
        let map: FrozenIntervalMap<u64, u8> = entries.iter().map(|&(a, b, k, v)| (interval(a, b, k), v)).collect();
        for x in (0..10).chain((u64::MAX - 5)..=u64::MAX) {
            let want = entries.iter().rev().find(|&&(a, b, k, _)| interval(a, b, k).contains(&x)).map(|e| e.3);
            prop_assert_eq!(map.get(&x).copied(), want);
        }
    }

    #[test]
    fn f64_code_is_the_numeric_order(a in any::<f64>(), b in any::<f64>()) {
        prop_assume!(!a.is_nan() && !b.is_nan());
        let (x, y) = (f(a), f(b));
        prop_assert_eq!(Some(x.cmp(&y)), a.partial_cmp(&b));
        prop_assert_eq!(NotNanF64::decode(&x.encode()), Some(x));
        prop_assert_eq!(x.get() == a, true);
        if a < f64::INFINITY { prop_assert_eq!(x.succ().unwrap().get(), a.next_up()); } else { prop_assert!(x.succ().is_none()); }
        if a > f64::NEG_INFINITY { prop_assert_eq!(x.pred().unwrap().get(), a.next_down()); } else { prop_assert!(x.pred().is_none()); }
        prop_assert_eq!(Interval::closed(x, y).contains(&f(0.0)), a <= 0.0 && 0.0 <= b);
    }

    #[test]
    fn f32_code_is_the_numeric_order(a in any::<f32>(), b in any::<f32>()) {
        prop_assume!(!a.is_nan() && !b.is_nan());
        let (x, y) = (NotNanF32::new(a).unwrap(), NotNanF32::new(b).unwrap());
        prop_assert_eq!(Some(x.cmp(&y)), a.partial_cmp(&b));
        prop_assert_eq!(NotNanF32::decode(&x.encode()), Some(x));
        if a < f32::INFINITY { prop_assert_eq!(x.succ().unwrap().get(), a.next_up()); }
    }
}

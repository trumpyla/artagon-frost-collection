//! Value domains: encodings, extrema, canonical cuts, ranks, and the law kit on every shipped domain.
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
use frost_interval::*;
use proptest::prelude::*;
use std::mem::size_of;
use std::net::Ipv4Addr;
use support::demo::*;
use support::laws;

fn m(v: u8) -> Month {
    Month::new(v).unwrap()
}
fn unit(x: f64) -> Unit {
    Unit::new(x).unwrap()
}

// --------------------------------------------------------------- zero cost

#[test]
fn domain_adds_no_bytes() {
    assert_eq!(
        size_of::<FrozenIntervalSet<UserId>>(),
        size_of::<FrozenIntervalSet<u64>>()
    );
    assert_eq!(
        size_of::<FrozenIntervalSet<Month>>(),
        size_of::<FrozenIntervalSet<u8>>()
    );
    assert_eq!(
        size_of::<FrozenIntervalSet<u64>>(),
        size_of::<(bool, Vec<u64>)>()
    );
    assert_eq!(size_of::<Month>(), 1);
    assert_eq!(size_of::<UserId>(), 8);
    assert_eq!(size_of::<FiniteSet<Color>>(), 16);
}

#[test]
fn equal_codes_give_equal_storage() {
    let a: FrozenIntervalSet<u64> = [u64::closed(3, 9), u64::at_least(100)]
        .into_iter()
        .collect();
    let b: FrozenIntervalSet<UserId> = [
        UserId::closed(UserId(3), UserId(9)),
        UserId::at_least(UserId(100)),
    ]
    .into_iter()
    .collect();
    assert_eq!(a.words(), b.words());
}

// -------------------------------------------------------- min/max cuts

#[test]
fn min_max_cuts() {
    assert_eq!(u64::min_cut(), Cut::Below(0));
    assert_eq!(u64::max_cut(), Cut::Above(u64::MAX));
    assert_eq!(Month::min_cut(), Cut::Below(m(1)));
    assert_eq!(Month::max_cut(), Cut::Above(m(12)));
    assert_eq!(String::min_cut(), Cut::Below(String::new()));
    assert_eq!(String::max_cut(), Cut::AboveAll); // no greatest string
    assert_eq!(Unit::min_cut(), Cut::Below(unit(0.0)));
    assert_eq!(Unit::max_cut(), Cut::Above(unit(1.0)));
    assert_eq!(Month::min_cut().canonical(), Cut::BelowAll);
    assert_eq!(Month::max_cut().canonical(), Cut::AboveAll);
    const { assert!(<u64 as OrderedDomainExt>::DISCRETE && !<String as OrderedDomainExt>::DISCRETE) };
}

#[test]
fn canonical_intervals() {
    assert_eq!(Month::closed(m(1), m(12)), Month::all());
    assert_eq!(Month::closed(m(2), m(4)), Month::closed_open(m(2), m(5)));
    assert_eq!(Month::open(m(1), m(5)), Month::closed(m(2), m(4)));
    assert_eq!(Month::greater_than(m(12)), Interval::empty());
    assert_eq!(u64::closed(0, u64::MAX), u64::all());
    assert_eq!(String::at_least(String::new()), String::all());
    assert!(String::less_than(String::new()).is_empty());
    let s = |x: &str| x.to_string();
    let touch: FrozenIntervalSet<String> =
        [String::closed(s("a"), s("b")), String::open(s("b"), s("c"))]
            .into_iter()
            .collect();
    assert_eq!(touch.interval_count(), 1);
    let gap: FrozenIntervalSet<String> = [
        String::closed_open(s("a"), s("b")),
        String::open(s("b"), s("c")),
    ]
    .into_iter()
    .collect();
    assert_eq!(gap.interval_count(), 2);
    assert!(!gap.contains(&s("b")) && gap.contains(&s("bz")));
}

// ------------------------------------ review findings, now regression tests

#[test]
fn f1_every_interval_is_canonical() {
    // There is no raw constructor: `Interval::closed` is the domain factory.
    assert_eq!(Interval::closed(2u64, 4), u64::closed(2, 4));
    assert_eq!(Interval::closed(2u64, 4), Interval::open(1u64, 5));
    assert!(Interval::closed(1u64, 5).is_connected(&Interval::closed(6u64, 9)));
    assert!(Interval::open(3u64, 4).is_empty());
    assert!(
        Interval::closed(1u64, 5)
            .gap(&Interval::closed(6u64, 9))
            .is_empty()
    );
    assert_eq!(
        Interval::closed(1u64, 5).gap(&Interval::closed(8u64, 9)),
        Interval::closed(6, 7)
    );
    // Dense: (3, 4) is not empty, and [1, 5] does not touch (5.5, 9].
    assert!(!Interval::open(unit(0.25), unit(0.5)).is_empty());
    assert!(!Interval::closed(unit(0.0), unit(0.25)).is_connected(
        &Interval::open(unit(0.25), unit(0.5)).intersection(&Interval::greater_than(unit(0.3)))
    ));
}

#[test]
fn f9_empty_interval_algebra() {
    let e = Interval::<u64>::empty();
    assert!(e.gap(&u64::closed(1, 2)).is_empty());
    assert!(e.is_connected(&u64::closed(1, 2)) && e.is_connected(&u64::at_least(1)));
    assert_eq!(e.hull(&u64::closed(1, 2)), u64::closed(1, 2));
    assert!(e.intersection(&u64::all()).is_empty());
    assert!(u64::all().is_superset(&e) && e.is_superset(&e) && !e.is_superset(&u64::closed(1, 2)));
}

#[test]
fn f4_boundary_of_a_cut_is_total() {
    use frost_interval::kernel::RawCut;
    assert_eq!(
        <Discrete as Density<u64>>::boundary(Cut::Above(3)),
        RawCut::At(4)
    );
    assert_eq!(
        <Discrete as Density<u64>>::boundary(Cut::Above(u64::MAX)),
        RawCut::AboveAll
    );
}

#[test]
fn f5_counts_use_rank() {
    let all = FrozenIntervalSet::<char>::all();
    // `count` uses `rank` once per interval. Frost forbids timing assertions in
    // correctness tests, so this test checks the result only.
    assert_eq!(all.count(), 0x11_0000 - 0x800);
    let s: FrozenIntervalSet<char> = [char::closed('\u{D7FE}', '\u{E001}')].into_iter().collect();
    assert_eq!(s.count(), 4);
    assert_eq!(
        s.iter().collect::<String>(),
        "\u{D7FE}\u{D7FF}\u{E000}\u{E001}"
    );
    assert_eq!(FrozenIntervalSet::<u64>::all().count(), 1 << 64);
}

#[test]
#[should_panic(expected = "FiniteDomain law")]
fn f3_out_of_universe_index_panics() {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct BadIdx(u8);
    impl Domain for BadIdx {}
    impl FiniteDomain for BadIdx {
        const CARDINALITY: usize = 4;
        fn index(&self) -> usize {
            self.0 as usize
        }
        fn from_index(i: usize) -> Option<Self> {
            (i < 4).then_some(BadIdx(i as u8))
        }
    }
    let _ = FiniteSet::<BadIdx>::empty().with(&BadIdx(9));
}

// ----------------------------------- exhaustive interval algebra (Month)

fn month_cuts() -> Vec<Cut<Month>> {
    let mut v = vec![Cut::BelowAll, Cut::AboveAll];
    for i in 1..=12 {
        v.push(Cut::Below(m(i)));
        v.push(Cut::Above(m(i)));
    }
    v
}

fn mask_of(r: &Interval<Month>) -> u16 {
    (1..=12)
        .filter(|&v| r.contains(&m(v)))
        .fold(0, |b, v| b | 1 << (v - 1))
}

fn is_interval(mask: u16) -> bool {
    mask == 0 || {
        let t = mask >> mask.trailing_zeros();
        t & (t + 1) == 0
    }
}

#[test]
fn month_interval_algebra_exhaustive() {
    let cuts = month_cuts();
    let intervals: Vec<Interval<Month>> = cuts
        .iter()
        .flat_map(|a| cuts.iter().map(move |b| Interval::new(*a, *b)))
        .collect();
    let masks: Vec<u16> = intervals.iter().map(mask_of).collect();
    for (a, &ma) in intervals.iter().zip(&masks) {
        assert_eq!(a.is_empty(), ma == 0);
        assert_eq!(
            a.first().map(Month::get),
            (ma != 0).then(|| ma.trailing_zeros() as u8 + 1)
        );
        assert_eq!(
            a.last().map(Month::get),
            (ma != 0).then(|| 16 - ma.leading_zeros() as u8)
        );
        for (b, &mb) in intervals.iter().zip(&masks) {
            // One canonical form per set of members.
            assert_eq!(a == b, ma == mb, "{a:?} vs {b:?}");
            assert_eq!(mask_of(&a.intersection(b)), ma & mb);
            assert_eq!(a.is_superset(b), mb & !ma == 0, "encloses {a:?} {b:?}");
            assert_eq!(
                a.is_connected(b),
                is_interval(ma | mb),
                "connected {a:?} {b:?}"
            );
            assert_eq!(a.is_connected(b), a.gap(b).is_empty());
            let hull = mask_of(&a.hull(b));
            assert!(is_interval(hull) && hull & (ma | mb) == (ma | mb));
            assert_eq!(
                hull.count_ones() - (ma | mb).count_ones(),
                mask_of(&a.gap(b)).count_ones()
            );
            assert_eq!(mask_of(&a.gap(b)), hull & !(ma | mb));
        }
    }
}

#[test]
fn month_exhaustive_subsets() {
    for mask in 0u32..(1 << 12) {
        let members: Vec<Month> = (1..=12)
            .filter(|v| mask >> (v - 1) & 1 == 1)
            .map(m)
            .collect();
        let s: FrozenIntervalSet<Month> = members.iter().map(|&v| Month::singleton(v)).collect();
        let c = s.complement();
        for v in 1..=12 {
            let inside = mask >> (v - 1) & 1 == 1;
            assert_eq!(s.contains(&m(v)), inside);
            assert_eq!(c.contains(&m(v)), !inside);
        }
        assert_eq!(s.count(), u128::from(mask.count_ones()));
        assert_eq!(c.count(), 12 - u128::from(mask.count_ones()));
        assert_eq!(s.iter().collect::<Vec<_>>(), members);
        let runs: FrozenIntervalSet<Month> = s
            .closed_intervals()
            .into_iter()
            .map(|(a, b)| Month::closed(a, b))
            .collect();
        assert_eq!(runs, s);
        assert_eq!(s.union(&c), FrozenIntervalSet::all());
        assert!(s.intersection(&c).is_empty());
    }
}

// ----------------------------------------------- sparse codec (Tier)

#[test]
fn sparse_codec_never_invents_members() {
    let all: FrozenIntervalSet<Tier> = [Tier::closed(Tier::Bronze, Tier::Gold)]
        .into_iter()
        .collect();
    assert!(all.is_all());
    assert_eq!(all.count(), 3);
    assert_eq!(
        all.iter().collect::<Vec<_>>(),
        [Tier::Bronze, Tier::Silver, Tier::Gold]
    );
    let two: FrozenIntervalSet<Tier> =
        [Tier::singleton(Tier::Bronze), Tier::singleton(Tier::Silver)]
            .into_iter()
            .collect();
    assert_eq!(
        two,
        [Tier::closed(Tier::Bronze, Tier::Silver)]
            .into_iter()
            .collect()
    );
    assert_eq!(two.complement().iter().collect::<Vec<_>>(), [Tier::Gold]);
    let t = [Tier::Bronze, Tier::Silver, Tier::Gold];
    for mask in 0u8..8 {
        let s: FrozenIntervalSet<Tier> = (0..3)
            .filter(|i| mask >> i & 1 == 1)
            .map(|i| Tier::singleton(t[i]))
            .collect();
        assert_eq!(s.count(), u128::from(mask.count_ones()));
        assert_eq!(s.complement().count(), 3 - u128::from(mask.count_ones()));
    }
}

// --------------------------------------------------- cyclic (Weekday)

#[test]
fn weekday_wraps() {
    use Weekday::*;
    let [a, b] = Weekday::wrap_closed(Sat, Mon);
    let s: FrozenIntervalSet<Weekday> = [a, b].into_iter().collect();
    assert_eq!(s.iter().collect::<Vec<_>>(), [Mon, Sat, Sun]);
    assert_eq!(s.closed_intervals(), [(Mon, Mon), (Sat, Sun)]);
    assert_eq!(s.cyclic_intervals(), [(Sat, Mon)]); // F6: one wrapped run
    assert_eq!(s.complement().cyclic_intervals(), [(Tue, Fri)]);
    assert_eq!(
        FrozenIntervalSet::<Weekday>::all().cyclic_intervals(),
        [(Mon, Sun)]
    );
    assert_eq!(Sun.wrapping_succ(), Mon);
    assert_eq!(Mon.wrapping_pred(), Sun);
    let [x, y] = Weekday::wrap_closed(Tue, Thu);
    assert!(y.is_empty());
    assert_eq!(x, Weekday::closed(Tue, Thu));
    let [p, q] = Weekday::wrap_closed_open(Fri, Tue);
    let t: FrozenIntervalSet<Weekday> = [p, q].into_iter().collect();
    assert_eq!(t.cyclic_intervals(), [(Fri, Mon)]);
    let [e, _] = Weekday::wrap_closed_open(Wed, Wed);
    assert!(e.is_empty());
}

#[test]
fn weekday_cyclic_intervals_exhaustive() {
    for mask in 0u8..128 {
        let inside = |d: Weekday| mask >> (d as u8) & 1 == 1;
        let s: FrozenIntervalSet<Weekday> = Weekday::ALL
            .iter()
            .filter(|&&d| inside(d))
            .map(|&d| Weekday::singleton(d))
            .collect();
        let runs = s.cyclic_intervals();
        // Expanding each run forward around the cycle gives the members, once each.
        let mut seen = 0u8;
        for &(first, last) in &runs {
            let mut d = first;
            loop {
                assert!(
                    inside(d) && seen >> (d as u8) & 1 == 0,
                    "mask {mask:#b} runs {runs:?}"
                );
                seen |= 1 << (d as u8);
                if d == last {
                    break;
                }
                d = d.wrapping_succ();
            }
        }
        assert_eq!(seen, mask);
        // One run per place where the cycle enters the set.
        let entries = Weekday::ALL
            .iter()
            .filter(|&&d| inside(d) && !inside(d.wrapping_pred()))
            .count();
        assert_eq!(
            runs.len(),
            if mask == 127 { 1 } else { entries },
            "mask {mask:#b} runs {runs:?}"
        );
    }
}

// ---------------------------------------------------- prefix (CIDR)

#[test]
fn cidr_prefixes_as_intervals() {
    let p = |a: [u8; 4], l| Ipv4Prefix::new(Ipv4Addr::from(a), l).unwrap();
    assert!(Ipv4Prefix::new(Ipv4Addr::new(10, 0, 0, 1), 8).is_none());
    assert!(Ipv4Prefix::new(Ipv4Addr::new(10, 0, 0, 0), 33).is_none());
    let s: FrozenIntervalSet<Ipv4Addr> = [
        p([10, 0, 0, 0], 8),
        p([10, 1, 0, 0], 16),
        p([192, 168, 0, 0], 16),
    ]
    .iter()
    .map(PrefixDomain::to_interval)
    .collect();
    assert_eq!(s.interval_count(), 2);
    assert!(s.contains(&Ipv4Addr::new(10, 200, 3, 4)));
    assert!(!s.contains(&Ipv4Addr::new(11, 0, 0, 0)));
    assert_eq!(s.count(), (1 << 24) + (1 << 16));
    let all: FrozenIntervalSet<Ipv4Addr> = [p([0, 0, 0, 0], 0).to_interval()].into_iter().collect();
    assert!(all.is_all());
    assert_eq!(all.count(), 1 << 32);
}

// ----------------------------------------- partial order (DownSet)

#[test]
fn downset_exhaustive_over_three_bits() {
    let universe: Vec<Perms> = (0..8).map(Perms).collect();
    let build =
        |gens: u8| -> DownSet<Perms> { (0..8).filter(|i| gens >> i & 1 == 1).map(Perms).collect() };
    let model = |gens: u8, x: Perms| (0..8u8).any(|g| gens >> g & 1 == 1 && x.0 & !g == 0);
    for ga in 0..=255u8 {
        let a = build(ga);
        // The generators are an antichain.
        for g in a.generators() {
            for h in a.generators() {
                assert!(g == h || !g.leq(h));
            }
        }
        for gb in (0..=255u8).step_by(7) {
            let b = build(gb);
            let (u, i) = (a.union(&b), a.intersection(&b));
            let mut same = true;
            for &x in &universe {
                let (pa, pb) = (model(ga, x), model(gb, x));
                assert_eq!(a.contains(&x), pa);
                assert_eq!(u.contains(&x), pa | pb);
                assert_eq!(i.contains(&x), pa & pb);
                same &= pa == pb;
            }
            assert_eq!(a == b, same);
        }
    }
}

// ------------------------------------------------------------- laws

#[test]
fn laws_hold_for_every_shipped_domain() {
    laws::check_cover_preserving_exhaustive::<u8>(1 << 9);
    laws::check_cover_preserving_exhaustive::<i8>(1 << 9);
    laws::check_cover_preserving_exhaustive::<u16>(1 << 17);
    laws::check_cover_preserving_exhaustive::<i16>(1 << 17);
    laws::check_cover_preserving_exhaustive::<Month>(64);
    laws::check_cover_preserving_exhaustive::<BoundedU8<0, 255>>(1 << 9);
    laws::check_cover_preserving_exhaustive::<BoundedU8<7, 7>>(4);
    laws::check_exhaustive::<Tier>(8);
    laws::check_ranked(Tier::Bronze, 8);
    laws::check_cover_preserving_exhaustive::<Weekday>(8);
    assert_eq!(laws::check_exhaustive::<char>(0x11_0000), 0x11_0000 - 0x800);
    laws::check_ranked('\0', 0x11_0000);

    let edge64 = [0u64, 1, 2, 7, u64::MAX - 1, u64::MAX];
    laws::check_ordered(&edge64);
    laws::check_discrete(&edge64);
    laws::check_cover_preserving_sampled(&edge64, 8);
    let ids: Vec<UserId> = edge64.iter().map(|&v| UserId(v)).collect();
    laws::check_ordered(&ids);
    laws::check_discrete(&ids);
    laws::check_cover_preserving_sampled(&ids, 8);
    let s64 = [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX];
    laws::check_ordered(&s64);
    laws::check_discrete(&s64);
    laws::check_cover_preserving_sampled(&s64, 8);
    let s32 = [i32::MIN, -1, 0, 1, i32::MAX];
    laws::check_ordered(&s32);
    laws::check_discrete(&s32);
    laws::check_cover_preserving_sampled(&s32, 8);
    let ips: Vec<Ipv4Addr> = [0u32, 1, 0x0A00_0000, u32::MAX - 1, u32::MAX]
        .map(Ipv4Addr::from)
        .to_vec();
    laws::check_ordered(&ips);
    laws::check_discrete(&ips);
    laws::check_cover_preserving_sampled(&ips, 8);
    let wide = [i128::MIN, i128::MIN + 1, -1, 0, 1, i128::MAX - 1, i128::MAX];
    laws::check_ordered(&wide);
    laws::check_discrete(&wide);

    let months: Vec<Month> = (1..=12).map(m).collect();
    laws::check_ordered(&months);
    laws::check_ordered(&[Tier::Bronze, Tier::Silver, Tier::Gold]);
    laws::check_ordered(&Weekday::ALL);
    laws::check_ordered(&['\0', 'a', '\u{D7FF}', '\u{E000}', char::MAX]);
    let strings: Vec<String> = ["", "\0", "a", "a\0", "ab", "b"].map(String::from).to_vec();
    laws::check_ordered(&strings);
    let units: Vec<Unit> = (0..=8).map(|k| unit(f64::from(k) / 8.0)).collect();
    laws::check_ordered(&units);
    assert_eq!(unit(-0.0), unit(0.0));
    assert!(Unit::new(f64::NAN).is_none() && Unit::new(1.5).is_none());

    laws::check_finite::<Color>();
    let perms: Vec<Perms> = (0..=255).step_by(5).map(Perms).collect();
    laws::check_lattice(&perms);
}

#[test]
fn set_algebra_laws_across_families() {
    let months: Vec<Month> = (1..=12).map(m).collect();
    let msets: Vec<FrozenIntervalSet<Month>> = [0u16, 1, 0x800, 0xFFF, 0x555, 0x0F0, 0xE07]
        .iter()
        .map(|mask| {
            months
                .iter()
                .filter(|v| mask >> (v.get() - 1) & 1 == 1)
                .map(|&v| Month::singleton(v))
                .collect()
        })
        .collect();
    laws::check_boolean_region(&msets, &months);

    let colors = [Color::Red, Color::Green, Color::Blue, Color::Yellow];
    let csets: Vec<FiniteSet<Color>> = (0u8..16)
        .map(|mask| {
            colors
                .iter()
                .filter(|c| mask >> c.index() & 1 == 1)
                .copied()
                .collect()
        })
        .collect();
    laws::check_boolean_region(&csets, &colors);
    let warm: FiniteSet<Color> = [Color::Red, Color::Yellow].into_iter().collect();
    assert_eq!(warm.count(), 2);
    assert_eq!(FiniteSet::<Color>::empty().count(), 0);
    assert_eq!(
        warm.complement().iter().collect::<Vec<_>>(),
        [Color::Green, Color::Blue]
    );

    let u = |k: u32| unit(f64::from(k) / 16.0);
    let usets: Vec<FrozenIntervalSet<Unit>> = vec![
        FrozenIntervalSet::empty(),
        FrozenIntervalSet::all(),
        [Unit::closed(u(0), u(4))].into_iter().collect(),
        [Unit::open(u(4), u(8)), Unit::singleton(u(16))]
            .into_iter()
            .collect(),
        [
            Unit::closed_open(u(2), u(6)),
            Unit::open_closed(u(8), u(16)),
        ]
        .into_iter()
        .collect(),
        [Unit::singleton(u(4)), Unit::greater_than(u(12))]
            .into_iter()
            .collect(),
    ];
    let probes: Vec<Unit> = (0..=16).map(u).collect();
    laws::check_boolean_region(&usets, &probes);
    // Bounded dense edges: [0, 1] is the whole domain.
    assert_eq!(Unit::closed(u(0), u(16)), Unit::all());
    assert_eq!(usets[2].complement().intervals()[0].lo(), &Cut::Above(u(4)));
}

// ---------------------------------------- unlawful impls are detected

mod unlawful {
    use super::*;

    /// A domain over `u8` values `0..=9` with a chosen encoding and step.
    macro_rules! digit_domain {
        ($name:ident, encode = |$e:ident| $encode:expr, decode = |$w:ident| $pre:expr, step = $step:expr) => {
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
            pub struct $name(pub u8);
            impl Domain for $name {}
            impl OrderedDomain for $name {
                type Density = Discrete;
                type Word = u32;
                const MIN: Option<Self> = Some($name(0));
                const MAX: Option<Self> = Some($name(9));
                fn encode(&self) -> u32 {
                    let $e = u32::from(self.0);
                    $encode
                }
                fn decode(word: &u32) -> Option<Self> {
                    let $w = *word;
                    $pre.filter(|v: &u32| *v <= 9).map(|v| $name(v as u8))
                }
            }
            impl DiscreteDomain for $name {
                #[allow(clippy::int_plus_one)]
                fn succ(&self) -> Option<Self> {
                    (self.0 + $step <= 9).then(|| $name(self.0 + $step))
                }
                fn pred(&self) -> Option<Self> {
                    self.0.checked_sub($step).map($name)
                }
            }
        };
    }

    digit_domain!(SkipSucc, encode = |v| v, decode = |w| Some(w), step = 2);
    digit_domain!(
        Reversed,
        encode = |v| 9 - v,
        decode = |w| 9u32.checked_sub(w),
        step = 1
    );
    digit_domain!(
        Lossy,
        encode = |v| v / 2,
        decode = |w| Some(w * 2),
        step = 1
    );
    digit_domain!(
        Doubled,
        encode = |v| v * 2,
        decode = |w| w.is_multiple_of(2).then_some(w / 2),
        step = 1
    );
    impl CoverPreserving for Doubled {} // wrong: the words have gaps
    digit_domain!(BadRank, encode = |v| v, decode = |w| Some(w), step = 1);
    impl Ranked for BadRank {
        fn rank(&self) -> u128 {
            u128::from(self.0) * 2
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct WrongMin(pub u8); // MIN is not the least value
    impl Domain for WrongMin {}
    impl OrderedDomain for WrongMin {
        type Density = Dense;
        const MIN: Option<Self> = Some(WrongMin(5));
        identity_encoding!();
    }

    #[test]
    #[should_panic(expected = "never reaches")]
    fn skipping_succ_is_detected() {
        laws::check_exhaustive::<SkipSucc>(32);
    }
    #[test]
    #[should_panic(expected = "encoding order")]
    fn order_reversing_encoding_is_detected() {
        laws::check_exhaustive::<Reversed>(32);
    }
    #[test]
    #[should_panic(expected = "encoding")]
    fn lossy_encoding_is_detected() {
        laws::check_ordered(&[Lossy(2), Lossy(3), Lossy(4)]);
    }
    #[test]
    #[should_panic(expected = "not cover-preserving")]
    fn wrong_cover_preserving_marker_is_detected() {
        laws::check_cover_preserving_exhaustive::<Doubled>(32);
    }
    #[test]
    #[should_panic(expected = "rank step")]
    fn wrong_rank_is_detected() {
        laws::check_ranked(BadRank(0), 9);
    }
    #[test]
    #[should_panic(expected = "MIN law")]
    fn wrong_min_is_detected() {
        laws::check_ordered(&[WrongMin(1), WrongMin(7)]);
    }
}

// ----------------------------------------------- property tests

fn val() -> impl Strategy<Value = u64> {
    prop_oneof![0u64..16, (u64::MAX - 3)..=u64::MAX]
}

fn rng() -> impl Strategy<Value = (u8, u64, u64)> {
    (0u8..9, val(), val())
}

fn build<V: OrderedDomain>(spec: &[(u8, u64, u64)], f: impl Fn(u64) -> V) -> Vec<Interval<V>> {
    spec.iter()
        .map(|&(k, a, b)| match k {
            0 => V::closed(f(a), f(b)),
            1 => V::open(f(a), f(b)),
            2 => V::closed_open(f(a), f(b)),
            3 => V::open_closed(f(a), f(b)),
            4 => V::at_least(f(a)),
            5 => V::greater_than(f(a)),
            6 => V::at_most(f(b)),
            7 => V::less_than(f(b)),
            _ => V::all(),
        })
        .collect()
}

/// u64 test value -> i128 spread over the whole i128 interval, order-preserving.
fn wide(x: u64) -> i128 {
    if x < 16 {
        i128::MIN + i128::from(x)
    } else {
        i128::MAX - i128::from(u64::MAX - x)
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3000))]

    #[test]
    fn u64_userid_i128_agree(a in prop::collection::vec(rng(), 0..8),
                             b in prop::collection::vec(rng(), 0..8)) {
        let (ra, rb) = (build(&a, |x| x), build(&b, |x| x));
        let sa: FrozenIntervalSet<u64> = ra.iter().cloned().collect();
        let sb: FrozenIntervalSet<u64> = rb.iter().cloned().collect();
        let xa: FrozenIntervalSet<UserId> = build(&a, UserId).into_iter().collect();
        let xb: FrozenIntervalSet<UserId> = build(&b, UserId).into_iter().collect();
        let wa: FrozenIntervalSet<i128> = build(&a, wide).into_iter().collect();
        let wb: FrozenIntervalSet<i128> = build(&b, wide).into_iter().collect();
        let ops64 = [&sa | &sb, &sa & &sb, &sa - &sb, &sa ^ &sb, !&sa];
        let ops_id = [&xa | &xb, &xa & &xb, &xa - &xb, &xa ^ &xb, !&xa];
        let ops_w = [&wa | &wb, &wa & &wb, &wa - &wb, &wa ^ &wb, !&wa];
        let model: [fn(bool, bool) -> bool; 5] =
            [|p, q| p | q, |p, q| p & q, |p, q| p & !q, |p, q| p ^ q, |p, _| !p];
        for k in 0..5 {
            prop_assert_eq!(ops64[k].words(), ops_id[k].words());
            for x in (0..20).chain((u64::MAX - 5)..=u64::MAX) {
                let (pa, pb) = (ra.iter().any(|r| r.contains(&x)), rb.iter().any(|r| r.contains(&x)));
                prop_assert_eq!(ops64[k].contains(&x), model[k](pa, pb));
                prop_assert_eq!(ops_w[k].contains(&wide(x)), model[k](pa, pb));
            }
        }
        let rebuilt: FrozenIntervalSet<u64> = sa.intervals().into_iter().collect();
        prop_assert_eq!(&rebuilt, &sa);
        // i128 uses ValueStore: every run starts and ends on a member, and the
        // values next to a run are outside it.
        for (f, l) in wa.closed_intervals() {
            prop_assert!(f <= l && wa.contains(&f) && wa.contains(&l));
            prop_assert!(f.pred().is_none_or(|p| !wa.contains(&p)));
            prop_assert!(l.succ().is_none_or(|n| !wa.contains(&n)));
        }
        let rebuilt_w: FrozenIntervalSet<i128> = wa.intervals().into_iter().collect();
        prop_assert_eq!(&rebuilt_w, &wa);
    }

    #[test]
    fn signed_codecs_preserve_order(a in any::<i64>(), b in any::<i64>(), c in any::<i8>(), d in any::<i8>()) {
        prop_assert_eq!(a.cmp(&b), a.encode().cmp(&b.encode()));
        prop_assert_eq!(i64::decode(&a.encode()), Some(a));
        prop_assert_eq!(c.cmp(&d), c.encode().cmp(&d.encode()));
    }
}

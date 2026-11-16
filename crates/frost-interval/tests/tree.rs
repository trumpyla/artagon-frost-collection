//! The boundary tree and `IntervalSet` against models.
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

use frost_interval::tree::BoundaryTree;
use proptest::prelude::*;
use std::collections::BTreeSet;

/// Small capacities, so a few hundred boundaries give a tree of height 4 or more.
type Small = BoundaryTree<u16, 4, 4>;

fn check_against(t: &Small, model: &BTreeSet<u16>) {
    t.check();
    assert_eq!(t.len(), model.len());
    assert_eq!(t.to_vec(), model.iter().copied().collect::<Vec<_>>());
}

#[test]
fn toggle_grows_and_shrinks_through_every_height() {
    let mut t = Small::new();
    let mut model = BTreeSet::new();
    // Ascending, descending, and interleaved orders hit splits on both ends.
    let order: Vec<u16> = (0..600)
        .chain((600..1200).rev())
        .chain((1200..1800).step_by(2))
        .chain((1201..1800).step_by(2))
        .collect();
    for &k in &order {
        assert!(t.toggle(k));
        model.insert(k);
    }
    check_against(&t, &model);
    assert!(t.height() >= 5, "height {}", t.height());
    // Remove in an order that empties the middle first.
    let mut rm: Vec<u16> = order.clone();
    rm.sort_by_key(|&k| (i32::from(k) - 900).abs());
    for (n, &k) in rm.iter().enumerate() {
        assert!(!t.toggle(k));
        model.remove(&k);
        if n % 97 == 0 {
            check_against(&t, &model);
        }
    }
    check_against(&t, &model);
    assert_eq!(t.height(), 1);
}

#[test]
fn rank_le_matches_model() {
    let keys: Vec<u16> = (0..500).map(|k| k * 3).collect();
    let t = Small::from_sorted(keys.clone());
    t.check();
    for probe in 0..1600u16 {
        assert_eq!(
            t.rank_le(&probe),
            keys.partition_point(|k| *k <= probe),
            "probe {probe}"
        );
    }
}

#[test]
fn from_sorted_fills_every_node_to_half() {
    for n in 0..300u16 {
        let t = Small::from_sorted(0..n);
        t.check();
        assert_eq!(t.len(), usize::from(n));
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    /// Any toggle sequence: the tree equals a sorted-set model, and every
    /// invariant holds after every step.
    #[test]
    fn toggle_sequence_matches_model(start in proptest::collection::btree_set(0u16..400, 0..120),
                                     ops in proptest::collection::vec(0u16..400, 0..300)) {
        let mut t = Small::from_sorted(start.iter().copied());
        let mut model = start;
        check_against(&t, &model);
        for k in ops {
            let added = t.toggle(k);
            prop_assert_eq!(added, model.insert(k));
            if !added {
                model.remove(&k);
            }
            t.check();
        }
        check_against(&t, &model);
        for probe in 0..400u16 {
            prop_assert_eq!(t.rank_le(&probe), model.range(..=probe).count());
        }
    }
}

// ------------------------------------------------------------ IntervalSet

use frost_interval::{FrozenIntervalSet, IntervalSet, OrderedDomainExt};
use support::demo::Month;

fn month(v: u8) -> Month {
    Month::new(v).unwrap()
}

fn frozen_months(mask: u16) -> FrozenIntervalSet<Month> {
    FrozenIntervalSet::from_intervals(
        (1..=12u8)
            .filter(|k| mask >> (k - 1) & 1 == 1)
            .map(|k| Month::singleton(month(k))),
    )
}

/// All 4,096 subsets of `Month` against every single insert and remove.
#[test]
fn month_insert_remove_exhaustive() {
    for mask in 0u16..4096 {
        let frozen = frozen_months(mask);
        for k in 1..=12u8 {
            let bit = 1u16 << (k - 1);
            let mut s = IntervalSet::from(&frozen);
            assert_eq!(s.insert(month(k)), mask & bit == 0);
            assert_eq!(
                s.freeze(),
                frozen_months(mask | bit),
                "insert {k} into {mask:#014b}"
            );
            let mut s = IntervalSet::from(&frozen);
            assert_eq!(s.remove(&month(k)), mask & bit != 0);
            assert_eq!(
                s.freeze(),
                frozen_months(mask & !bit),
                "remove {k} from {mask:#014b}"
            );
            // The complement is O(1) and stays correct under updates.
            let mut c = IntervalSet::from(&frozen).complement();
            c.insert(month(k));
            assert_eq!(c.freeze(), frozen_months((!mask | bit) & 0xfff));
        }
        for k in 1..=12u8 {
            assert_eq!(
                IntervalSet::from(&frozen).contains(&month(k)),
                mask >> (k - 1) & 1 == 1
            );
        }
    }
}

/// Every subset, built from empty by inserts in a scrambled order and
/// from `all()` by removes.
#[test]
fn month_build_by_updates_exhaustive() {
    let order = [7u8, 1, 12, 4, 9, 2, 11, 6, 3, 10, 5, 8];
    for mask in 0u16..4096 {
        let mut up = IntervalSet::<Month>::empty();
        let mut down = IntervalSet::<Month>::all();
        for &k in &order {
            if mask >> (k - 1) & 1 == 1 {
                up.insert(month(k));
            } else {
                down.remove(&month(k));
            }
        }
        let want = frozen_months(mask);
        assert_eq!(up.freeze(), want);
        assert_eq!(down.freeze(), want);
        assert_eq!(up.interval_count(), want.interval_count());
    }
}

#[test]
fn edges_of_bounded_and_gapped_domains() {
    let mut s = IntervalSet::<u8>::empty();
    s.insert(0);
    s.insert(255);
    assert_eq!(
        s.boundary_count(),
        2,
        "MIN is the start flag and MAX has no upper boundary"
    );
    assert_eq!(
        s.freeze(),
        FrozenIntervalSet::from_intervals([0..=0, 255..=255])
    );
    s.extend(1..=254);
    assert!(s.is_all());
    s.remove(&0);
    s.remove(&255);
    assert_eq!(s.freeze(), FrozenIntervalSet::of(1..=254));

    let mut w = IntervalSet::<i64>::empty();
    w.extend([i64::MIN, i64::MAX, -1, 0]);
    assert_eq!(
        w.freeze(),
        FrozenIntervalSet::from_intervals([i64::MIN..=i64::MIN, -1..=0, i64::MAX..=i64::MAX])
    );

    // The two values around the surrogate gap are neighbors in `char`.
    let mut c = IntervalSet::<char>::empty();
    c.insert('\u{D7FF}');
    c.insert('\u{E000}');
    assert_eq!(c.interval_count(), 1);
    assert_eq!(c.freeze(), FrozenIntervalSet::of('\u{D7FF}'..='\u{E000}'));
}

#[test]
fn dense_domain_updates() {
    let mut s = IntervalSet::<String>::empty();
    for w in ["pear", "apple", "fig"] {
        assert!(s.insert(w.to_string()));
    }
    assert!(!s.insert("fig".into()));
    assert!(
        s.contains(&"apple".into()) && !s.contains(&"apple\0".into()) && !s.contains(&"b".into())
    );
    assert_eq!(
        s.boundary_count(),
        6,
        "a value in a dense domain has a Below and an Above boundary"
    );
    let want = FrozenIntervalSet::from_intervals(
        ["apple", "fig", "pear"].map(|w| String::singleton(w.to_string())),
    );
    assert_eq!(s.freeze(), want);
    assert_eq!(IntervalSet::from(&want), s);

    // A dense domain with both extrema: `{MAX}` has one boundary, `{MIN}` none.
    let unit = |x: f64| support::demo::Unit::new(x).unwrap();
    let mut u = IntervalSet::empty();
    u.insert(unit(1.0));
    assert_eq!(u.boundary_count(), 1);
    assert_eq!(
        u.freeze(),
        FrozenIntervalSet::of(support::demo::Unit::singleton(unit(1.0)))
    );
    u.insert(unit(0.0));
    assert_eq!(u.boundary_count(), 2);
    assert!(u.contains(&unit(0.0)) && !u.contains(&unit(0.5)) && u.contains(&unit(1.0)));

    let mut all = IntervalSet::<String>::all();
    all.remove(&String::new());
    all.remove(&"m".to_string());
    assert!(
        !all.contains(&String::new()) && all.contains(&"\0".into()) && !all.contains(&"m".into())
    );
    assert_eq!(
        all.freeze(),
        FrozenIntervalSet::from_intervals([
            String::singleton(String::new()),
            String::singleton("m".into())
        ])
        .complement()
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    /// Random insert and remove sequences on `i64` against a sorted-set model.
    /// About a thousand boundaries: the default tree has height 2 or 3.
    #[test]
    fn i64_updates_match_model(ops in proptest::collection::vec((any::<bool>(), -1500i64..1500), 0..3000)) {
        let mut s = IntervalSet::<i64>::empty();
        let mut model = BTreeSet::new();
        for (add, v) in ops {
            if add {
                prop_assert_eq!(s.insert(v), model.insert(v));
            } else {
                prop_assert_eq!(s.remove(&v), model.remove(&v));
            }
        }
        let want = FrozenIntervalSet::from_intervals(model.iter().map(|&v| v..=v));
        prop_assert_eq!(s.freeze(), want.clone());
        prop_assert_eq!(s.boundary_count(), want.boundary_count());
        for v in -1510i64..1510 {
            prop_assert_eq!(s.contains(&v), model.contains(&v));
        }
        let thawed = IntervalSet::from(&want);
        prop_assert!(thawed == s);
    }
}

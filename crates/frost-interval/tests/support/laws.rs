//! Law checks for domain impls. The domain traits are safe traits with laws
//! the compiler cannot check. Call these from a test for every impl.
//! Each function panics with the violated law and the witness values.

use core::fmt::Debug;
use frost_interval::cut::Cut;
use frost_interval::domain::{
    CoverPreserving, DiscreteDomain, Extrema, FiniteDomain, Lattice, MachineWord, OrderedDomain,
    OrderedDomainExt, Poset, Ranked,
};
use frost_interval::interval::Interval;
use frost_interval::region::{BooleanRegion, Convex, Region};

/// Order, extrema, the encoding, canonical cuts, and interval membership
/// over `samples`.
pub fn check_ordered<V: OrderedDomain + Debug>(samples: &[V]) {
    for v in samples {
        let word = v.encode();
        assert_eq!(
            V::decode(&word).as_ref(),
            Some(v),
            "encoding round trip of {v:?}"
        );
        assert!(
            v.clone().into_word() == word,
            "into_word differs from encode for {v:?}"
        );
        assert!(
            v.with_word(|i| *i == word),
            "with_word differs from encode for {v:?}"
        );
        for w in samples {
            assert_eq!(
                v.cmp(w),
                word.cmp(&w.encode()),
                "encoding order of {v:?} and {w:?}"
            );
        }
        if let Some(m) = V::MIN {
            assert!(m <= *v, "MIN law: {v:?} is below MIN {m:?}");
        }
        if let Some(m) = V::MAX {
            assert!(*v <= m, "MAX law: {v:?} is above MAX {m:?}");
        }
        for c in [Cut::Below(v.clone()), Cut::Above(v.clone())] {
            let k = c.clone().canonical();
            assert_eq!(
                k.clone().canonical(),
                k,
                "canonical is not idempotent for {c:?}"
            );
            // A canonical cut keeps the position: the same samples lie below it.
            for x in samples {
                assert_eq!(
                    c.le_below(x),
                    k.le_below(x),
                    "canonical moved {c:?} across {x:?}"
                );
            }
        }
    }
    assert_eq!(
        V::min_cut().canonical(),
        Cut::BelowAll,
        "min_cut must canonicalize to BelowAll"
    );
    assert_eq!(
        V::max_cut().canonical(),
        Cut::AboveAll,
        "max_cut must canonicalize to AboveAll"
    );
    for a in samples {
        for b in samples {
            let intervals = [
                (Interval::closed(a.clone(), b.clone()), true, true),
                (Interval::open(a.clone(), b.clone()), false, false),
                (Interval::closed_open(a.clone(), b.clone()), true, false),
                (Interval::open_closed(a.clone(), b.clone()), false, true),
            ];
            for (r, lo_in, hi_in) in &intervals {
                for x in samples {
                    let lo_ok = if *lo_in { a <= x } else { a < x };
                    let hi_ok = if *hi_in { x <= b } else { x < b };
                    assert_eq!(
                        r.contains(x),
                        lo_ok && hi_ok,
                        "interval {r:?} and value {x:?}"
                    );
                }
            }
        }
    }
}

/// `succ`/`pred` laws over `samples`.
pub fn check_discrete<V: DiscreteDomain + Debug>(samples: &[V]) {
    for v in samples {
        match v.succ() {
            Some(s) => {
                assert!(s > *v, "succ law: succ({v:?}) = {s:?} is not greater");
                assert_eq!(s.pred().as_ref(), Some(v), "pred(succ({v:?})) != {v:?}");
                assert_ne!(V::MAX.as_ref(), Some(v), "succ(MAX) must be None");
                for x in samples {
                    assert!(
                        !(v < x && *x < s),
                        "{x:?} lies strictly between {v:?} and its succ {s:?}"
                    );
                }
            }
            None => assert_eq!(
                V::MAX.as_ref(),
                Some(v),
                "succ({v:?}) is None but {v:?} is not MAX"
            ),
        }
        match v.pred() {
            Some(p) => {
                assert!(p < *v, "pred law: pred({v:?}) = {p:?} is not less");
                assert_eq!(p.succ().as_ref(), Some(v), "succ(pred({v:?})) != {v:?}");
                assert_ne!(V::MIN.as_ref(), Some(v), "pred(MIN) must be None");
            }
            None => assert_eq!(
                V::MIN.as_ref(),
                Some(v),
                "pred({v:?}) is None but {v:?} is not MIN"
            ),
        }
    }
}

/// Walks up to `steps` values from `start` with `succ` and checks the laws of
/// a discrete domain with machine words, in linear time and constant memory:
/// steps, encoding round trip and order, and words between two neighbors
/// (they must encode no value). Returns the last value and the number visited.
pub fn check_walk<V>(start: V, steps: usize) -> (V, usize)
where
    V: DiscreteDomain + Copy + Debug,
    V::Word: MachineWord,
{
    let mut prev = start;
    assert_eq!(
        V::decode(&prev.encode()),
        Some(prev),
        "encoding round trip of {prev:?}"
    );
    let mut count = 1;
    while count <= steps {
        let Some(v) = prev.succ() else {
            assert_eq!(
                Some(prev),
                V::MAX,
                "succ({prev:?}) is None but {prev:?} is not MAX"
            );
            break;
        };
        assert!(prev < v, "succ law: succ({prev:?}) = {v:?} is not greater");
        assert_eq!(v.pred(), Some(prev), "pred(succ({prev:?})) != {prev:?}");
        assert_eq!(
            V::decode(&v.encode()),
            Some(v),
            "encoding round trip of {v:?}"
        );
        let (a, b) = (prev.encode().to_u128(), v.encode().to_u128());
        assert!(a < b, "encoding order of {prev:?} and {v:?}");
        // Words between two neighbors must encode no value (capped for huge gaps).
        for n in (a + 1..b).take(1 << 16) {
            let word = V::Word::try_from(n)
                .ok()
                .expect("a word in the span fits the word type");
            if let Some(x) = V::decode(&word) {
                panic!(
                    "word {n} decodes to {x:?} but succ never reaches it (between {prev:?} and {v:?})"
                );
            }
        }
        prev = v;
        count += 1;
    }
    (prev, count)
}

/// Walks the whole domain from `MIN` (at most `limit` values) with
/// [`check_walk`], and checks both ends. Returns the number of values.
pub fn check_exhaustive<V>(limit: usize) -> usize
where
    V: DiscreteDomain + Copy + Debug,
    V::Word: MachineWord,
{
    let (min, max) = (Extrema::<V>::MIN, Extrema::<V>::MAX);
    assert_eq!(min.pred(), None, "pred(MIN) must be None");
    assert_eq!(max.succ(), None, "succ(MAX) must be None");
    let (last, count) = check_walk(min, limit);
    assert!(count <= limit, "domain has more than {limit} values");
    assert_eq!(last, max, "the succ walk from MIN must end at MAX");
    count
}

/// `check_ordered` and `check_discrete` on the samples, plus a walk of
/// `2 * steps` values around `MIN`, `MAX`, and each sample.
pub fn check_sampled<V>(samples: &[V], steps: usize)
where
    V: DiscreteDomain + Copy + Debug,
    V::Word: MachineWord,
{
    check_ordered(samples);
    check_discrete(samples);
    for start in walk_starts(samples, steps) {
        check_walk(start, 2 * steps);
    }
}

fn walk_starts<V: DiscreteDomain + Copy>(samples: &[V], steps: usize) -> Vec<V> {
    samples
        .iter()
        .chain([&Extrema::<V>::MIN, &Extrema::<V>::MAX])
        .map(|&s| {
            (0..steps)
                .try_fold(s, |v, _| v.pred().ok_or(v))
                .unwrap_or_else(|v| v)
        })
        .collect()
}

/// The marker law of [`CoverPreserving`]: `encode(succ v) == encode(v) + 1`,
/// for `steps` values from `start`.
pub fn check_cover_preserving<V>(start: V, steps: usize)
where
    V: CoverPreserving + Debug,
    V::Word: MachineWord,
{
    let mut prev = start;
    for _ in 0..steps {
        let Some(v) = prev.succ() else { break };
        assert_eq!(
            v.encode().to_u128(),
            prev.encode().to_u128() + 1,
            "not cover-preserving: encode jumps between {prev:?} and {v:?}"
        );
        prev = v;
    }
}

/// The laws of [`Ranked`]: `rank(MIN) == 0` and `rank(succ v) == rank(v) + 1`,
/// for `steps` values from `start`.
pub fn check_ranked<V: Ranked + Debug>(start: V, steps: usize) {
    assert_eq!(Extrema::<V>::MIN.rank(), 0, "rank(MIN) must be 0");
    let mut prev = start;
    for _ in 0..steps {
        let Some(v) = prev.succ() else { break };
        assert_eq!(
            v.rank(),
            prev.rank() + 1,
            "rank step from {prev:?} to {v:?}"
        );
        prev = v;
    }
}

/// Every check for a cover-preserving domain with machine words, around the
/// samples and both ends.
pub fn check_cover_preserving_sampled<V>(samples: &[V], steps: usize)
where
    V: CoverPreserving + Debug,
    V::Word: MachineWord,
{
    check_sampled(samples, steps);
    for start in walk_starts(samples, steps) {
        check_cover_preserving(start, 2 * steps);
        check_ranked(start, 2 * steps);
    }
}

/// Every check for a cover-preserving domain with machine words, over the
/// whole domain. Returns the number of values.
pub fn check_cover_preserving_exhaustive<V>(limit: usize) -> usize
where
    V: CoverPreserving + Debug,
    V::Word: MachineWord,
{
    let count = check_exhaustive::<V>(limit);
    check_cover_preserving(Extrema::<V>::MIN, limit);
    check_ranked(Extrema::<V>::MIN, limit);
    let span = Extrema::<V>::MAX.encode().to_u128() - Extrema::<V>::MIN.encode().to_u128() + 1;
    assert_eq!(
        span, count as u128,
        "the words of the domain are not an interval"
    );
    count
}

/// `index` is a bijection onto `0..CARDINALITY`.
pub fn check_finite<V: FiniteDomain + Debug>() {
    for i in 0..V::CARDINALITY {
        let v =
            V::from_index(i).unwrap_or_else(|| panic!("from_index({i}) is None below CARDINALITY"));
        assert_eq!(v.index(), i, "index(from_index({i}))");
    }
    assert!(
        V::from_index(V::CARDINALITY).is_none(),
        "from_index(CARDINALITY) must be None"
    );
}

/// Reflexive, antisymmetric, transitive.
pub fn check_partial_order<V: Poset + Debug>(samples: &[V]) {
    for a in samples {
        assert!(a.leq(a), "reflexive: {a:?}");
        for b in samples {
            if a.leq(b) && b.leq(a) {
                assert_eq!(a, b, "antisymmetric");
            }
            for c in samples {
                if a.leq(b) && b.leq(c) {
                    assert!(a.leq(c), "transitive: {a:?} {b:?} {c:?}");
                }
            }
        }
    }
}

/// `join` is the least upper bound and `meet` the greatest lower bound.
pub fn check_lattice<V: Lattice + Debug>(samples: &[V]) {
    check_partial_order(samples);
    for a in samples {
        for b in samples {
            let (j, m) = (a.join(b), a.meet(b));
            assert!(
                a.leq(&j) && b.leq(&j),
                "join({a:?}, {b:?}) is not an upper bound"
            );
            assert!(
                m.leq(a) && m.leq(b),
                "meet({a:?}, {b:?}) is not a lower bound"
            );
            for c in samples {
                if a.leq(c) && b.leq(c) {
                    assert!(j.leq(c), "join({a:?}, {b:?}) is not the least upper bound");
                }
                if c.leq(a) && c.leq(b) {
                    assert!(
                        c.leq(&m),
                        "meet({a:?}, {b:?}) is not the greatest lower bound"
                    );
                }
            }
        }
    }
}

/// Boolean-algebra laws of a region family against point membership.
pub fn check_boolean_region<S: BooleanRegion + Debug>(sets: &[S], probes: &[S::Value])
where
    S::Value: Debug,
{
    assert!(S::empty().is_empty());
    for a in sets {
        let na = a.complement();
        assert_eq!(&na.complement(), a, "double complement");
        assert_eq!(a.union(&na), S::all(), "a | !a == all");
        assert!(a.intersection(&na).is_empty(), "a & !a is empty");
        for b in sets {
            let (u, i, d) = (a.union(b), a.intersection(b), a.difference(b));
            assert_eq!(
                u.complement(),
                na.intersection(&b.complement()),
                "De Morgan"
            );
            assert_eq!(u, b.union(a), "union commutes");
            for x in probes {
                let (pa, pb) = (a.contains(x), b.contains(x));
                assert_eq!(u.contains(x), pa | pb, "union at {x:?}");
                assert_eq!(i.contains(x), pa & pb, "intersection at {x:?}");
                assert_eq!(d.contains(x), pa & !pb, "difference at {x:?}");
                assert_eq!(na.contains(x), !pa, "complement at {x:?}");
            }
        }
    }
}

/// Region laws against point membership.
pub fn check_region<R: Region + Debug>(regions: &[R], probes: &[R::Value]) {
    let (empty, all) = (R::empty(), R::all());
    assert!(empty.is_empty() && !all.is_empty());
    for p in probes {
        assert!(all.contains(p) && !empty.contains(p));
    }
    for a in regions {
        assert!(a.is_superset(&empty) && all.is_superset(a) && a.is_superset(a));
        assert_eq!(&a.intersection(&all), a, "a & all == a");
        assert!(a.intersection(&empty).is_empty());
        for b in regions {
            let i = a.intersection(b);
            assert_eq!(i, b.intersection(a), "intersection commutes");
            assert_eq!(
                a.is_superset(b),
                &i == b,
                "is_superset({a:?}, {b:?}) must equal (a & b == b)"
            );
            assert_eq!(a.is_superset(b), b.is_subset(a));
            assert_eq!(a.is_disjoint(b), i.is_empty(), "is_disjoint({a:?}, {b:?})");
            assert!(a.is_superset(&i) && b.is_superset(&i));
            for p in probes {
                assert_eq!(
                    i.contains(p),
                    a.contains(p) && b.contains(p),
                    "{a:?} & {b:?}"
                );
            }
        }
    }
}

/// Hull laws: `hull` holds both inputs and is the smallest such region among
/// `regions`.
pub fn check_convex<R: Convex + Debug>(regions: &[R]) {
    for a in regions {
        assert_eq!(&a.hull(a), a);
        assert_eq!(&a.hull(&R::empty()), a);
        for b in regions {
            let h = a.hull(b);
            assert_eq!(h, b.hull(a), "hull commutes");
            assert!(
                h.is_superset(a) && h.is_superset(b),
                "hull({a:?}, {b:?}) = {h:?}"
            );
            for c in regions {
                if c.is_superset(a) && c.is_superset(b) {
                    assert!(
                        c.is_superset(&h),
                        "hull({a:?}, {b:?}) = {h:?} is not minimal: {c:?}"
                    );
                }
            }
        }
    }
}

//! Semantic and comparison-work regressions for sorted set search kernels.
#![cfg(feature = "alloc")]

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

use core::cmp::Ordering;
use frost::family::FrozenOrderedSet;
use std::cell::Cell;
use std::collections::BTreeSet;

struct Measured<'a> {
    key: i64,
    representative: &'static str,
    comparisons: &'a Cell<usize>,
}

impl PartialEq for Measured<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for Measured<'_> {}
impl PartialOrd for Measured<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Measured<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.comparisons.set(self.comparisons.get() + 1);
        self.key.cmp(&other.key)
    }
}

fn measured<'a>(
    keys: impl IntoIterator<Item = i64>,
    representative: &'static str,
    comparisons: &'a Cell<usize>,
) -> FrozenOrderedSet<Measured<'a>> {
    keys.into_iter()
        .map(|key| Measured {
            key,
            representative,
            comparisons,
        })
        .collect()
}

#[test]
fn skewed_operations_skip_large_gaps_and_preserve_left_representatives() {
    let comparisons = Cell::new(0);
    let large = measured((0..65_536).map(|key| key * 2), "large", &comparisons);
    let small = measured([65_001, 65_536, 131_070], "small", &comparisons);

    for (left, right, label) in [(&large, &small, "large"), (&small, &large, "small")] {
        comparisons.set(0);
        assert_eq!(left.intersection_len(right), 2);
        assert!(comparisons.get() < 160, "{} comparisons", comparisons.get());

        comparisons.set(0);
        let result = left.intersection(right).collect::<Vec<_>>();
        assert_eq!(
            result.iter().map(|item| item.key).collect::<Vec<_>>(),
            [65_536, 131_070]
        );
        assert!(result.iter().all(|item| item.representative == label));
        for item in result {
            let stored = left
                .as_slice()
                .iter()
                .find(|stored| stored.key == item.key)
                .unwrap();
            assert!(core::ptr::eq(item, stored));
        }
        assert!(comparisons.get() < 160, "{} comparisons", comparisons.get());

        comparisons.set(0);
        assert!(!left.is_disjoint(right));
        assert!(comparisons.get() < 100, "{} comparisons", comparisons.get());
    }

    comparisons.set(0);
    assert_eq!(
        small
            .difference(&large)
            .map(|item| item.key)
            .collect::<Vec<_>>(),
        [65_001]
    );
    assert!(comparisons.get() < 160, "{} comparisons", comparisons.get());

    let subset = measured([65_536, 131_070], "subset", &comparisons);
    comparisons.set(0);
    assert!(subset.is_subset(&large));
    assert!(comparisons.get() < 120, "{} comparisons", comparisons.get());
    comparisons.set(0);
    assert!(!small.is_subset(&large));
    assert!(comparisons.get() < 100, "{} comparisons", comparisons.get());
}

#[test]
fn lazy_construction_does_no_comparisons_and_separated_ranges_stop_immediately() {
    let comparisons = Cell::new(0);
    let low = measured(0..4096, "low", &comparisons);
    let high = measured(8192..12_288, "high", &comparisons);

    for (left, right) in [(&low, &high), (&high, &low)] {
        comparisons.set(0);
        let mut intersection = left.intersection(right);
        let mut difference = left.difference(right);
        assert_eq!(comparisons.get(), 0);

        assert!(intersection.next().is_none());
        assert!(comparisons.get() <= 2);
        assert_eq!(intersection.size_hint(), (0, Some(0)));
        assert_eq!(intersection.clone().count(), 0);
        assert!(intersection.next().is_none());

        comparisons.set(0);
        assert_eq!(
            difference.next().map(|item| item.key),
            left.first().map(|item| item.key)
        );
        assert!(comparisons.get() <= 2);
        assert_eq!(difference.clone().count(), left.len() - 1);
    }
}

#[test]
fn same_storage_queries_and_lazy_operations_need_no_key_comparisons() {
    let comparisons = Cell::new(0);
    let set = measured(0..4096, "self", &comparisons);
    comparisons.set(0);

    assert_eq!(set.intersection_len(&set), set.len());
    assert_eq!(set.union_len(&set), set.len());
    assert_eq!(set.difference_len(&set), 0);
    assert_eq!(set.symmetric_difference_len(&set), 0);
    assert!(set.is_subset(&set));
    assert!(!set.is_disjoint(&set));
    assert!(set.difference(&set).next().is_none());

    let mut intersection = set.intersection(&set);
    for expected in set.as_slice() {
        assert!(core::ptr::eq(intersection.next().unwrap(), expected));
        assert_eq!(
            intersection.clone().count(),
            set.len() - usize::try_from(expected.key).unwrap() - 1
        );
    }
    assert!(intersection.next().is_none());
    assert_eq!(comparisons.get(), 0);
}

fn check_remaining<'a>(mut cursor: impl Iterator<Item = &'a i64> + Clone) {
    loop {
        let remaining = cursor.clone().copied().collect::<Vec<_>>();
        let (lower, upper) = cursor.size_hint();
        assert!(lower <= remaining.len());
        assert!(upper.is_none_or(|upper| remaining.len() <= upper));
        assert_eq!(cursor.clone().count(), remaining.len());
        let folded = cursor.clone().fold(Vec::new(), |mut values, item| {
            values.push(*item);
            values
        });
        assert_eq!(folded, remaining);
        match cursor.next() {
            Some(item) => assert_eq!(Some(item), remaining.first()),
            None => break,
        }
    }
    assert_eq!(cursor.next(), None);
    assert_eq!(cursor.clone().count(), 0);
}

#[test]
fn skewed_searches_match_standard_sets_and_survive_partial_consumption() {
    let large: FrozenOrderedSet<_> = (0..4096_i64).map(|key| key * 2).collect();
    let large_std: BTreeSet<_> = large.iter().copied().collect();
    for offset in [-1, 0, 1, 37] {
        let small: FrozenOrderedSet<_> = (0..24_i64).map(|key| key * 353 + offset).collect();
        let small_std: BTreeSet<_> = small.iter().copied().collect();
        let common = small_std
            .intersection(&large_std)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(
            small.intersection(&large).copied().collect::<Vec<_>>(),
            common
        );
        assert_eq!(
            large.intersection(&small).copied().collect::<Vec<_>>(),
            common
        );
        assert_eq!(small.intersection_len(&large), common.len());
        assert_eq!(large.intersection_len(&small), common.len());
        assert_eq!(small.is_subset(&large), small_std.is_subset(&large_std));
        assert_eq!(small.is_disjoint(&large), common.is_empty());
        assert_eq!(large.is_disjoint(&small), common.is_empty());
        assert_eq!(
            small.difference(&large).copied().collect::<Vec<_>>(),
            small_std
                .difference(&large_std)
                .copied()
                .collect::<Vec<_>>()
        );
        check_remaining(small.intersection(&large));
        check_remaining(large.intersection(&small));
        check_remaining(small.difference(&large));
    }
}

/// Force the public `next` path even when the iterator specializes `count`.
fn count_next(mut cursor: impl Iterator) -> usize {
    let mut count = 0;
    while cursor.next().is_some() {
        count += 1;
    }
    count
}

#[test]
fn all_skewed_scalar_and_iterator_entrypoints_stay_below_linear_comparison_work() {
    let comparisons = Cell::new(0);
    let large_len = 65_536_i64;
    let small_len = 64_i64;
    let large = measured((0..large_len).map(|key| key * 2), "large", &comparisons);
    let small = measured(
        (0..small_len).map(|key| key * (large_len / small_len) * 2 + key % 2),
        "small",
        &comparisons,
    );
    // This public behavior budget permits different seeking implementations;
    // it rejects a scan of the entire large operand for every entry point.
    let budget = 64 * (2 * (65_536_usize.ilog2() as usize + 1) + 4);
    assert!(budget < large.len() / 4);
    for (name, expected, run) in [
        (
            "intersection_len",
            32,
            &(|| small.intersection_len(&large)) as &dyn Fn() -> usize,
        ),
        ("intersection_len reverse", 32, &|| {
            large.intersection_len(&small)
        }),
        ("difference_len", 32, &|| small.difference_len(&large)),
        ("difference_len reverse", 65_504, &|| {
            large.difference_len(&small)
        }),
        ("union_len", 65_568, &|| large.union_len(&small)),
        ("symmetric_difference_len", 65_536, &|| {
            small.symmetric_difference_len(&large)
        }),
        ("intersection next", 32, &|| {
            count_next(small.intersection(&large))
        }),
        ("intersection next reverse", 32, &|| {
            count_next(large.intersection(&small))
        }),
        ("intersection count", 32, &|| {
            small.intersection(&large).count()
        }),
        ("difference next", 32, &|| {
            count_next(small.difference(&large))
        }),
        ("difference count", 32, &|| small.difference(&large).count()),
        ("is_subset", 0, &|| usize::from(small.is_subset(&large))),
        ("is_disjoint", 0, &|| usize::from(small.is_disjoint(&large))),
        ("is_disjoint reverse", 0, &|| {
            usize::from(large.is_disjoint(&small))
        }),
    ] {
        comparisons.set(0);
        assert_eq!(run(), expected, "{name}");
        assert!(
            comparisons.get() <= budget,
            "{name}: {} comparisons exceed {budget}",
            comparisons.get()
        );
    }
}

#[test]
fn skewed_predicates_skip_gaps_when_the_answer_requires_the_whole_small_operand() {
    let comparisons = Cell::new(0);
    let large = measured((0..65_536).map(|key| key * 2), "large", &comparisons);
    let present = measured((0..64).map(|key| key * 2048), "present", &comparisons);
    let absent = measured((0..64).map(|key| key * 2048 + 1), "absent", &comparisons);
    let late_miss = measured(
        (0..64).map(|key| key * 2048 + i64::from(key == 63)),
        "late miss",
        &comparisons,
    );
    let budget = 64 * (2 * (65_536_usize.ilog2() as usize + 1) + 4);
    for (name, expected, run) in [
        (
            "subset all present",
            true,
            &(|| present.is_subset(&large)) as &dyn Fn() -> bool,
        ),
        ("subset late missing member", false, &|| {
            late_miss.is_subset(&large)
        }),
        ("disjoint all absent", true, &|| absent.is_disjoint(&large)),
        ("disjoint all absent reverse", true, &|| {
            large.is_disjoint(&absent)
        }),
    ] {
        comparisons.set(0);
        assert_eq!(run(), expected, "{name}");
        assert!(
            comparisons.get() <= budget,
            "{name}: {} comparisons exceed {budget}",
            comparisons.get()
        );
    }
}

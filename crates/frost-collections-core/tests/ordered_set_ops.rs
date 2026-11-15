//! Ordered set algebra: semantic, iterator-state, and kernel regressions.
#![cfg(feature = "alloc")]

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

use core::cmp::Ordering;
use core::iter::FusedIterator;
use frost::family::FrozenOrderedSet;
use proptest::prelude::*;
use std::collections::BTreeSet;

fn check_against_standard(left: &[i16], right: &[i16]) {
    let left_set: FrozenOrderedSet<_> = left.iter().copied().collect();
    let right_set: FrozenOrderedSet<_> = right.iter().copied().collect();
    let left_std: BTreeSet<_> = left.iter().copied().collect();
    let right_std: BTreeSet<_> = right.iter().copied().collect();

    let union = left_std.union(&right_std).copied().collect::<Vec<_>>();
    let intersection = left_std
        .intersection(&right_std)
        .copied()
        .collect::<Vec<_>>();
    let difference = left_std.difference(&right_std).copied().collect::<Vec<_>>();
    let symmetric = left_std
        .symmetric_difference(&right_std)
        .copied()
        .collect::<Vec<_>>();

    assert_eq!(
        left_set.union(&right_set).copied().collect::<Vec<_>>(),
        union
    );
    assert_eq!(
        left_set
            .intersection(&right_set)
            .copied()
            .collect::<Vec<_>>(),
        intersection
    );
    assert_eq!(
        left_set.difference(&right_set).copied().collect::<Vec<_>>(),
        difference
    );
    assert_eq!(
        left_set
            .symmetric_difference(&right_set)
            .copied()
            .collect::<Vec<_>>(),
        symmetric
    );

    assert_eq!(left_set.union_len(&right_set), union.len());
    assert_eq!(left_set.intersection_len(&right_set), intersection.len());
    assert_eq!(left_set.difference_len(&right_set), difference.len());
    assert_eq!(
        left_set.symmetric_difference_len(&right_set),
        symmetric.len()
    );

    assert_eq!(left_set.union(&right_set).count(), union.len());
    assert_eq!(
        left_set.intersection(&right_set).count(),
        intersection.len()
    );
    assert_eq!(left_set.difference(&right_set).count(), difference.len());
    assert_eq!(
        left_set.symmetric_difference(&right_set).count(),
        symmetric.len()
    );

    assert_eq!(
        left_set.is_subset(&right_set),
        left_std.is_subset(&right_std)
    );
    assert_eq!(
        left_set.is_superset(&right_set),
        left_std.is_superset(&right_std)
    );
    assert_eq!(
        left_set.is_disjoint(&right_set),
        left_std.is_disjoint(&right_std)
    );
}

// Review-bundle scenarios exercise the public algebra contract regardless of
// which merge/search strategy is selected internally.
proptest! {
    #[test]
    fn skewed_algebra_matches_standard_sets(
        small in proptest::collection::vec(-5000_i16..5000, 0..12),
        large in proptest::collection::vec(-5000_i16..5000, 0..3000),
    ) {
        check_against_standard(&small, &large);
        check_against_standard(&large, &small);
    }

    #[test]
    fn shared_prefix_and_uneven_tails_match_standard_sets(
        shared in proptest::collection::vec(-200_i16..0, 0..60),
        tail in proptest::collection::vec(0_i16..5000, 0..2000),
        probe in proptest::collection::vec(0_i16..5000, 0..6),
    ) {
        let mut left = shared.clone();
        left.extend_from_slice(&probe);
        let mut right = shared;
        right.extend_from_slice(&tail);
        check_against_standard(&left, &right);
        check_against_standard(&right, &left);
    }
}

#[test]
fn empty_singleton_identical_disjoint_and_overlapping_sets() {
    let cases: &[&[i16]] = &[
        &[],
        &[1],
        &[2],
        &[1, 2, 3],
        &[2, 4, 6],
        &[1, 3, 5, 7],
        &[10, 20, 30],
        &[3, 1, 1, 2, 3],
    ];
    for left in cases {
        for right in cases {
            check_against_standard(left, right);
        }
    }

    let same: FrozenOrderedSet<_> = [1, 2, 3].into_iter().collect();
    assert_eq!(same.union(&same).copied().collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(
        same.intersection(&same).copied().collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(same.difference(&same).next().is_none());
    assert!(same.symmetric_difference(&same).next().is_none());
    assert!(same.is_subset(&same));
    assert!(same.is_superset(&same));
    assert!(!same.is_disjoint(&same));
}

// Intentionally implements neither Clone nor Debug. Equivalent keys keep
// observable metadata, while equality and ordering share one coherent relation.
struct Member {
    key: u8,
    label: &'static str,
}

impl PartialEq for Member {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for Member {}
impl PartialOrd for Member {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Member {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

#[test]
fn overlap_keeps_the_left_representative_without_cloning_elements() {
    let left: FrozenOrderedSet<_> = [(1, "left one"), (3, "left three"), (5, "left five")]
        .map(|(key, label)| Member { key, label })
        .into_iter()
        .collect();
    let right: FrozenOrderedSet<_> = [(2, "right two"), (3, "right three"), (4, "right four")]
        .map(|(key, label)| Member { key, label })
        .into_iter()
        .collect();

    assert_eq!(
        left.union(&right)
            .map(|item| (item.key, item.label))
            .collect::<Vec<_>>(),
        [
            (1, "left one"),
            (2, "right two"),
            (3, "left three"),
            (4, "right four"),
            (5, "left five")
        ]
    );
    let shared = left.intersection(&right).next().unwrap();
    assert!(core::ptr::eq(shared, &raw const left.as_slice()[1]));
    assert_eq!(
        right.intersection(&left).next().unwrap().label,
        "right three"
    );
    assert_eq!(
        left.difference(&right)
            .map(|item| item.key)
            .collect::<Vec<_>>(),
        [1, 5]
    );
    assert_eq!(
        left.symmetric_difference(&right)
            .map(|item| item.key)
            .collect::<Vec<_>>(),
        [1, 2, 4, 5]
    );

    // Both cursor traits are independent of the element's Clone/Debug traits.
    assert!(format!("{:?}", left.union(&right).clone()).contains("SetUnion"));
    assert!(format!("{:?}", left.intersection(&right).clone()).contains("SetIntersection"));
    assert!(format!("{:?}", left.difference(&right).clone()).contains("SetDifference"));
    assert!(
        format!("{:?}", left.symmetric_difference(&right).clone())
            .contains("SetSymmetricDifference")
    );

    let borrowed: FrozenOrderedSet<&Member> = left.union(&right).collect();
    assert_eq!(borrowed.len(), 5);
    assert!(core::ptr::eq(
        borrowed.as_slice()[2],
        &raw const left.as_slice()[1]
    ));
}

fn check_cursor<'a, I>(mut cursor: I)
where
    I: Iterator<Item = &'a i32> + FusedIterator + Clone + std::fmt::Debug,
{
    loop {
        let actual = cursor.clone().copied().collect::<Vec<_>>();
        let (lower, upper) = cursor.size_hint();
        assert!(
            lower <= actual.len(),
            "{cursor:?}: lower bound exceeds remaining count"
        );
        assert!(upper.is_none_or(|upper| actual.len() <= upper));
        assert_eq!(cursor.clone().count(), actual.len());
        // `fold` must yield the same references in the same order as `next`.
        let stepped: Vec<*const i32> = cursor.clone().map(core::ptr::from_ref).collect();
        let folded = cursor.clone().fold(Vec::new(), |mut seen, item| {
            seen.push(core::ptr::from_ref(item));
            seen
        });
        assert_eq!(folded, stepped, "{cursor:?}");

        match cursor.next() {
            Some(next) => assert_eq!(Some(next), actual.first()),
            None => break,
        }
    }
    assert_eq!(cursor.next(), None);
    assert_eq!(cursor.next(), None);
    assert_eq!(cursor.clone().count(), 0);
}

#[test]
fn hints_clones_counts_and_fused_exhaustion_track_partial_consumption() {
    let fixtures = [vec![], vec![1], vec![1, 2, 3], vec![2, 4, 6], vec![10, 11]];
    for left in &fixtures {
        for right in &fixtures {
            let left: FrozenOrderedSet<_> = left.iter().copied().collect();
            let right: FrozenOrderedSet<_> = right.iter().copied().collect();
            check_cursor(left.union(&right));
            check_cursor(left.intersection(&right));
            check_cursor(left.difference(&right));
            check_cursor(left.symmetric_difference(&right));
        }
    }
}

/// `fold` selects its kernel once. It must agree with stepwise traversal from
/// every partially consumed state, for balanced merges, both seek directions
/// (size ratios beyond the seek threshold), identical and separated operands.
#[test]
fn folds_match_stepwise_traversal_for_merge_and_seek_kernels() {
    let dense: Vec<i32> = (0..400).collect();
    let fixtures = [
        vec![],
        vec![7],
        vec![0, 199, 399],
        (0..400).step_by(2).collect::<Vec<_>>(),
        (1..400).step_by(3).collect(),
        dense.clone(),
        (1_000..1_010).collect(),
        vec![-5, 150, 151, 152, 900],
    ];
    for left in &fixtures {
        for right in &fixtures {
            let left: FrozenOrderedSet<_> = left.iter().copied().collect();
            let right: FrozenOrderedSet<_> = right.iter().copied().collect();
            check_cursor(left.union(&right));
            check_cursor(left.intersection(&right));
            check_cursor(left.difference(&right));
            check_cursor(left.symmetric_difference(&right));
        }
    }
    let same: FrozenOrderedSet<_> = dense.into_iter().collect();
    check_cursor(same.union(&same));
    check_cursor(same.difference(&same));
    check_cursor(same.symmetric_difference(&same));
    assert_eq!(
        same.union(&same).map(|item| i64::from(*item)).sum::<i64>(),
        79_800
    );
    assert_eq!(same.difference(&same).fold(0, |count, _| count + 1), 0);
}

#[test]
fn keys_may_borrow_nonstatic_data() {
    let storage = [String::from("a"), String::from("b"), String::from("c")];
    let left: FrozenOrderedSet<&str> = [&storage[0][..], &storage[1][..]].into_iter().collect();
    let right: FrozenOrderedSet<&str> = [&storage[1][..], &storage[2][..]].into_iter().collect();
    assert_eq!(
        left.union(&right).copied().collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    assert_eq!(
        left.intersection(&right).copied().collect::<Vec<_>>(),
        ["b"]
    );
    assert_eq!(left.difference_len(&right), 1);
}

#[test]
fn scalar_predicates_short_circuit_and_scan_linearly() {
    use std::cell::Cell;
    use std::rc::Rc;

    struct Counted {
        key: u16,
        comparisons: Rc<Cell<usize>>,
    }
    impl PartialEq for Counted {
        fn eq(&self, other: &Self) -> bool {
            self.key == other.key
        }
    }
    impl Eq for Counted {}
    impl PartialOrd for Counted {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for Counted {
        fn cmp(&self, other: &Self) -> Ordering {
            self.comparisons.set(self.comparisons.get() + 1);
            self.key.cmp(&other.key)
        }
    }

    let comparisons = Rc::new(Cell::new(0));
    let make_set = |keys: Vec<u16>| -> FrozenOrderedSet<Counted> {
        keys.into_iter()
            .map(|key| Counted {
                key,
                comparisons: Rc::clone(&comparisons),
            })
            .collect()
    };
    let left = make_set((0..1000).step_by(2).collect());
    let interleaved = make_set((1..1000).step_by(2).collect());
    let separate = make_set((1000..1010).collect());

    comparisons.set(0);
    assert!(left.is_disjoint(&separate));
    assert!(comparisons.get() <= 2);
    comparisons.set(0);
    assert_eq!(left.intersection_len(&separate), 0);
    assert!(comparisons.get() <= 2);
    comparisons.set(0);
    assert!(!left.is_subset(&separate));
    assert_eq!(comparisons.get(), 0);

    comparisons.set(0);
    assert!(!left.is_disjoint(&left));
    assert!(comparisons.get() <= 3);
    comparisons.set(0);
    assert_eq!(left.intersection_len(&interleaved), 0);
    assert!(comparisons.get() <= left.len() + interleaved.len() + 2);
}

proptest! {
    #[test]
    fn algebra_matches_btree_sets_for_random_inputs(
        left in proptest::collection::vec(-100_i16..100, 0..100),
        right in proptest::collection::vec(-100_i16..100, 0..100),
    ) {
        check_against_standard(&left, &right);
        check_against_standard(&right, &left);
    }
}

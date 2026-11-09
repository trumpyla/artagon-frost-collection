//! Frozen bitmap algebra, logical universes, padding, and full-width members.

#[cfg(feature = "roaring-0_12")]
macro_rules! roaring_tests {
    ($module:ident, $backend:ident, $element:ty) => {
        mod $module {
            use frost::backend::roaring::$backend;
            use frost::{FreezeWith, Retain};
            use proptest::prelude::*;
            use std::collections::BTreeSet;

            fn check(left: $backend, right: $backend) {
                let left_members: BTreeSet<$element> = left.iter().collect();
                let right_members: BTreeSet<$element> = right.iter().collect();
                let expected_union: BTreeSet<_> =
                    left_members.union(&right_members).copied().collect();
                let expected_intersection: BTreeSet<_> =
                    left_members.intersection(&right_members).copied().collect();
                let expected_difference: BTreeSet<_> =
                    left_members.difference(&right_members).copied().collect();
                let expected_symmetric: BTreeSet<_> = left_members
                    .symmetric_difference(&right_members)
                    .copied()
                    .collect();
                let original_left = left.clone();
                let original_right = right.clone();
                let left = left.freeze_with(Retain);
                let right = right.freeze_with(Retain);

                let counts: [u64; 4] = [
                    left.union_len(&right),
                    left.intersection_len(&right),
                    left.difference_len(&right),
                    left.symmetric_difference_len(&right),
                ];
                let expected = [
                    expected_union,
                    expected_intersection,
                    expected_difference,
                    expected_symmetric,
                ];
                let results = [
                    left.union(&right),
                    left.intersection(&right),
                    left.difference(&right),
                    left.symmetric_difference(&right),
                ];
                for ((result, expected), count) in results.into_iter().zip(expected).zip(counts) {
                    assert_eq!(result.iter().collect::<BTreeSet<_>>(), expected);
                    assert_eq!(count, u64::try_from(expected.len()).unwrap());
                    assert_eq!(result.cardinality(), count);
                }

                assert_eq!(
                    left.is_subset(&right),
                    left_members.is_subset(&right_members)
                );
                assert_eq!(
                    left.is_superset(&right),
                    left_members.is_superset(&right_members)
                );
                assert_eq!(
                    left.is_disjoint(&right),
                    left_members.is_disjoint(&right_members)
                );
                assert_eq!(left.into_inner(), original_left);
                assert_eq!(right.into_inner(), original_right);
            }

            #[test]
            fn sparse_dense_empty_and_extreme_members() {
                let sparse: $backend = [0, 1, 65_535, 65_536, 65_537, <$element>::MAX]
                    .into_iter()
                    .collect();
                let dense: $backend = (0..10_000)
                    .chain([65_536, <$element>::MAX - 1, <$element>::MAX])
                    .collect();
                check(sparse.clone(), dense.clone());
                check(dense, sparse.clone());
                check(sparse.clone(), sparse.clone());
                check(sparse.clone(), $backend::new());
                check($backend::new(), sparse);
                check($backend::new(), $backend::new());
            }

            proptest! {
                #![proptest_config(ProptestConfig::with_cases(48))]

                #[test]
                fn algebra_matches_reference_membership(
                    left in prop::collection::vec(any::<$element>(), 0..96),
                    right in prop::collection::vec(any::<$element>(), 0..96),
                    common in prop::collection::vec(0..1024 as $element, 0..24),
                ) {
                    check(
                        left.into_iter().chain(common.iter().copied()).collect(),
                        right.into_iter().chain(common).collect(),
                    );
                }
            }
        }
    };
}

#[cfg(feature = "roaring-0_12")]
roaring_tests!(bitmap, RoaringBitmap, u32);
#[cfg(feature = "roaring-0_12")]
roaring_tests!(treemap, RoaringTreemap, u64);

#[cfg(feature = "fixedbitset-0_5")]
mod fixed {
    use frost::backend::fixedbitset::FixedBitSet;
    use frost::{FreezeWith, Frozen, Retain};
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    const WORD_BITS: usize = std::mem::size_of::<usize>() * 8;

    fn assert_result(result: Frozen<FixedBitSet>, expected: &BTreeSet<usize>, universe_len: usize) {
        assert_eq!(result.universe_len(), universe_len);
        assert_eq!(result.ones().collect::<BTreeSet<_>>(), *expected);
        assert_eq!(result.cardinality(), u64::try_from(expected.len()).unwrap());
        // Native traversal includes raw padding. New results must sanitize it,
        // rather than merely conceal it behind the frozen member iterator.
        assert!(result.into_inner().ones().all(|index| index < universe_len));
    }

    fn check(left: FixedBitSet, right: FixedBitSet) {
        let left_len = left.len();
        let right_len = right.len();
        let left_members: BTreeSet<_> = (0..left_len).filter(|&i| left.contains(i)).collect();
        let right_members: BTreeSet<_> = (0..right_len).filter(|&i| right.contains(i)).collect();
        let union: BTreeSet<_> = left_members.union(&right_members).copied().collect();
        let intersection: BTreeSet<_> =
            left_members.intersection(&right_members).copied().collect();
        let difference: BTreeSet<_> = left_members.difference(&right_members).copied().collect();
        let symmetric: BTreeSet<_> = left_members
            .symmetric_difference(&right_members)
            .copied()
            .collect();
        let left_blocks = left.as_slice().to_vec();
        let right_blocks = right.as_slice().to_vec();
        let left = left.freeze_with(Retain);
        let right = right.freeze_with(Retain);

        let counts: [usize; 4] = [
            left.union_len(&right),
            left.intersection_len(&right),
            left.difference_len(&right),
            left.symmetric_difference_len(&right),
        ];
        assert_eq!(
            counts,
            [
                union.len(),
                intersection.len(),
                difference.len(),
                symmetric.len()
            ]
        );
        assert_eq!(
            left.is_subset(&right),
            left_members.is_subset(&right_members)
        );
        assert_eq!(
            left.is_superset(&right),
            left_members.is_superset(&right_members)
        );
        assert_eq!(
            left.is_disjoint(&right),
            left_members.is_disjoint(&right_members)
        );

        assert_result(left.union(&right), &union, left_len.max(right_len));
        assert_result(
            left.intersection(&right),
            &intersection,
            left_len.min(right_len),
        );
        assert_result(left.difference(&right), &difference, left_len);
        assert_result(
            left.symmetric_difference(&right),
            &symmetric,
            left_len.max(right_len),
        );
        assert_eq!(left.universe_len(), left_len);
        assert_eq!(right.universe_len(), right_len);
        assert_eq!(left.into_inner().as_slice(), left_blocks);
        assert_eq!(right.into_inner().as_slice(), right_blocks);
    }

    #[test]
    fn empty_partial_and_unequal_universes_ignore_dirty_padding() {
        let lengths = [
            0,
            1,
            WORD_BITS - 1,
            WORD_BITS,
            WORD_BITS + 1,
            2 * WORD_BITS - 1,
            2 * WORD_BITS,
            2 * WORD_BITS + 1,
        ];
        for left_len in lengths {
            for right_len in lengths {
                for left_word in [0, usize::MAX, usize::MAX / 3] {
                    for right_word in [0, usize::MAX, !(usize::MAX / 3)] {
                        let left = FixedBitSet::with_capacity_and_blocks(
                            left_len,
                            std::iter::repeat_n(left_word, left_len.div_ceil(WORD_BITS)),
                        );
                        let right = FixedBitSet::with_capacity_and_blocks(
                            right_len,
                            std::iter::repeat_n(right_word, right_len.div_ceil(WORD_BITS)),
                        );
                        check(left, right);
                    }
                }
            }
        }
    }

    #[test]
    fn padding_cannot_become_a_member_in_a_larger_universe() {
        let short = FixedBitSet::with_capacity_and_blocks(1, [usize::MAX]);
        let long = FixedBitSet::with_capacity_and_blocks(WORD_BITS + 1, [0, 1]);
        check(short.clone(), long.clone());
        check(long.clone(), short.clone());
        let short = short.freeze_with(Retain);
        let long = long.freeze_with(Retain);
        assert_eq!(
            short.union(&long).ones().collect::<Vec<_>>(),
            [0, WORD_BITS]
        );
        assert_eq!(
            long.difference(&short).ones().collect::<Vec<_>>(),
            [WORD_BITS]
        );
        assert!(short.is_disjoint(&long));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn algebra_matches_logical_membership_with_arbitrary_blocks(
            left_len in 0_usize..1025,
            right_len in 0_usize..1025,
            left in prop::collection::vec(any::<usize>(), 0..18),
            right in prop::collection::vec(any::<usize>(), 0..18),
        ) {
            check(
                FixedBitSet::with_capacity_and_blocks(left_len, left),
                FixedBitSet::with_capacity_and_blocks(right_len, right),
            );
        }
    }
}

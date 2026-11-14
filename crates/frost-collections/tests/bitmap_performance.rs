//! Regressions for dense kernels that split shared words and unequal tails.

#![cfg(feature = "fixedbitset-0_5")]

use frost::backend::fixedbitset::FixedBitSet;
use frost::{FreezeWith, Frozen, Retain};
use std::collections::BTreeSet;

const WORD_BITS: usize = core::mem::size_of::<usize>() * 8;

fn members(bits: &FixedBitSet) -> BTreeSet<usize> {
    (0..bits.len())
        .filter(|&index| bits.contains(index))
        .collect()
}

fn pattern(len: usize, stride: usize) -> FixedBitSet {
    let mut bits = FixedBitSet::with_capacity(len);
    bits.extend((0..len).step_by(stride));
    // Deliberately retain dirty native padding to exercise both boundaries.
    if !len.is_multiple_of(WORD_BITS) {
        *bits.as_mut_slice().last_mut().unwrap() |= usize::MAX << (len % WORD_BITS);
    }
    bits
}

fn check(left: FixedBitSet, right: FixedBitSet) {
    let left_len = left.len();
    let right_len = right.len();
    let left_members = members(&left);
    let right_members = members(&right);
    let expected: [BTreeSet<_>; 4] = [
        left_members.union(&right_members).copied().collect(),
        left_members.intersection(&right_members).copied().collect(),
        left_members.difference(&right_members).copied().collect(),
        left_members
            .symmetric_difference(&right_members)
            .copied()
            .collect(),
    ];
    let left = left.freeze_with(Retain);
    let right = right.freeze_with(Retain);
    let counts = [
        left.union_len(&right),
        left.intersection_len(&right),
        left.difference_len(&right),
        left.symmetric_difference_len(&right),
    ];
    let results: [Frozen<FixedBitSet>; 4] = [
        left.union(&right),
        left.intersection(&right),
        left.difference(&right),
        left.symmetric_difference(&right),
    ];
    let lengths = [
        left_len.max(right_len),
        left_len.min(right_len),
        left_len,
        left_len.max(right_len),
    ];
    for (((result, expected), count), len) in
        results.into_iter().zip(expected).zip(counts).zip(lengths)
    {
        assert_eq!(count, expected.len());
        assert_eq!(result.universe_len(), len);
        assert_eq!(result.ones().collect::<BTreeSet<_>>(), expected);
        assert_eq!(result.cardinality(), u64::try_from(count).unwrap());
        assert!(result.into_inner().ones().all(|index| index < len));
    }
    assert_eq!(
        left.is_subset(&right),
        left_members.is_subset(&right_members)
    );
    assert_eq!(
        left.is_disjoint(&right),
        left_members.is_disjoint(&right_members)
    );
}

#[test]
fn long_shared_word_spans_and_partial_tails() {
    for length in [
        257 * WORD_BITS,
        257 * WORD_BITS + 1,
        257 * WORD_BITS + WORD_BITS - 1,
    ] {
        check(pattern(length, 3), pattern(length, 5));
    }
}

#[test]
fn long_zero_extended_suffixes_in_both_directions() {
    for short_len in [
        0,
        1,
        WORD_BITS - 1,
        WORD_BITS,
        WORD_BITS + 1,
        17 * WORD_BITS + 3,
    ] {
        let short = pattern(short_len, 3);
        let long = pattern(257 * WORD_BITS + 11, 5);
        check(short.clone(), long.clone());
        check(long, short);
    }
}

#[test]
fn both_boundaries_mask_dirty_padding_without_losing_real_members() {
    let short_len = 2 * WORD_BITS + 1;
    let long_len = 256 * WORD_BITS + 1;
    let short = FixedBitSet::with_capacity_and_blocks(
        short_len,
        std::iter::repeat_n(usize::MAX, short_len.div_ceil(WORD_BITS)),
    );
    let long = FixedBitSet::with_capacity_and_blocks(
        long_len,
        std::iter::repeat_n(usize::MAX, long_len.div_ceil(WORD_BITS)),
    );
    check(short.clone(), long.clone());
    check(long, short);
}

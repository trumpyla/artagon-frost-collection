//! Finite exhaustive checks against independent membership and sequence models.
//!
//! These enumerate every input in the stated domains; they do not prove the
//! algorithms for arbitrary collection sizes. See `docs/verification.md`.
#![cfg(feature = "alloc")]

use frost::adaptive::InlineVec;
use frost::family::FrozenOrderedSet;
use std::cell::RefCell;

const UNIVERSE: u8 = 6;
const SETS: u8 = 1 << UNIVERSE;

fn members(mask: u8) -> Vec<u8> {
    (0..UNIVERSE).filter(|bit| mask & (1 << bit) != 0).collect()
}

fn check_remaining<'a, I>(iter: &I, expected: &[u8])
where
    I: Iterator<Item = &'a u8> + Clone,
{
    // Every possible consumed prefix exercises the iterator's residual state,
    // including the specialized count/fold implementations and exhaustion.
    for consumed in 0..=expected.len() {
        let mut remaining = iter.clone();
        for value in &expected[..consumed] {
            assert_eq!(remaining.next(), Some(value));
        }
        let tail = &expected[consumed..];
        let (lower, upper) = remaining.size_hint();
        assert!(lower <= tail.len());
        assert!(upper.is_none_or(|upper| upper >= tail.len()));
        assert_eq!(remaining.clone().count(), tail.len());
        assert_eq!(
            remaining.clone().fold(Vec::new(), |mut values, &value| {
                values.push(value);
                values
            }),
            tail
        );
        assert_eq!(remaining.by_ref().copied().collect::<Vec<_>>(), tail);
        assert_eq!(remaining.next(), None);
        assert_eq!(remaining.next(), None);
    }
}

#[test]
fn ordered_algebra_exhausts_all_pairs_of_six_member_sets() {
    for left_mask in 0..SETS {
        for right_mask in 0..SETS {
            let left = FrozenOrderedSet::try_from_sorted_unique(members(left_mask)).unwrap();
            let right = FrozenOrderedSet::try_from_sorted_unique(members(right_mask)).unwrap();
            let union = members(left_mask | right_mask);
            let intersection = members(left_mask & right_mask);
            let difference = members(left_mask & !right_mask);
            let symmetric = members(left_mask ^ right_mask);

            check_remaining(&left.union(&right), &union);
            check_remaining(&left.intersection(&right), &intersection);
            check_remaining(&left.difference(&right), &difference);
            check_remaining(&left.symmetric_difference(&right), &symmetric);
            assert_eq!(left.union_len(&right), union.len());
            assert_eq!(left.intersection_len(&right), intersection.len());
            assert_eq!(left.difference_len(&right), difference.len());
            assert_eq!(left.symmetric_difference_len(&right), symmetric.len());
            assert_eq!(left.is_subset(&right), left_mask & !right_mask == 0);
            assert_eq!(left.is_superset(&right), right_mask & !left_mask == 0);
            assert_eq!(left.is_disjoint(&right), left_mask & right_mask == 0);
        }
    }
}

#[derive(Debug)]
struct Tracked<'a, const STEPS: usize> {
    id: usize,
    value: u8,
    drops: &'a RefCell<[u8; STEPS]>,
}

impl<const STEPS: usize> Drop for Tracked<'_, STEPS> {
    fn drop(&mut self) {
        let mut counts = self.drops.borrow_mut();
        counts[self.id] += 1;
        assert_eq!(counts[self.id], 1, "an element was destroyed twice");
    }
}

fn check_inline_trace<const CAPACITY: usize, const STEPS: usize>(mut trace: usize) {
    const OPERATIONS: usize = 7;

    let drops = RefCell::new([0; STEPS]);
    let mut actual = InlineVec::<_, CAPACITY>::new();
    let mut expected = Vec::new();
    let mut created = 0;
    for _ in 0..STEPS {
        let operation = trace % OPERATIONS;
        trace /= OPERATIONS;
        match operation {
            0 | 1 => {
                let value = u8::try_from(operation).unwrap();
                actual.push(Tracked {
                    id: created,
                    value,
                    drops: &drops,
                });
                expected.push((created, value));
                created += 1;
            }
            2 => {
                let popped = actual.pop();
                assert_eq!(
                    popped.as_ref().map(|item| (item.id, item.value)),
                    expected.pop()
                );
                drop(popped);
            }
            3 => {
                if !expected.is_empty() {
                    let removed = actual.remove(0);
                    assert_eq!((removed.id, removed.value), expected.remove(0));
                    drop(removed);
                }
            }
            4 => {
                actual.retain(|item| item.value == 0);
                expected.retain(|item| item.1 == 0);
            }
            5 => actual = actual.freeze().thaw(),
            6 => actual.shrink_to_fit(),
            _ => unreachable!(),
        }
        assert_eq!(
            actual
                .iter()
                .map(|item| (item.id, item.value))
                .collect::<Vec<_>>(),
            expected
        );
        assert!(actual.capacity() >= actual.len());
        for id in 0..created {
            let still_owned = expected.iter().any(|item| item.0 == id);
            assert_eq!(drops.borrow()[id], u8::from(!still_owned));
        }
    }
    drop(actual);
    assert!(drops.borrow()[..created].iter().all(|&count| count == 1));
    assert!(drops.borrow()[created..].iter().all(|&count| count == 0));
}

#[test]
fn inline_ownership_exhausts_five_operation_traces() {
    // 7^5 traces at each capacity include spill, shrink, retention, and
    // freeze/thaw with non-Clone values and destruction after every prefix.
    for trace in 0..7_usize.pow(5) {
        check_inline_trace::<0, 5>(trace);
        check_inline_trace::<2, 5>(trace);
    }
}

#[test]
fn inline_ownership_exhausts_three_operation_traces() {
    // A separately named, smaller domain for interpreters such as Miri. It
    // includes the capacity-two spill and does not replace the five-step test.
    for trace in 0..7_usize.pow(3) {
        check_inline_trace::<0, 3>(trace);
        check_inline_trace::<2, 3>(trace);
    }
}

#[cfg(feature = "fixedbitset-0_5")]
mod dense {
    use frost::backend::fixedbitset::FixedBitSet;
    use frost::{FreezeWith, Frozen, Retain};

    const WORD: usize = usize::BITS as usize;
    const POSITIONS: [usize; 6] = [0, WORD - 1, WORD, WORD + 1, 2 * WORD - 1, 2 * WORD];
    const LENGTHS: [usize; 8] = [
        0,
        1,
        WORD - 1,
        WORD,
        WORD + 1,
        2 * WORD - 1,
        2 * WORD,
        2 * WORD + 1,
    ];

    fn bitmap(len: usize, mask: u8, dirty_padding: bool) -> Frozen<FixedBitSet> {
        let mut blocks = vec![0; len.div_ceil(WORD)];
        for (bit, position) in POSITIONS.into_iter().enumerate() {
            if mask & (1 << bit) != 0 {
                assert!(position < len);
                blocks[position / WORD] |= 1 << (position % WORD);
            }
        }
        if dirty_padding && !len.is_multiple_of(WORD) {
            *blocks.last_mut().unwrap() |= usize::MAX << (len % WORD);
        }
        FixedBitSet::with_capacity_and_blocks(len, blocks).freeze_with(Retain)
    }

    fn expected(mask: u8) -> Vec<usize> {
        POSITIONS
            .into_iter()
            .enumerate()
            .filter_map(|(bit, position)| (mask & (1 << bit) != 0).then_some(position))
            .collect()
    }

    fn check_result(result: Frozen<FixedBitSet>, mask: u8, len: usize) {
        assert_eq!(result.universe_len(), len);
        assert_eq!(result.ones().collect::<Vec<_>>(), expected(mask));
        assert_eq!(result.cardinality(), u64::from(mask.count_ones()));
        assert!(result.into_inner().ones().all(|position| position < len));
    }

    #[test]
    fn bitmap_algebra_exhausts_boundary_members_and_dirty_padding() {
        // These six independent membership bits straddle three machine words.
        // Enumerate all valid masks at eight logical lengths, independently
        // choosing clean or all-one trailing padding on each input.
        let inputs: Vec<_> = LENGTHS
            .into_iter()
            .flat_map(|len| {
                (0_u8..64).filter_map(move |mask| {
                    expected(mask)
                        .iter()
                        .all(|&position| position < len)
                        .then_some((len, mask))
                })
            })
            .collect();
        for &(left_len, left_mask) in &inputs {
            for &(right_len, right_mask) in &inputs {
                for padding in 0..4 {
                    let left = bitmap(left_len, left_mask, padding & 1 != 0);
                    let right = bitmap(right_len, right_mask, padding & 2 != 0);
                    let masks = [
                        left_mask | right_mask,
                        left_mask & right_mask,
                        left_mask & !right_mask,
                        left_mask ^ right_mask,
                    ];
                    assert_eq!(
                        [
                            left.union_len(&right),
                            left.intersection_len(&right),
                            left.difference_len(&right),
                            left.symmetric_difference_len(&right)
                        ],
                        masks.map(|mask| usize::try_from(mask.count_ones()).unwrap())
                    );
                    assert_eq!(left.is_subset(&right), left_mask & !right_mask == 0);
                    assert_eq!(left.is_superset(&right), right_mask & !left_mask == 0);
                    assert_eq!(left.is_disjoint(&right), left_mask & right_mask == 0);
                    check_result(left.union(&right), masks[0], left_len.max(right_len));
                    check_result(left.intersection(&right), masks[1], left_len.min(right_len));
                    check_result(left.difference(&right), masks[2], left_len);
                    check_result(
                        left.symmetric_difference(&right),
                        masks[3],
                        left_len.max(right_len),
                    );
                }
            }
        }
    }
}

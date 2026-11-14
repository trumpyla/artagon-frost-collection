//! Optional bounded Kani proofs. Normal tests only type-check and exercise the
//! property helpers; they are not a substitute for a successful `cargo kani`.
#![no_std]

extern crate alloc;

// Reuse the exact unsafe implementation, not a rewritten model of it. The
// public collection properties below also call the normal Frost dependency.
#[cfg(any(kani, test))]
#[allow(dead_code)]
#[path = "../../../crates/frost-collections-core/src/adaptive/seq/buf.rs"]
mod inline_buf;

// `SET-REP` and `SET-BOOLEAN` refinement on the real ordered merge kernels.
#[cfg(any(kani, test))]
mod set_laws;

#[cfg(any(kani, test))]
mod checks {
    use alloc::vec::Vec;
    use core::cell::Cell;
    use frost::backend::fixedbitset::FixedBitSet;
    use frost::family::FrozenOrderedSet;
    use frost::{FreezeWith, Retain};

    use super::inline_buf::InlineBuf;

    fn members(mask: u8) -> Vec<u8> {
        (0..4).filter(|bit| mask & (1 << bit) != 0).collect()
    }

    pub(super) fn ordered_masks(left_mask: u8, right_mask: u8) {
        assert!(left_mask < 16 && right_mask < 16);
        let left = FrozenOrderedSet::try_from_sorted_unique(members(left_mask)).unwrap();
        let right = FrozenOrderedSet::try_from_sorted_unique(members(right_mask)).unwrap();
        let expected = [
            left_mask | right_mask,
            left_mask & right_mask,
            left_mask & !right_mask,
            left_mask ^ right_mask,
        ];
        let actual = [
            left.union(&right).copied().collect::<Vec<_>>(),
            left.intersection(&right).copied().collect(),
            left.difference(&right).copied().collect(),
            left.symmetric_difference(&right).copied().collect(),
        ];
        let counts = [
            left.union_len(&right),
            left.intersection_len(&right),
            left.difference_len(&right),
            left.symmetric_difference_len(&right),
        ];
        for index in 0..4 {
            assert_eq!(actual[index], members(expected[index]));
            assert_eq!(counts[index], actual[index].len());
        }
        assert_eq!(left.is_subset(&right), left_mask & !right_mask == 0);
        assert_eq!(left.is_superset(&right), right_mask & !left_mask == 0);
        assert_eq!(left.is_disjoint(&right), left_mask & right_mask == 0);
    }

    pub(super) fn sorted_validation(values: [u8; 4], len: usize) {
        assert!(len <= 4);
        let input = Vec::from(&values[..len]);
        let first_invalid = (1..len).find(|&index| values[index - 1] >= values[index]);
        match FrozenOrderedSet::try_from_sorted_unique(input) {
            Ok(set) => {
                assert_eq!(first_invalid, None);
                assert_eq!(set.as_slice(), &values[..len]);
            }
            Err(rejected) => {
                assert_eq!(Some(rejected.error.index), first_invalid);
                assert_eq!(rejected.input, &values[..len]);
            }
        }
    }

    pub(super) fn inline_transitions(actions: [u8; 5], values: [u8; 5]) {
        let mut buffer = InlineBuf::<u8, 3>::new();
        let mut reference = [0; 3];
        let mut len = 0;
        for step in 0..5 {
            match actions[step] & 3 {
                0 => {
                    let result = buffer.try_push(values[step]);
                    if len == 3 {
                        assert_eq!(result, Err(values[step]));
                    } else {
                        assert_eq!(result, Ok(()));
                        reference[len] = values[step];
                        len += 1;
                    }
                }
                1 => {
                    let popped = buffer.pop();
                    if len == 0 {
                        assert_eq!(popped, None);
                    } else {
                        len -= 1;
                        assert_eq!(popped, Some(reference[len]));
                    }
                }
                2 => {
                    let requested = usize::from(values[step] & 3);
                    buffer.truncate(requested);
                    len = len.min(requested);
                }
                3 => {
                    if len != 0 {
                        buffer.as_mut_slice()[0] = values[step];
                        reference[0] = values[step];
                    }
                }
                _ => unreachable!(),
            }
            assert_eq!(buffer.len(), len);
            assert_eq!(buffer.is_full(), len == 3);
            assert_eq!(buffer.as_slice(), &reference[..len]);
        }
    }

    struct Token<'a> {
        index: usize,
        drops: &'a [Cell<u8>; 3],
    }

    impl Drop for Token<'_> {
        fn drop(&mut self) {
            let slot = &self.drops[self.index];
            assert_eq!(slot.get(), 0, "double destruction");
            slot.set(1);
        }
    }

    pub(super) fn inline_ownership(len: usize, consumed: usize, directions: u8, heap: bool) {
        assert!(len <= 3 && consumed <= 3);
        let drops = [const { Cell::new(0) }; 3];
        let mut buffer = InlineBuf::<_, 3>::new();
        for index in 0..len {
            buffer.push(Token {
                index,
                drops: &drops,
            });
        }
        if heap {
            let values = buffer.take_into_vec(len);
            assert_eq!(buffer.len(), 0);
            assert_eq!(values.len(), len);
            for index in 0..len {
                assert_eq!(values[index].index, index);
            }
            drop(values);
        } else {
            let mut iter = buffer.take_iter();
            assert_eq!(buffer.len(), 0);
            assert_eq!(iter.as_slice().len(), len);
            for step in 0..consumed {
                if directions & (1 << step) == 0 {
                    drop(iter.next());
                } else {
                    drop(iter.next_back());
                }
            }
            drop(iter);
        }
        drop(buffer);
        for (index, count) in drops.iter().enumerate() {
            assert_eq!(count.get(), u8::from(index < len));
        }
    }

    pub(super) fn zero_capacity() {
        let mut buffer = InlineBuf::<u8, 0>::new();
        assert_eq!(buffer.try_push(7), Err(7));
        assert_eq!(buffer.pop(), None);
        assert!(buffer.as_slice().is_empty());
        assert_eq!(buffer.take_iter().next(), None);
        assert!(buffer.take_into_vec(0).is_empty());
    }

    pub(super) fn dense_padding(left_len: usize, right_len: usize, words: [usize; 2]) {
        assert!(left_len <= 6 && right_len <= 6);
        // Bits outside the logical universes are unconstrained, including all
        // high machine-word bits. Expected membership uses a scalar bit oracle.
        let left = FixedBitSet::with_capacity_and_blocks(left_len, [words[0]]).freeze_with(Retain);
        let right =
            FixedBitSet::with_capacity_and_blocks(right_len, [words[1]]).freeze_with(Retain);
        let mut expected: [Vec<usize>; 4] = core::array::from_fn(|_| Vec::new());
        for position in 0..6 {
            let in_left = position < left_len && words[0] & (1 << position) != 0;
            let in_right = position < right_len && words[1] & (1 << position) != 0;
            for (set, present) in expected.iter_mut().zip([
                in_left || in_right,
                in_left && in_right,
                in_left && !in_right,
                in_left != in_right,
            ]) {
                if present {
                    set.push(position);
                }
            }
        }
        let results = [
            left.union(&right),
            left.intersection(&right),
            left.difference(&right),
            left.symmetric_difference(&right),
        ];
        let counts = [
            left.union_len(&right),
            left.intersection_len(&right),
            left.difference_len(&right),
            left.symmetric_difference_len(&right),
        ];
        let lengths = [
            left_len.max(right_len),
            left_len.min(right_len),
            left_len,
            left_len.max(right_len),
        ];
        for (index, result) in results.into_iter().enumerate() {
            assert_eq!(result.universe_len(), lengths[index]);
            assert_eq!(result.ones().collect::<Vec<_>>(), expected[index]);
            assert_eq!(counts[index], expected[index].len());
            assert_eq!(result.cardinality(), expected[index].len() as u64);
            assert!(
                result
                    .into_inner()
                    .ones()
                    .all(|position| position < lengths[index])
            );
        }
        assert_eq!(left.is_subset(&right), expected[2].is_empty());
        assert_eq!(left.is_disjoint(&right), expected[1].is_empty());
    }
}

#[cfg(kani)]
mod proofs {
    use super::checks;

    #[kani::proof]
    #[kani::unwind(16)]
    fn ordered_set_algebra() {
        let left: u8 = kani::any();
        let right: u8 = kani::any();
        kani::assume(left < 16 && right < 16);
        checks::ordered_masks(left, right);
    }

    #[kani::proof]
    #[kani::unwind(16)]
    fn sorted_constructor_preserves_or_rejects_input() {
        let len: usize = kani::any();
        kani::assume(len <= 4);
        checks::sorted_validation(kani::any(), len);
    }

    #[kani::proof]
    #[kani::unwind(16)]
    fn inline_prefix_preserved() {
        checks::inline_transitions(kani::any(), kani::any());
    }

    #[kani::proof]
    #[kani::unwind(16)]
    fn inline_transfer_drops_each_owner_once() {
        let len: usize = kani::any();
        let consumed: usize = kani::any();
        kani::assume(len <= 3 && consumed <= 3);
        checks::inline_ownership(len, consumed, kani::any(), kani::any());
    }

    #[kani::proof]
    #[kani::unwind(4)]
    fn inline_zero_capacity() {
        checks::zero_capacity();
    }

    #[kani::proof]
    #[kani::unwind(32)]
    fn dense_padding_does_not_create_members() {
        let left_len: usize = kani::any();
        let right_len: usize = kani::any();
        kani::assume(left_len <= 6 && right_len <= 6);
        checks::dense_padding(left_len, right_len, kani::any());
    }
}

#[cfg(test)]
mod tests {
    use super::checks;

    #[test]
    fn property_helpers_execute_against_production_code() {
        for left in 0..16 {
            for right in 0..16 {
                checks::ordered_masks(left, right);
            }
        }
        for len in 0..=4 {
            checks::sorted_validation([0, 1, 2, 3], len);
            checks::sorted_validation([1, 0, 1, 0], len);
        }
        for len in 0..=3 {
            for consumed in 0..=3 {
                for directions in 0..8 {
                    checks::inline_ownership(len, consumed, directions, false);
                    checks::inline_ownership(len, consumed, directions, true);
                }
            }
        }
        checks::zero_capacity();
        for mut trace in 0..4_usize.pow(5) {
            let actions = core::array::from_fn(|_| {
                let action = (trace % 4) as u8;
                trace /= 4;
                action
            });
            checks::inline_transitions(actions, [1, 2, 3, 4, 5]);
        }
        for left_len in 0..=6 {
            for right_len in 0..=6 {
                checks::dense_padding(left_len, right_len, [usize::MAX, 0]);
                checks::dense_padding(left_len, right_len, [usize::MAX / 3, !(usize::MAX / 3)]);
            }
        }
    }
}

//! Value iteration for compressed and dense bitsets.
//!
//! Bitset iterators yield member indices by value. They cannot implement
//! reference-based element capabilities because those indices are not stored
//! as addressable elements. Cardinality is separate from allocation size.

use crate::frozen::sealed::Sealed;
use crate::read::{Cardinality, ElementType, SetContains, ValueElements};
use crate::{Eligible, FreezeWith, Frozen, Retain};

#[cfg(any(feature = "roaring-0_12", feature = "int-set"))]
fn union_cardinality(left: u64, right: u64, overlap: u64) -> u64 {
    (left - overlap)
        .checked_add(right)
        .expect("set cardinality exceeds u64::MAX")
}

#[cfg(any(feature = "roaring-0_12", feature = "int-set"))]
fn symmetric_difference_cardinality(left: u64, right: u64, overlap: u64) -> u64 {
    (left - overlap)
        .checked_add(right - overlap)
        .expect("set cardinality exceeds u64::MAX")
}

#[cfg(any(feature = "roaring-0_12", feature = "int-set"))]
macro_rules! roaring_adapter {
    ($bitmap:ident, $element:ty, $iter:ty) => {
        impl Sealed for roaring::$bitmap {}
        impl Eligible for roaring::$bitmap {}

        impl FreezeWith<Retain> for roaring::$bitmap {
            type Output = Frozen<Self>;

            fn freeze_with(self, _: Retain) -> Self::Output {
                Frozen::new_unchecked(self)
            }
        }

        impl Cardinality for roaring::$bitmap {
            fn cardinality(&self) -> u64 {
                self.len()
            }
        }

        impl ElementType for roaring::$bitmap {
            type Item = $element;
        }

        impl ValueElements for roaring::$bitmap {
            type Values<'a> = $iter;

            fn values(&self) -> Self::Values<'_> {
                self.iter()
            }
        }

        impl SetContains<$element> for roaring::$bitmap {
            fn contains(&self, value: &$element) -> bool {
                roaring::$bitmap::contains(self, *value)
            }
        }

        impl crate::write::SetInsert for roaring::$bitmap {
            fn insert(&mut self, value: $element) -> bool {
                roaring::$bitmap::insert(self, value)
            }
        }

        impl crate::write::SetRemove<$element> for roaring::$bitmap {
            fn remove(&mut self, value: &$element) -> bool {
                roaring::$bitmap::remove(self, *value)
            }
        }

        impl Frozen<roaring::$bitmap> {
            /// Returns the number of represented members without narrowing it.
            pub fn cardinality(&self) -> u64 {
                self.backend().len()
            }

            /// Reports whether the specified integer is a member.
            pub fn contains(&self, value: $element) -> bool {
                self.backend().contains(value)
            }

            /// Iterates over members by value, in ascending order.
            pub fn iter(&self) -> <roaring::$bitmap as ValueElements>::Values<'_> {
                self.backend().iter()
            }

            /// Materialize the union using the backend's container operations.
            /// Neither input is changed; the returned bitmap is frozen.
            #[must_use]
            pub fn union(&self, other: &Self) -> Self {
                Self::new_unchecked(self.backend() | other.backend())
            }

            /// Materialize the intersection using the backend's container operations.
            /// Neither input is changed; the returned bitmap is frozen.
            #[must_use]
            pub fn intersection(&self, other: &Self) -> Self {
                Self::new_unchecked(self.backend() & other.backend())
            }

            /// Materialize members of `self` absent from `other` using the
            /// backend's container operations. Neither input is changed.
            #[must_use]
            pub fn difference(&self, other: &Self) -> Self {
                Self::new_unchecked(self.backend() - other.backend())
            }

            /// Materialize members occurring in exactly one input using the
            /// backend's container operations. Neither input is changed.
            #[must_use]
            pub fn symmetric_difference(&self, other: &Self) -> Self {
                Self::new_unchecked(self.backend() ^ other.backend())
            }

            /// Count union members without allocating or materializing a bitmap.
            /// Inputs must have native cardinalities representable by `u64`;
            /// an already unrepresentable native count is not repaired here.
            ///
            /// # Panics
            /// If the result's cardinality exceeds `u64::MAX`. This is distinct
            /// from supporting `u64::MAX` as an individual member value.
            pub fn union_len(&self, other: &Self) -> u64 {
                union_cardinality(
                    self.cardinality(),
                    other.cardinality(),
                    self.intersection_len(other),
                )
            }

            /// Count intersection members without allocating or materializing a bitmap.
            pub fn intersection_len(&self, other: &Self) -> u64 {
                self.backend().intersection_len(other.backend())
            }

            /// Count members of `self` absent from `other` without allocating.
            pub fn difference_len(&self, other: &Self) -> u64 {
                self.backend().difference_len(other.backend())
            }

            /// Count members occurring in exactly one input without allocating.
            /// Inputs must have native cardinalities representable by `u64`;
            /// an already unrepresentable native count is not repaired here.
            ///
            /// # Panics
            /// If the result's cardinality exceeds `u64::MAX`. Overlap is
            /// subtracted before addition to avoid intermediate overflow.
            pub fn symmetric_difference_len(&self, other: &Self) -> u64 {
                symmetric_difference_cardinality(
                    self.cardinality(),
                    other.cardinality(),
                    self.intersection_len(other),
                )
            }

            /// Whether every member of `self` occurs in `other`, without allocating.
            pub fn is_subset(&self, other: &Self) -> bool {
                self.backend().is_subset(other.backend())
            }

            /// Whether every member of `other` occurs in `self`, without allocating.
            pub fn is_superset(&self, other: &Self) -> bool {
                self.backend().is_superset(other.backend())
            }

            /// Whether the inputs have no common member, without allocating.
            pub fn is_disjoint(&self, other: &Self) -> bool {
                self.backend().is_disjoint(other.backend())
            }
        }
    };
}

#[cfg(any(feature = "roaring-0_12", feature = "int-set"))]
roaring_adapter!(RoaringBitmap, u32, roaring::bitmap::Iter<'a>);
#[cfg(any(feature = "roaring-0_12", feature = "int-set"))]
roaring_adapter!(RoaringTreemap, u64, roaring::treemap::Iter<'a>);

#[cfg(feature = "fixedbitset-0_5")]
mod dense {
    use super::{
        Cardinality, ElementType, Eligible, FreezeWith, Frozen, Retain, Sealed, SetContains,
        ValueElements,
    };
    use fixedbitset::FixedBitSet;

    const WORD_BITS: usize = core::mem::size_of::<usize>() * 8;

    /// Read a boundary word, masking native padding and zero-extending beyond
    /// the logical universe. Bulk loops use slices and do not call this helper.
    #[inline]
    fn boundary_word(bits: &FixedBitSet, index: usize) -> usize {
        let full_words = bits.len() / WORD_BITS;
        match index.cmp(&full_words) {
            core::cmp::Ordering::Less => bits.as_slice()[index],
            core::cmp::Ordering::Equal => {
                let tail_bits = bits.len() % WORD_BITS;
                if tail_bits == 0 {
                    0
                } else {
                    bits.as_slice()[index] & ((1_usize << tail_bits) - 1)
                }
            }
            core::cmp::Ordering::Greater => 0,
        }
    }

    /// Combine full shared words in a straight slice loop. At most two boundary
    /// words need padding masks; between them only one source can have members.
    /// Splitting these spans removes per-word zero-extension state from the hot
    /// loop and lets iterator `fold` specialize to independent slice loops.
    fn combined_words<'a, F>(
        left: &'a FixedBitSet,
        right: &'a FixedBitSet,
        universe_len: usize,
        combine: F,
    ) -> impl Iterator<Item = usize> + 'a
    where
        F: Fn(usize, usize) -> usize + Copy + 'a,
    {
        let output_full = universe_len / WORD_BITS;
        let output_words = universe_len.div_ceil(WORD_BITS);
        let shared_full = (left.len() / WORD_BITS)
            .min(right.len() / WORD_BITS)
            .min(output_full);
        let shared = left.as_slice()[..shared_full]
            .iter()
            .zip(&right.as_slice()[..shared_full])
            .map(move |(&left, &right)| combine(left, right));

        // The shorter input can end inside this word. Mask it before combining
        // so dirty native padding cannot become a member of a larger universe.
        let boundary = (shared_full < output_words).then(|| {
            combine(
                boundary_word(left, shared_full),
                boundary_word(right, shared_full),
            )
        });

        let (longer, left_is_longer) = if left.len() >= right.len() {
            (left, true)
        } else {
            (right, false)
        };
        let suffix_start = (shared_full + 1).min(output_full);
        let suffix = longer.as_slice()[suffix_start..output_full]
            .iter()
            .map(move |&word| {
                if left_is_longer {
                    combine(word, 0)
                } else {
                    combine(0, word)
                }
            });
        // If the result ends in the first boundary, it is already emitted.
        let tail =
            (output_full > shared_full && !universe_len.is_multiple_of(WORD_BITS)).then(|| {
                combine(
                    boundary_word(left, output_full),
                    boundary_word(right, output_full),
                )
            });
        shared.chain(boundary).chain(suffix).chain(tail)
    }

    fn count_words(words: impl Iterator<Item = usize>) -> usize {
        // Every input word represents disjoint positions within a logical
        // universe whose length is a usize. Consequently the sum is at most
        // that universe length; per-word checked addition cannot discover an
        // overflow and would obstruct optimized popcount reduction loops.
        words.map(|word| word.count_ones() as usize).sum()
    }

    /// One or two words need no span-iterator setup. The bulk reduction retains
    /// its straight slice loops for larger logical universes.
    fn combined_count<F>(
        left: &FixedBitSet,
        right: &FixedBitSet,
        universe_len: usize,
        combine: F,
    ) -> usize
    where
        F: Fn(usize, usize) -> usize + Copy,
    {
        if universe_len <= 2 * WORD_BITS {
            if universe_len == 0 {
                return 0;
            }
            let first = combine(boundary_word(left, 0), boundary_word(right, 0));
            let count = first.count_ones() as usize;
            return if universe_len <= WORD_BITS {
                count
            } else {
                count
                    + combine(boundary_word(left, 1), boundary_word(right, 1)).count_ones() as usize
            };
        }
        count_words(combined_words(left, right, universe_len, combine))
    }

    fn materialize_combined<F>(
        left: &FixedBitSet,
        right: &FixedBitSet,
        universe_len: usize,
        combine: F,
    ) -> FixedBitSet
    where
        F: Fn(usize, usize) -> usize + Copy,
    {
        if universe_len == 0 {
            return FixedBitSet::with_capacity(0);
        }
        if universe_len <= 2 * WORD_BITS {
            let first = combine(boundary_word(left, 0), boundary_word(right, 0));
            if universe_len <= WORD_BITS {
                return FixedBitSet::with_capacity_and_blocks(universe_len, [first]);
            }
            let second = combine(boundary_word(left, 1), boundary_word(right, 1));
            return FixedBitSet::with_capacity_and_blocks(universe_len, [first, second]);
        }
        FixedBitSet::with_capacity_and_blocks(
            universe_len,
            combined_words(left, right, universe_len, combine),
        )
    }

    fn combined_is_empty<F>(
        left: &FixedBitSet,
        right: &FixedBitSet,
        universe_len: usize,
        combine: F,
    ) -> bool
    where
        F: Fn(usize, usize) -> usize + Copy,
    {
        if universe_len <= 2 * WORD_BITS {
            return universe_len == 0
                || (combine(boundary_word(left, 0), boundary_word(right, 0)) == 0
                    && (universe_len <= WORD_BITS
                        || combine(boundary_word(left, 1), boundary_word(right, 1)) == 0));
        }
        combined_words(left, right, universe_len, combine).all(|word| word == 0)
    }

    /// Iterates over set-bit indices within a dense bitset's logical universe.
    ///
    /// Unlike the native block iterator, this excludes any nonzero padding bits
    /// supplied through block-based construction or mutable block access.
    /// It is created by [`ValueElements::values`] or [`Frozen::ones`].
    #[derive(Clone)]
    pub struct FixedBitSetValues<'a> {
        inner: fixedbitset::Ones<'a>,
        universe_len: usize,
    }

    impl core::fmt::Debug for FixedBitSetValues<'_> {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter
                .debug_struct("FixedBitSetValues")
                .field("universe_len", &self.universe_len)
                .finish_non_exhaustive()
        }
    }

    impl Iterator for FixedBitSetValues<'_> {
        type Item = usize;

        fn next(&mut self) -> Option<usize> {
            // Native iteration is ascending, so all remaining values are also
            // out of bounds after the first padding index is encountered.
            self.inner.next().filter(|&index| index < self.universe_len)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, self.inner.size_hint().1)
        }
    }

    impl DoubleEndedIterator for FixedBitSetValues<'_> {
        fn next_back(&mut self) -> Option<usize> {
            self.inner
                .by_ref()
                .rev()
                .find(|&index| index < self.universe_len)
        }
    }

    impl core::iter::FusedIterator for FixedBitSetValues<'_> {}

    impl Sealed for FixedBitSet {}
    impl Eligible for FixedBitSet {}

    impl FreezeWith<Retain> for FixedBitSet {
        type Output = Frozen<Self>;

        fn freeze_with(self, _: Retain) -> Self::Output {
            Frozen::new_unchecked(self)
        }
    }

    impl Cardinality for FixedBitSet {
        fn cardinality(&self) -> u64 {
            // usize never exceeds u64 on supported Rust targets.
            self.count_ones(..) as u64
        }
    }

    impl ElementType for FixedBitSet {
        type Item = usize;
    }

    impl ValueElements for FixedBitSet {
        type Values<'a> = FixedBitSetValues<'a>;

        fn values(&self) -> Self::Values<'_> {
            FixedBitSetValues {
                inner: self.ones(),
                universe_len: self.len(),
            }
        }
    }

    impl SetContains<usize> for FixedBitSet {
        fn contains(&self, value: &usize) -> bool {
            FixedBitSet::contains(self, *value)
        }
    }

    impl Frozen<FixedBitSet> {
        /// Returns the number of set bits, independent of the universe size.
        pub fn cardinality(&self) -> u64 {
            Cardinality::cardinality(self.backend())
        }

        /// Returns the exclusive upper bound on representable member indices.
        pub fn universe_len(&self) -> usize {
            self.backend().len()
        }

        /// Reports whether an index is set; out-of-universe indices return false.
        pub fn contains(&self, value: usize) -> bool {
            self.backend().contains(value)
        }

        /// Iterates over set-bit indices within the logical universe.
        ///
        /// Indices are yielded by value in ascending order. Padding bits outside
        /// the logical universe are excluded even when the native blocks contain
        /// nonzero padding.
        pub fn ones(&self) -> FixedBitSetValues<'_> {
            ValueElements::values(self.backend())
        }

        /// Materialize the union with the larger input universe.
        ///
        /// Uses word operations, ignores padding outside each input's logical
        /// universe, and clears padding in the result. Inputs remain unchanged.
        #[must_use]
        pub fn union(&self, other: &Self) -> Self {
            let universe_len = self.universe_len().max(other.universe_len());
            Self::new_unchecked(materialize_combined(
                self.backend(),
                other.backend(),
                universe_len,
                |a, b| a | b,
            ))
        }

        /// Materialize the intersection with the smaller input universe.
        ///
        /// Uses word operations, ignores padding outside each input's logical
        /// universe, and clears padding in the result. Inputs remain unchanged.
        #[must_use]
        pub fn intersection(&self, other: &Self) -> Self {
            let universe_len = self.universe_len().min(other.universe_len());
            Self::new_unchecked(materialize_combined(
                self.backend(),
                other.backend(),
                universe_len,
                |a, b| a & b,
            ))
        }

        /// Materialize members of `self` absent from `other`, retaining `self`'s universe.
        ///
        /// Uses word operations, ignores padding outside each input's logical
        /// universe, and clears padding in the result. Inputs remain unchanged.
        #[must_use]
        pub fn difference(&self, other: &Self) -> Self {
            let universe_len = self.universe_len();
            Self::new_unchecked(materialize_combined(
                self.backend(),
                other.backend(),
                universe_len,
                |a, b| a & !b,
            ))
        }

        /// Materialize members occurring in exactly one input, with the larger universe.
        ///
        /// Uses word operations, ignores padding outside each input's logical
        /// universe, and clears padding in the result. Inputs remain unchanged.
        #[must_use]
        pub fn symmetric_difference(&self, other: &Self) -> Self {
            let universe_len = self.universe_len().max(other.universe_len());
            Self::new_unchecked(materialize_combined(
                self.backend(),
                other.backend(),
                universe_len,
                |a, b| a ^ b,
            ))
        }

        /// Count union members with word operations and no allocation.
        /// Padding outside either input's logical universe is ignored.
        pub fn union_len(&self, other: &Self) -> usize {
            combined_count(
                self.backend(),
                other.backend(),
                self.universe_len().max(other.universe_len()),
                |a, b| a | b,
            )
        }

        /// Count intersection members with word operations and no allocation.
        /// Padding outside either input's logical universe is ignored.
        pub fn intersection_len(&self, other: &Self) -> usize {
            combined_count(
                self.backend(),
                other.backend(),
                self.universe_len().min(other.universe_len()),
                |a, b| a & b,
            )
        }

        /// Count members of `self` absent from `other`, without allocating.
        /// Padding outside either input's logical universe is ignored.
        pub fn difference_len(&self, other: &Self) -> usize {
            combined_count(
                self.backend(),
                other.backend(),
                self.universe_len(),
                |a, b| a & !b,
            )
        }

        /// Count members occurring in exactly one input, without allocating.
        /// Padding outside either input's logical universe is ignored.
        pub fn symmetric_difference_len(&self, other: &Self) -> usize {
            combined_count(
                self.backend(),
                other.backend(),
                self.universe_len().max(other.universe_len()),
                |a, b| a ^ b,
            )
        }

        /// Whether every logical member of `self` occurs in `other`.
        /// Uses word operations, stops at the first mismatch, and allocates nothing.
        pub fn is_subset(&self, other: &Self) -> bool {
            combined_is_empty(
                self.backend(),
                other.backend(),
                self.universe_len(),
                |a, b| a & !b,
            )
        }

        /// Whether every logical member of `other` occurs in `self`.
        /// Uses word operations, stops at the first mismatch, and allocates nothing.
        pub fn is_superset(&self, other: &Self) -> bool {
            other.is_subset(self)
        }

        /// Whether the inputs have no logical member in common.
        /// Uses word operations, stops at the first common member, and allocates nothing.
        pub fn is_disjoint(&self, other: &Self) -> bool {
            combined_is_empty(
                self.backend(),
                other.backend(),
                self.universe_len().min(other.universe_len()),
                |a, b| a & b,
            )
        }
    }
}

#[cfg(feature = "fixedbitset-0_5")]
pub use dense::FixedBitSetValues;

#[cfg(all(test, any(feature = "roaring-0_12", feature = "int-set")))]
mod cardinality_tests {
    use super::{symmetric_difference_cardinality, union_cardinality};

    #[test]
    fn overlap_prevents_spurious_intermediate_overflow() {
        assert_eq!(union_cardinality(u64::MAX, u64::MAX, u64::MAX), u64::MAX);
        assert_eq!(union_cardinality(u64::MAX, 3, 3), u64::MAX);
        assert_eq!(
            symmetric_difference_cardinality(u64::MAX, u64::MAX, u64::MAX),
            0,
        );
        assert_eq!(
            symmetric_difference_cardinality(u64::MAX, u64::MAX, u64::MAX - 1),
            2,
        );
    }

    #[test]
    #[should_panic(expected = "set cardinality exceeds u64::MAX")]
    fn union_rejects_actual_cardinality_overflow() {
        union_cardinality(u64::MAX, 1, 0);
    }

    #[test]
    #[should_panic(expected = "set cardinality exceeds u64::MAX")]
    fn symmetric_difference_rejects_actual_cardinality_overflow() {
        symmetric_difference_cardinality(u64::MAX, 1, 0);
    }
}

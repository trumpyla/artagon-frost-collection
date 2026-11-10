//! Width-specific native operations. No backend type escapes the public API.

use alloc::vec::Vec;
use core::{fmt, hash::Hash, ops::ControlFlow};

use super::Operation;

/// Probe stride for run detection when a Roaring value is built from sorted
/// codes: runs of at least `2 * RANGE_RUN - 1` consecutive codes are always
/// inserted as one range, and shorter runs down to `RANGE_RUN` are when the
/// probe lands inside them. The value is a structural allowance (two endpoints
/// against `RANGE_RUN` pushes), not a measured machine crossover; see
/// `docs/integer-benchmarks.md` for the workloads it was checked against.
const RANGE_RUN: usize = 32;

/// The only implementations are unsigned primitive widths. Codec laws never
/// justify unchecked indexing, layout casts, or construction of Rust values.
pub(crate) trait Word: Copy + Ord + Hash + fmt::Debug + 'static {
    type Roaring: Clone + Eq + Send + Sync;
    type RoaringIter<'a>: Iterator<Item = Self> + Clone
    where
        Self: 'a;

    fn to_u64(self) -> u64;
    fn try_from_u64(value: u64) -> Option<Self>;
    fn from_u64(value: u64) -> Self {
        Self::try_from_u64(value).expect("code fits the stored word width")
    }
    fn roaring_from_sorted(words: &[Self]) -> Self::Roaring;
    fn roaring_from_ascending(codes: impl Iterator<Item = Self>) -> Self::Roaring;
    fn roaring_containers(value: &Self::Roaring) -> u64;
    fn roaring_from_dense(base: u64, words: &[u64]) -> Self::Roaring;
    fn roaring_iter(value: &Self::Roaring) -> Self::RoaringIter<'_>;
    fn roaring_visit_runs(value: &Self::Roaring, visit: impl FnMut(u64, u64));
    fn roaring_try_visit_runs<B>(
        value: &Self::Roaring,
        visit: impl FnMut(u64, u64) -> ControlFlow<B>,
    ) -> ControlFlow<B>;
    fn roaring_estimate(value: &Self::Roaring) -> (u64, u128);
    fn roaring_len(value: &Self::Roaring) -> u64;
    fn roaring_bounds(value: &Self::Roaring) -> Option<(Self, Self)>;
    fn roaring_contains(value: &Self::Roaring, member: Self) -> bool;
    fn roaring_insert(value: &mut Self::Roaring, member: Self) -> bool;
    fn roaring_remove(value: &mut Self::Roaring, member: Self) -> bool;
    fn roaring_intersection_len(left: &Self::Roaring, right: &Self::Roaring) -> u64;
    fn roaring_is_subset(left: &Self::Roaring, right: &Self::Roaring) -> bool;
    fn roaring_is_disjoint(left: &Self::Roaring, right: &Self::Roaring) -> bool;
    fn roaring_binary(left: &Self::Roaring, right: &Self::Roaring, op: Operation) -> Self::Roaring;
}

macro_rules! word {
    ($word:ty, $backend:ty, $iterator:ty, $dense:ident, $runs:ident, $try_runs:ident, $estimate:ident, $containers:ident) => {
        impl Word for $word {
            type Roaring = $backend;
            type RoaringIter<'a> = $iterator;

            fn to_u64(self) -> u64 {
                u64::from(self)
            }
            fn try_from_u64(value: u64) -> Option<Self> {
                Self::try_from(value).ok()
            }
            fn roaring_from_sorted(words: &[Self]) -> Self::Roaring {
                // Sparse stretches are appended in bulk; long runs are inserted
                // as ranges, so a run costs its endpoints rather than one push
                // per member before `optimize` rediscovers the run container.
                // Runs are detected with one comparison per `RANGE_RUN` codes:
                // strictly ascending codes `RANGE_RUN - 1` apart are consecutive
                // exactly when their difference is `RANGE_RUN - 1`. A run shorter
                // than `2 * RANGE_RUN - 1` may be missed depending on alignment;
                // that only changes how the identical value is built, never
                // which value, and keeps run-free inputs at bulk-append speed.
                let mut result = <$backend>::new();
                let mut segment = 0;
                let mut probe = 0;
                while probe + RANGE_RUN <= words.len() {
                    let last = probe + RANGE_RUN - 1;
                    if (words[last] - words[probe]).to_u64() != RANGE_RUN as u64 - 1 {
                        probe += RANGE_RUN;
                        continue;
                    }
                    let mut start = probe;
                    while start > segment && words[start - 1] + 1 == words[start] {
                        start -= 1;
                    }
                    let mut end = last;
                    while end + 1 < words.len() && words[end].checked_add(1) == Some(words[end + 1])
                    {
                        end += 1;
                    }
                    result
                        .append(words[segment..start].iter().copied())
                        .expect("internal codes are strictly ascending and unique");
                    result.insert_range(words[start]..=words[end]);
                    segment = end + 1;
                    probe = segment;
                }
                result
                    .append(words[segment..].iter().copied())
                    .expect("internal codes are strictly ascending and unique");
                result.optimize();
                result
            }
            fn roaring_from_ascending(codes: impl Iterator<Item = Self>) -> Self::Roaring {
                let mut result = <$backend>::from_sorted_iter(codes)
                    .expect("internal codes are strictly ascending and unique");
                result.optimize();
                result
            }
            fn roaring_containers(value: &Self::Roaring) -> u64 {
                $containers(value)
            }
            fn roaring_from_dense(base: u64, words: &[u64]) -> Self::Roaring {
                $dense(base, words)
            }
            fn roaring_iter(value: &Self::Roaring) -> Self::RoaringIter<'_> {
                value.iter()
            }
            fn roaring_visit_runs(value: &Self::Roaring, visit: impl FnMut(u64, u64)) {
                $runs(value, visit);
            }
            fn roaring_try_visit_runs<B>(
                value: &Self::Roaring,
                visit: impl FnMut(u64, u64) -> ControlFlow<B>,
            ) -> ControlFlow<B> {
                $try_runs(value, visit)
            }
            fn roaring_estimate(value: &Self::Roaring) -> (u64, u128) {
                $estimate(value)
            }
            fn roaring_len(value: &Self::Roaring) -> u64 {
                value.len()
            }
            fn roaring_bounds(value: &Self::Roaring) -> Option<(Self, Self)> {
                Some((value.min()?, value.max()?))
            }
            fn roaring_contains(value: &Self::Roaring, member: Self) -> bool {
                value.contains(member)
            }
            fn roaring_insert(value: &mut Self::Roaring, member: Self) -> bool {
                value.insert(member)
            }
            fn roaring_remove(value: &mut Self::Roaring, member: Self) -> bool {
                value.remove(member)
            }
            fn roaring_intersection_len(left: &Self::Roaring, right: &Self::Roaring) -> u64 {
                left.intersection_len(right)
            }
            fn roaring_is_subset(left: &Self::Roaring, right: &Self::Roaring) -> bool {
                left.is_subset(right)
            }
            fn roaring_is_disjoint(left: &Self::Roaring, right: &Self::Roaring) -> bool {
                left.is_disjoint(right)
            }
            fn roaring_binary(
                left: &Self::Roaring,
                right: &Self::Roaring,
                op: Operation,
            ) -> Self::Roaring {
                // Optimize a newly computed native binary result once. Tiny
                // clone-and-edit paths deliberately bypass this full scan, as
                // they did in the reviewed main implementation.
                let mut result = match op {
                    Operation::Union => left | right,
                    Operation::Intersection => left & right,
                    Operation::Difference => left - right,
                    Operation::SymmetricDifference => left ^ right,
                };
                result.optimize();
                result
            }
        }
    };
}

word!(
    u32,
    roaring::RoaringBitmap,
    roaring::bitmap::Iter<'a>,
    dense32,
    runs32,
    try_runs32,
    estimate32,
    containers32
);
word!(
    u64,
    roaring::RoaringTreemap,
    roaring::treemap::Iter<'a>,
    dense64,
    runs64,
    try_runs64,
    estimate64,
    containers64
);

fn containers32(value: &roaring::RoaringBitmap) -> u64 {
    u64::from(value.statistics().n_containers)
}

fn containers64(value: &roaring::RoaringTreemap) -> u64 {
    value
        .bitmaps()
        .map(|(_, bitmap)| containers32(bitmap))
        .sum()
}

fn runs32(value: &roaring::RoaringBitmap, mut visit: impl FnMut(u64, u64)) {
    let mut iter = value.iter();
    while let Some(run) = iter.next_range() {
        visit(u64::from(*run.start()), u64::from(*run.end()));
    }
}

fn runs64(value: &roaring::RoaringTreemap, mut visit: impl FnMut(u64, u64)) {
    for (prefix, bitmap) in value.bitmaps() {
        let base = u64::from(prefix) << 32;
        runs32(bitmap, |start, end| visit(base | start, base | end));
    }
}

/// Offer the run start before finding its end. Native `next_range` can traverse
/// a long run across containers; a bounded scout must be able to stop before
/// that traversal when the next start already proves there are too many runs.
fn try_runs32<B>(
    value: &roaring::RoaringBitmap,
    mut visit: impl FnMut(u64, u64) -> ControlFlow<B>,
) -> ControlFlow<B> {
    let mut iter = value.iter();
    while let Some(start) = iter.clone().next() {
        visit(u64::from(start), u64::from(start))?;
        let range = iter
            .next_range()
            .expect("peeked native iterator is nonempty");
        let end = *range.end();
        if start < end {
            visit(u64::from(start) + 1, u64::from(end))?;
        }
    }
    ControlFlow::Continue(())
}

fn try_runs64<B>(
    value: &roaring::RoaringTreemap,
    mut visit: impl FnMut(u64, u64) -> ControlFlow<B>,
) -> ControlFlow<B> {
    for (prefix, bitmap) in value.bitmaps() {
        let base = u64::from(prefix) << 32;
        try_runs32(bitmap, |start, end| visit(base | start, base | end))?;
    }
    ControlFlow::Continue(())
}

fn bitmap_payload(value: &roaring::RoaringBitmap) -> u128 {
    let stats = value.statistics();
    // This is the same deterministic container proxy used by the planner, not
    // a claim about allocator metadata or retained Vec capacity.
    u128::from(stats.n_containers) * 32
        + u128::from(stats.n_values_array_containers) * 2
        + u128::from(stats.n_bitset_containers) * 8192
        + u128::from(stats.n_bytes_run_containers)
}

fn estimate32(value: &roaring::RoaringBitmap) -> (u64, u128) {
    (u64::from(!value.is_empty()), 32 + bitmap_payload(value))
}

fn estimate64(value: &roaring::RoaringTreemap) -> (u64, u128) {
    value
        .bitmaps()
        .fold((0, 32), |(partitions, score), (_, bitmap)| {
            (partitions + 1, score + 80 + bitmap_payload(bitmap))
        })
}

fn dense32(base: u64, words: &[u64]) -> roaring::RoaringBitmap {
    let mut bitmaps = dense_bitmaps::<{ 1024 * 1024 }>(base, words).into_iter();
    let result = bitmaps
        .next()
        .map_or_else(roaring::RoaringBitmap::new, |(prefix, bitmap)| {
            debug_assert_eq!(prefix, 0);
            bitmap
        });
    debug_assert!(bitmaps.next().is_none());
    result
}

fn dense64(base: u64, words: &[u64]) -> roaring::RoaringTreemap {
    roaring::RoaringTreemap::from_bitmaps(dense_bitmaps::<{ 1024 * 1024 }>(base, words))
}

/// Safe, endian-independent dense promotion with bounded scratch storage. A
/// chunk never crosses an upper-32-bit partition. It also stays below 2^32 bits:
/// roaring 0.12's byte import rejects that exact full-width extent even at zero.
fn dense_bitmaps<const CHUNK_WORDS: usize>(
    base: u64,
    words: &[u64],
) -> Vec<(u32, roaring::RoaringBitmap)> {
    // The const parameter allows boundary tests to exercise identical chunking
    // with a small buffer. Production uses at most 8 MiB of byte staging.
    const {
        assert!(CHUNK_WORDS > 0 && CHUNK_WORDS <= 1024 * 1024);
    }
    let mut result = Vec::new();
    let mut current: Option<(u32, roaring::RoaringBitmap)> = None;
    let mut index = 0;
    while index < words.len() {
        let start = base + index as u64 * 64;
        let prefix = (start >> 32) as u32;
        let offset = u32::try_from(start & u64::from(u32::MAX)).expect("masked 32-bit offset");
        let until_partition_end = ((1_u64 << 32) - u64::from(offset)) / 64;
        let count = (words.len() - index)
            .min(CHUNK_WORDS)
            .min(usize::try_from(until_partition_end).unwrap_or(usize::MAX));
        let chunk = &words[index..index + count];
        if chunk.iter().any(|&word| word != 0) {
            if current
                .as_ref()
                .is_some_and(|(active, _)| *active != prefix)
            {
                let (prefix, mut bitmap) = current.take().expect("active partition");
                bitmap.optimize();
                result.push((prefix, bitmap));
            }
            let (_, bitmap) =
                current.get_or_insert_with(|| (prefix, roaring::RoaringBitmap::new()));
            let mut bytes = Vec::with_capacity(count * 8);
            for word in chunk {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            *bitmap |= roaring::RoaringBitmap::from_lsb0_bytes(offset, &bytes);
        }
        index += count;
    }
    if let Some((prefix, mut bitmap)) = current {
        bitmap.optimize();
        result.push((prefix, bitmap));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn dense_byte_promotion_preserves_lsb_positions_and_width_ends() {
        let words = [0x8000_0080_0000_0101, 0x0100_0000_0000_0080];
        let expected: Vec<_> = words
            .iter()
            .enumerate()
            .flat_map(|(index, word)| {
                (0..64).filter_map(move |bit| {
                    (word & (1_u64 << bit) != 0).then_some(64 + index as u64 * 64 + bit)
                })
            })
            .collect();
        assert_eq!(dense64(64, &words).iter().collect::<Vec<_>>(), expected);
        assert_eq!(
            dense32(64, &words)
                .iter()
                .map(u64::from)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(dense32(u64::from(u32::MAX) - 63, &[u64::MAX]).len(), 64);
        let at_end = dense64(u64::MAX - 63, &[1 | (1 << 63)]);
        assert_eq!(at_end.iter().collect::<Vec<_>>(), [u64::MAX - 63, u64::MAX]);
    }

    #[test]
    fn dense_promotion_chunks_without_losing_partition_boundaries() {
        let base = (1_u64 << 32) - 128;
        let words = [1, 1 << 63, 0, 1 << 31, 1];
        let result = roaring::RoaringTreemap::from_bitmaps(dense_bitmaps::<1>(base, &words));
        assert_eq!(
            result.iter().collect::<Vec<_>>(),
            vec![base, (1 << 32) - 1, (1 << 32) + 95, (1 << 32) + 128]
        );
        assert_eq!(result.bitmaps().count(), 2);
        assert_eq!(dense_bitmaps::<2>(base, &[0; 5]), []);
    }

    /// The run-aware builder must agree with the straightforward sorted import
    /// for runs shorter and longer than the range threshold, runs touching
    /// container and partition boundaries, and runs ending at the width maximum.
    #[test]
    fn sorted_roaring_builder_matches_sorted_import_across_run_shapes() {
        let run = u32::try_from(RANGE_RUN).expect("small constant");
        let mut codes64: Vec<u64> = Vec::new();
        codes64.extend(0..u64::from(run) - 1); // one short of a range
        codes64.extend(1_000..1_000 + u64::from(run)); // exactly a range
        codes64.extend(5_003..5_003 + 2 * u64::from(run) - 1); // always detected, unaligned
        codes64.extend((9_000..12_000).step_by(2)); // long run-free stretch between runs
        codes64.extend(65_500..65_700); // crosses a container boundary
        codes64.extend((1 << 32) - 40..(1 << 32) + 40); // crosses a partition
        codes64.extend([1 << 40, (1 << 40) + 2, (1 << 40) + 4]);
        codes64.extend(u64::MAX - 100..=u64::MAX); // inclusive run at the top
        let built = <u64 as Word>::roaring_from_sorted(&codes64);
        let imported = roaring::RoaringTreemap::from_sorted_iter(codes64.iter().copied()).unwrap();
        assert_eq!(built, imported);
        assert_eq!(built.len(), codes64.len() as u64);

        let codes32: Vec<u32> = [0_u32, 1, 2]
            .into_iter()
            .chain(100..100 + run)
            .chain(u32::MAX - run..=u32::MAX)
            .collect();
        let built = <u32 as Word>::roaring_from_sorted(&codes32);
        let imported = roaring::RoaringBitmap::from_sorted_iter(codes32.iter().copied()).unwrap();
        assert_eq!(built, imported);
        assert_eq!(
            <u32 as Word>::roaring_from_sorted(&[]),
            roaring::RoaringBitmap::new()
        );
        assert_eq!(
            <u32 as Word>::roaring_from_sorted(&[7])
                .iter()
                .collect::<Vec<_>>(),
            [7]
        );
    }

    /// Every run length around the probe stride at every alignment relative to
    /// the probe grid, between sparse neighbours. A run the probe misses is
    /// appended member by member; the built value must be the same.
    #[test]
    fn sorted_roaring_builder_is_independent_of_run_alignment() {
        for length in [1_u32, 2, 31, 32, 33, 62, 63, 64, 65, 100] {
            for lead in 0..2 * u32::try_from(RANGE_RUN).expect("small constant") {
                // `lead` sparse codes, the run, then a sparse tail.
                let mut codes: Vec<u32> = (0..lead).map(|index| index * 3).collect();
                let start = lead * 3 + 10;
                codes.extend(start..start + length);
                codes.extend((0..40).map(|index| start + length + 5 + index * 2));
                let built = <u32 as Word>::roaring_from_sorted(&codes);
                assert!(
                    built.iter().eq(codes.iter().copied()),
                    "run of {length} after {lead} sparse codes"
                );
                assert_eq!(built.len(), codes.len() as u64);
            }
        }
    }
}

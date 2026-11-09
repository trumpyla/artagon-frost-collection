//! Borrowed frozen set algebra and direct bitmap kernels.
//!
//! Run with `cargo run --example set_operations --all-features`.

use frost::prelude::*;

fn main() {
    let left: FrozenOrderedSet<_> = [1, 2, 4, 7].into_iter().collect();
    let right: FrozenOrderedSet<_> = [2, 3, 4, 8].into_iter().collect();

    // Lazy borrowed results need no element clones or intermediate set.
    assert_eq!(
        left.intersection(&right).copied().collect::<Vec<_>>(),
        [2, 4]
    );
    assert_eq!(
        left.union(&right).copied().collect::<Vec<_>>(),
        [1, 2, 3, 4, 7, 8]
    );
    assert_eq!(left.difference(&right).copied().collect::<Vec<_>>(), [1, 7]);
    assert_eq!(
        left.symmetric_difference(&right)
            .copied()
            .collect::<Vec<_>>(),
        [1, 3, 7, 8]
    );
    assert_eq!(left.intersection_len(&right), 2);
    assert_eq!(left.union_len(&right), 6);
    assert_eq!(left.difference_len(&right), 2);
    assert_eq!(left.symmetric_difference_len(&right), 4);
    assert!(!left.is_disjoint(&right));

    // Materialize only when the result needs its own ownership.
    let common: FrozenOrderedSet<_> = left.intersection(&right).copied().collect();
    assert!(common.is_subset(&left));
    assert!(left.is_superset(&common));

    #[cfg(feature = "std")]
    {
        let allowed: FrozenHashSet<_> = ["read", "write", "execute"].into_iter().collect();
        let requested: FrozenHashSet<_> = ["read", "execute"].into_iter().collect();
        assert!(requested.is_subset(&allowed));
        assert_eq!(allowed.intersection_len(&requested), 2);
        let missing: FrozenHashSet<_> = requested.difference(&allowed).copied().collect();
        assert!(missing.is_empty());
    }

    #[cfg(feature = "roaring-0_12")]
    {
        use frost::backend::roaring::{RoaringBitmap, RoaringTreemap};

        let even = (0..100_u32).step_by(2).collect::<RoaringBitmap>().seal();
        let thirds = (0..100_u32).step_by(3).collect::<RoaringBitmap>().seal();
        // Counts use the bitmap kernel without allocating a result bitmap.
        assert_eq!(even.intersection_len(&thirds), 17);
        let common = even.intersection(&thirds);
        assert_eq!(common.cardinality(), 17);
        assert!(common.is_subset(&even));

        let high = [1, u64::MAX].into_iter().collect::<RoaringTreemap>().seal();
        let last = [u64::MAX].into_iter().collect::<RoaringTreemap>().seal();
        assert_eq!(high.difference(&last).iter().collect::<Vec<_>>(), [1]);
    }

    #[cfg(feature = "fixedbitset-0_5")]
    {
        use frost::backend::fixedbitset::FixedBitSet;

        let mut short = FixedBitSet::with_capacity(10);
        short.extend([1, 5]);
        let mut long = FixedBitSet::with_capacity(70);
        long.extend([5, 69]);
        let short = short.seal();
        let long = long.seal();
        let combined = short.union(&long);
        assert_eq!(combined.universe_len(), 70);
        assert_eq!(combined.ones().collect::<Vec<_>>(), [1, 5, 69]);
        let common = short.intersection(&long);
        assert_eq!(common.universe_len(), 10);
        assert_eq!(common.ones().collect::<Vec<_>>(), [5]);
        assert_eq!(short.symmetric_difference_len(&long), 2);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn executes_set_operations_example() {
        super::main();
    }
}

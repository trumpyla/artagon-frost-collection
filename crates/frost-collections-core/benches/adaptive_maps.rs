//! Policy-sensitive destructive operations on adaptive ordered maps.
//!
//! Run `cargo bench --bench adaptive_maps --features std --locked`.
//! Map construction, incoming batch construction, and remaining fixture drop
//! are excluded. Drain/extend work (including internal moves and allocation)
//! is included. Large custom thresholds are deliberate stress cases, not a
//! recommendation for the default policy.

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

use frost_collections_test_support::bench as support;

use frost::adaptive::{AdaptiveOrdered, AdaptiveOrderedMap};
use std::collections::BTreeMap;

fn main() {
    support::header();
    // Destructive operations are timed individually to exclude setup. These
    // sizes keep the measured work well above the timer's per-call overhead.
    for size in [4096_usize, 16_384] {
        let end = support::as_u64(size);
        let checksum = end * (end - 1) / 2;
        support::measure_with_setup(
            &format!("adaptive.sorted_threshold_{size}.pop_first_drain"),
            checksum,
            || {
                let mut map = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(size));
                map.extend((0..end).map(|key| (key, key)));
                map
            },
            |map| {
                let mut sum = 0;
                while let Some((key, _)) = map.pop_first() {
                    sum += key;
                }
                sum
            },
        );
        support::measure_with_setup(
            &format!("btree.{size}.pop_first_drain"),
            checksum,
            || (0..end).map(|key| (key, key)).collect::<BTreeMap<_, _>>(),
            |map| {
                let mut sum = 0;
                while let Some((key, _)) = map.pop_first() {
                    sum += key;
                }
                sum
            },
        );
        support::measure_with_setup(
            &format!(
                "adaptive.sorted_threshold_{}.extend_interleaved_{size}",
                2 * size
            ),
            2 * end,
            || {
                let mut map =
                    AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(2 * size));
                map.extend((0..end).map(|key| (2 * key, key)));
                let batch = (0..end).map(|key| (2 * key + 1, key)).collect::<Vec<_>>();
                (map, batch)
            },
            |(map, batch)| {
                map.extend(batch.drain(..));
                support::as_u64(map.len())
            },
        );
        support::measure_with_setup(
            &format!("btree.{size}.extend_interleaved"),
            2 * end,
            || {
                let map = (0..end)
                    .map(|key| (2 * key, key))
                    .collect::<BTreeMap<_, _>>();
                let batch = (0..end).map(|key| (2 * key + 1, key)).collect::<Vec<_>>();
                (map, batch)
            },
            |(map, batch)| {
                map.extend(batch.drain(..));
                support::as_u64(map.len())
            },
        );
    }
}

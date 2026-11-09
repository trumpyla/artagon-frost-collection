//! Candidate-only exact contiguous payload observations; no timing.
//!
//! Some(n) is the actual Sorted/Dense buffer size. None for Roaring means this
//! API does not report its discontiguous allocation size; it never means zero.

use frost::integer::{FrozenIntSet, IntSetPolicy, IntSetRepr};

fn set(values: impl IntoIterator<Item = u64>, policy: IntSetPolicy) -> FrozenIntSet<u64> {
    FrozenIntSet::try_from_iter(values, policy).unwrap()
}

fn report(
    name: &str,
    value: &FrozenIntSet<u64>,
    expected_representation: IntSetRepr,
    expected_cardinality: u64,
    expected_payload: Option<usize>,
) {
    assert_eq!(value.representation(), expected_representation);
    assert_eq!(value.cardinality(), expected_cardinality);
    assert_eq!(value.contiguous_payload_bytes(), expected_payload);
    println!(
        "{name},{:?},{},{:?},{},{}",
        value.representation(),
        value.cardinality(),
        value.contiguous_payload_bytes(),
        value.limits().max_dense_bytes(),
        value.limits().max_roaring_partitions(),
    );
}

fn main() {
    println!(
        "case,representation,cardinality,contiguous_payload_bytes_option,dense_budget_bytes,roaring_partition_limit"
    );
    let dense = set((0_u64..2_000_000).step_by(2), IntSetPolicy::dense());
    report(
        "million_even_input",
        &dense,
        IntSetRepr::Dense,
        1_000_000,
        Some(250_000),
    );
    let union = dense.union(&set([1], IntSetPolicy::roaring()));
    assert!(
        union
            .iter()
            .eq((0..2_000_000).filter(|value| value % 2 == 0 || *value == 1))
    );
    report(
        "million_even_union_singleton",
        &union,
        IntSetRepr::Dense,
        1_000_001,
        Some(250_000),
    );
    let mut updated = dense;
    for value in (1_u64..64).step_by(2) {
        updated = updated.union(&set([value], IntSetPolicy::roaring()));
    }
    assert!(
        updated
            .iter()
            .eq((0..2_000_000).filter(|value| value % 2 == 0 || *value < 64))
    );
    report(
        "million_even_after_32_updates",
        &updated,
        IntSetRepr::Dense,
        1_000_032,
        Some(250_000),
    );

    let strict = IntSetPolicy::dense().with_dense_budget(8);
    let limited = set([0], strict).union(&set([1 << 20], strict));
    assert!(limited.iter().eq([0, 1 << 20]));
    assert_eq!(limited.limits().max_dense_bytes(), 8);
    report(
        "eight_byte_dense_budget_union",
        &limited,
        IntSetRepr::Sorted,
        2,
        Some(16),
    );

    const LENGTH: u64 = 1 << 22;
    const GAP: u64 = 1 << 27;
    let left = set(0..LENGTH, IntSetPolicy::dense());
    let right = set(GAP..GAP + LENGTH, IntSetPolicy::dense());
    report(
        "distant_dense_left",
        &left,
        IntSetRepr::Dense,
        LENGTH,
        Some(524_288),
    );
    report(
        "distant_dense_right",
        &right,
        IntSetRepr::Dense,
        LENGTH,
        Some(524_288),
    );
    let combined = left.union(&right);
    assert!(combined.iter().eq((0..LENGTH).chain(GAP..GAP + LENGTH)));
    report(
        "distant_dense_union",
        &combined,
        IntSetRepr::Roaring,
        2 * LENGTH,
        None,
    );
}

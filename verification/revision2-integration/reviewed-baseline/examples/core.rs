//! Core-only frozen arrays, including an explicit consuming return to mutation.
use frost::prelude::*;

fn main() {
    let frozen = [3, 1, 2].seal();
    assert_eq!(frozen.get(1), Some(&1));
    let mut values = frozen.into_inner();
    values.sort_unstable();
    assert_eq!(values, [1, 2, 3]);
}

#[test]
fn example() {
    main();
}

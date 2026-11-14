//! Preserve inline storage or explicitly compact it to a frozen slice.
use frost::backend::{arrayvec::ArrayVec, smallvec::SmallVec};
use frost::prelude::*;
use frost::write::TryPush;

fn main() {
    let mut fixed = ArrayVec::<u32, 2>::new();
    TryPush::try_push(&mut fixed, 2).unwrap();
    TryPush::try_push(&mut fixed, 3).unwrap();
    assert_eq!(TryPush::try_push(&mut fixed, 5).unwrap_err().value, 5);
    let frozen = fixed.seal();
    assert_eq!(frozen.iter().sum::<u32>(), 5);

    let mut small = SmallVec::<[u32; 4]>::new();
    small.extend([2, 3]);
    let frozen = small.seal();
    assert!(!frozen.spilled());
    let compact = frozen.into_inner().compact();
    assert_eq!(compact.as_ref(), &[2, 3]);
}

#[test]
fn example() {
    main();
}

//! Typed integer codes, explicit plans, and representation-independent algebra.
//!
//! Run with `cargo run --example integer_sets --features int-set`.

use frost::TryFreezeWith;
use frost::integer::{FrozenIntSet, IntKey, IntSetPolicy};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct UserId(u64);

impl IntKey for UserId {
    type Word = u64;

    fn encode(self) -> u64 {
        self.0
    }

    fn decode(word: u64) -> Option<Self> {
        Some(Self(word))
    }
}

fn main() {
    let signed = vec![7_i32, -9, 0, -9]
        .try_freeze_with(IntSetPolicy::auto())
        .unwrap();
    assert_eq!(signed.iter().collect::<Vec<_>>(), [-9, 0, 7]);

    let members = FrozenIntSet::try_from_iter(
        [UserId(1_000_000_000_001), UserId(1_000_000_000_003)],
        IntSetPolicy::dense(),
    )
    .unwrap();
    let permitted = FrozenIntSet::try_from_iter(
        [UserId(1_000_000_000_002), UserId(1_000_000_000_003)],
        IntSetPolicy::roaring(),
    )
    .unwrap();
    assert_eq!(members.intersection_len(&permitted), 1);
    assert_eq!(
        members.intersection(&permitted).iter().collect::<Vec<_>>(),
        [UserId(1_000_000_000_003)]
    );
    assert_eq!(members.union(&permitted).cardinality(), 3);

    // A valid universe specifies which domain values may be introduced.
    let missing = permitted.difference(&members);
    assert_eq!(
        missing.iter().collect::<Vec<_>>(),
        [UserId(1_000_000_000_002)]
    );
    println!("signed members: {:?}", signed.iter().collect::<Vec<_>>());
    println!("selected integer storage: {:?}", members.representation());

    // Storage is shared: clones increment a reference count in O(1). A result
    // that equals an operand reuses it when its result policy permits that
    // representation. Chunked storage shares untouched containers between a
    // snapshot and the versions derived from it.
    let snapshot: FrozenIntSet<u32> = FrozenIntSet::try_from_iter(
        (0..8).flat_map(|chunk| (0..100).map(move |i| chunk * 65_536 + i * 7)),
        IntSetPolicy::chunked(),
    )
    .unwrap();
    let copy = snapshot.clone();
    assert!(copy.shares_storage_with(&snapshot));
    let next = snapshot.union(&[65_536 * 3 + 1].into_iter().collect());
    assert_eq!(next.chunk_count(), 8);
    assert_eq!(next.shared_chunks(&snapshot), 7);
    assert!(snapshot.intersection(&next).shares_storage_with(&snapshot));
    println!(
        "snapshot update shares {} of {} containers",
        next.shared_chunks(&snapshot),
        next.chunk_count()
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn integer_example_executes() {
        super::main();
    }
}

//! Run with `cargo run --example snapshots --features arc-swap-1`.

use std::sync::{Arc, Barrier};

use frost::concurrent::SnapshotStore;
use frost::family::FrozenVec;

fn main() {
    let store = Arc::new(SnapshotStore::new(FrozenVec::from(["red", "green"])));
    let original = store.load();
    let barrier = Arc::new(Barrier::new(2));

    let writer = {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            store.publish(FrozenVec::from(["blue", "gold"]))
        })
    };
    barrier.wait();
    let replaced = writer.join().expect("snapshot publisher panicked");

    // Holding one Arc keeps a consistent version across multiple reads,
    // even after another thread has published a replacement.
    assert!(Arc::ptr_eq(&original, &replaced));
    assert_eq!(original.as_slice(), &["red", "green"]);
    // Scoped queries use a temporary guard. Keep a guard for several short
    // related queries, or convert it into an owned Arc for longer retention.
    assert_eq!(store.peek(|version| version.as_slice().len()), 2);
    let current = store.load_guard();
    assert_eq!(current.as_slice(), &["blue", "gold"]);
    println!("Original colors: {:?}", original.as_slice());
    println!("Current colors: {:?}", current.as_slice());

    // Updates retry if another writer publishes between deriving and
    // comparing. Keep this callback retry-safe; it can execute more than once.
    let counter = SnapshotStore::new(FrozenVec::from([0_usize]));
    let previous = counter.update(|version| FrozenVec::from([version.as_slice()[0] + 1]));
    assert_eq!(previous.as_slice(), &[0]);
    assert_eq!(counter.peek(|version| version.as_slice()[0]), 1);
}

#[cfg(test)]
mod tests {
    #[test]
    fn example_runs() {
        super::main();
    }
}

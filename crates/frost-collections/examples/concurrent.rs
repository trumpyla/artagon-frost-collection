//! Run with `cargo run --example concurrent --features dashmap-6`.

use std::sync::{Arc, Barrier};

use frost::backend::dashmap::DashMap;
use frost::concurrent::{ConcurrentLookup, ConcurrentUpdate};

fn main() {
    let counters = Arc::new(DashMap::new());
    counters.insert("requests".to_owned(), 0_usize);
    let barrier = Arc::new(Barrier::new(4));

    let workers: Vec<_> = (0..4)
        .map(|_| {
            let counters = Arc::clone(&counters);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                for _ in 0..1_000 {
                    // Exclusive entry access makes this increment atomic with
                    // respect to other updates of the same entry.
                    counters
                        .with_entry_mut("requests", |_, count| *count += 1)
                        .expect("the requests counter was initialized");
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("counter worker panicked");
    }

    // Owned extraction drops the backend guard before this call returns.
    let count = counters.get_cloned("requests").unwrap();
    assert_eq!(count, 4_000);
    println!("Requests handled: {count}");
}

#[cfg(test)]
mod tests {
    #[test]
    fn example_runs() {
        super::main();
    }
}

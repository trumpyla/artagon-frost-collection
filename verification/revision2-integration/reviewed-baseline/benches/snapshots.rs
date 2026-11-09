//! Snapshot read throughput with persistent, synchronized reader threads.
//!
//! Run `cargo bench --bench snapshots --features arc-swap-1 --locked`.
//! Worker creation/join and snapshot construction are excluded. A batch
//! includes its start barrier and result-channel rendezvous. Reported ns/read
//! is elapsed batch time divided by all readers' operations (aggregate
//! throughput), not an individual read's latency. This workload has no writer.

mod support;

use frost::concurrent::SnapshotStore;
use frost::family::FrozenVec;
use std::hint::black_box;
use std::sync::{Barrier, mpsc};
use std::time::Instant;

fn readers(name: &str, reader_count: usize, read: impl Fn() -> u64 + Sync) {
    const PER_READER: u32 = 200_000;
    let operations = PER_READER * u32::try_from(reader_count).expect("reader count fits u32");
    let expected = u64::from(operations) * 3;
    let start = Barrier::new(reader_count + 1);
    let (send, receive) = mpsc::sync_channel(reader_count);
    let mut samples = [0.0; support::SAMPLE_COUNT];

    std::thread::scope(|scope| {
        for _ in 0..reader_count {
            let send = send.clone();
            let read = &read;
            let start = &start;
            scope.spawn(move || {
                // The first batch warms the worker's ArcSwap fast-path state.
                for _ in 0..=support::SAMPLE_COUNT {
                    start.wait();
                    let mut sum = 0_u64;
                    for _ in 0..PER_READER {
                        sum += black_box(read());
                    }
                    send.send(sum).expect("benchmark coordinator exists");
                }
            });
        }
        drop(send);
        for round in 0..=support::SAMPLE_COUNT {
            let started = Instant::now();
            start.wait();
            let checksum = (0..reader_count)
                .map(|_| receive.recv().expect("reader completed its batch"))
                .sum::<u64>();
            let elapsed = started.elapsed();
            assert_eq!(checksum, expected, "benchmark answer: {name}");
            if round > 0 {
                samples[round - 1] = elapsed.as_secs_f64() * 1e9 / f64::from(operations);
            }
        }
    });
    support::report(
        &format!("snapshot.{name}.{reader_count}_readers.aggregate_ns_per_read"),
        operations,
        samples,
    );
}

fn main() {
    support::header();
    let available = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let mut reader_counts = vec![1, available.min(4)];
    reader_counts.dedup();
    eprintln!("available_parallelism={available}; readers={reader_counts:?}; writer=none");
    let store = SnapshotStore::new(FrozenVec::from((0..64_u64).collect::<Vec<_>>()));
    let raw = arc_swap::ArcSwap::from_pointee(FrozenVec::from((0..64_u64).collect::<Vec<_>>()));
    for count in reader_counts {
        readers("owned_load", count, || {
            black_box(&store).load().as_slice()[black_box(3)]
        });
        readers("guard_load", count, || {
            black_box(&store).load_guard().as_slice()[black_box(3)]
        });
        readers("peek", count, || {
            black_box(&store).peek(|snapshot| snapshot.as_slice()[black_box(3)])
        });
        readers("raw_arc_swap_guard", count, || {
            black_box(&raw).load().as_slice()[black_box(3)]
        });
    }
}

//! Small, dependency-free timing support shared by the benchmark targets.
//!
//! Samples are warm-cache wall-clock measurements, not statistical confidence
//! intervals. Raw samples remain in execution order so drift is visible.

#![allow(dead_code)] // Each standalone benchmark uses a different subset.

use std::hint::black_box;
use std::time::{Duration, Instant};

pub const SAMPLE_COUNT: usize = 7;
const MIN_BATCH: Duration = Duration::from_millis(4);

pub fn header() {
    println!(
        "case,iterations,median_ns,min_ns,max_ns,sample_1_ns,sample_2_ns,sample_3_ns,sample_4_ns,sample_5_ns,sample_6_ns,sample_7_ns"
    );
}

pub fn as_u64(value: usize) -> u64 {
    u64::try_from(value).expect("benchmark cardinality fits u64")
}

pub fn report(name: &str, iterations: u32, samples: [f64; SAMPLE_COUNT]) {
    let mut sorted = samples;
    sorted.sort_by(f64::total_cmp);
    print!(
        "{name},{iterations},{:.3},{:.3},{:.3}",
        sorted[SAMPLE_COUNT / 2],
        sorted[0],
        sorted[SAMPLE_COUNT - 1]
    );
    for sample in samples {
        print!(",{sample:.3}");
    }
    println!();
}

fn elapsed(iterations: u32, kernel: &mut impl FnMut() -> u64) -> Duration {
    let started = Instant::now();
    for _ in 0..iterations {
        black_box(kernel());
    }
    started.elapsed()
}

/// Measure a reusable operation with inputs prepared by the caller.
pub fn measure(name: &str, expected: u64, mut kernel: impl FnMut() -> u64) {
    assert_eq!(kernel(), expected, "benchmark answer: {name}");
    let mut iterations = 1_u32;
    while elapsed(iterations, &mut kernel) < MIN_BATCH && iterations < 1 << 24 {
        iterations *= 2;
    }
    let mut samples = [0.0; SAMPLE_COUNT];
    for sample in &mut samples {
        *sample = elapsed(iterations, &mut kernel).as_secs_f64() * 1e9 / f64::from(iterations);
    }
    report(name, iterations, samples);
}

fn elapsed_with_setup<T>(
    iterations: u32,
    setup: &mut impl FnMut() -> T,
    kernel: &mut impl FnMut(&mut T) -> u64,
) -> Duration {
    let mut elapsed = Duration::ZERO;
    for _ in 0..iterations {
        let mut input = setup();
        let started = Instant::now();
        black_box(kernel(black_box(&mut input)));
        elapsed += started.elapsed();
        // Input construction and the remaining fixture's destruction are
        // outside timing. Destruction performed by the operation is included.
        drop(input);
    }
    elapsed
}

/// Measure a destructive operation on a freshly prepared input each time.
///
/// This times each invocation separately to keep setup outside the region;
/// use it for operations lasting microseconds or longer, where timer overhead
/// is small. Allocations and drops performed inside `kernel` are included.
pub fn measure_with_setup<T>(
    name: &str,
    expected: u64,
    mut setup: impl FnMut() -> T,
    mut kernel: impl FnMut(&mut T) -> u64,
) {
    assert_eq!(kernel(&mut setup()), expected, "benchmark answer: {name}");
    let mut iterations = 1_u32;
    while elapsed_with_setup(iterations, &mut setup, &mut kernel) < MIN_BATCH && iterations < 256 {
        iterations *= 2;
    }
    let mut samples = [0.0; SAMPLE_COUNT];
    for sample in &mut samples {
        *sample = elapsed_with_setup(iterations, &mut setup, &mut kernel).as_secs_f64() * 1e9
            / f64::from(iterations);
    }
    report(name, iterations, samples);
}

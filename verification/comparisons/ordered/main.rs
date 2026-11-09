#![allow(dead_code)]

#[path = "incoming.rs"]
mod incoming;
#[path = "prior.rs"]
mod prior;
mod support;

use std::collections::BTreeSet;
use std::hint::black_box;
use support::{as_u64, measure};

fn case(name: &str, left: Vec<u32>, right: Vec<u32>) {
    let std_left: BTreeSet<_> = left.iter().copied().collect();
    let std_right: BTreeSet<_> = right.iter().copied().collect();
    let common = as_u64(std_left.intersection(&std_right).count());
    let sum: u64 = std_left
        .intersection(&std_right)
        .map(|&key| u64::from(key))
        .sum();
    let subset = u64::from(std_left.is_subset(&std_right));
    let disjoint = u64::from(std_left.is_disjoint(&std_right));
    for incoming_first in [false, true] {
        macro_rules! run {
            ($module:ident) => {{
                let prefix = format!(
                    "{name}.{}.order{}",
                    stringify!($module),
                    u8::from(incoming_first)
                );
                measure(&format!("{prefix}.count"), common, || {
                    as_u64($module::intersection_len(
                        black_box(&left),
                        black_box(&right),
                    ))
                });
                measure(&format!("{prefix}.reverse_count"), common, || {
                    as_u64($module::intersection_len(
                        black_box(&right),
                        black_box(&left),
                    ))
                });
                measure(&format!("{prefix}.next"), sum, || {
                    let mut result = 0;
                    for &key in $module::SetIntersection::new(black_box(&left), black_box(&right)) {
                        result += u64::from(key);
                    }
                    result
                });
                measure(&format!("{prefix}.fold"), sum, || {
                    $module::SetIntersection::new(black_box(&left), black_box(&right))
                        .fold(0, |sum, &key| sum + u64::from(key))
                });
                measure(&format!("{prefix}.subset"), subset, || {
                    u64::from($module::is_subset(black_box(&left), black_box(&right)))
                });
                measure(&format!("{prefix}.disjoint"), disjoint, || {
                    u64::from($module::is_disjoint(black_box(&left), black_box(&right)))
                });
            }};
        }
        if incoming_first {
            run!(incoming);
            run!(prior);
        } else {
            run!(prior);
            run!(incoming);
        }
    }
    measure(&format!("{name}.std.count"), common, || {
        as_u64(
            black_box(&std_left)
                .intersection(black_box(&std_right))
                .count(),
        )
    });
}

fn main() {
    support::header();
    case(
        "balanced65536",
        (0..65_536).map(|x| 2 * x).collect(),
        (0..65_536).map(|x| 3 * x).collect(),
    );
    case(
        "spread64_65536",
        (0..64).map(|x| x * 1024 * 2 + 2).collect(),
        (0..65_536).map(|x| 2 * x).collect(),
    );
    case(
        "spread64_1048576",
        (0..64).map(|x| x * 16384 * 2 + 2).collect(),
        (0..1_048_576).map(|x| 2 * x).collect(),
    );
    case(
        "spread1024_1048576",
        (0..1024).map(|x| x * 1024 * 2 + 2).collect(),
        (0..1_048_576).map(|x| 2 * x).collect(),
    );
    case("head64_65536", (0..64).collect(), (0..65_536).collect());
    case(
        "tail64_65536",
        (65_472..65_536).collect(),
        (0..65_536).collect(),
    );
    for ratio in [31, 32, 33] {
        case(
            &format!("ratio{ratio}"),
            (0..256).map(|x| x * ratio).collect(),
            (0..256 * ratio).collect(),
        );
    }
    let mut left: Vec<u32> = (0..4096).collect();
    left.extend(200_000..204_096);
    let mut right: Vec<u32> = (0..8191).collect();
    right.push(200_000);
    case("balanced_then_gap", left, right);
    case(
        "interleaved_miss",
        (0..4096).map(|x| 2 * x).collect(),
        (0..4096).map(|x| 2 * x + 1).collect(),
    );
    case("separated", (0..4096).collect(), (8192..12_288).collect());
    let same: Vec<u32> = (0..65_536).collect();
    measure("same.prior.count", 65_536, || {
        as_u64(prior::intersection_len(black_box(&same), black_box(&same)))
    });
    measure("same.incoming.count", 65_536, || {
        as_u64(incoming::intersection_len(
            black_box(&same),
            black_box(&same),
        ))
    });
}

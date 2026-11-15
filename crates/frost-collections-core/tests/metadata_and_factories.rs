//! Contracts of the deterministic workload factories in `tests/support` and
//! their use as oracles against Frost families. The factory guarantees are
//! checked with independent `BTreeSet`/`BTreeMap` computations over the
//! emitted values, not with the generator's own bookkeeping.
#![cfg(feature = "alloc")]

// The package under test, under the name its users import.
extern crate frost_collections_core as frost;

use frost_collections_test_support as support;

use std::collections::{BTreeMap, BTreeSet};

use frost::family::{FrozenOrderedMap, FrozenOrderedSet};
use support::workload::{
    Class, Emit, Layout, MAX_MEMBERS, Order, Partition, SequenceSpec, SetPairSpec, Side,
    WorkloadError, WorkloadSpec,
};

#[test]
fn final_u64_chunk_and_partial_chunks_are_handled_without_wrapping() {
    // FACTORY-1 regression: the last valid 2^16 container of u64.
    let last_chunk = u64::MAX >> 16;
    let layout = Layout::Chunks {
        first_chunk: last_chunk,
        chunk_bits: 16,
        per_chunk: 2,
    };
    let one = SetPairSpec::new(1, 1, 0, 0, layout).build().unwrap();
    assert_eq!(one.left, vec![last_chunk << 16]);
    assert_eq!(one.max_code, Some(last_chunk << 16));
    let two = SetPairSpec::new(1, 2, 0, 0, layout).build().unwrap();
    assert_eq!(two.left, vec![last_chunk << 16, u64::MAX]);
    assert_eq!(two.max_code, Some(u64::MAX));
    assert_eq!(
        SetPairSpec::new(1, 3, 0, 0, layout).build().err(),
        Some(WorkloadError::Overflow("chunk code"))
    );
    // A partially filled final container keeps its start code only.
    let partial = SetPairSpec::new(
        1,
        7,
        0,
        0,
        Layout::Chunks {
            first_chunk: 0,
            chunk_bits: 4,
            per_chunk: 5,
        },
    )
    .build()
    .unwrap();
    assert_eq!(partial.left, vec![0, 3, 6, 9, 15, 16, 19]);
    assert_eq!(partial.max_code, Some(19));
    assert!(
        !partial.left.contains(&31),
        "partial container has no end code"
    );
}

#[test]
fn layout_parameters_are_validated_even_for_empty_output() {
    // FACTORY-2 regression: invalid parameters are rejected with zero members.
    assert_eq!(
        SetPairSpec::new(1, 0, 0, 0, Layout::Stride { base: 0, step: 0 })
            .build()
            .err(),
        Some(WorkloadError::ZeroParameter("step"))
    );
    assert_eq!(
        SetPairSpec::new(
            1,
            0,
            0,
            0,
            Layout::Chunks {
                first_chunk: 0,
                chunk_bits: 0,
                per_chunk: 1
            }
        )
        .build()
        .err(),
        Some(WorkloadError::ChunkBitsOutOfRange(0))
    );
    assert_eq!(
        SetPairSpec::new(1, 0, 0, 0, Layout::Uniform { base: 0, span: 0 })
            .build()
            .err(),
        Some(WorkloadError::ZeroParameter("span"))
    );
    // An unaddressable first container is rejected with zero members too.
    assert_eq!(
        SetPairSpec::new(
            1,
            0,
            0,
            0,
            Layout::Chunks {
                first_chunk: 1 << 48,
                chunk_bits: 16,
                per_chunk: 1
            }
        )
        .build()
        .err(),
        Some(WorkloadError::Overflow("chunk code"))
    );
    assert!(
        SetPairSpec::new(
            1,
            0,
            0,
            0,
            Layout::Chunks {
                first_chunk: (1 << 48) - 1,
                chunk_bits: 16,
                per_chunk: 1
            }
        )
        .build()
        .is_ok()
    );
    // The maximal uniform code is reachable without wrapping.
    let top = SetPairSpec::new(
        1,
        1,
        0,
        0,
        Layout::Uniform {
            base: u64::MAX,
            span: 1,
        },
    )
    .build()
    .unwrap();
    assert_eq!(top.left, vec![u64::MAX]);
}

#[test]
fn extreme_counts_are_rejected_before_any_generation() {
    let huge = Layout::Uniform {
        base: 0,
        span: u64::MAX,
    };
    assert_eq!(
        SetPairSpec::new(1, MAX_MEMBERS + 1, 0, 0, huge)
            .build()
            .err(),
        Some(WorkloadError::ExceedsSupportedSize {
            requested: MAX_MEMBERS + 1,
            limit: MAX_MEMBERS
        })
    );
    assert_eq!(
        SetPairSpec::new(1, 1, 0, 0, huge)
            .with_emit(
                Emit {
                    order: Order::Ascending,
                    duplicates: MAX_MEMBERS
                },
                Emit::ASCENDING
            )
            .build()
            .err(),
        Some(WorkloadError::ExceedsSupportedSize {
            requested: MAX_MEMBERS + 1,
            limit: MAX_MEMBERS
        })
    );
    assert_eq!(
        SetPairSpec::new(1, 1, 0, 0, huge)
            .with_emit(
                Emit {
                    order: Order::Ascending,
                    duplicates: usize::MAX
                },
                Emit::ASCENDING
            )
            .build()
            .err(),
        Some(WorkloadError::Overflow("side length"))
    );
    // An empty right side with duplicates is rejected before the (large)
    // left side would be generated.
    assert_eq!(
        SetPairSpec::new(1, MAX_MEMBERS, 0, 0, huge)
            .with_emit(
                Emit::ASCENDING,
                Emit {
                    order: Order::Ascending,
                    duplicates: 1
                }
            )
            .build()
            .err(),
        Some(WorkloadError::DuplicatesOfEmptySide(Side::Right))
    );
}

fn specs() -> Vec<SetPairSpec> {
    let mut specs = vec![
        SetPairSpec::new(1, 0, 0, 0, Layout::Uniform { base: 0, span: 1 }),
        SetPairSpec::new(1, 0, 128, 0, Layout::Stride { base: 0, step: 1 }),
        SetPairSpec::new(
            7,
            4096,
            4096,
            1365,
            Layout::Uniform {
                base: 1_000_003,
                span: 10_000,
            },
        ),
        SetPairSpec::new(
            7,
            65_536,
            64,
            64,
            Layout::Stride {
                base: 2,
                step: 1024,
            },
        ),
        SetPairSpec::new(
            3,
            300,
            100,
            50,
            Layout::Runs {
                base: 65_000,
                run: 1024,
                gap: 64_512,
            },
        )
        .with_partition(Partition::Blocks([
            Class::RightOnly,
            Class::Shared,
            Class::LeftOnly,
        ])),
        SetPairSpec::new(
            11,
            40,
            40,
            8,
            Layout::Chunks {
                first_chunk: 3,
                chunk_bits: 16,
                per_chunk: 5,
            },
        )
        .with_partition(Partition::Shuffled)
        .with_emit(
            Emit {
                order: Order::Shuffled,
                duplicates: 17,
            },
            Emit {
                order: Order::Descending,
                duplicates: 3,
            },
        ),
        SetPairSpec::new(
            5,
            9,
            9,
            9,
            Layout::Chunks {
                first_chunk: 0,
                chunk_bits: 4,
                per_chunk: 16,
            },
        ),
    ];
    for partition in [Partition::Interleaved, Partition::Shuffled] {
        specs.push(
            SetPairSpec::new(99, 500, 20, 7, Layout::Uniform { base: 0, span: 600 })
                .with_partition(partition),
        );
    }
    specs
}

fn check_order(values: &[u64], order: Order, label: &str) {
    match order {
        Order::Ascending => assert!(values.windows(2).all(|w| w[0] <= w[1]), "{label} ascending"),
        Order::Descending => assert!(
            values.windows(2).all(|w| w[0] >= w[1]),
            "{label} descending"
        ),
        Order::Shuffled => {}
    }
}

#[test]
fn set_pair_is_deterministic_and_identified_by_parameters() {
    for spec in specs() {
        let first = spec.build().unwrap();
        let second = spec.build().unwrap();
        assert_eq!(first.left, second.left, "{}", spec.id());
        assert_eq!(first.right, second.right, "{}", spec.id());
        assert_eq!(first.id, spec.id());
        assert!(!first.id.contains(','), "id must be CSV safe: {}", first.id);
        assert!(first.id.contains(&format!("seed={}", spec.seed)));
        let mut reseeded = spec.clone();
        reseeded.seed ^= 0x5555;
        assert_ne!(reseeded.id(), spec.id());
    }
    // A different seed changes seeded layouts and shuffles, and only those.
    let uniform = SetPairSpec::new(
        1,
        64,
        64,
        16,
        Layout::Uniform {
            base: 0,
            span: 1000,
        },
    );
    let mut other = uniform.clone();
    other.seed = 2;
    assert_ne!(uniform.build().unwrap().left, other.build().unwrap().left);
    let stride = SetPairSpec::new(1, 64, 64, 16, Layout::Stride { base: 0, step: 3 });
    let mut other = stride.clone();
    other.seed = 2;
    assert_eq!(stride.build().unwrap().left, other.build().unwrap().left);
}

#[test]
fn set_pair_guarantees_hold_against_btreeset() {
    for spec in specs() {
        let pair = spec
            .build()
            .unwrap_or_else(|e| panic!("{}: {e}", spec.id()));
        let left: BTreeSet<u64> = pair.left.iter().copied().collect();
        let right: BTreeSet<u64> = pair.right.iter().copied().collect();
        let label = spec.id();
        assert_eq!(left.len(), spec.left, "{label}: left distinct");
        assert_eq!(right.len(), spec.right, "{label}: right distinct");
        assert_eq!(
            left.intersection(&right).count(),
            spec.shared,
            "{label}: shared"
        );
        assert_eq!(
            pair.left.len(),
            spec.left + spec.left_emit.duplicates,
            "{label}"
        );
        assert_eq!(
            pair.right.len(),
            spec.right + spec.right_emit.duplicates,
            "{label}"
        );
        check_order(&pair.left, spec.left_emit.order, &label);
        check_order(&pair.right, spec.right_emit.order, &label);
        assert!(left.iter().eq(pair.left_unique.iter()), "{label}");
        assert!(right.iter().eq(pair.right_unique.iter()), "{label}");
        assert!(left.intersection(&right).eq(pair.shared.iter()), "{label}");
        let union: BTreeSet<u64> = left.union(&right).copied().collect();
        match pair.max_code {
            Some(max) => assert!(union.iter().all(|&v| v <= max), "{label}: bound"),
            None => assert!(union.is_empty(), "{label}: empty bound"),
        }
        match spec.layout {
            Layout::Uniform { base, span } => {
                assert!(
                    union.iter().all(|&v| v >= base && v < base + span),
                    "{label}"
                );
            }
            Layout::Stride { base, step } => {
                assert!(union.iter().all(|&v| (v - base) % step == 0), "{label}");
                assert_eq!(
                    union.iter().copied().collect::<Vec<_>>(),
                    (0..union.len() as u64)
                        .map(|i| base + i * step)
                        .collect::<Vec<_>>(),
                    "{label}"
                );
            }
            Layout::Runs { base, run, gap } => {
                for &v in &union {
                    assert!((v - base) % (run + gap) < run, "{label}: {v} is in a gap");
                }
            }
            Layout::Chunks {
                first_chunk,
                chunk_bits,
                per_chunk,
            } => {
                let mut per_container: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
                for &v in &union {
                    per_container
                        .entry(v >> chunk_bits)
                        .or_default()
                        .push(v & ((1 << chunk_bits) - 1));
                }
                let chunks: Vec<u64> = per_container.keys().copied().collect();
                let expected: Vec<u64> = (first_chunk..first_chunk + chunks.len() as u64).collect();
                assert_eq!(chunks, expected, "{label}: contiguous containers");
                let last = (1_u64 << chunk_bits) - 1;
                for (chunk, offsets) in &per_container {
                    assert!(offsets.len() as u64 <= per_chunk, "{label}");
                    if offsets.len() as u64 == per_chunk && per_chunk >= 2 {
                        assert_eq!(offsets[0], 0, "{label}: chunk {chunk} start");
                        assert_eq!(*offsets.last().unwrap(), last, "{label}: chunk {chunk} end");
                    }
                }
            }
        }
        if let Partition::Blocks(order) = spec.partition {
            // Every member of an earlier block precedes every member of a
            // later block in the ascending union.
            let class_of = |v: u64| {
                let l = left.contains(&v);
                let r = right.contains(&v);
                match (l, r) {
                    (true, true) => Class::Shared,
                    (true, false) => Class::LeftOnly,
                    _ => Class::RightOnly,
                }
            };
            let rank = |c: Class| order.iter().position(|&o| o == c).unwrap();
            let ranks: Vec<usize> = union.iter().map(|&v| rank(class_of(v))).collect();
            assert!(
                ranks.windows(2).all(|w| w[0] <= w[1]),
                "{label}: block order"
            );
        }
    }
}

#[test]
fn interleaved_partition_spreads_classes_evenly() {
    let spec = SetPairSpec::new(1, 400, 400, 200, Layout::Stride { base: 0, step: 1 });
    let pair = spec.build().unwrap();
    // Left-only, shared, right-only each have 200 members over 600 codes:
    // no class may be absent from any window of six consecutive codes.
    let left: BTreeSet<u64> = pair.left.iter().copied().collect();
    let right: BTreeSet<u64> = pair.right.iter().copied().collect();
    for window in (0..600_u64).collect::<Vec<_>>().windows(6) {
        let mut seen = [false; 3];
        for &v in window {
            let idx = match (left.contains(&v), right.contains(&v)) {
                (true, true) => 1,
                (true, false) => 0,
                _ => 2,
            };
            seen[idx] = true;
        }
        assert_eq!(seen, [true; 3], "window starting at {}", window[0]);
    }
}

#[test]
fn invalid_layout_parameters_are_rejected_with_the_offending_parameter() {
    let uniform = Layout::Uniform { base: 0, span: 10 };
    assert_eq!(
        SetPairSpec::new(1, 8, 8, 0, uniform).build().err(),
        Some(WorkloadError::SpanTooSmall {
            needed: 16,
            span: 10
        })
    );
    assert_eq!(
        SetPairSpec::new(
            1,
            2,
            0,
            0,
            Layout::Uniform {
                base: u64::MAX - 1,
                span: 4
            }
        )
        .build()
        .err(),
        Some(WorkloadError::Overflow("base + span"))
    );
    assert_eq!(
        SetPairSpec::new(1, 2, 0, 0, Layout::Stride { base: 0, step: 0 })
            .build()
            .err(),
        Some(WorkloadError::ZeroParameter("step"))
    );
    assert_eq!(
        SetPairSpec::new(
            1,
            3,
            0,
            0,
            Layout::Stride {
                base: 0,
                step: u64::MAX / 2 + 1
            }
        )
        .build()
        .err(),
        Some(WorkloadError::Overflow("stride offset"))
    );
    assert_eq!(
        SetPairSpec::new(
            1,
            2,
            0,
            0,
            Layout::Runs {
                base: 0,
                run: 0,
                gap: 1
            }
        )
        .build()
        .err(),
        Some(WorkloadError::ZeroParameter("run"))
    );
}

#[test]
fn invalid_chunk_parameters_are_rejected_with_the_offending_parameter() {
    assert_eq!(
        SetPairSpec::new(
            1,
            2,
            0,
            0,
            Layout::Chunks {
                first_chunk: 0,
                chunk_bits: 64,
                per_chunk: 1
            }
        )
        .build()
        .err(),
        Some(WorkloadError::ChunkBitsOutOfRange(64))
    );
    assert_eq!(
        SetPairSpec::new(
            1,
            2,
            0,
            0,
            Layout::Chunks {
                first_chunk: 0,
                chunk_bits: 2,
                per_chunk: 5
            }
        )
        .build()
        .err(),
        Some(WorkloadError::PerChunkExceedsChunk {
            per_chunk: 5,
            chunk_bits: 2
        })
    );
    assert_eq!(
        SetPairSpec::new(
            1,
            2,
            0,
            0,
            Layout::Chunks {
                first_chunk: 1 << 48,
                chunk_bits: 16,
                per_chunk: 1
            }
        )
        .build()
        .err(),
        Some(WorkloadError::Overflow("chunk code"))
    );
}

#[test]
fn invalid_structure_parameters_are_rejected_with_the_offending_parameter() {
    let uniform = Layout::Uniform { base: 0, span: 10 };
    assert_eq!(
        SetPairSpec::new(1, 5, 2, 3, uniform).build().err(),
        Some(WorkloadError::SharedExceedsSide {
            shared: 3,
            left: 5,
            right: 2
        })
    );
    assert_eq!(
        SetPairSpec::new(1, 2, 2, 1, uniform)
            .with_partition(Partition::Blocks([
                Class::Shared,
                Class::Shared,
                Class::LeftOnly
            ]))
            .build()
            .err(),
        Some(WorkloadError::InvalidBlockOrder([
            Class::Shared,
            Class::Shared,
            Class::LeftOnly
        ]))
    );
    assert_eq!(
        SetPairSpec::new(1, 0, 2, 0, uniform)
            .with_emit(
                Emit {
                    order: Order::Ascending,
                    duplicates: 1
                },
                Emit::ASCENDING
            )
            .build()
            .err(),
        Some(WorkloadError::DuplicatesOfEmptySide(Side::Left))
    );
    let wide = SetPairSpec::new(
        1,
        2,
        0,
        0,
        Layout::Stride {
            base: u64::from(u32::MAX),
            step: 1,
        },
    )
    .build()
    .unwrap();
    assert_eq!(
        wide.to_u32().err(),
        Some(WorkloadError::ValueOutOfRange {
            value: u64::from(u32::MAX) + 1,
            target: "u32"
        })
    );
    // Rejection happens before any generation: an invalid spec has no id-only
    // side effects and an id is still available for reporting.
    assert!(
        SetPairSpec::new(1, 5, 2, 3, uniform)
            .id()
            .contains("shared=3")
    );
}

#[test]
fn frozen_ordered_set_matches_factory_oracle() {
    for spec in specs() {
        let pair = spec.build().unwrap();
        let oracle = pair.oracle();
        let left: FrozenOrderedSet<u64> = pair.left.iter().copied().collect();
        let right: FrozenOrderedSet<u64> = pair.right.iter().copied().collect();
        let label = &pair.id;
        assert!(left.iter().eq(oracle.left.iter()), "{label}");
        assert!(right.iter().eq(oracle.right.iter()), "{label}");
        assert!(
            left.intersection(&right).eq(oracle.intersection.iter()),
            "{label}"
        );
        assert!(left.union(&right).eq(oracle.union.iter()), "{label}");
        assert!(
            left.difference(&right).eq(oracle.difference.iter()),
            "{label}"
        );
        assert!(
            left.symmetric_difference(&right)
                .eq(oracle.symmetric_difference.iter()),
            "{label}"
        );
        assert_eq!(
            left.intersection_len(&right),
            oracle.intersection.len(),
            "{label}"
        );
        assert_eq!(left.union_len(&right), oracle.union.len(), "{label}");
        assert_eq!(
            left.difference_len(&right),
            oracle.difference.len(),
            "{label}"
        );
        assert_eq!(
            left.symmetric_difference_len(&right),
            oracle.symmetric_difference.len(),
            "{label}"
        );
        assert_eq!(left.is_subset(&right), oracle.is_subset, "{label}");
        assert_eq!(left.is_superset(&right), oracle.is_superset, "{label}");
        assert_eq!(left.is_disjoint(&right), oracle.is_disjoint, "{label}");
        // The oracle over the exact structure agrees with the spec.
        assert_eq!(oracle.intersection.len(), spec.shared);
        assert_eq!(oracle.is_disjoint, spec.shared == 0);
        assert_eq!(oracle.is_subset, spec.shared == spec.left);
    }
}

#[test]
fn sequence_entries_keep_last_value_for_duplicate_keys() {
    let spec = SequenceSpec {
        seed: 4,
        unique: 50,
        layout: Layout::Uniform {
            base: 10,
            span: 200,
        },
        emit: Emit {
            order: Order::Shuffled,
            duplicates: 30,
        },
    };
    let sequence = spec.build().unwrap();
    assert_eq!(sequence.id, spec.id());
    assert_eq!(sequence.values.len(), 80);
    assert_eq!(sequence.unique.len(), 50);
    let expected = sequence.last_value_oracle();
    assert_eq!(expected.len(), 50);
    let map: FrozenOrderedMap<u64, usize> = sequence.entries().into_iter().collect();
    assert!(map.iter().eq(expected.iter()));
    // At least one key was really repeated, so last-value-wins was exercised.
    assert!(expected.values().any(|&position| position >= 50));
}

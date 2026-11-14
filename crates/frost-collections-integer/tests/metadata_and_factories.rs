//! Workload factories against the integer profile. These cases were split
//! from the core package's `metadata_and_factories` target because they need
//! `frost::integer`; shared helpers are copies of that target's helpers.

// The package under test, under the name its users import.
extern crate frost_collections_integer as frost;

use frost_collections_test_support as support;

use frost::integer::{FrozenIntSet, IntSetError, IntSetPolicy};

use support::workload::{Class, Layout, Partition, SetPairSpec, WorkloadSpec};

#[test]
fn frozen_int_set_matches_factory_oracle_across_chunk_boundaries() {
    let specs = [
        SetPairSpec::new(
            21,
            400,
            400,
            100,
            Layout::Chunks {
                first_chunk: 1,
                chunk_bits: 16,
                per_chunk: 64,
            },
        )
        .with_partition(Partition::Shuffled),
        SetPairSpec::new(
            22,
            3000,
            300,
            300,
            Layout::Runs {
                base: 65_500,
                run: 100,
                gap: 36,
            },
        )
        .with_partition(Partition::Blocks([
            Class::LeftOnly,
            Class::Shared,
            Class::RightOnly,
        ])),
        SetPairSpec::new(
            23,
            256,
            256,
            0,
            Layout::Uniform {
                base: u64::from(u32::MAX) - 4096,
                span: 8192,
            },
        ),
    ];
    let policies = [
        IntSetPolicy::auto(),
        IntSetPolicy::sorted(),
        IntSetPolicy::dense(),
        IntSetPolicy::roaring(),
        IntSetPolicy::chunked(),
    ];
    for spec in specs {
        let pair = spec.build().unwrap();
        let oracle = pair.oracle();
        for (li, left_policy) in policies.iter().enumerate() {
            let left = match FrozenIntSet::try_from_iter(pair.left.iter().copied(), *left_policy) {
                Ok(set) => set,
                Err(IntSetError::DenseBudgetExceeded { .. }) => continue,
                Err(error) => panic!("{}: {error}", pair.id),
            };
            for right_policy in &policies[li..] {
                let right =
                    match FrozenIntSet::try_from_iter(pair.right.iter().copied(), *right_policy) {
                        Ok(set) => set,
                        Err(IntSetError::DenseBudgetExceeded { .. }) => continue,
                        Err(error) => panic!("{}: {error}", pair.id),
                    };
                let label = format!("{} {:?}/{:?}", pair.id, left_policy, right_policy);
                assert!(left.iter().eq(oracle.left.iter().copied()), "{label}");
                assert!(
                    left.intersection(&right)
                        .iter()
                        .eq(oracle.intersection.iter().copied()),
                    "{label}"
                );
                assert!(
                    left.union(&right).iter().eq(oracle.union.iter().copied()),
                    "{label}"
                );
                assert!(
                    left.difference(&right)
                        .iter()
                        .eq(oracle.difference.iter().copied()),
                    "{label}"
                );
                assert!(
                    left.symmetric_difference(&right)
                        .iter()
                        .eq(oracle.symmetric_difference.iter().copied()),
                    "{label}"
                );
                assert_eq!(
                    left.intersection_len(&right),
                    oracle.intersection.len() as u64,
                    "{label}"
                );
                assert_eq!(left.is_subset(&right), oracle.is_subset, "{label}");
                assert_eq!(left.is_disjoint(&right), oracle.is_disjoint, "{label}");
            }
        }
    }
}

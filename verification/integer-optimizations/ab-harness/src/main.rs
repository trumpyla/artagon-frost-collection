//! Same-binary A/B harness for the integer profile.
//!
//! Two revisions of the crate (`frost_baseline`, prepared by
//! `prepare-baseline.sh`, and the working tree as `frost`) are linked into one
//! executable and timed on identical datasets. Every case first validates its
//! answer against the standard library; then the two implementations are
//! measured alternately (old, new, old, new, …) and the reported figure is the
//! minimum of the per-round medians. Linking both into one binary removes the
//! code-layout differences that made separate benchmark binaries disagree by up
//! to 2× on unchanged control loops (see `docs/integer-benchmarks.md`).
//!
//! Output is CSV on stdout: `case,old_min_ns,new_min_ns,old_over_new,old_repr,new_repr`.
//! `AB_FILTER=a,b` restricts cases to names containing one of the substrings;
//! the first positional argument is the number of alternating rounds (default 3).

use std::collections::{BTreeMap, BTreeSet};
use std::hint::black_box;
use std::time::{Duration, Instant};

type Kernel = Box<dyn FnMut() -> u64>;
type Cases = Vec<(String, Kernel)>;
type Reprs = BTreeMap<String, String>;

const SAMPLES: usize = 5;
const MIN_BATCH: Duration = Duration::from_millis(4);

fn elapsed(iterations: u32, kernel: &mut Kernel) -> Duration {
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(kernel());
    }
    start.elapsed()
}

fn median_ns(kernel: &mut Kernel) -> f64 {
    let mut iterations = 1_u32;
    while elapsed(iterations, kernel) < MIN_BATCH && iterations < 1 << 24 {
        iterations *= 2;
    }
    let mut samples = [0.0; SAMPLES];
    for sample in &mut samples {
        *sample = elapsed(iterations, kernel).as_secs_f64() * 1e9 / f64::from(iterations);
    }
    samples.sort_by(f64::total_cmp);
    samples[SAMPLES / 2]
}

/// Generic workload definition, instantiated once per linked revision.
macro_rules! implementation {
    ($module:ident, $krate:ident) => {
        mod $module {
            use super::*;
            use $krate::integer::{FrozenIntSet, IntKey, IntSetError, IntSetPolicy};

            pub(super) fn policies() -> [(&'static str, IntSetPolicy); 5] {
                [
                    ("auto", IntSetPolicy::auto()),
                    ("sorted", IntSetPolicy::sorted()),
                    ("dense", IntSetPolicy::dense()),
                    ("roaring", IntSetPolicy::roaring()),
                    ("chunked", IntSetPolicy::chunked()),
                ]
            }

            fn build<K: IntKey>(values: &[K], policy: IntSetPolicy) -> Option<FrozenIntSet<K>> {
                match FrozenIntSet::try_from_iter(values.iter().copied(), policy) {
                    Ok(set) => Some(set),
                    Err(IntSetError::DenseBudgetExceeded { .. }) => None,
                    Err(error) => panic!("construction failed: {error}"),
                }
            }

            fn repr<K: IntKey>(set: &FrozenIntSet<K>) -> String {
                format!(
                    "{:?}/{}",
                    set.representation(),
                    set.contiguous_payload_bytes()
                        .map_or_else(|| "partitioned".to_string(), |bytes| bytes.to_string())
                )
            }

            fn apply<K: IntKey>(op: &str, a: &FrozenIntSet<K>, b: &FrozenIntSet<K>) -> FrozenIntSet<K> {
                match op {
                    "intersection" => a.intersection(b),
                    "union" => a.union(b),
                    "difference" => a.difference(b),
                    _ => a.symmetric_difference(b),
                }
            }

            pub(super) fn dataset<K: IntKey + std::fmt::Debug + 'static>(
                name: &str,
                left: &[K],
                right: &[K],
                additions: &[K],
                cases: &mut Cases,
                reprs: &mut Reprs,
            ) {
                let left_std: BTreeSet<K> = left.iter().copied().collect();
                let right_std: BTreeSet<K> = right.iter().copied().collect();
                let expected: BTreeMap<&str, Vec<K>> = [
                    ("intersection", left_std.intersection(&right_std).copied().collect()),
                    ("union", left_std.union(&right_std).copied().collect()),
                    ("difference", left_std.difference(&right_std).copied().collect()),
                    ("symmetric_difference", left_std.symmetric_difference(&right_std).copied().collect()),
                ]
                .into_iter()
                .collect();
                for (label, policy) in policies() {
                    let Some(l) = build(left, policy) else { continue };
                    let Some(r) = build(right, policy) else { continue };
                    reprs.insert(format!("{name}.{label}.construct"), repr(&l));
                    let input = left.to_vec();
                    cases.push((format!("{name}.{label}.construct"), Box::new(move || {
                        FrozenIntSet::try_from_iter(black_box(&input).iter().copied(), policy).unwrap().cardinality()
                    })));
                    let (a, b) = (l.clone(), r.clone());
                    cases.push((format!("{name}.{label}.intersection_count"), Box::new(move || black_box(&a).intersection_len(black_box(&b)))));
                    let (a, b) = (l.clone(), r.clone());
                    assert_eq!(a.is_subset(&b), left_std.is_subset(&right_std));
                    cases.push((format!("{name}.{label}.is_subset"), Box::new(move || u64::from(black_box(&a).is_subset(black_box(&b))))));
                    for op in ["intersection", "union", "difference", "symmetric_difference"] {
                        let (a, b) = (l.clone(), r.clone());
                        let result = apply(op, &a, &b);
                        assert_eq!(result.iter().collect::<Vec<_>>(), expected[op], "{name}.{label}.{op}");
                        reprs.insert(format!("{name}.{label}.{op}_materialize"), repr(&result));
                        cases.push((format!("{name}.{label}.{op}_materialize"), Box::new(move || {
                            black_box(apply(op, black_box(&a), black_box(&b))).cardinality()
                        })));
                    }
                    // Snapshot-style incremental growth, clone, and equality.
                    let singletons: Vec<_> = additions
                        .iter()
                        .map(|&v| FrozenIntSet::try_from_iter([v], IntSetPolicy::sorted()).unwrap())
                        .collect();
                    let mut expect: BTreeSet<K> = left_std.clone();
                    expect.extend(additions.iter().copied());
                    let mut current = l.clone();
                    for s in &singletons {
                        current = current.union(s);
                    }
                    assert_eq!(current.iter().collect::<Vec<_>>(), expect.iter().copied().collect::<Vec<_>>());
                    reprs.insert(format!("{name}.{label}.snapshot_update"), repr(&current));
                    let base = l.clone();
                    cases.push((format!("{name}.{label}.snapshot_update"), Box::new(move || {
                        let mut current = black_box(&base).clone();
                        for s in black_box(&singletons) {
                            current = current.union(s);
                        }
                        current.cardinality()
                    })));
                    let base = l.clone();
                    cases.push((format!("{name}.{label}.clone"), Box::new(move || black_box(black_box(&base).clone()).cardinality())));
                    let twin = build(left, policy).unwrap();
                    let base = l.clone();
                    assert!(base == twin);
                    cases.push((format!("{name}.{label}.equality"), Box::new(move || u64::from(black_box(&base) == black_box(&twin)))));
                }
                // Mixed representation pairs over the explicit policies.
                let explicit: Vec<_> = policies().into_iter().filter(|(l, _)| *l != "auto").collect();
                for (ll, lp) in &explicit {
                    for (rl, rp) in &explicit {
                        if ll == rl {
                            continue;
                        }
                        let Some(l) = build(left, *lp) else { continue };
                        let Some(r) = build(right, *rp) else { continue };
                        let (a, b) = (l.clone(), r.clone());
                        cases.push((format!("{name}.mixed_{ll}_{rl}.intersection_count"), Box::new(move || black_box(&a).intersection_len(black_box(&b)))));
                        for op in ["intersection", "union", "difference"] {
                            let (a, b) = (l.clone(), r.clone());
                            let result = apply(op, &a, &b);
                            assert_eq!(result.iter().collect::<Vec<_>>(), expected[op]);
                            reprs.insert(format!("{name}.mixed_{ll}_{rl}.{op}_materialize"), repr(&result));
                            cases.push((format!("{name}.mixed_{ll}_{rl}.{op}_materialize"), Box::new(move || {
                                black_box(apply(op, black_box(&a), black_box(&b))).cardinality()
                            })));
                        }
                    }
                }
            }

            /// Cases aimed at the paths this series changes: arrival orders in
            /// construction, `fold` traversal, small Sorted updates, identity
            /// shortcuts on Roaring roots, sparse Chunked algebra, and the
            /// ordered family's cursors. Every answer is checked before timing.
            pub(super) fn series(cases: &mut Cases, reprs: &mut Reprs) {
                use $krate::family::FrozenOrderedSet;
                let mut state = 0x9E37_79B9_7F4A_7C15_u64;
                let mut next = move || {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    state
                };
                // Construction: one multiset per shape, three arrival orders.
                let dense: Vec<u32> = (0..200_000).map(|v| v * 3).collect();
                let sparse: Vec<u64> = (0..200_000_u64).map(|v| v * 1_000_003).collect();
                let shuffle32 = |values: &[u32], next: &mut dyn FnMut() -> u64| {
                    let mut out = values.to_vec();
                    for i in (1..out.len()).rev() {
                        out.swap(i, (next() % (i as u64 + 1)) as usize);
                    }
                    out
                };
                let dense_shuffled = shuffle32(&dense, &mut next);
                let mut sparse_shuffled = sparse.clone();
                for i in (1..sparse_shuffled.len()).rev() {
                    sparse_shuffled.swap(i, (next() % (i as u64 + 1)) as usize);
                }
                let mut dense_reversed = dense.clone();
                dense_reversed.reverse();
                for (label, input) in [
                    ("u32_dense_ascending", dense.clone()),
                    ("u32_dense_shuffled", dense_shuffled),
                    ("u32_dense_descending", dense_reversed),
                ] {
                    for (policy_label, policy) in [("auto", IntSetPolicy::auto()), ("sorted", IntSetPolicy::sorted())] {
                        let built = build(&input, policy).unwrap();
                        assert!(built.iter().eq(dense.iter().copied()));
                        reprs.insert(format!("series.construct.{label}.{policy_label}"), repr(&built));
                        let input = input.clone();
                        cases.push((format!("series.construct.{label}.{policy_label}"), Box::new(move || {
                            FrozenIntSet::try_from_iter(black_box(&input).iter().copied(), policy).unwrap().cardinality()
                        })));
                    }
                }
                for (label, input) in [("u64_sparse_ascending", sparse.clone()), ("u64_sparse_shuffled", sparse_shuffled)] {
                    let built = build(&input, IntSetPolicy::auto()).unwrap();
                    assert!(built.iter().eq(sparse.iter().copied()));
                    reprs.insert(format!("series.construct.{label}.auto"), repr(&built));
                    cases.push((format!("series.construct.{label}.auto"), Box::new(move || {
                        FrozenIntSet::try_from_iter(black_box(&input).iter().copied(), IntSetPolicy::auto()).unwrap().cardinality()
                    })));
                }
                // Traversal: `fold` adapters against a `for` loop.
                let expected_sum: u64 = dense.iter().map(|&v| u64::from(v)).sum();
                for (label, policy) in policies().into_iter().skip(1) {
                    let set = build(&dense, policy).unwrap();
                    assert_eq!(set.iter().map(u64::from).sum::<u64>(), expected_sum);
                    let folded = set.clone();
                    cases.push((format!("series.iterate.{label}.fold_sum"), Box::new(move || {
                        black_box(&folded).iter().fold(0_u64, |sum, v| sum + u64::from(v))
                    })));
                    let encoded = set.clone();
                    cases.push((format!("series.iterate.{label}.encoded_fold_sum"), Box::new(move || {
                        black_box(&encoded).encoded_values().fold(0_u64, |sum, v| sum + u64::from(v))
                    })));
                    cases.push((format!("series.iterate.{label}.for_loop_sum"), Box::new(move || {
                        let mut sum = 0_u64;
                        for v in black_box(&set) {
                            sum += u64::from(v);
                        }
                        sum
                    })));
                }
                // Small updates of a large Sorted set.
                let base = build(&sparse, IntSetPolicy::auto()).unwrap();
                let absent = build(&[77_u64], IntSetPolicy::auto()).unwrap();
                let present = build(&[sparse[sparse.len() / 2]], IntSetPolicy::auto()).unwrap();
                let hundred: Vec<u64> = (0..100_u64).map(|v| v * 900_000_011 + 5).collect();
                let hundred = build(&hundred, IntSetPolicy::auto()).unwrap();
                assert_eq!(base.union(&absent).cardinality(), base.cardinality() + 1);
                assert_eq!(base.difference(&present).cardinality(), base.cardinality() - 1);
                reprs.insert("series.sorted_update.union_one".into(), repr(&base.union(&absent)));
                let (a, b) = (base.clone(), absent.clone());
                cases.push(("series.sorted_update.union_one".into(), Box::new(move || black_box(&a).union(black_box(&b)).cardinality())));
                let (a, b) = (base.clone(), present);
                cases.push(("series.sorted_update.difference_one".into(), Box::new(move || black_box(&a).difference(black_box(&b)).cardinality())));
                let (a, b) = (base.clone(), hundred);
                cases.push(("series.sorted_update.union_hundred".into(), Box::new(move || black_box(&a).union(black_box(&b)).cardinality())));
                // Identity shortcuts on a Roaring root with many containers.
                let scattered: Vec<u64> = (0..4096_u64).map(|v| ((v * 104_729) << 20) | (v * 13)).collect();
                for (label, policy) in [("roaring", IntSetPolicy::roaring()), ("sorted", IntSetPolicy::sorted()), ("chunked", IntSetPolicy::chunked())] {
                    let set = build(&scattered, policy).unwrap();
                    let twin = set.clone();
                    let empty = FrozenIntSet::<u64>::new();
                    assert_eq!(set.union(&twin).cardinality(), 4096);
                    let (a, b) = (set.clone(), twin);
                    cases.push((format!("series.identity.{label}.union_shared_root"), Box::new(move || black_box(&a).union(black_box(&b)).cardinality())));
                    cases.push((format!("series.identity.{label}.union_empty"), Box::new(move || black_box(&set).union(black_box(&empty)).cardinality())));
                }
                let run: Vec<u32> = (0..65_536).collect();
                let run = build(&run, IntSetPolicy::roaring()).unwrap();
                let additions: Vec<_> = (0..64_u32).map(|v| build(&[2_000_000 + v * 97], IntSetPolicy::sorted()).unwrap()).collect();
                cases.push(("series.identity.roaring.update_chain_64".into(), Box::new(move || {
                    let mut current = black_box(&run).clone();
                    for addition in black_box(&additions) {
                        current = current.union(addition);
                    }
                    current.cardinality()
                })));
                // Sparse containers: 2,048 prefixes with 40 members each.
                let sparse_chunks = |offset: u32| -> Vec<u32> {
                    (0..2048_u32).flat_map(|p| (0..40_u32).map(move |i| (p << 16) + i * 1500 + offset * (i % 2))).collect()
                };
                let (left, right) = (sparse_chunks(0), sparse_chunks(1));
                let (ls, rs): (BTreeSet<u32>, BTreeSet<u32>) = (left.iter().copied().collect(), right.iter().copied().collect());
                for (label, policy) in [("chunked", IntSetPolicy::chunked()), ("roaring", IntSetPolicy::roaring())] {
                    let (a, b) = (build(&left, policy).unwrap(), build(&right, policy).unwrap());
                    for op in ["intersection", "union", "difference", "symmetric_difference"] {
                        let expected: Vec<u32> = match op {
                            "intersection" => &ls & &rs,
                            "union" => &ls | &rs,
                            "difference" => &ls - &rs,
                            _ => &ls ^ &rs,
                        }
                        .into_iter()
                        .collect();
                        assert!(apply(op, &a, &b).iter().eq(expected.iter().copied()));
                        let (a, b) = (a.clone(), b.clone());
                        cases.push((format!("series.sparse_containers.{label}.{op}"), Box::new(move || {
                            black_box(apply(op, black_box(&a), black_box(&b))).cardinality()
                        })));
                    }
                }
                // Ordered family cursors through `fold` adapters.
                let big: Vec<u64> = (0..100_000_u64).map(|v| v * 2).collect();
                let other: Vec<u64> = (0..100_000_u64).map(|v| v * 3).collect();
                let big_set: FrozenOrderedSet<u64> = big.iter().copied().collect();
                let other_set: FrozenOrderedSet<u64> = other.iter().copied().collect();
                let one_set: FrozenOrderedSet<u64> = [77_u64].into_iter().collect();
                let (bs, os): (BTreeSet<u64>, BTreeSet<u64>) = (big.iter().copied().collect(), other.iter().copied().collect());
                assert_eq!(big_set.union(&other_set).map(|v| *v).sum::<u64>(), (&bs | &os).iter().sum::<u64>());
                assert_eq!(big_set.difference(&other_set).map(|v| *v).sum::<u64>(), (&bs - &os).iter().sum::<u64>());
                assert_eq!(big_set.symmetric_difference(&other_set).map(|v| *v).sum::<u64>(), (&bs ^ &os).iter().sum::<u64>());
                let (a, b) = (big_set.clone(), one_set.clone());
                cases.push(("series.ordered.union_one.fold_sum".into(), Box::new(move || black_box(&a).union(black_box(&b)).map(|v| *v).sum())));
                let (a, b) = (big_set.clone(), one_set.clone());
                cases.push(("series.ordered.difference_one.fold_sum".into(), Box::new(move || black_box(&a).difference(black_box(&b)).map(|v| *v).sum())));
                let (a, b) = (big_set.clone(), one_set);
                cases.push(("series.ordered.union_one.collect".into(), Box::new(move || black_box(&a).union(black_box(&b)).copied().collect::<Vec<_>>().len() as u64)));
                let (a, b) = (big_set.clone(), other_set.clone());
                cases.push(("series.ordered.union_balanced.fold_sum".into(), Box::new(move || black_box(&a).union(black_box(&b)).map(|v| *v).sum())));
                let (a, b) = (big_set.clone(), other_set.clone());
                cases.push(("series.ordered.symmetric_difference_balanced.fold_sum".into(), Box::new(move || black_box(&a).symmetric_difference(black_box(&b)).map(|v| *v).sum())));
                let (a, b) = (big_set, other_set);
                cases.push(("series.ordered.intersection_balanced.fold_sum".into(), Box::new(move || black_box(&a).intersection(black_box(&b)).map(|v| *v).sum())));
            }

            /// Family and adaptive paths changed after the independent review:
            /// bulk extension, bulk ordered collection, and end queries.
            pub(super) fn families(cases: &mut Cases) {
                use $krate::adaptive::{AdaptiveHashMap, InlineVec};
                use $krate::family::{MutableHashSet, MutableOrderedMap, MutableOrderedSet};
                let mut state = 0xD1B5_4A32_D192_ED03_u64;
                let mut next = move || {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    state
                };
                let random: Vec<u64> = (0..200_000).map(|_| next()).collect();
                let mut sorted = random.clone();
                sorted.sort_unstable();
                let bytes = vec![7_u8; 1 << 20];
                let mut spilled: InlineVec<u8, 16> = InlineVec::new();
                spilled.extend_from_slice(&bytes);
                assert_eq!(spilled.len(), bytes.len());
                cases.push(("families.inline.extend_from_slice_spilled_1mib".into(), Box::new(move || {
                    let mut out: InlineVec<u8, 16> = InlineVec::new();
                    out.extend_from_slice(black_box(&bytes));
                    out.len() as u64
                })));
                let words: Vec<u64> = (0..1 << 17).collect();
                cases.push(("families.inline.extend_from_slice_spilled_u64".into(), Box::new(move || {
                    let mut out: InlineVec<u64, 8> = InlineVec::new();
                    out.extend_from_slice(black_box(&words));
                    out.len() as u64
                })));
                let input = random.clone();
                assert_eq!(input.iter().copied().collect::<MutableHashSet<u64>>().len(), 200_000);
                cases.push(("families.hash_set.from_iter_200k".into(), Box::new(move || {
                    black_box(&input).iter().copied().collect::<MutableHashSet<u64>>().len() as u64
                })));
                let input = random.clone();
                cases.push(("families.adaptive_hash_map.from_iter_200k".into(), Box::new(move || {
                    black_box(&input).iter().map(|&key| (key, key)).collect::<AdaptiveHashMap<u64, u64>>().len() as u64
                })));
                for (label, input) in [("random", random.clone()), ("sorted", sorted.clone())] {
                    let expected: BTreeSet<u64> = input.iter().copied().collect();
                    let map: MutableOrderedMap<u64, u64> = input.iter().map(|&key| (key, key)).collect();
                    assert!(map.keys().eq(expected.iter()));
                    let set: MutableOrderedSet<u64> = input.iter().copied().collect();
                    assert!(set.iter().eq(expected.iter()));
                    let pairs = input.clone();
                    cases.push((format!("families.ordered_map.from_iter_200k_{label}"), Box::new(move || {
                        black_box(&pairs).iter().map(|&key| (key, key)).collect::<MutableOrderedMap<u64, u64>>().len() as u64
                    })));
                    cases.push((format!("families.ordered_set.from_iter_200k_{label}"), Box::new(move || {
                        black_box(&input).iter().copied().collect::<MutableOrderedSet<u64>>().len() as u64
                    })));
                }
                let map: MutableOrderedMap<u64, u64> = sorted.iter().map(|&key| (key, key)).collect();
                assert_eq!(map.iter().last().map(|(key, _)| *key), sorted.last().copied());
                assert_eq!(map.keys().max(), sorted.last());
                let (a, b, c) = (map.clone(), map.clone(), map);
                cases.push(("families.ordered_map.iter_last_200k".into(), Box::new(move || *black_box(&a).iter().last().unwrap().0)));
                cases.push(("families.ordered_map.keys_max_200k".into(), Box::new(move || *black_box(&b).keys().max().unwrap())));
                cases.push(("families.ordered_map.iter_nth_100k".into(), Box::new(move || *black_box(&c).iter().nth(100_000).unwrap().0)));
            }

            /// Cases aimed at specific kernels: dense window arrangements and
            /// Roaring construction from run-shaped and sparse inputs.
            pub(super) fn kernels(cases: &mut Cases, reprs: &mut Reprs) {
                let dense = IntSetPolicy::dense();
                // 1M-bit windows: equal, nested, and offset arrangements.
                let full: Vec<u32> = (0..(1 << 20)).step_by(3).collect();
                let nested: Vec<u32> = ((1 << 18)..(1 << 19)).step_by(5).collect();
                let offset: Vec<u32> = ((1 << 19)..(3 << 19)).step_by(7).collect();
                let arrangements = [("equal", &full, &full), ("nested", &full, &nested), ("offset", &full, &offset)];
                for (shape, l, r) in arrangements {
                    let (a, b) = (build(l, dense).unwrap(), build(r, dense).unwrap());
                    let (sa, sb): (BTreeSet<u32>, BTreeSet<u32>) = (l.iter().copied().collect(), r.iter().copied().collect());
                    for op in ["union", "intersection", "difference", "symmetric_difference"] {
                        let result = apply(op, &a, &b);
                        let expected: Vec<u32> = match op {
                            "union" => &sa | &sb,
                            "intersection" => &sa & &sb,
                            "difference" => &sa - &sb,
                            _ => &sa ^ &sb,
                        }
                        .into_iter()
                        .collect();
                        assert_eq!(result.iter().collect::<Vec<_>>(), expected);
                        reprs.insert(format!("kernel.dense_{shape}.{op}"), repr(&result));
                        let (a, b) = (a.clone(), b.clone());
                        cases.push((format!("kernel.dense_{shape}.{op}"), Box::new(move || {
                            black_box(apply(op, black_box(&a), black_box(&b))).cardinality()
                        })));
                    }
                    let (a, b) = (a.clone(), b.clone());
                    assert_eq!(b.is_subset(&a), sb.is_subset(&sa));
                    cases.push((format!("kernel.dense_{shape}.is_subset"), Box::new(move || u64::from(black_box(&b).is_subset(black_box(&a))))));
                }
                // Roaring construction shapes.
                let long_run: Vec<u32> = (0..1_000_000).collect();
                let runs_of_64: Vec<u32> = (0..16_384_u32).flat_map(|r| (0..64).map(move |v| r * 128 + v)).collect();
                let runs_of_8: Vec<u32> = (0..65_536_u32).flat_map(|r| (0..8).map(move |v| r * 16 + v)).collect();
                let sparse: Vec<u32> = (0..200_000_u32).map(|i| i * 3).collect();
                let wide_runs: Vec<u64> = (0..64_u64).flat_map(|p| ((p << 32)..(p << 32) + 4096)).collect();
                for (label, input) in [("long_run_1m", &long_run), ("runs_of_64", &runs_of_64), ("runs_of_8", &runs_of_8), ("sparse_200k", &sparse)] {
                    let set = build(input, IntSetPolicy::roaring()).unwrap();
                    assert_eq!(set.iter().collect::<Vec<_>>(), *input);
                    reprs.insert(format!("kernel.roaring_construct.{label}"), repr(&set));
                    let input = input.clone();
                    cases.push((format!("kernel.roaring_construct.{label}"), Box::new(move || {
                        FrozenIntSet::try_from_iter(black_box(&input).iter().copied(), IntSetPolicy::roaring()).unwrap().cardinality()
                    })));
                }
                let set = build(&wide_runs, IntSetPolicy::roaring()).unwrap();
                assert_eq!(set.cardinality(), wide_runs.len() as u64);
                reprs.insert("kernel.roaring_construct.u64_partition_runs".into(), repr(&set));
                cases.push(("kernel.roaring_construct.u64_partition_runs".into(), Box::new(move || {
                    FrozenIntSet::try_from_iter(black_box(&wide_runs).iter().copied(), IntSetPolicy::roaring()).unwrap().cardinality()
                })));
                // Auto planning that ends in Roaring also pays construction.
                let auto_runs = build(&runs_of_64, IntSetPolicy::auto()).unwrap();
                reprs.insert("kernel.auto_construct.runs_of_64".into(), repr(&auto_runs));
                cases.push(("kernel.auto_construct.runs_of_64".into(), Box::new(move || {
                    FrozenIntSet::try_from_iter(black_box(&runs_of_64).iter().copied(), IntSetPolicy::auto()).unwrap().cardinality()
                })));
            }
        }
    };
}

implementation!(old, frost_baseline);
implementation!(new, frost);

enum Data {
    U32(Vec<u32>, Vec<u32>, Vec<u32>),
    U64(Vec<u64>, Vec<u64>, Vec<u64>),
    I64(Vec<i64>, Vec<i64>, Vec<i64>),
}

fn datasets(f: &mut dyn FnMut(&str, Data)) {
    let left32: Vec<u32> = (0..16_384).step_by(2).map(|v| 1_000_003 + v).collect();
    let right32: Vec<u32> = (0..16_384).step_by(3).map(|v| 1_000_034 + v).collect();
    let add32: Vec<u32> = (0..64).map(|v| 2_000_000 + v * 97).collect();
    f("u32.cluster", Data::U32(left32, right32, add32.clone()));
    let large: Vec<u32> = (0..65_536).collect();
    let small: Vec<u32> = (0..64).map(|v| v * 1024).collect();
    f("u32.skew", Data::U32(large, small, add32.clone()));
    let runs_left: Vec<u32> = (0..4).flat_map(|b| (0..1024).map(move |v| b * 65_536 + v)).collect();
    let runs_right: Vec<u32> = runs_left.iter().map(|v| v + 512).collect();
    f("u32.runs", Data::U32(runs_left, runs_right, add32));
    let base = 1_000_000_000_000_003_u64;
    let cl: Vec<u64> = (0..16_384).step_by(2).map(|v| base + v).collect();
    let cr: Vec<u64> = (0..16_384).step_by(3).map(|v| base + 79 + v).collect();
    let add64: Vec<u64> = (0..64).map(|v| base + 1_000_003 + v * 97).collect();
    f("u64.large_base", Data::U64(cl, cr, add64.clone()));
    let sl: Vec<u64> = (0..4096).map(|v| ((v * 104_729) << 32) | (v * 13)).collect();
    let sr: Vec<u64> = (0..4096).map(|v| ((v * 209_458) << 32) | (v * 26)).collect();
    f("u64.scattered_prefixes", Data::U64(sl, sr, add64));
    let il: Vec<i64> = (-8192..8192).step_by(2).collect();
    let ir: Vec<i64> = (-8161..8223).step_by(3).collect();
    let addi: Vec<i64> = (0..64).map(|v| 100_000 + v * 97).collect();
    f("i64.across_zero", Data::I64(il, ir, addi));
}

fn main() {
    let rounds: usize = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(3);
    let mut old_cases: Cases = Vec::new();
    let mut new_cases: Cases = Vec::new();
    let mut old_reprs = Reprs::new();
    let mut new_reprs = Reprs::new();
    datasets(&mut |name, data| match data {
        Data::U32(l, r, a) => {
            old::dataset(name, &l, &r, &a, &mut old_cases, &mut old_reprs);
            new::dataset(name, &l, &r, &a, &mut new_cases, &mut new_reprs);
        }
        Data::U64(l, r, a) => {
            old::dataset(name, &l, &r, &a, &mut old_cases, &mut old_reprs);
            new::dataset(name, &l, &r, &a, &mut new_cases, &mut new_reprs);
        }
        Data::I64(l, r, a) => {
            old::dataset(name, &l, &r, &a, &mut old_cases, &mut old_reprs);
            new::dataset(name, &l, &r, &a, &mut new_cases, &mut new_reprs);
        }
    });
    old::kernels(&mut old_cases, &mut old_reprs);
    new::kernels(&mut new_cases, &mut new_reprs);
    old::series(&mut old_cases, &mut old_reprs);
    new::series(&mut new_cases, &mut new_reprs);
    old::families(&mut old_cases);
    new::families(&mut new_cases);
    let _ = (old::policies(), new::policies());
    let mut old_map: BTreeMap<String, Kernel> = old_cases.into_iter().collect();
    let mut new_map: BTreeMap<String, Kernel> = new_cases.into_iter().collect();
    let filter = std::env::var("AB_FILTER").unwrap_or_default();
    let names: Vec<String> = new_map
        .keys()
        .filter(|name| filter.is_empty() || filter.split(',').any(|f| name.contains(f)))
        .cloned()
        .collect();
    println!("case,old_min_ns,new_min_ns,old_over_new,old_repr,new_repr");
    for name in names {
        let mut new_kernel = new_map.remove(&name).expect("listed case");
        let mut old_kernel = old_map.remove(&name);
        let mut old_best = f64::INFINITY;
        let mut new_best = f64::INFINITY;
        for _ in 0..rounds {
            if let Some(kernel) = old_kernel.as_mut() {
                old_best = old_best.min(median_ns(kernel));
            }
            new_best = new_best.min(median_ns(&mut new_kernel));
        }
        let old_repr = old_reprs.get(&name).cloned().unwrap_or_default();
        let new_repr = new_reprs.get(&name).cloned().unwrap_or_default();
        if old_kernel.is_some() {
            println!("{name},{old_best:.1},{new_best:.1},{:.3},{old_repr},{new_repr}", old_best / new_best);
        } else {
            println!("{name},,{new_best:.1},new,,{new_repr}");
        }
    }
    eprintln!("done");
}

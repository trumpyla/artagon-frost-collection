//! Same-binary A/B harness: baseline (`frost_baseline`) vs revised (`frost`)
//! integer profile on identical datasets. Every case validates its answer, then
//! both implementations are measured alternately (old, new, old, new); the
//! reported figure is the minimum of the per-round medians.

use std::collections::{BTreeMap, BTreeSet};
use std::hint::black_box;
use std::time::{Duration, Instant};

type Kernel = Box<dyn FnMut() -> u64>;
type Cases = Vec<(String, Kernel)>;

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
    for s in &mut samples {
        *s = elapsed(iterations, kernel).as_secs_f64() * 1e9 / f64::from(iterations);
    }
    samples.sort_by(f64::total_cmp);
    samples[SAMPLES / 2]
}

macro_rules! implementation {
    ($module:ident, $krate:ident, $policies:expr) => {
        mod $module {
            use super::*;
            use $krate::integer::{FrozenIntSet, IntKey, IntSetError, IntSetPolicy};

            pub(super) fn policies() -> Vec<(&'static str, IntSetPolicy)> {
                $policies
            }

            fn build<K: IntKey>(values: &[K], policy: IntSetPolicy) -> Option<FrozenIntSet<K>> {
                match FrozenIntSet::try_from_iter(values.iter().copied(), policy) {
                    Ok(set) => Some(set),
                    Err(IntSetError::DenseBudgetExceeded { .. }) => None,
                    Err(error) => panic!("construction failed: {error}"),
                }
            }

            pub(super) fn dataset<K: IntKey + std::fmt::Debug + 'static>(
                name: &str,
                left: &[K],
                right: &[K],
                additions: &[K],
                cases: &mut Cases,
                reprs: &mut BTreeMap<String, String>,
            ) {
                let left_std: BTreeSet<K> = left.iter().copied().collect();
                let right_std: BTreeSet<K> = right.iter().copied().collect();
                let expected = [
                    left_std.intersection(&right_std).copied().collect::<Vec<_>>(),
                    left_std.union(&right_std).copied().collect::<Vec<_>>(),
                    left_std.difference(&right_std).copied().collect::<Vec<_>>(),
                    left_std.symmetric_difference(&right_std).copied().collect::<Vec<_>>(),
                ];
                for (label, policy) in policies() {
                    let Some(l) = build(left, policy) else { continue };
                    let Some(r) = build(right, policy) else { continue };
                    reprs.insert(format!("{name}.left.{label}"), format!("{:?}", l.representation()));
                    let input = left.to_vec();
                    cases.push((format!("{name}.{label}.construct"), Box::new(move || {
                        FrozenIntSet::try_from_iter(black_box(&input).iter().copied(), policy).unwrap().cardinality()
                    })));
                    let (a, b) = (l.clone(), r.clone());
                    cases.push((format!("{name}.{label}.intersection_count"), Box::new(move || black_box(&a).intersection_len(black_box(&b)))));
                    for (op, idx) in [("intersection", 0), ("union", 1), ("difference", 2), ("symmetric_difference", 3)] {
                        let (a, b) = (l.clone(), r.clone());
                        let result = match op {
                            "intersection" => a.intersection(&b),
                            "union" => a.union(&b),
                            "difference" => a.difference(&b),
                            _ => a.symmetric_difference(&b),
                        };
                        assert_eq!(result.iter().collect::<Vec<_>>(), expected[idx], "{name}.{label}.{op}");
                        reprs.insert(format!("{name}.{label}.{op}"), format!("{:?}", result.representation()));
                        cases.push((format!("{name}.{label}.{op}_materialize"), Box::new(move || {
                            let result = match op {
                                "intersection" => black_box(&a).intersection(black_box(&b)),
                                "union" => black_box(&a).union(black_box(&b)),
                                "difference" => black_box(&a).difference(black_box(&b)),
                                _ => black_box(&a).symmetric_difference(black_box(&b)),
                            };
                            black_box(result).cardinality()
                        })));
                    }
                    // Snapshot pattern, clone, equality.
                    let singletons: Vec<_> = additions.iter().map(|&v| FrozenIntSet::try_from_iter([v], IntSetPolicy::sorted()).unwrap()).collect();
                    let mut expect: BTreeSet<K> = left_std.clone();
                    expect.extend(additions.iter().copied());
                    let mut current = l.clone();
                    for s in &singletons { current = current.union(s); }
                    assert_eq!(current.iter().collect::<Vec<_>>(), expect.iter().copied().collect::<Vec<_>>());
                    reprs.insert(format!("{name}.{label}.snapshot"), format!("{:?}", current.representation()));
                    let base = l.clone();
                    cases.push((format!("{name}.{label}.snapshot_update"), Box::new(move || {
                        let mut current = black_box(&base).clone();
                        for s in black_box(&singletons) { current = current.union(s); }
                        current.cardinality()
                    })));
                    let base = l.clone();
                    cases.push((format!("{name}.{label}.clone"), Box::new(move || black_box(black_box(&base).clone()).cardinality())));
                    let twin = build(left, policy).unwrap();
                    let base = l.clone();
                    assert!(base == twin);
                    cases.push((format!("{name}.{label}.equality"), Box::new(move || u64::from(black_box(&base) == black_box(&twin)))));
                }
                // Mixed pairs over the explicit policies both versions share.
                let mixed: Vec<(&str, IntSetPolicy)> = policies().into_iter().filter(|(l, _)| *l != "auto").collect();
                for (ll, lp) in &mixed {
                    for (rl, rp) in &mixed {
                        let Some(l) = build(left, *lp) else { continue };
                        let Some(r) = build(right, *rp) else { continue };
                        let (a, b) = (l.clone(), r.clone());
                        cases.push((format!("{name}.mixed_{ll}_{rl}.intersection_count"), Box::new(move || black_box(&a).intersection_len(black_box(&b)))));
                        for (op, idx) in [("intersection", 0), ("union", 1), ("difference", 2)] {
                            let (a, b) = (l.clone(), r.clone());
                            let result = match op { "intersection" => a.intersection(&b), "union" => a.union(&b), _ => a.difference(&b) };
                            assert_eq!(result.iter().collect::<Vec<_>>(), expected[idx]);
                            reprs.insert(format!("{name}.mixed_{ll}_{rl}.{op}"), format!("{:?}", result.representation()));
                            cases.push((format!("{name}.mixed_{ll}_{rl}.{op}_materialize"), Box::new(move || {
                                let result = match op {
                                    "intersection" => black_box(&a).intersection(black_box(&b)),
                                    "union" => black_box(&a).union(black_box(&b)),
                                    _ => black_box(&a).difference(black_box(&b)),
                                };
                                black_box(result).cardinality()
                            })));
                        }
                    }
                }
            }

            /// Cases from the adversarial review, exercising the fixed paths directly.
            pub(super) fn review(cases: &mut Cases, reprs: &mut BTreeMap<String, String>) {
                let base = 1_u64 << 40;
                let big: Vec<u64> = (0..1_000_000_u64).map(|i| base + i * 2).collect();
                let dense = build(&big, IntSetPolicy::dense()).unwrap();
                let singleton = build(&[base + 1], IntSetPolicy::roaring()).unwrap();
                let union = dense.union(&singleton);
                assert_eq!(union.cardinality(), 1_000_001);
                reprs.insert("review.dense1m_union_roaring_singleton".into(), format!("{:?}", union.representation()));
                let (a, b) = (dense.clone(), singleton.clone());
                cases.push(("review.dense1m_union_roaring_singleton".into(), Box::new(move || black_box(black_box(&a).union(black_box(&b))).cardinality())));
                let tiny = build(&[base + 2, base + 4, base + 6, base + 7], IntSetPolicy::sorted()).unwrap();
                let difference = dense.difference(&tiny);
                assert_eq!(difference.cardinality(), 999_997);
                reprs.insert("review.dense1m_difference_sorted4".into(), format!("{:?}", difference.representation()));
                let (a, b) = (dense.clone(), tiny.clone());
                cases.push(("review.dense1m_difference_sorted4".into(), Box::new(move || black_box(black_box(&a).difference(black_box(&b))).cardinality())));
                let island_a: Vec<u64> = (0..(1_u64 << 20)).collect();
                let island_b: Vec<u64> = ((1_u64 << 27)..((1_u64 << 27) + (1 << 20))).collect();
                let (ia, ib) = (build(&island_a, IntSetPolicy::dense()).unwrap(), build(&island_b, IntSetPolicy::dense()).unwrap());
                let far = ia.union(&ib);
                assert_eq!(far.cardinality(), 2 << 20);
                reprs.insert("review.far_apart_dense_union".into(), format!("{:?} payload={}", far.representation(), payload(&far)));
                cases.push(("review.far_apart_dense_union".into(), Box::new(move || black_box(black_box(&ia).union(black_box(&ib))).cardinality())));
                let big32: Vec<u32> = (0..200_000_u32).map(|i| i * 3).collect();
                for (label, policy) in [("roaring", IntSetPolicy::roaring()), ("sorted", IntSetPolicy::sorted()), ("dense", IntSetPolicy::dense())] {
                    let x = build(&big32, policy).unwrap();
                    let y = build(&big32, policy).unwrap();
                    assert!(x == y);
                    cases.push((format!("review.equality_200k_{label}"), Box::new(move || u64::from(black_box(&x) == black_box(&y)))));
                    let input = big32.clone();
                    cases.push((format!("review.construct_200k_{label}"), Box::new(move || FrozenIntSet::try_from_iter(black_box(&input).iter().copied(), policy).unwrap().cardinality())));
                }
                let run: Vec<u32> = (0..1000).collect();
                let auto = build(&run, IntSetPolicy::auto()).unwrap();
                reprs.insert("review.run1000_auto".into(), format!("{:?}", auto.representation()));
                let probes: Vec<u32> = (0..1000).map(|i| (i * 7919) % 1100).collect();
                cases.push(("review.run1000_auto_contains_x1000".into(), Box::new(move || black_box(&probes).iter().filter(|p| auto.contains(p)).count() as u64)));
            }

            #[allow(unused_variables)]
            fn payload<K: IntKey>(set: &FrozenIntSet<K>) -> u64 {
                payload_of(set)
            }
            $crate::payload_impl!($module, $krate);
        }
    };
}

#[macro_export]
macro_rules! payload_impl {
    (old, $krate:ident) => {
        fn payload_of<K: IntKey>(set: &FrozenIntSet<K>) -> u64 { set.cardinality() * 8 }
    };
    (new, $krate:ident) => {
        fn payload_of<K: IntKey>(set: &FrostIntSetAlias<K>) -> u64 { set.payload_bytes() }
        type FrostIntSetAlias<K> = FrozenIntSet<K>;
    };
}

implementation!(old, frost_baseline, vec![
    ("auto", IntSetPolicy::auto()),
    ("sorted", IntSetPolicy::sorted()),
    ("dense", IntSetPolicy::dense()),
    ("roaring", IntSetPolicy::roaring()),
]);
implementation!(new, frost, vec![
    ("auto", IntSetPolicy::auto()),
    ("sorted", IntSetPolicy::sorted()),
    ("dense", IntSetPolicy::dense()),
    ("roaring", IntSetPolicy::roaring()),
    ("chunked", IntSetPolicy::chunked()),
]);

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

enum Data {
    U32(Vec<u32>, Vec<u32>, Vec<u32>),
    U64(Vec<u64>, Vec<u64>, Vec<u64>),
    I64(Vec<i64>, Vec<i64>, Vec<i64>),
}

fn main() {
    let rounds: usize = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(2);
    let mut old_cases: Cases = Vec::new();
    let mut new_cases: Cases = Vec::new();
    let mut old_reprs = BTreeMap::new();
    let mut new_reprs = BTreeMap::new();
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
    old::review(&mut old_cases, &mut old_reprs);
    new::review(&mut new_cases, &mut new_reprs);
    let _ = old::policies();
    let mut old_map: BTreeMap<String, Kernel> = old_cases.into_iter().collect();
    let mut new_map: BTreeMap<String, Kernel> = new_cases.into_iter().collect();
    println!("case,old_min_ns,new_min_ns,old_over_new,old_repr,new_repr");
    let filter = std::env::var("AB_FILTER").unwrap_or_default();
    let names: Vec<String> = new_map
        .keys()
        .filter(|name| filter.is_empty() || filter.split(',').any(|f| name.contains(f)))
        .cloned()
        .collect();
    for name in names {
        let mut new_kernel = new_map.remove(&name).unwrap();
        let mut old_kernel = old_map.remove(&name);
        let mut old_best = f64::INFINITY;
        let mut new_best = f64::INFINITY;
        for _ in 0..rounds {
            if let Some(k) = old_kernel.as_mut() {
                old_best = old_best.min(median_ns(k));
            }
            new_best = new_best.min(median_ns(&mut new_kernel));
        }
        let repr_key = name.rsplit_once('.').map(|(prefix, _)| prefix.to_string()).unwrap_or_default();
        let op_key = name.trim_end_matches("_materialize").trim_end_matches("_count").to_string();
        let old_repr = old_reprs.get(&op_key).or_else(|| old_reprs.get(&name)).or_else(|| old_reprs.get(&repr_key)).cloned().unwrap_or_default();
        let new_repr = new_reprs.get(&op_key).or_else(|| new_reprs.get(&name)).or_else(|| new_reprs.get(&repr_key)).cloned().unwrap_or_default();
        if old_kernel.is_some() {
            println!("{name},{old_best:.1},{new_best:.1},{:.3},{old_repr},{new_repr}", old_best / new_best);
        } else {
            println!("{name},,{new_best:.1},new,,{new_repr}");
        }
    }
    eprintln!("done");
}

use frost::integer::{FrozenIntSet, IntSetPolicy};
use std::hint::black_box;
use std::time::Instant;

fn time<T>(label: &str, iters: u32, mut f: impl FnMut() -> T) -> T {
    let mut out = None;
    let start = Instant::now();
    for _ in 0..iters { out = Some(black_box(f())); }
    println!("  {label:<62} {:>10.1} ns/op", start.elapsed().as_nanos() as f64 / iters as f64);
    out.unwrap()
}

/// Three-region dense union: left-only prefix, overlap, right-only suffix; fused popcount.
fn union3(a_base: u64, a: &[u64], b_base: u64, b: &[u64]) -> (u64, Vec<u64>, u64) {
    let (a_s, b_s) = (a_base / 64, b_base / 64);
    let (a_e, b_e) = (a_s + a.len() as u64, b_s + b.len() as u64);
    let start = a_s.min(b_s); let end = a_e.max(b_e);
    let mut out = Vec::with_capacity((end - start) as usize);
    let mut card = 0u64;
    let mut block = start;
    // prefix
    let (first, first_s, first_e) = if a_s <= b_s { (a, a_s, a_e) } else { (b, b_s, b_e) };
    let (second, second_s, second_e) = if a_s <= b_s { (b, b_s, b_e) } else { (a, a_s, a_e) };
    let prefix_len = (second_s.min(first_e) - first_s) as usize;
    for &w in &first[..prefix_len] { card += w.count_ones() as u64; out.push(w); }
    block += prefix_len as u64;
    if second_s > first_e { // gap
        let gap = (second_s - first_e) as usize; out.resize(out.len() + gap, 0); block += gap as u64;
    }
    // overlap
    let ov_end = first_e.min(second_e);
    if block < ov_end {
        let n = (ov_end - block) as usize;
        let fa = &first[(block - first_s) as usize..][..n];
        let sb = &second[(block - second_s) as usize..][..n];
        for (x, y) in fa.iter().zip(sb) { let w = x | y; card += w.count_ones() as u64; out.push(w); }
        block += n as u64;
    }
    // suffix
    let (tail, tail_s) = if first_e > second_e { (first, first_s) } else { (second, second_s) };
    let n = (end - block) as usize;
    for &w in &tail[(block - tail_s) as usize..][..n] { card += w.count_ones() as u64; out.push(w); }
    (start * 64, out, card)
}

fn main() {
    println!("== Attainable mixed-op cost: clone dense + flip bits ==");
    let base = 1u64 << 40;
    let big: Vec<u64> = (0..1_000_000u64).map(|i| base + i * 2).collect();
    let big_dense = FrozenIntSet::try_from_iter(big.iter().copied(), IntSetPolicy::auto()).unwrap();
    let singleton = FrozenIntSet::try_from_iter([base + 1], IntSetPolicy::roaring()).unwrap();
    time("frost big_dense ∪ roaring{1} (→ Sorted, 7.8 MiB)", 20, || big_dense.union(&singleton));
    let words: Vec<u64> = { let mut w = vec![0u64; 2_000_000 / 64 + 1]; for &v in &big { let o = v - base; w[(o >> 6) as usize] |= 1 << (o & 63); } w };
    time("prototype: clone 244 KiB window + set 1 bit (→ Dense)", 2000, || { let mut w = words.clone(); w[0] |= 2; w });

    println!("\n== Dense∪Dense kernel: frost vs three-region fused prototype ==");
    let d1v: Vec<u32> = (0..2_000_000u32).filter(|i| i % 3 != 0).collect();
    let d2v: Vec<u32> = (0..2_000_000u32).filter(|i| i % 5 != 0).collect();
    let d1 = FrozenIntSet::try_from_iter(d1v.iter().copied(), IntSetPolicy::dense()).unwrap();
    let d2 = FrozenIntSet::try_from_iter(d2v.iter().copied(), IntSetPolicy::dense()).unwrap();
    let w1: Vec<u64> = { let mut w = vec![0u64; 2_000_000 / 64 + 1]; for &v in &d1v { w[(v >> 6) as usize] |= 1 << (v & 63); } w };
    let w2: Vec<u64> = { let mut w = vec![0u64; 2_000_000 / 64 + 1]; for &v in &d2v { w[(v >> 6) as usize] |= 1 << (v & 63); } w };
    let u = time("frost Dense∪Dense (materialized, incl. cardinality)", 200, || d1.union(&d2));
    let (_, out, card) = time("prototype union3 (alloc + OR + fused popcount)", 200, || union3(0, &w1, 0, &w2));
    assert_eq!(card, u.cardinality()); assert_eq!(out.len(), w1.len());
    // offset bases to exercise prefix/suffix paths
    let (_, out, card) = time("prototype union3 with shifted bases (prefix+overlap+suffix)", 200, || union3(0, &w1, 64 * 1000, &w2));
    black_box((out, card));
}

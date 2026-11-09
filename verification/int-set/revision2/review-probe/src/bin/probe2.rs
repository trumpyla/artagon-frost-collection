use frost::integer::{FrozenIntSet, IntSetPolicy};
use roaring::RoaringBitmap;
use std::hint::black_box;
use std::time::Instant;

fn time<T>(label: &str, iters: u32, mut f: impl FnMut() -> T) -> T {
    let mut out = None;
    let start = Instant::now();
    for _ in 0..iters { out = Some(black_box(f())); }
    println!("  {label:<62} {:>10.1} ns/op", start.elapsed().as_nanos() as f64 / iters as f64);
    out.unwrap()
}

fn skip_below(items: &[u32], target: u32) -> &[u32] {
    if items.last().is_none_or(|&l| l < target) { return &[]; }
    let mut lower = 1; let mut upper = 1;
    while upper < items.len() && items[upper] < target { lower = upper + 1; upper = upper.saturating_mul(2).min(items.len()); }
    let off = items[lower..upper].partition_point(|&x| x < target);
    &items[lower + off..]
}

fn gallop(sorted: &[u32], roar: &RoaringBitmap) -> u64 {
    let mut s = sorted; let mut it = roar.iter(); let mut count = 0;
    let mut cur = it.next();
    while let (Some(&a), Some(b)) = (s.first(), cur) {
        if a == b { count += 1; s = &s[1..]; cur = it.next(); }
        else if a < b { s = skip_below(s, b); }
        else { it.advance_to(a); cur = it.next(); }
    }
    count
}

fn via_bits(sorted: &[u32], roar: &RoaringBitmap) -> u64 {
    let base = sorted[0] & !63; let last = *sorted.last().unwrap();
    let mut bits = vec![0u64; ((last - base) / 64 + 1) as usize];
    for &v in sorted { let o = v - base; bits[(o >> 6) as usize] |= 1 << (o & 63); }
    let bytes: Vec<u8> = bits.iter().flat_map(|w| w.to_le_bytes()).collect();
    RoaringBitmap::from_lsb0_bytes(base, &bytes).intersection_len(roar)
}

fn main() {
    println!("target features: popcnt={} avx2={}", cfg!(target_feature = "popcnt"), cfg!(target_feature = "avx2"));
    let base = 1_000_000u32;
    let left: Vec<u32> = (0..8192).map(|i| base + i * 2).collect();
    let right: Vec<u32> = (0..5462).map(|i| base + i * 3 + 1).collect();
    let l_sorted = FrozenIntSet::try_from_iter(left.iter().copied(), IntSetPolicy::sorted()).unwrap();
    let r_roar = FrozenIntSet::try_from_iter(right.iter().copied(), IntSetPolicy::roaring()).unwrap();
    let r_native = RoaringBitmap::from_sorted_iter(right.iter().copied()).unwrap();
    let expect = l_sorted.intersection_len(&r_roar);
    println!("\n== Sorted∩Roaring balanced (8192 vs 5462) ==");
    time("frost (probe sorted into roaring.contains)", 2000, || l_sorted.intersection_len(&r_roar));
    assert_eq!(gallop(&left, &r_native), expect);
    time("prototype: galloping via skip_below + Iter::advance_to", 2000, || gallop(&left, &r_native));
    assert_eq!(via_bits(&left, &r_native), expect);
    time("prototype: sorted→bits→from_lsb0_bytes + native", 2000, || via_bits(&left, &r_native));

    println!("\n== Sorted∩Roaring skewed (65536 sorted vs 64 roaring) ==");
    let big: Vec<u32> = (0..65536u32).collect();
    let small: Vec<u32> = (0..64u32).map(|i| i * 1024).collect();
    let bs = FrozenIntSet::try_from_iter(big.iter().copied(), IntSetPolicy::sorted()).unwrap();
    let sr = FrozenIntSet::try_from_iter(small.iter().copied(), IntSetPolicy::roaring()).unwrap();
    let srn = RoaringBitmap::from_sorted_iter(small.iter().copied()).unwrap();
    let e = bs.intersection_len(&sr);
    time("frost", 5000, || bs.intersection_len(&sr));
    assert_eq!(gallop(&big, &srn), e);
    time("prototype: galloping", 5000, || gallop(&big, &srn));
    println!("\n== Sorted∩Roaring skewed reversed (64 sorted vs 65536 roaring) ==");
    let ss = FrozenIntSet::try_from_iter(small.iter().copied(), IntSetPolicy::sorted()).unwrap();
    let br = FrozenIntSet::try_from_iter(big.iter().copied(), IntSetPolicy::roaring()).unwrap();
    let brn = RoaringBitmap::from_sorted_iter(big.iter().copied()).unwrap();
    let e = ss.intersection_len(&br);
    time("frost", 5000, || ss.intersection_len(&br));
    assert_eq!(gallop(&small, &brn), e);
    time("prototype: galloping", 5000, || gallop(&small, &brn));

    println!("\n== Dense kernels, this build's target features ==");
    let d1 = FrozenIntSet::try_from_iter((0..2_000_000u32).filter(|i| i % 3 != 0), IntSetPolicy::dense()).unwrap();
    let d2 = FrozenIntSet::try_from_iter((0..2_000_000u32).filter(|i| i % 5 != 0), IntSetPolicy::dense()).unwrap();
    time("frost Dense∩Dense intersection_len (31250 words)", 500, || d1.intersection_len(&d2));
    time("frost Dense∪Dense union (materialized)", 200, || d1.union(&d2));
    time("frost Dense is_subset (31250 words)", 500, || d1.is_subset(&d2));
    let w1: Vec<u64> = (0..31250u64).map(|i| i.wrapping_mul(0x9E3779B97F4A7C15)).collect();
    let w2: Vec<u64> = (0..31250u64).map(|i| i.wrapping_mul(0xC2B2AE3D27D4EB4F)).collect();
    time("reference zip(&).count_ones.sum", 500, || w1.iter().zip(&w2).map(|(a, b)| (a & b).count_ones() as u64).sum::<u64>());
    time("reference zip(|).collect", 200, || w1.iter().zip(&w2).map(|(a, b)| a | b).collect::<Vec<u64>>());
    time("reference zip(&!).all(==0)", 500, || w1.iter().zip(&w2).all(|(a, b)| a & !b == 0));
}

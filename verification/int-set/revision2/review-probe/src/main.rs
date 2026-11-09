use frost::integer::{FrozenIntSet, IntSetPolicy, IntSetRepr};
use roaring::RoaringBitmap;
use std::hint::black_box;
use std::time::Instant;

fn time<T>(label: &str, iters: u32, mut f: impl FnMut() -> T) -> T {
    let mut out = None;
    let start = Instant::now();
    for _ in 0..iters {
        out = Some(black_box(f()));
    }
    let per = start.elapsed().as_nanos() as f64 / iters as f64;
    println!("  {label:<58} {per:>12.1} ns/op");
    out.unwrap()
}

fn repr(s: &FrozenIntSet<u32>) -> IntSetRepr { s.representation() }
fn repr64(s: &FrozenIntSet<u64>) -> IntSetRepr { s.representation() }

fn main() {
    println!("\n== 1. Auto planner on a consecutive run 0..1000 (u32) ==");
    let run: Vec<u32> = (0..1000).collect();
    let auto = FrozenIntSet::try_from_iter(run.iter().copied(), IntSetPolicy::auto()).unwrap();
    let dense = FrozenIntSet::try_from_iter(run.iter().copied(), IntSetPolicy::dense()).unwrap();
    println!("  auto picked {:?}; dense payload would be {} bytes", repr(&auto), (1000 / 64 + 1) * 8);
    let probe: Vec<u32> = (0..1000).map(|i| (i * 7919) % 1100).collect();
    time("contains x1000 (auto=Roaring)", 2000, || probe.iter().filter(|p| auto.contains(p)).count());
    time("contains x1000 (forced Dense)", 2000, || probe.iter().filter(|p| dense.contains(p)).count());
    let other_run: FrozenIntSet<u32> = (500..1500).collect();
    println!("  other auto repr {:?}", repr(&other_run));
    time("intersection_len auto∩auto", 20000, || auto.intersection_len(&other_run));
    let other_dense = FrozenIntSet::try_from_iter(500..1500u32, IntSetPolicy::dense()).unwrap();
    time("intersection_len dense∩dense", 20000, || dense.intersection_len(&other_dense));

    println!("\n== 2. Mixed-representation algebra collapses to Sorted ==");
    let base = 1u64 << 40;
    let big: Vec<u64> = (0..1_000_000u64).map(|i| base + i * 2).collect(); // 2M-wide window, 1M members
    let big_dense = FrozenIntSet::try_from_iter(big.iter().copied(), IntSetPolicy::auto()).unwrap();
    println!("  big: repr {:?}, cardinality {}, dense payload ≈ {} KiB", repr64(&big_dense), big_dense.cardinality(), (2_000_000 / 64) * 8 / 1024);
    let singleton = FrozenIntSet::try_from_iter([base + 1], IntSetPolicy::roaring()).unwrap();
    let u = time("big_dense ∪ roaring{1}", 20, || big_dense.union(&singleton));
    println!("  -> result repr {:?}; sorted payload = {} KiB", repr64(&u), u.cardinality() * 8 / 1024);
    let tiny_sorted = FrozenIntSet::try_from_iter([base + 2, base + 4, base + 6], IntSetPolicy::sorted()).unwrap();
    let d = time("big_dense ∖ sorted{3}", 20, || big_dense.difference(&tiny_sorted));
    println!("  -> result repr {:?}; sorted payload = {} KiB", repr64(&d), d.cardinality() * 8 / 1024);
    // what the snapshot-store test does: repeated union with singletons
    let mut current = big_dense.clone();
    for step in 0..3u64 {
        let s = FrozenIntSet::try_from_iter([base + 3_000_000 + step], IntSetPolicy::roaring()).unwrap();
        current = current.union(&s);
        println!("  after update {step}: repr {:?}", repr64(&current));
    }

    println!("\n== 3. Dense∪Dense beyond the window cap falls back to Sorted, not Roaring ==");
    let a: Vec<u64> = (0..(4u64 << 20)).collect();                  // 4M consecutive at 0   (512 KiB dense)
    let b: Vec<u64> = ((1u64 << 27)..((1u64 << 27) + (4u64 << 20))).collect(); // 4M consecutive at 2^27
    let a_set = FrozenIntSet::try_from_iter(a.iter().copied(), IntSetPolicy::dense()).unwrap();
    let b_set = FrozenIntSet::try_from_iter(b.iter().copied(), IntSetPolicy::dense()).unwrap();
    println!("  a repr {:?}, b repr {:?} (each 512 KiB payload); union window = 16 MiB > 8 MiB cap", repr64(&a_set), repr64(&b_set));
    let start = Instant::now();
    let ab = a_set.union(&b_set);
    let el = start.elapsed();
    println!("  union took {:?}; result repr {:?}; sorted payload = {} MiB", el, repr64(&ab), ab.cardinality() * 8 / (1024 * 1024));
    let start = Instant::now();
    let ra = roaring::RoaringTreemap::from_sorted_iter(a.iter().copied()).unwrap();
    let rb = roaring::RoaringTreemap::from_sorted_iter(b.iter().copied()).unwrap();
    let mut rab = &ra | &rb; rab.optimize();
    println!("  roaring equivalent built+unioned in {:?}; serialized size ≈ {} bytes", start.elapsed(), rab.serialized_size());

    println!("\n== 4. Operation window cap ignores the constructing policy ==");
    let lo = FrozenIntSet::try_from_iter([0u64, 5], IntSetPolicy::dense().with_dense_budget(8)).unwrap();
    let hi = FrozenIntSet::try_from_iter([1u64 << 20, (1u64 << 20) + 5], IntSetPolicy::dense().with_dense_budget(8)).unwrap();
    let w = lo.union(&hi);
    println!("  both built with 8-byte budget; union repr {:?}, window payload = {} KiB", repr64(&w), (1u64 << 20) / 64 * 8 / 1024);

    println!("\n== 5. Sorted/Roaring and Dense/Roaring: probing vs promotion/merge ==");
    let base = 1_000_000u32;
    let left: Vec<u32> = (0..8192).map(|i| base + i * 2).collect();
    let right: Vec<u32> = (0..5462).map(|i| base + i * 3 + 1).collect();
    let l_sorted = FrozenIntSet::try_from_iter(left.iter().copied(), IntSetPolicy::sorted()).unwrap();
    let l_dense = FrozenIntSet::try_from_iter(left.iter().copied(), IntSetPolicy::dense()).unwrap();
    let r_roar = FrozenIntSet::try_from_iter(right.iter().copied(), IntSetPolicy::roaring()).unwrap();
    let r_roar_native = RoaringBitmap::from_sorted_iter(right.iter().copied()).unwrap();
    let expect = l_sorted.intersection_len(&r_roar);
    time("frost Sorted∩Roaring intersection_len", 2000, || l_sorted.intersection_len(&r_roar));
    let n = time("prototype: merge sorted slice with roaring iter", 2000, || {
        let mut it = r_roar_native.iter();
        let mut s = &left[..];
        let mut count = 0u64;
        let mut cur = it.next();
        while let (Some(&a), Some(b)) = (s.first(), cur) {
            match a.cmp(&b) {
                std::cmp::Ordering::Less => s = &s[1..],
                std::cmp::Ordering::Greater => cur = it.next(),
                std::cmp::Ordering::Equal => { count += 1; s = &s[1..]; cur = it.next(); }
            }
        }
        count
    });
    assert_eq!(n, expect);
    let n = time("prototype: promote sorted→Roaring (from_sorted_iter) + native ∩len", 2000, || {
        let promoted = RoaringBitmap::from_sorted_iter(left.iter().copied()).unwrap();
        promoted.intersection_len(&r_roar_native)
    });
    assert_eq!(n, expect);
    time("frost Dense∩Roaring intersection_len", 2000, || l_dense.intersection_len(&r_roar));
    // dense window as raw bytes, promoted via from_lsb0_bytes
    let dbase = (base & !63) as u32;
    let words = ((left.last().unwrap() - dbase) / 64 + 1) as usize;
    let mut bits = vec![0u64; words];
    for &v in &left { let o = v - dbase; bits[(o / 64) as usize] |= 1 << (o % 64); }
    let bytes: Vec<u8> = bits.iter().flat_map(|w| w.to_le_bytes()).collect();
    let n = time("prototype: promote dense→Roaring (from_lsb0_bytes) + native ∩len", 2000, || {
        let promoted = RoaringBitmap::from_lsb0_bytes(dbase, &bytes);
        promoted.intersection_len(&r_roar_native)
    });
    assert_eq!(n, expect);
    let l_roar_native = RoaringBitmap::from_sorted_iter(left.iter().copied()).unwrap();
    time("native Roaring∩Roaring intersection_len (reference)", 2000, || l_roar_native.intersection_len(&r_roar_native));

    println!("\n== 6. Roaring construction: collect()+optimize vs from_sorted_iter ==");
    let big32: Vec<u32> = (0..200_000u32).map(|i| i * 3).collect();
    time("collect::<RoaringBitmap>() + optimize", 50, || { let mut r: RoaringBitmap = big32.iter().copied().collect(); r.optimize(); r });
    time("RoaringBitmap::from_sorted_iter + optimize", 50, || { let mut r = RoaringBitmap::from_sorted_iter(big32.iter().copied()).unwrap(); r.optimize(); r });
    let _ = time("frost try_from_iter(.., roaring())", 50, || FrozenIntSet::try_from_iter(big32.iter().copied(), IntSetPolicy::roaring()).unwrap());
    let _ = time("frost try_from_iter(.., sorted())", 50, || FrozenIntSet::try_from_iter(big32.iter().copied(), IntSetPolicy::sorted()).unwrap());

    println!("\n== 7. Equality: element-wise vs native ==");
    let x = FrozenIntSet::try_from_iter(big32.iter().copied(), IntSetPolicy::roaring()).unwrap();
    let y = x.clone();
    time("frost FrozenIntSet == (Roaring, Roaring)", 200, || x == y);
    let xn = RoaringBitmap::from_sorted_iter(big32.iter().copied()).unwrap();
    let yn = xn.clone();
    time("native RoaringBitmap ==", 200, || xn == yn);
    let xs = FrozenIntSet::try_from_iter(big32.iter().copied(), IntSetPolicy::sorted()).unwrap();
    let ys = xs.clone();
    time("frost FrozenIntSet == (Sorted, Sorted)", 200, || xs == ys);
    time("slice == (reference)", 200, || big32[..] == big32[..]);

    println!("\n== 8. Dense∩Dense kernel vs plain slice zip (popcount codegen) ==");
    let d1 = FrozenIntSet::try_from_iter((0..2_000_000u32).filter(|i| i % 3 != 0), IntSetPolicy::dense()).unwrap();
    let d2 = FrozenIntSet::try_from_iter((0..2_000_000u32).filter(|i| i % 5 != 0), IntSetPolicy::dense()).unwrap();
    time("frost Dense∩Dense intersection_len (2M-bit windows)", 500, || d1.intersection_len(&d2));
    let u = time("frost Dense∪Dense union (materialized)", 200, || d1.union(&d2));
    println!("  union repr {:?}", repr(&u));
    let w1: Vec<u64> = (0..31250u64).map(|i| i.wrapping_mul(0x9E3779B97F4A7C15)).collect();
    let w2: Vec<u64> = (0..31250u64).map(|i| i.wrapping_mul(0xC2B2AE3D27D4EB4F)).collect();
    time("reference: zip(&,popcnt).sum over 31250 words", 500, || w1.iter().zip(&w2).map(|(a, b)| (a & b).count_ones() as u64).sum::<u64>());
    time("reference: zip(|) collect 31250 words", 200, || w1.iter().zip(&w2).map(|(a, b)| a | b).collect::<Vec<u64>>());
}

//! The README example, compiled.
use frost_interval::{
    FrozenIntervalMap, FrozenIntervalSet, Interval, NotNanF64, Region, newtype_domain,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct OrderId(u64);
newtype_domain!(OrderId(u64): discrete, cover_preserving);

fn common_part<R: Region>(rs: &[R]) -> R {
    rs.iter().fold(R::all(), |acc, r| acc.intersection(r))
}

fn main() {
    // Any std range form, any domain. All three sets store u64 words.
    let ids: FrozenIntervalSet<OrderId> = FrozenIntervalSet::of(OrderId(10)..OrderId(20));
    let signed: FrozenIntervalSet<i64> = FrozenIntervalSet::from_intervals([-5..=-1, 3..=9]);
    let floats: FrozenIntervalSet<NotNanF64> = [NotNanF64::interval(0.0..1.0).unwrap()]
        .into_iter()
        .collect();
    assert_eq!(ids.count(), 10);
    assert!(signed.contains(&-3) && floats.contains(&NotNanF64::new(0.5).unwrap()));

    // The encoding lifts to sets: ids meet plain numbers in word space.
    let block: FrozenIntervalSet<u64> = FrozenIntervalSet::of(15..100);
    let both = FrozenIntervalSet::<OrderId>::from_words(&ids.to_words().intersection(&block));
    assert_eq!(both.closed_intervals(), [(OrderId(15), OrderId(19))]);

    // A map with one linear merge kernel.
    let a: FrozenIntervalMap<u64, u32> = [(0..10, 1), (5..20, 2)].into_iter().collect();
    let b: FrozenIntervalMap<u64, u32> = [(8..12, 10)].into_iter().collect();
    assert_eq!(a.union_with(&b, |x, y| x + y).get(&9), Some(&12));

    // One algebra for every region family.
    assert_eq!(
        common_part(&[Interval::closed(1u64, 9), Interval::at_least(7)]),
        Interval::closed(7, 9)
    );
    println!("readme example ok");
}

//! Frozen hash-set algebra, representative ownership, and iterator contracts.
#![cfg(feature = "std")]

use core::hash::{BuildHasher, Hash, Hasher};
use core::iter::FusedIterator;
use frost::family::FrozenHashSet;
use std::cell::Cell;
use std::collections::{BTreeMap, HashSet};
use std::hash::DefaultHasher;
use std::rc::Rc;

struct LeftPolicy;
impl BuildHasher for LeftPolicy {
    type Hasher = DefaultHasher;

    fn build_hasher(&self) -> Self::Hasher {
        DefaultHasher::new()
    }
}

struct RightPolicy;
struct CollisionHasher;
impl Hasher for CollisionHasher {
    fn finish(&self) -> u64 {
        0
    }

    fn write(&mut self, _: &[u8]) {}
}
impl BuildHasher for RightPolicy {
    type Hasher = CollisionHasher;

    fn build_hasher(&self) -> Self::Hasher {
        CollisionHasher
    }
}

// No Clone or Debug implementation: algebra borrows the stored representatives.
struct Member {
    id: u32,
    origin: &'static str,
}
impl PartialEq for Member {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Member {}
impl Hash for Member {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

fn members<'a>(ids: &'a [u32], origin: &'static str) -> impl Iterator<Item = Member> + 'a {
    ids.iter().map(move |&id| Member { id, origin })
}

#[test]
fn representative_retention_survives_smaller_right_scans_and_collisions() {
    let left = FrozenHashSet::from_iter_with_hasher(members(&[1, 2, 3, 4], "left"), LeftPolicy);
    let right = FrozenHashSet::from_iter_with_hasher(members(&[2, 5], "right"), RightPolicy);

    // `right` is smaller, but the returned object must still come from `left`.
    let common = left.intersection(&right).next().unwrap();
    assert_eq!((common.id, common.origin), (2, "left"));
    assert!(core::ptr::eq(common, left.get(common).unwrap()));
    let common = right.intersection(&left).next().unwrap();
    assert_eq!((common.id, common.origin), (2, "right"));

    let union: BTreeMap<_, _> = left.union(&right).map(|m| (m.id, m.origin)).collect();
    assert_eq!(
        union,
        [
            (1, "left"),
            (2, "left"),
            (3, "left"),
            (4, "left"),
            (5, "right")
        ]
        .into()
    );
    let reversed: BTreeMap<_, _> = right.union(&left).map(|m| (m.id, m.origin)).collect();
    assert_eq!(reversed[&2], "right");

    let difference: BTreeMap<_, _> = left.difference(&right).map(|m| (m.id, m.origin)).collect();
    assert_eq!(difference, [(1, "left"), (3, "left"), (4, "left")].into());
    let symmetric: BTreeMap<_, _> = left
        .symmetric_difference(&right)
        .map(|m| (m.id, m.origin))
        .collect();
    assert_eq!(
        symmetric,
        [(1, "left"), (3, "left"), (4, "left"), (5, "right")].into()
    );
    assert_eq!(left.intersection_len(&right), 1);
    assert_eq!(left.union_len(&right), 5);
    assert_eq!(left.difference_len(&right), 3);
    assert_eq!(left.symmetric_difference_len(&right), 4);

    // Clone and Debug need no element or hasher traits beyond those above.
    assert_eq!(left.intersection(&right).clone().count(), 1);
    assert_eq!(left.union(&right).clone().count(), 5);
    assert_eq!(left.difference(&right).clone().count(), 3);
    assert_eq!(left.symmetric_difference(&right).clone().count(), 4);
    assert!(format!("{:?}", left.intersection(&right)).contains("SetIntersection"));
    assert!(format!("{:?}", left.union(&right)).contains("SetUnion"));
    assert!(format!("{:?}", left.difference(&right)).contains("SetDifference"));
    assert!(format!("{:?}", left.symmetric_difference(&right)).contains("SetSymmetricDifference"));
}

fn check_hints_and_fusion<I>(mut iter: I)
where
    I: Iterator + Clone + FusedIterator,
    I::Item: PartialEq + core::fmt::Debug,
{
    loop {
        let remainder: Vec<_> = iter.clone().collect();
        let (lower, upper) = iter.size_hint();
        assert!(lower <= remainder.len());
        assert!(upper.is_none_or(|upper| remainder.len() <= upper));
        let folded = iter.clone().fold(Vec::new(), |mut output, member| {
            output.push(member);
            output
        });
        assert_eq!(folded, remainder);
        let next = iter.next();
        assert_eq!(next.as_ref(), remainder.first());
        if next.is_none() {
            break;
        }
    }
    assert_eq!(iter.size_hint(), (0, Some(0)));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None);
    assert_eq!(iter.clone().count(), 0);
}

#[test]
fn empty_self_disjoint_and_overlap_obey_iterator_contracts() {
    for (left, right) in [
        (vec![], vec![]),
        (vec![1, 2, 3], vec![]),
        (vec![], vec![1, 2, 3]),
        (vec![1, 2], vec![1, 2]),
        (vec![1, 2], vec![3, 4]),
        (vec![1, 2, 3, 4], vec![2, 5]),
        (vec![2, 5], vec![1, 2, 3, 4]),
    ] {
        let left: FrozenHashSet<_> = left.into_iter().collect();
        let right: FrozenHashSet<_> = right.into_iter().collect();
        check_hints_and_fusion(left.union(&right));
        check_hints_and_fusion(left.intersection(&right));
        check_hints_and_fusion(left.difference(&right));
        check_hints_and_fusion(left.symmetric_difference(&right));
        assert_eq!(left.union_len(&left), left.len());
        assert_eq!(left.intersection_len(&left), left.len());
        assert_eq!(left.difference_len(&left), 0);
        assert_eq!(left.symmetric_difference_len(&left), 0);
        check_hints_and_fusion(left.union(&left));
        check_hints_and_fusion(left.intersection(&left));
        check_hints_and_fusion(left.difference(&left));
        check_hints_and_fusion(left.symmetric_difference(&left));
    }
}

struct ObservedPolicy(Rc<Cell<usize>>);
impl BuildHasher for ObservedPolicy {
    type Hasher = DefaultHasher;

    fn build_hasher(&self) -> Self::Hasher {
        self.0.set(self.0.get() + 1);
        DefaultHasher::new()
    }
}

#[test]
fn creating_cloning_and_inspecting_iterators_does_not_probe_membership() {
    let probes = Rc::new(Cell::new(0));
    let left = FrozenHashSet::from_iter_with_hasher([1, 2, 3], ObservedPolicy(Rc::clone(&probes)));
    let right = FrozenHashSet::from_iter_with_hasher([2, 4], ObservedPolicy(Rc::clone(&probes)));
    probes.set(0);
    let mut intersection = left.intersection(&right);
    assert_eq!(intersection.clone().size_hint(), (0, Some(2)));
    assert_eq!(left.union(&right).clone().size_hint().1, Some(5));
    assert_eq!(left.difference(&right).clone().size_hint().1, Some(3));
    assert_eq!(
        left.symmetric_difference(&right).clone().size_hint().1,
        Some(5)
    );
    assert!(format!("{intersection:?}").contains("SetIntersection"));
    assert_eq!(probes.get(), 0);
    assert_eq!(intersection.next(), Some(&2));
    assert!(probes.get() > 0);
}

#[test]
fn an_operand_only_traversed_does_not_need_a_hasher() {
    let empty: FrozenHashSet<u32, ()> = FrozenHashSet::default();
    let values: FrozenHashSet<_> = [1, 2].into_iter().collect();
    assert_eq!(values.union(&empty).count(), 2);
    assert_eq!(empty.difference(&values).count(), 0);
}

proptest::proptest! {
    #[test]
    fn algebra_and_counts_match_native_sets(
        left in proptest::collection::vec(0_u16..256, 0..128),
        right in proptest::collection::vec(0_u16..256, 0..128),
    ) {
        let native_left: HashSet<_> = left.iter().copied().collect();
        let native_right: HashSet<_> = right.iter().copied().collect();
        let left = FrozenHashSet::from_iter_with_hasher(left, LeftPolicy);
        let right = FrozenHashSet::from_iter_with_hasher(right, RightPolicy);

        proptest::prop_assert_eq!(left.union_len(&right), native_left.union(&native_right).count());
        proptest::prop_assert_eq!(left.intersection_len(&right), native_left.intersection(&native_right).count());
        proptest::prop_assert_eq!(left.difference_len(&right), native_left.difference(&native_right).count());
        proptest::prop_assert_eq!(left.symmetric_difference_len(&right), native_left.symmetric_difference(&native_right).count());
        proptest::prop_assert_eq!(left.is_subset(&right), native_left.is_subset(&native_right));
        proptest::prop_assert_eq!(left.is_superset(&right), native_left.is_superset(&native_right));
        proptest::prop_assert_eq!(left.is_disjoint(&right), native_left.is_disjoint(&native_right));

        proptest::prop_assert_eq!(
            left.union(&right).copied().collect::<HashSet<_>>(),
            native_left.union(&native_right).copied().collect::<HashSet<_>>()
        );
        proptest::prop_assert_eq!(
            left.intersection(&right).copied().collect::<HashSet<_>>(),
            native_left.intersection(&native_right).copied().collect::<HashSet<_>>()
        );
        proptest::prop_assert_eq!(
            left.difference(&right).copied().collect::<HashSet<_>>(),
            native_left.difference(&native_right).copied().collect::<HashSet<_>>()
        );
        proptest::prop_assert_eq!(
            left.symmetric_difference(&right).copied().collect::<HashSet<_>>(),
            native_left.symmetric_difference(&native_right).copied().collect::<HashSet<_>>()
        );
    }
}

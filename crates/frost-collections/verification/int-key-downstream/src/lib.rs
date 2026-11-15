//! A separately compiled caller implementing Frost's public codec contract.
//!
//! Distinct logical domains do not become interchangeable just because both use
//! a u64 storage word:
//!
//! ```compile_fail,E0308
//! use frost::integer::FrozenIntSet;
//! use frost_int_downstream::{OrderId, UserId};
//! let users: FrozenIntSet<_> = [UserId(1)].into_iter().collect();
//! let orders: FrozenIntSet<_> = [OrderId(1)].into_iter().collect();
//! let _ = users.union(&orders);
//! ```

use frost::integer::{FrozenIntSet, IntKey, IntSetPolicy};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserId(pub u64);

impl IntKey for UserId {
    type Word = u64;
    fn encode(self) -> u64 {
        self.0
    }
    fn decode(word: u64) -> Option<Self> {
        Some(Self(word))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrderId(pub u64);

impl IntKey for OrderId {
    type Word = u64;
    fn encode(self) -> u64 {
        self.0
    }
    fn decode(word: u64) -> Option<Self> {
        Some(Self(word))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub i64);

impl IntKey for Timestamp {
    type Word = u64;
    fn encode(self) -> u64 {
        self.0.encode()
    }
    fn decode(word: u64) -> Option<Self> {
        i64::decode(word).map(Self)
    }
}

// Generic consumers do not need to name any private backend associated type.
pub fn build<K: IntKey>(input: impl IntoIterator<Item = K>) -> FrozenIntSet<K> {
    FrozenIntSet::try_from_iter(input, IntSetPolicy::auto()).unwrap()
}

pub fn generic_send_sync<K: IntKey + Send + Sync>() {
    fn require<T: Send + Sync>() {}
    require::<FrozenIntSet<K>>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use frost::TryFreezeWith;
    use frost::read::ValueElements;

    #[test]
    fn downstream_keys_and_private_width_mapping_are_usable_generically() {
        let ids = build([UserId(u64::MAX), UserId(2), UserId(2)]);
        assert_eq!(
            ids.iter().collect::<Vec<_>>(),
            [UserId(2), UserId(u64::MAX)]
        );
        assert_eq!(ids.encoded_values().collect::<Vec<_>>(), [2_u64, u64::MAX]);
        let timestamps = build([Timestamp(i64::MAX), Timestamp(-1), Timestamp(i64::MIN)]);
        assert_eq!(
            timestamps.values().collect::<Vec<_>>(),
            [Timestamp(i64::MIN), Timestamp(-1), Timestamp(i64::MAX)]
        );
        generic_send_sync::<UserId>();
        generic_send_sync::<Timestamp>();
    }

    #[test]
    fn downstream_freeze_plan_and_forced_backends_agree() {
        let input = vec![UserId(65), UserId(1), UserId(64), UserId(1)];
        let sorted = input
            .clone()
            .try_freeze_with(IntSetPolicy::sorted())
            .unwrap();
        let dense = input
            .clone()
            .try_freeze_with(IntSetPolicy::dense())
            .unwrap();
        let roaring = input.try_freeze_with(IntSetPolicy::roaring()).unwrap();
        assert_eq!(sorted, dense);
        assert_eq!(dense, roaring);
        assert_eq!(sorted.union(&roaring), dense);
    }
}

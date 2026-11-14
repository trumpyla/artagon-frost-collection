use crate::read::{ElementType, MapTypes};

/// Read one live map entry while its backend keeps the borrowed value valid.
///
/// The callback runs exactly once if a matching entry is found, and is not
/// called otherwise. Its borrowed key and value cannot escape the callback.
/// The lookup does not freeze the collection or coordinate other lookups.
/// Backend-specific blocking and progress guarantees still apply.
#[cfg_attr(
    feature = "dashmap-6",
    doc = r#"
```
# extern crate frost_collections_concurrent as frost;
use frost::backend::dashmap::DashMap;
use frost::concurrent::ConcurrentLookup;
let map = DashMap::new();
map.insert(String::from("answer"), 42);
assert_eq!(map.with_entry("answer", |key, value| (key.len(), *value)), Some((6, 42)));
assert_eq!(map.get_cloned("answer"), Some(42));
```

A guard-backed reference cannot escape:

```compile_fail
# extern crate frost_collections_concurrent as frost;
use frost::backend::dashmap::DashMap;
use frost::concurrent::ConcurrentLookup;
let map = DashMap::new();
map.insert(String::from("answer"), 42);
let leaked: &i32 = map.with_entry("answer", |_, value| value).unwrap();
println!("{leaked}");
```
"#
)]
pub trait ConcurrentLookup<Q: ?Sized>: MapTypes {
    /// Run `read` with a matching stored key and value while protection is held.
    fn with_entry<R>(&self, key: &Q, read: impl FnOnce(&Self::Key, &Self::Value) -> R)
    -> Option<R>;

    /// Clone a value under the backend's protection and release that protection.
    ///
    /// The clone is independent of later entry replacement. Cloning a type
    /// containing shared interior state, such as `Arc<AtomicUsize>`, still
    /// shares that state. The clone operation itself runs inside the callback.
    fn get_cloned(&self, key: &Q) -> Option<Self::Value>
    where
        Self::Value: Clone,
    {
        self.with_entry(key, |_, value| value.clone())
    }
}

/// Exclusively update the value of one existing live map entry.
///
/// The callback receives no mutable key, preserving the backend's key
/// invariants. It runs exactly once if the entry exists. A missing key is not
/// inserted. Exclusive access covers this callback only; a read followed by
/// an update is not an atomic read-modify-write operation. Put that operation
/// entirely inside this callback instead.
#[cfg_attr(
    feature = "dashmap-6",
    doc = r#"
```compile_fail
# extern crate frost_collections_concurrent as frost;
use frost::backend::dashmap::DashMap;
use frost::concurrent::ConcurrentUpdate;
let map = DashMap::new();
map.insert("counter", 0);
let leaked: &mut i32 = map.with_entry_mut(&"counter", |_, value| value).unwrap();
*leaked += 1;
```
"#
)]
#[cfg_attr(
    feature = "crossbeam-skiplist-0_1",
    doc = r"
Skip-list entries do not provide exclusive mutable value access:

```compile_fail,E0599
# extern crate frost_collections_concurrent as frost;
use frost::backend::crossbeam_skiplist::SkipMap;
use frost::concurrent::ConcurrentUpdate;
let map: SkipMap<i32, i32> = SkipMap::new();
map.insert(1, 2);
map.with_entry_mut(&1, |_, value| *value += 1);
```
"
)]
pub trait ConcurrentUpdate<Q: ?Sized>: ConcurrentLookup<Q> {
    /// Run `update` with exclusive access to an existing entry's value.
    fn with_entry_mut<R>(
        &self,
        key: &Q,
        update: impl FnOnce(&Self::Key, &mut Self::Value) -> R,
    ) -> Option<R>;
}

/// Inspect one live set's stored representative within a protected callback.
///
/// The callback is called exactly once on a hit and never on a miss. A set
/// member cannot be mutated through this API. Use [`crate::read::SetContains`]
/// when only membership is needed; its result is a single live observation.
#[cfg_attr(
    feature = "dashmap-6",
    doc = r#"
```compile_fail
# extern crate frost_collections_concurrent as frost;
use frost::backend::dashmap::DashSet;
use frost::concurrent::ConcurrentSetLookup;
let set = DashSet::new();
set.insert(String::from("member"));
let leaked: &String = set.with_value("member", |value| value).unwrap();
println!("{leaked}");
```
"#
)]
pub trait ConcurrentSetLookup<Q: ?Sized>: ElementType {
    /// Inspect the stored representative while backend protection is held.
    fn with_value<R>(&self, key: &Q, read: impl FnOnce(&Self::Item) -> R) -> Option<R>;

    /// Clone the stored representative and release backend protection.
    fn get_cloned(&self, key: &Q) -> Option<Self::Item>
    where
        Self::Item: Clone,
    {
        self.with_value(key, Clone::clone)
    }
}

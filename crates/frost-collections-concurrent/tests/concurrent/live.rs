#[cfg(any(feature = "dashmap-6", feature = "scc-3"))]
fn exercise_mutable_map<M>(map: M)
where
    M: frost::concurrent::ConcurrentLookup<str>
        + frost::concurrent::ConcurrentUpdate<str>
        + frost::read::MapTypes<Key = String, Value = usize>
        + Send
        + Sync
        + 'static,
{
    use std::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::{Arc, Barrier};

    let callbacks = Cell::new(0);
    assert_eq!(
        map.with_entry("counter", |key, value| {
            callbacks.set(callbacks.get() + 1);
            (key.clone(), *value)
        }),
        Some(("counter".to_owned(), 0))
    );
    assert_eq!(callbacks.get(), 1);
    assert_eq!(map.with_entry("missing", |_, _| callbacks.set(99)), None);
    assert_eq!(
        map.with_entry_mut("missing", |_, _| callbacks.set(99)),
        None
    );
    assert_eq!(callbacks.get(), 1);

    let result = catch_unwind(AssertUnwindSafe(|| {
        map.with_entry_mut("counter", |_, value| {
            *value = 5;
            panic!("intentional callback panic");
        });
    }));
    assert!(result.is_err());
    // Unwinding drops the backend guard, but does not undo earlier mutations.
    assert_eq!(map.get_cloned("counter"), Some(5));
    assert_eq!(
        map.with_entry_mut("counter", |_, value| *value = 0),
        Some(())
    );

    let map = Arc::new(map);
    let barrier = Arc::new(Barrier::new(4));
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let map = Arc::clone(&map);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                for _ in 0..1_000 {
                    assert_eq!(
                        map.with_entry_mut("counter", |key, value| {
                            assert_eq!(key, "counter");
                            *value += 1;
                        }),
                        Some(())
                    );
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(map.get_cloned("counter"), Some(4_000));
}

#[cfg(feature = "dashmap-6")]
mod dashmap_tests {
    use frost::backend::dashmap::{DashMap, DashSet};
    use frost::concurrent::ConcurrentSetLookup;
    use frost::read::SetContains;

    #[test]
    fn borrowed_lookup_and_exclusive_updates_under_contention() {
        let map = DashMap::new();
        map.insert("counter".to_owned(), 0);
        super::exercise_mutable_map(map);
    }

    #[test]
    fn set_callbacks_can_return_owned_members() {
        let set = DashSet::new();
        assert!(set.insert("member".to_owned()));
        assert_eq!(set.with_value("member", String::len), Some(6));
        let member = ConcurrentSetLookup::get_cloned(&set, "member").unwrap();
        assert!(SetContains::contains(&set, "member"));
        assert_eq!(set.with_value("missing", |_| panic!("absent")), None::<()>);
        assert!(set.remove("member").is_some());
        assert_eq!(member, "member");
        assert!(!SetContains::contains(&set, "member"));
    }
}

#[cfg(feature = "scc-3")]
mod scc_tests {
    use frost::backend::scc::{HashMap, HashSet};
    use frost::concurrent::ConcurrentSetLookup;
    use frost::read::SetContains;

    #[test]
    fn borrowed_lookup_and_exclusive_updates_under_contention() {
        let map = HashMap::new();
        map.insert_sync("counter".to_owned(), 0).unwrap();
        super::exercise_mutable_map(map);
    }

    #[test]
    fn set_callbacks_can_return_owned_members() {
        let set = HashSet::new();
        set.insert_sync("member".to_owned()).unwrap();
        assert_eq!(set.with_value("member", String::len), Some(6));
        let member = ConcurrentSetLookup::get_cloned(&set, "member").unwrap();
        assert!(SetContains::contains(&set, "member"));
        assert_eq!(set.with_value("missing", |_| panic!("absent")), None::<()>);
        assert!(set.remove_sync("member").is_some());
        assert_eq!(member, "member");
        assert!(!SetContains::contains(&set, "member"));
    }
}

#[cfg(feature = "crossbeam-skiplist-0_1")]
mod skiplist_tests {
    use std::sync::{Arc, Barrier};

    use frost::backend::crossbeam_skiplist::{SkipMap, SkipSet};
    use frost::concurrent::{ConcurrentLookup, ConcurrentSetLookup};
    use frost::read::SetContains;

    #[test]
    fn borrowed_lookup_and_owned_extraction() {
        let map = SkipMap::new();
        map.insert("key".to_owned(), "value".to_owned());
        assert_eq!(
            map.with_entry("key", |key, value| key.len() + value.len()),
            Some(8)
        );
        let value = ConcurrentLookup::get_cloned(&map, "key").unwrap();
        assert!(map.remove("key").is_some());
        assert_eq!(value, "value");
        assert_eq!(
            map.with_entry("missing", |_, _| panic!("absent")),
            None::<()>
        );

        let set = SkipSet::new();
        set.insert("member".to_owned());
        assert_eq!(set.with_value("member", String::len), Some(6));
        let member = ConcurrentSetLookup::get_cloned(&set, "member").unwrap();
        assert!(SetContains::contains(&set, "member"));
        assert_eq!(set.with_value("missing", |_| panic!("absent")), None::<()>);
        assert!(set.remove("member").is_some());
        assert_eq!(member, "member");
        assert!(!SetContains::contains(&set, "member"));
    }

    #[test]
    fn concurrent_insertions_preserve_order_after_writers_join() {
        let map = Arc::new(SkipMap::new());
        let set = Arc::new(SkipSet::new());
        let barrier = Arc::new(Barrier::new(4));
        let threads: Vec<_> = (0..4)
            .map(|worker| {
                let map = Arc::clone(&map);
                let set = Arc::clone(&set);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    for index in (0..250).rev() {
                        let key = index * 4 + worker;
                        map.insert(key, key * 2);
                        set.insert(key);
                        assert_eq!(ConcurrentLookup::get_cloned(&*map, &key), Some(key * 2));
                        assert_eq!(ConcurrentSetLookup::get_cloned(&*set, &key), Some(key));
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        // Native traversal is checked only after every writer has finished.
        let entries: Vec<_> = map
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect();
        let members: Vec<_> = set.iter().map(|entry| *entry.value()).collect();
        assert_eq!(
            entries,
            (0..1_000).map(|key| (key, key * 2)).collect::<Vec<_>>()
        );
        assert_eq!(members, (0..1_000).collect::<Vec<_>>());
    }
}

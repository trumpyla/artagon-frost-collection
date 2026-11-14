//! Optional Loom exploration of the production retry loop, with a trusted
//! instrumented atomic store. This does not model arc-swap's implementation.

#[cfg(test)]
#[path = "../../../crates/frost-collections/src/concurrent/publication.rs"]
mod publication;

#[cfg(test)]
mod tests {
    use core::ops::Deref;
    use loom::sync::{Arc, Mutex};
    use loom::thread;

    use super::publication;

    #[derive(Clone, Copy, Debug)]
    struct Version {
        identity: usize,
        value: usize,
    }

    impl Deref for Version {
        type Target = usize;

        fn deref(&self) -> &usize {
            &self.value
        }
    }

    fn explore<const WRITERS: usize>() {
        let mut builder = loom::model::Builder::new();
        builder.max_threads = WRITERS + 1;
        builder.max_branches = 256;
        // Do not let environment-variable defaults silently truncate a proof
        // run while retaining a green test status.
        builder.max_permutations = None;
        builder.max_duration = None;
        builder.preemption_bound = None;
        builder.checkpoint_file = None;
        builder.expect_explicit_explore = false;
        builder.check(|| {
            let store = Arc::new(Mutex::new(Version {
                identity: 0,
                value: 0,
            }));
            let workers: Vec<_> = (0..WRITERS)
                .map(|_| {
                    let store = Arc::clone(&store);
                    thread::spawn(move || {
                        let current = *store.lock().unwrap();
                        publication::update(
                            current,
                            |value| value + 1,
                            |expected, candidate| {
                                let mut current = store.lock().unwrap();
                                let previous = *current;
                                if current.identity == expected.identity {
                                    *current = Version {
                                        identity: previous.identity + 1,
                                        value: candidate,
                                    };
                                    Ok(previous)
                                } else {
                                    Err((candidate, previous))
                                }
                            },
                        )
                    })
                })
                .collect();
            let mut displaced: Vec<_> = workers
                .into_iter()
                .map(|worker| worker.join().unwrap().value)
                .collect();
            displaced.sort_unstable();
            assert_eq!(displaced, (0..WRITERS).collect::<Vec<_>>());
            let final_version = store.lock().unwrap();
            assert_eq!(final_version.value, WRITERS);
            assert_eq!(final_version.identity, WRITERS);
        });
    }

    #[test]
    fn two_writers_have_no_lost_updates() {
        explore::<2>();
    }

    #[test]
    fn three_writers_have_no_lost_updates() {
        explore::<3>();
    }
}

//! Exhaustive schedules for the actual snapshot retry loop.
//!
//! The backend below is an explicitly trusted, sequentially consistent atomic
//! store of immutable version handles. It is not an implementation or a model
//! of arc-swap's memory ordering, reference counting, or reclamation. The
//! production retry loop is included directly so its control flow cannot drift
//! from this test. See `docs/verification.md` for the complete bounds.
#![cfg(feature = "std")]

#[path = "../../frost-collections-concurrent/src/concurrent/publication.rs"]
mod publication;

use core::ops::Deref;
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Version {
    identity: usize,
    value: u32,
}

impl Deref for Version {
    type Target = u32;

    fn deref(&self) -> &u32 {
        &self.value
    }
}

#[derive(Clone, Copy, Debug)]
enum Transformation {
    // Noncommuting transformations make the successful commit order observable.
    Append(u32),
    Identity,
}

impl Transformation {
    fn apply(self, value: u32) -> u32 {
        match self {
            Self::Append(digit) => 4 * value + digit,
            Self::Identity => value,
        }
    }
}

#[derive(Debug)]
enum Request {
    Load,
    Commit {
        expected: Version,
        candidate: u32,
    },
    Finished {
        previous: Version,
        derivations: usize,
    },
}

#[derive(Debug)]
enum Response {
    Loaded(Version),
    Committed(Result<Version, (u32, Version)>),
}

struct Worker {
    requests: Receiver<Request>,
    responses: Sender<Response>,
    pending: Option<Request>,
    attempts: usize,
    displaced: Option<Version>,
}

impl Worker {
    fn advance(&mut self, response: Response) {
        self.responses.send(response).unwrap();
        match self.requests.recv().unwrap() {
            Request::Finished {
                previous,
                derivations,
            } => {
                assert_eq!(
                    Some(previous),
                    self.displaced,
                    "wrong displaced version returned"
                );
                assert_eq!(
                    derivations, self.attempts,
                    "derivation did not match attempts"
                );
            }
            next => self.pending = Some(next),
        }
    }
}

struct AtomicStoreModel {
    current: Version,
    committed_order: Vec<usize>,
    failed_attempts: usize,
}

impl AtomicStoreModel {
    fn apply(
        &mut self,
        worker: &mut Worker,
        writer: usize,
        transformation: Transformation,
    ) -> Response {
        match worker.pending.take().unwrap() {
            Request::Load => Response::Loaded(self.current),
            Request::Commit {
                expected,
                candidate,
            } => {
                worker.attempts += 1;
                assert_eq!(candidate, transformation.apply(expected.value));
                if expected.identity == self.current.identity {
                    assert_eq!(expected, self.current);
                    assert!(worker.displaced.is_none(), "a writer committed twice");
                    worker.displaced = Some(self.current);
                    let previous = self.current;
                    self.current = Version {
                        identity: self.current.identity + 1,
                        value: candidate,
                    };
                    self.committed_order.push(writer);
                    Response::Committed(Ok(previous))
                } else {
                    self.failed_attempts += 1;
                    Response::Committed(Err((candidate, self.current)))
                }
            }
            Request::Finished { .. } => unreachable!("completion is not an atomic step"),
        }
    }
}

fn writer(
    transformation: Transformation,
    requests: &Sender<Request>,
    responses: &Receiver<Response>,
) {
    requests.send(Request::Load).unwrap();
    let Response::Loaded(current) = responses.recv().unwrap() else {
        panic!("initial load received a CAS result");
    };
    let mut derivations = 0;
    let previous = publication::update(
        current,
        |&value| {
            derivations += 1;
            transformation.apply(value)
        },
        |&expected, candidate| {
            requests
                .send(Request::Commit {
                    expected,
                    candidate,
                })
                .unwrap();
            let Response::Committed(result) = responses.recv().unwrap() else {
                panic!("CAS received a load result");
            };
            result
        },
    );
    requests
        .send(Request::Finished {
            previous,
            derivations,
        })
        .unwrap();
}

struct Execution {
    unexplored: Vec<Vec<usize>>,
    failed_attempts: usize,
    atomic_steps: usize,
}

fn execute(prefix: &[usize], transformations: &[Transformation]) -> Execution {
    std::thread::scope(|scope| {
        let mut workers: Vec<_> = transformations
            .iter()
            .map(|&transformation| {
                let (request_tx, requests) = mpsc::channel();
                let (responses, response_rx) = mpsc::channel();
                scope.spawn(move || writer(transformation, &request_tx, &response_rx));
                Worker {
                    requests,
                    responses,
                    pending: None,
                    attempts: 0,
                    displaced: None,
                }
            })
            .collect();
        for worker in &mut workers {
            worker.pending = Some(worker.requests.recv().unwrap());
            assert!(matches!(worker.pending, Some(Request::Load)));
        }

        let mut store = AtomicStoreModel {
            current: Version {
                identity: 0,
                value: 0,
            },
            committed_order: Vec::new(),
            failed_attempts: 0,
        };
        let mut trace = Vec::new();
        let mut unexplored = Vec::new();
        // One initial load per writer; at most n(n+1)/2 CAS attempts because
        // each failed attempt must observe another writer's successful commit.
        let step_bound = workers.len() + workers.len() * (workers.len() + 1) / 2;

        loop {
            let enabled: Vec<_> = workers
                .iter()
                .enumerate()
                .filter_map(|(id, worker)| worker.pending.as_ref().map(|_| id))
                .collect();
            if enabled.is_empty() {
                break;
            }
            assert!(
                trace.len() < step_bound,
                "retry loop exceeded the model's termination bound"
            );
            let selected = prefix.get(trace.len()).copied().unwrap_or(enabled[0]);
            assert!(
                enabled.contains(&selected),
                "schedule prefix is not replayable"
            );
            if trace.len() >= prefix.len() {
                // Replay every other enabled choice later. Pure local work is
                // executed until the next atomic boundary before choosing again.
                for &alternative in &enabled[1..] {
                    let mut branch = trace.clone();
                    branch.push(alternative);
                    unexplored.push(branch);
                }
            }
            trace.push(selected);
            let worker = &mut workers[selected];
            let response = store.apply(worker, selected, transformations[selected]);
            worker.advance(response);
        }

        assert!(
            trace.len() >= prefix.len(),
            "schedule prefix extends beyond termination"
        );
        assert_eq!(
            store.committed_order.len(),
            workers.len(),
            "an update was lost"
        );
        assert_eq!(store.current.identity, workers.len());
        let serial_result = store
            .committed_order
            .iter()
            .fold(0, |value, &writer| transformations[writer].apply(value));
        assert_eq!(
            store.current.value, serial_result,
            "result is not a serial execution"
        );
        assert!(workers.iter().all(|worker| worker.displaced.is_some()));
        Execution {
            unexplored,
            failed_attempts: store.failed_attempts,
            atomic_steps: trace.len(),
        }
    })
}

#[derive(Debug, PartialEq, Eq)]
struct Exploration {
    schedules: usize,
    maximum_failures: usize,
    maximum_steps: usize,
}

fn exhaust(transformations: &[Transformation]) -> Exploration {
    let mut pending = vec![Vec::new()];
    let mut schedules = 0;
    let mut maximum_failures = 0;
    let mut maximum_steps = 0;
    while let Some(prefix) = pending.pop() {
        let execution = execute(&prefix, transformations);
        pending.extend(execution.unexplored);
        maximum_failures = maximum_failures.max(execution.failed_attempts);
        maximum_steps = maximum_steps.max(execution.atomic_steps);
        schedules += 1;
    }
    assert!(schedules > 1);
    assert!(maximum_failures >= transformations.len() - 1);
    println!(
        "{schedules} schedules; at most {maximum_steps} atomic steps and {maximum_failures} failed attempts"
    );
    Exploration {
        schedules,
        maximum_failures,
        maximum_steps,
    }
}

#[test]
fn three_writers_exhaust_all_load_and_compare_exchange_orders() {
    let explored = exhaust(&[
        Transformation::Append(1),
        Transformation::Append(2),
        Transformation::Append(3),
    ]);
    assert_eq!(
        explored,
        Exploration {
            schedules: 198,
            maximum_failures: 3,
            maximum_steps: 9
        }
    );
}

#[test]
fn equal_contents_still_require_retry_after_identity_changes() {
    let explored = exhaust(&[Transformation::Identity, Transformation::Append(1)]);
    assert_eq!(
        explored,
        Exploration {
            schedules: 6,
            maximum_failures: 1,
            maximum_steps: 5
        }
    );
}

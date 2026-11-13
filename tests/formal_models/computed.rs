//! `KNOW-BOUNDS` and `DELTA-REPLAY` from
//! `docs/design/algebra/execution-and-computed-sets.md`, plus positive
//! recursion and context compatibility. Frost implements none of these yet.

use super::common::{is_subset, subsets, words};

const N: u32 = 3;

/// A transfer rule: `(lower, upper)` of the result from the operands' bounds.
type Rule = fn((u32, u32), (u32, u32)) -> (u32, u32);

fn union_rule((la, ua): (u32, u32), (lb, ub): (u32, u32)) -> (u32, u32) {
    (la | lb, ua | ub)
}
fn intersection_rule((la, ua): (u32, u32), (lb, ub): (u32, u32)) -> (u32, u32) {
    (la & lb, ua & ub)
}
fn difference_rule((la, ua): (u32, u32), (lb, ub): (u32, u32)) -> (u32, u32) {
    (la & !ub, ua & !lb)
}
fn xor_rule((la, ua): (u32, u32), (lb, ub): (u32, u32)) -> (u32, u32) {
    ((la & !ub) | (lb & !ua), (ua & !lb) | (ub & !la))
}

/// The exact operation a rule bounds.
type SetOp = fn(u32, u32) -> u32;

const RULES: [(Rule, SetOp); 4] = [
    (union_rule, |a, b| a | b),
    (intersection_rule, |a, b| a & b),
    (difference_rule, |a, b| a & !b),
    (xor_rule, |a, b| a ^ b),
];

fn intervals() -> Vec<(u32, u32)> {
    subsets(N)
        .flat_map(|l| {
            subsets(N)
                .filter(move |&u| is_subset(l, u))
                .map(move |u| (l, u))
        })
        .collect()
}

/// Whether `rule` is sound for `op`, and whether it is exact: its bounds equal
/// the meet and join of the result over every realization of independent operands.
fn check(rule: Rule, op: SetOp) -> (bool, bool) {
    let (mut sound, mut exact) = (true, true);
    for a in intervals() {
        for b in intervals() {
            let (lower, upper) = rule(a, b);
            let (mut meet, mut join) = ((1 << N) - 1, 0);
            for x in subsets(N).filter(|&x| is_subset(a.0, x) && is_subset(x, a.1)) {
                for y in subsets(N).filter(|&y| is_subset(b.0, y) && is_subset(y, b.1)) {
                    let actual = op(x, y);
                    sound &= is_subset(lower, actual) && is_subset(actual, upper);
                    meet &= actual;
                    join |= actual;
                }
            }
            exact &= lower == meet && upper == join;
        }
    }
    (sound, exact)
}

/// `KNOW-BOUNDS`: each transfer rule is sound and, for independent operands, exact.
#[test]
fn knowledge_bounds_are_sound_and_exact_for_independent_operands() {
    assert_eq!(intervals().len(), 27);
    for (rule, op) in RULES {
        assert_eq!(check(rule, op), (true, true));
    }
}

/// Treating the deny lower bound as complete is unsound; a correct but loose
/// rule is sound and not exact.
#[test]
fn wrong_knowledge_rules_are_rejected() {
    let deny_lower: Rule = |(la, ua), (lb, _)| (la & !lb, ua & !lb);
    let loose_xor: Rule = |(_, ua), (_, ub)| (0, ua | ub);
    assert!(!check(deny_lower, |a, b| a & !b).0);
    assert_eq!(check(loose_xor, |a, b| a ^ b), (true, false));
}

#[derive(Clone, Copy, Debug)]
enum Event {
    Insert(u32),
    Delete(u32),
}

fn replay(base: u32, events: &[Event]) -> u32 {
    events.iter().fold(base, |set, event| match *event {
        Event::Insert(k) => set | (1 << k),
        Event::Delete(k) => set & !(1 << k),
    })
}

/// Reference: each key's last event wins; untouched keys keep the base.
fn last_event_wins(base: u32, events: &[Event]) -> u32 {
    (0..2)
        .filter(|&k| {
            match events
                .iter()
                .rev()
                .find(|e| matches!(e, Event::Insert(x) | Event::Delete(x) if *x == k))
            {
                Some(Event::Insert(_)) => true,
                Some(Event::Delete(_)) => false,
                None => base & (1 << k) != 0,
            }
        })
        .map(|k| 1 << k)
        .sum()
}

fn histories() -> Vec<Vec<Event>> {
    words(
        &[
            Event::Insert(0),
            Event::Insert(1),
            Event::Delete(0),
            Event::Delete(1),
        ],
        4,
    )
}

/// `DELTA-REPLAY`: ordered replay matches the last-event reference, and
/// compaction at any cut preserves the result.
#[test]
fn ordered_replay_matches_last_event_wins_and_compaction() {
    for base in subsets(2) {
        for events in histories() {
            assert_eq!(replay(base, &events), last_event_wins(base, &events));
            for cut in 0..=events.len() {
                assert_eq!(
                    replay(replay(base, &events[..cut]), &events[cut..]),
                    replay(base, &events)
                );
            }
        }
    }
    // Delete then insert must keep the key.
    assert_eq!(replay(0, &[Event::Delete(0), Event::Insert(0)]), 1);
}

/// Replays that ignore deletions or ignore order disagree with the reference.
#[test]
fn wrong_replays_are_rejected() {
    let ignore_deletes = |base: u32, events: &[Event]| {
        events.iter().fold(base, |set, e| {
            if let Event::Insert(k) = e {
                set | (1 << k)
            } else {
                set
            }
        })
    };
    let unordered = |base: u32, events: &[Event]| {
        let inserts: u32 = events
            .iter()
            .filter_map(|e| {
                if let Event::Insert(k) = e {
                    Some(1 << k)
                } else {
                    None
                }
            })
            .fold(0, |a, b| a | b);
        let deletes: u32 = events
            .iter()
            .filter_map(|e| {
                if let Event::Delete(k) = e {
                    Some(1 << k)
                } else {
                    None
                }
            })
            .fold(0, |a, b| a | b);
        (base | inserts) & !deletes
    };
    let disagrees = |wrong: &dyn Fn(u32, &[Event]) -> u32| {
        subsets(2).any(|base| {
            histories()
                .iter()
                .any(|events| wrong(base, events) != last_event_wins(base, events))
        })
    };
    assert!(disagrees(&ignore_deletes));
    assert!(disagrees(&unordered));
    // Inserts-then-deletes drops a key deleted and then inserted again.
    assert_eq!(unordered(0, &[Event::Delete(0), Event::Insert(0)]), 0);
}

/// Positive reachability over every directed graph on three nodes reaches its
/// fixed point within three strict steps and matches breadth-first search.
#[test]
fn positive_recursion_reaches_its_fixed_point_in_at_most_n_steps() {
    for graph in subsets(N * N) {
        let edges: Vec<(u32, u32)> = (0..N * N)
            .filter(|i| graph & (1 << i) != 0)
            .map(|i| (i / N, i % N))
            .collect();
        for start in 0..N {
            let (mut reached, mut steps) = (1 << start, 0);
            loop {
                let next = edges
                    .iter()
                    .filter(|(a, _)| reached & (1 << a) != 0)
                    .fold(reached, |r, (_, b)| r | (1 << b));
                if next == reached {
                    break;
                }
                reached = next;
                steps += 1;
                assert!(steps <= N);
            }
            let mut seen = 1 << start;
            let mut queue = vec![start];
            while let Some(a) = queue.pop() {
                for &(_, b) in edges.iter().filter(|(x, _)| *x == a) {
                    if seen & (1 << b) == 0 {
                        seen |= 1 << b;
                        queue.push(b);
                    }
                }
            }
            assert_eq!(reached, seen);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Context {
    kind: u8,
    tenant: u8,
    dictionary: u8,
    revision: u8,
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Complete(u32),
    IncompatibleContext,
}

fn checked_union(a: (Context, u32), b: (Context, u32)) -> Outcome {
    if a.0 == b.0 {
        Outcome::Complete(a.1 | b.1)
    } else {
        Outcome::IncompatibleContext
    }
}

/// An empty-operand shortcut hides a compatibility error that the checked
/// operation reports, for every field of the context.
#[test]
fn empty_shortcuts_must_not_skip_context_checks() {
    let base = Context {
        kind: 0,
        tenant: 0,
        dictionary: 0,
        revision: 0,
    };
    let shortcut = |a: (Context, u32), b: (Context, u32)| {
        if b.1 == 0 {
            Outcome::Complete(a.1)
        } else {
            checked_union(a, b)
        }
    };
    assert_eq!(checked_union((base, 1), (base, 2)), Outcome::Complete(3));
    for other in [
        Context { kind: 1, ..base },
        Context { tenant: 1, ..base },
        Context {
            dictionary: 1,
            ..base
        },
        Context {
            revision: 1,
            ..base
        },
    ] {
        assert_eq!(
            checked_union((base, 1), (other, 0)),
            Outcome::IncompatibleContext
        );
        assert_eq!(shortcut((base, 1), (other, 0)), Outcome::Complete(1));
    }
}

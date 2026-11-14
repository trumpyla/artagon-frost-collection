//! Operation sequences on the real mutable set families against a `BTreeSet`
//! model, with freeze and thaw as a step. This is the `SET-BOOLEAN` membership
//! model applied to state, and `FROZEN-DENOTE` for set freezing.
//!
//! Small scope is exhausted with `exhaust`; longer sequences over more keys
//! are sampled and shrunk with `proptest-state-machine`.

use super::common::Key3;
use exhaust::Exhaust;
use frost::family::{FrozenHashSet, FrozenOrderedSet, MutableHashSet, MutableOrderedSet};
use proptest::prelude::*;
use proptest::test_runner::Config;
use proptest_state_machine::{ReferenceStateMachine, StateMachineTest, prop_state_machine};
use std::collections::BTreeSet;
use std::hash::{BuildHasherDefault, DefaultHasher};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Exhaust)]
enum Step<K> {
    Insert(K),
    Remove(K),
    FreezeThaw,
}

type Hasher = BuildHasherDefault<DefaultHasher>;

/// The real families under test, driven in lockstep.
struct Real<K> {
    ordered: MutableOrderedSet<K>,
    hashed: MutableHashSet<K, Hasher>,
}

impl<K: Copy + Ord + core::hash::Hash + core::fmt::Debug> Real<K> {
    fn new() -> Self {
        Self {
            ordered: MutableOrderedSet::default(),
            hashed: MutableHashSet::default(),
        }
    }

    /// Apply `step` to both families and to `model`; insert and remove must
    /// report the same change as the model.
    fn apply(mut self, model: &mut BTreeSet<K>, step: Step<K>) -> Self {
        match step {
            Step::Insert(k) => {
                let changed = model.insert(k);
                assert_eq!(self.ordered.insert(k), changed);
                assert_eq!(self.hashed.insert(k), changed);
            }
            Step::Remove(k) => {
                let changed = model.remove(&k);
                assert_eq!(self.ordered.remove(&k), changed);
                assert_eq!(self.hashed.remove(&k), changed);
            }
            Step::FreezeThaw => {
                let ordered: FrozenOrderedSet<K> = self.ordered.freeze();
                let hashed: FrozenHashSet<K, Hasher> = self.hashed.freeze();
                assert!(ordered.iter().eq(model.iter()), "frozen ordered set");
                assert_eq!(hashed.iter().copied().collect::<BTreeSet<_>>(), *model);
                self.ordered = ordered.thaw();
                self.hashed = hashed.thaw();
            }
        }
        self
    }

    fn check(&self, model: &BTreeSet<K>, keys: impl IntoIterator<Item = K>) {
        assert_eq!(self.ordered.len(), model.len());
        assert_eq!(self.hashed.len(), model.len());
        assert!(self.ordered.iter().eq(model.iter()));
        assert_eq!(self.hashed.iter().copied().collect::<BTreeSet<_>>(), *model);
        for k in keys {
            assert_eq!(self.ordered.contains(&k), model.contains(&k));
            assert_eq!(self.hashed.contains(&k), model.contains(&k));
        }
    }
}

/// Every sequence of at most four steps over three keys (a `None` slot is no
/// step): 8⁴ = 4,096 sequences, each checked after every step.
#[test]
fn every_short_operation_sequence_matches_the_model() {
    let mut sequences = 0;
    for steps in <[Option<Step<Key3>>; 4]>::exhaust() {
        let (mut real, mut model) = (Real::new(), BTreeSet::new());
        for step in steps.into_iter().flatten() {
            real = real.apply(&mut model, step);
            real.check(&model, Key3::exhaust());
        }
        sequences += 1;
    }
    assert_eq!(sequences, 4096);
}

struct Model;

impl ReferenceStateMachine for Model {
    type State = BTreeSet<u8>;
    type Transition = Step<u8>;

    fn init_state() -> BoxedStrategy<Self::State> {
        Just(BTreeSet::new()).boxed()
    }

    fn transitions(_: &Self::State) -> BoxedStrategy<Step<u8>> {
        prop_oneof![
            4 => (0_u8..16).prop_map(Step::Insert),
            2 => (0_u8..16).prop_map(Step::Remove),
            1 => Just(Step::FreezeThaw),
        ]
        .boxed()
    }

    fn apply(mut state: Self::State, step: &Step<u8>) -> Self::State {
        match *step {
            Step::Insert(k) => {
                state.insert(k);
            }
            Step::Remove(k) => {
                state.remove(&k);
            }
            Step::FreezeThaw => {}
        }
        state
    }
}

struct Families;

impl StateMachineTest for Families {
    type SystemUnderTest = (Real<u8>, BTreeSet<u8>);
    type Reference = Model;

    fn init_test(_: &BTreeSet<u8>) -> Self::SystemUnderTest {
        (Real::new(), BTreeSet::new())
    }

    fn apply(
        (real, mut shadow): Self::SystemUnderTest,
        reference: &BTreeSet<u8>,
        step: Step<u8>,
    ) -> Self::SystemUnderTest {
        let real = real.apply(&mut shadow, step);
        assert_eq!(&shadow, reference);
        (real, shadow)
    }

    fn check_invariants((real, _): &Self::SystemUnderTest, reference: &BTreeSet<u8>) {
        real.check(reference, 0..16);
    }
}

prop_state_machine! {
    #![proptest_config(Config::default())]

    /// Sampled sequences of up to 40 steps over 16 keys, shrunk on failure.
    #[test]
    fn sampled_operation_sequences_match_the_model(sequential 1..40 => Families);
}

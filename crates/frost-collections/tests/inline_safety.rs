//! Drop accounting and panic safety for the `unsafe` inline representation.
//! Run under Miri: `cargo +nightly miri test --test inline_safety`.
#![cfg(feature = "std")]

use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use frost::prelude::*;

#[derive(Debug)]
struct Counted {
    drops: Rc<Cell<usize>>,
    clones_left: Rc<Cell<usize>>,
}

impl Clone for Counted {
    fn clone(&self) -> Self {
        let left = self.clones_left.get();
        assert!(left > 0, "clone budget exhausted");
        self.clones_left.set(left - 1);
        Self {
            drops: Rc::clone(&self.drops),
            clones_left: Rc::clone(&self.clones_left),
        }
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

fn make(n: usize, budget: usize) -> (Vec<Counted>, Rc<Cell<usize>>) {
    let drops = Rc::new(Cell::new(0));
    let clones_left = Rc::new(Cell::new(budget));
    let v = (0..n)
        .map(|_| Counted {
            drops: Rc::clone(&drops),
            clones_left: Rc::clone(&clones_left),
        })
        .collect();
    (v, drops)
}

#[test]
fn drop_counts_inline_and_heap() {
    for n in [0, 1, 3, 4, 5, 9] {
        let (v, drops) = make(n, 0);
        let f: InlineFrozenVec<Counted, 4> = v.freeze_with(Inline::<4>);
        assert_eq!(f.len(), n);
        assert_eq!(drops.get(), 0, "no drops while moving inline");
        drop(f);
        assert_eq!(drops.get(), n, "each element dropped exactly once");
    }
}

#[test]
fn thaw_transfers_ownership_without_double_drop() {
    for n in [0, 2, 4, 6] {
        let (v, drops) = make(n, 0);
        let f: InlineFrozenVec<Counted, 4> = v.freeze_with(Inline::<4>);
        let back = f.thaw();
        assert_eq!(drops.get(), 0);
        assert_eq!(back.len(), n);
        drop(back);
        assert_eq!(drops.get(), n);
    }
}

#[test]
fn panicking_clone_drops_partial_copy_exactly_once() {
    let (v, drops) = make(4, 2); // third clone panics
    let f: InlineFrozenVec<Counted, 4> = v.freeze_with(Inline::<4>);
    let r = catch_unwind(AssertUnwindSafe(|| f.clone()));
    assert!(r.is_err());
    assert_eq!(drops.get(), 2, "two finished clones dropped during unwind");
    drop(f);
    assert_eq!(drops.get(), 6);
}

#[test]
fn zero_sized_and_zero_capacity() {
    let z: InlineFrozenVec<(), 3> = vec![(), (), ()].freeze_with(Inline::<3>);
    assert_eq!(z.len(), 3);
    assert_eq!(z.thaw().len(), 3);
    let none: InlineFrozenVec<String, 0> = vec!["x".to_owned()].freeze_with(Inline::<0>);
    assert!(!none.is_inline());
    let empty: InlineFrozenVec<String, 0> = InlineFrozenVec::default();
    assert!(empty.is_inline() && empty.is_empty());
}

#[test]
fn inline_vec_ops_drop_exactly_once() {
    let (v, drops) = make(6, 0);
    let mut iv: InlineVec<Counted, 4> = InlineVec::new();
    for (i, x) in v.into_iter().enumerate() {
        iv.push(x);
        if i == 3 {
            assert!(iv.is_inline());
        }
    }
    assert!(!iv.is_inline());
    drop(iv.pop());
    drop(iv.remove(0));
    assert_eq!(drops.get(), 2);
    iv.shrink_to_fit(); // 4 left: back inline, moved not dropped
    assert!(iv.is_inline());
    assert_eq!(drops.get(), 2);
    let mut n = 0;
    iv.retain(|_| {
        n += 1;
        n % 2 == 0
    });
    assert_eq!(drops.get(), 4);
    drop(iv);
    assert_eq!(drops.get(), 6);
}

#[test]
fn partially_consumed_owning_iterators_drop_the_rest() {
    for n in [0, 3, 4, 7] {
        let (v, drops) = make(n, 0);
        let f: InlineFrozenVec<Counted, 4> = v.freeze_with(Inline::<4>);
        let mut it = f.into_iter();
        let first = it.next();
        let last = it.next_back();
        drop((first, last));
        drop(it);
        assert_eq!(drops.get(), n);
    }
}

#[test]
fn panicking_retain_predicate_leaves_no_leak_or_double_drop() {
    let (v, drops) = make(4, 0);
    let mut iv: InlineVec<Counted, 4> = v.into_iter().collect();
    let mut seen = 0;
    let r = catch_unwind(AssertUnwindSafe(|| {
        iv.retain(|_| {
            seen += 1;
            assert!(seen < 3, "boom");
            true
        });
    }));
    assert!(r.is_err());
    drop(iv);
    assert_eq!(drops.get(), 4, "every element dropped exactly once");
}

#[test]
fn panicking_hint_after_spilling_drops_every_produced_value() {
    struct PanickingHint {
        produced: Rc<Cell<usize>>,
        dropped: Rc<Cell<usize>>,
    }

    impl Iterator for PanickingHint {
        type Item = Counted;

        fn next(&mut self) -> Option<Counted> {
            self.produced.set(self.produced.get() + 1);
            Some(Counted {
                drops: Rc::clone(&self.dropped),
                clones_left: Rc::new(Cell::new(0)),
            })
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            panic!("remaining heap reservation hint panicked");
        }
    }

    let produced = Rc::new(Cell::new(0));
    let dropped = Rc::new(Cell::new(0));
    let failure = catch_unwind(AssertUnwindSafe(|| {
        PanickingHint {
            produced: Rc::clone(&produced),
            dropped: Rc::clone(&dropped),
        }
        .collect::<InlineFrozenVec<_, 4>>()
    }));
    assert!(failure.is_err());
    assert!(produced.get() > 4, "only a spill reaches the hint");
    assert_eq!(dropped.get(), produced.get());
}

#[test]
fn rejected_element_drop_panic_restores_the_unvisited_suffix() {
    struct Value {
        id: usize,
        dropped: Rc<RefCell<Vec<usize>>>,
    }

    impl Drop for Value {
        fn drop(&mut self) {
            self.dropped.borrow_mut().push(self.id);
            assert_ne!(self.id, 1, "rejected element destructor panicked");
        }
    }

    let dropped = Rc::new(RefCell::new(Vec::new()));
    let mut values: InlineVec<_, 4> = (0..4)
        .map(|id| Value {
            id,
            dropped: Rc::clone(&dropped),
        })
        .collect();
    let failure = catch_unwind(AssertUnwindSafe(|| values.retain(|value| value.id != 1)));
    assert!(failure.is_err());
    assert_eq!(
        values.iter().map(|value| value.id).collect::<Vec<_>>(),
        [0, 2, 3]
    );
    drop(values);
    dropped.borrow_mut().sort_unstable();
    assert_eq!(*dropped.borrow(), [0, 1, 2, 3]);
}

#[test]
fn owning_iterator_drop_panic_still_drops_its_remaining_range() {
    struct Value {
        id: usize,
        dropped: Rc<RefCell<Vec<usize>>>,
    }

    impl Drop for Value {
        fn drop(&mut self) {
            self.dropped.borrow_mut().push(self.id);
            assert_ne!(self.id, 2, "one remaining element destructor panicked");
        }
    }

    for length in [4, 7] {
        let dropped = Rc::new(RefCell::new(Vec::new()));
        let values: InlineVec<_, 4> = (0..length)
            .map(|id| Value {
                id,
                dropped: Rc::clone(&dropped),
            })
            .collect();
        let mut iter = values.into_iter();
        drop(iter.next());
        drop(iter.next_back());
        let failure = catch_unwind(AssertUnwindSafe(|| drop(iter)));
        assert!(failure.is_err());
        dropped.borrow_mut().sort_unstable();
        assert_eq!(*dropped.borrow(), (0..length).collect::<Vec<_>>());
    }
}

#[test]
fn over_aligned_elements_survive_mutation_spill_and_thaw() {
    #[repr(align(256))]
    struct Aligned(String);

    let mut values: InlineVec<_, 2> = [Aligned(String::from("a")), Aligned(String::from("b"))]
        .into_iter()
        .collect();
    assert_eq!(values.as_ptr().addr() % align_of::<Aligned>(), 0);
    values.push(Aligned(String::from("c")));
    assert!(!values.is_inline());
    assert_eq!(values.remove(1).0, "b");
    values.shrink_to_fit();
    assert!(values.is_inline());
    let mut values = values.freeze().thaw();
    assert_eq!(values.as_ptr().addr() % align_of::<Aligned>(), 0);
    values[0].0.push('!');
    assert_eq!(
        values.into_iter().map(|value| value.0).collect::<Vec<_>>(),
        ["a!", "c"]
    );
}

#[test]
fn zero_sized_destructors_run_once_through_inline_ownership() {
    thread_local! {
        static DROPS: Cell<usize> = const { Cell::new(0) };
    }

    struct Dropped;
    impl Drop for Dropped {
        fn drop(&mut self) {
            DROPS.with(|drops| drops.set(drops.get() + 1));
        }
    }

    DROPS.with(|drops| drops.set(0));
    let values: InlineVec<_, 4> = (0..4).map(|_| Dropped).collect();
    let mut iter = values.freeze().thaw().into_iter();
    drop(iter.next());
    drop(iter.next_back());
    drop(iter);
    assert_eq!(DROPS.with(Cell::get), 4);
}

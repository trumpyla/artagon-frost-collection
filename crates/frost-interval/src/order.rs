//! `DownSet<V>`: a downward-closed set over a partial order, stored as the
//! antichain of its maximal elements. This is the set form for domains where
//! `Cut` has no meaning (permission sets, labels, versions under a partial order).

use crate::domain::{Lattice, Poset};
use alloc::vec::Vec;

/// `{ x | x <= g for some generator g }`. The generators are an antichain.
#[derive(Clone, Debug)]
pub struct DownSet<V> {
    maximal: Vec<V>,
}

impl<V: Poset> DownSet<V> {
    /// The empty down-set.
    pub fn empty() -> Self {
        DownSet {
            maximal: Vec::new(),
        }
    }
    /// Adds `v` and everything below it. Returns `false` if it was covered.
    pub fn insert(&mut self, v: V) -> bool {
        if self.contains(&v) {
            return false;
        }
        self.maximal.retain(|m| !m.leq(&v));
        self.maximal.push(v);
        true
    }
    /// `true` if `v` is at or below one of the maximal elements.
    pub fn contains(&self, v: &V) -> bool {
        self.maximal.iter().any(|m| v.leq(m))
    }
    /// `true` if the down-set has no members.
    pub fn is_empty(&self) -> bool {
        self.maximal.is_empty()
    }
    /// The maximal elements, in no fixed order.
    pub fn generators(&self) -> &[V] {
        &self.maximal
    }
    /// `true` if every member of `self` is a member of `o`.
    pub fn is_subset(&self, o: &Self) -> bool {
        self.maximal.iter().all(|m| o.contains(m))
    }
    #[must_use]
    /// The union of `self` and `o`.
    pub fn union(&self, o: &Self) -> Self {
        let mut out = self.clone();
        for m in &o.maximal {
            out.insert(m.clone());
        }
        out
    }
}

impl<V: Lattice> DownSet<V> {
    /// Uses `down(a) ∩ down(b) = down(meet(a, b))`.
    #[must_use]
    pub fn intersection(&self, o: &Self) -> Self {
        let mut out = Self::empty();
        for a in &self.maximal {
            for b in &o.maximal {
                out.insert(a.meet(b));
            }
        }
        out
    }
}

/// Equal when they hold the same values; generator order does not matter.
impl<V: Poset> PartialEq for DownSet<V> {
    fn eq(&self, o: &Self) -> bool {
        self.maximal.len() == o.maximal.len() && self.is_subset(o) && o.is_subset(self)
    }
}

impl<V: Poset> FromIterator<V> for DownSet<V> {
    fn from_iter<I: IntoIterator<Item = V>>(it: I) -> Self {
        let mut s = Self::empty();
        for v in it {
            s.insert(v);
        }
        s
    }
}

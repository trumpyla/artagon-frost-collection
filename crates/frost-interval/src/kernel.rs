//! Domain-free kernels. Everything here works on sorted boundaries only and
//! does not know the value type `V`.
//!
//! This is the point of the encoding design: every domain whose word is a
//! `u64` (`u64`, `i64`, `usize`, `UserId`, `NotNanF64`, a time of day, ...) uses the
//! same `RawSet<Vec<u64>>` code. The typed collections in `set` and `map` are
//! thin wrappers that convert cuts to boundaries and call these kernels.
//!
//! | Layout | Used for | Bytes per boundary |
//! |---|---|---|
//! | `Vec<I>` | discrete domains: a boundary is the word of a `Below` cut | `size_of::<I>()` |
//! | [`SideBitVec<I>`] | dense domains: a word plus one `Below`/`Above` bit | `size_of::<I>()` + 1 bit |

use alloc::collections::BinaryHeap;
use alloc::{vec, vec::Vec};
use core::cmp::Ordering;

/// A position on the boundary line. Same names as the edge cuts.
/// Derived `Ord` is correct: one variant carries data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RawCut<B> {
    /// Below every boundary.
    BelowAll,
    /// At this boundary.
    At(B),
    /// Above every boundary.
    AboveAll,
}

/// Which side of a word a dense boundary sits on. `Below < Above`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Side {
    /// The boundary is just below the word.
    Below = 0,
    /// The boundary is just above the word.
    Above = 1,
}

/// A `(lower, upper)` pair of positions.
pub type RawInterval<B> = (
    RawCut<<B as Boundaries>::Owned>,
    RawCut<<B as Boundaries>::Owned>,
);

/// A sorted sequence of boundaries.
pub trait Boundaries: Clone + PartialEq {
    /// An owned boundary.
    type Owned: Ord + Clone;
    /// A borrowed boundary.
    type Ref<'a>: Ord + Copy
    where
        Self: 'a;

    /// An empty sequence with room for `n` boundaries.
    fn with_capacity(n: usize) -> Self;
    /// The number of boundaries.
    fn len(&self) -> usize;
    /// `true` if there are no boundaries.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// The boundary at position `i`.
    ///
    /// # Panics
    ///
    /// Panics if `i >= self.len()`.
    fn get(&self, i: usize) -> Self::Ref<'_>;
    /// Appends an owned boundary.
    fn push(&mut self, b: Self::Owned);
    /// Appends a copy of a borrowed boundary.
    fn push_ref(&mut self, b: Self::Ref<'_>);
    /// An owned copy of a boundary.
    fn own(b: Self::Ref<'_>) -> Self::Owned;
    /// The heap memory, in bytes.
    fn heap_bytes(&self) -> usize;
}

impl<I: Ord + Clone> Boundaries for Vec<I> {
    type Owned = I;
    type Ref<'a>
        = &'a I
    where
        I: 'a;

    fn with_capacity(n: usize) -> Self {
        Vec::with_capacity(n)
    }
    #[inline]
    fn len(&self) -> usize {
        <[I]>::len(self)
    }
    #[inline]
    fn get(&self, i: usize) -> &I {
        &self[i]
    }
    #[inline]
    fn push(&mut self, b: I) {
        Vec::push(self, b);
    }
    #[inline]
    fn push_ref(&mut self, b: &I) {
        Vec::push(self, b.clone());
    }
    #[inline]
    fn own(b: &I) -> I {
        b.clone()
    }
    fn heap_bytes(&self) -> usize {
        self.capacity() * size_of::<I>()
    }
}

/// Number of discrete boundaries at or below the word `x`.
#[inline]
pub fn segment_of<I: Ord>(bounds: &[I], x: &I) -> usize {
    bounds.partition_point(|b| b <= x)
}

/// Dense layout, struct-of-arrays: words plus one side bit per boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideBitVec<I> {
    words: Vec<I>,
    sides: Vec<u64>, // bit i = 1: boundary i is Above(words[i])
}

impl<I: Ord> SideBitVec<I> {
    #[inline]
    fn side(&self, i: usize) -> Side {
        if (self.sides[i >> 6] >> (i & 63)) & 1 == 1 {
            Side::Above
        } else {
            Side::Below
        }
    }
    #[inline]
    fn push_side(&mut self, s: Side) {
        let i = self.words.len() - 1;
        if i.is_multiple_of(64) {
            self.sides.push(0);
        }
        self.sides[i >> 6] |= (s as u64) << (i & 63);
    }
    /// Number of boundaries at or below `Below(x)`. It searches the words and
    /// reads a side bit only on a tie.
    #[inline]
    pub fn segment_of(&self, x: &I) -> usize {
        let p = self.words.partition_point(|b| b < x);
        if p < self.words.len() && self.words[p] == *x && self.side(p) == Side::Below {
            p + 1
        } else {
            p
        }
    }
}

impl<I: Ord + Clone> Boundaries for SideBitVec<I> {
    type Owned = (I, Side);
    type Ref<'a>
        = (&'a I, Side)
    where
        I: 'a;

    fn with_capacity(n: usize) -> Self {
        SideBitVec {
            words: Vec::with_capacity(n),
            sides: Vec::with_capacity(n.div_ceil(64)),
        }
    }
    #[inline]
    fn len(&self) -> usize {
        self.words.len()
    }
    #[inline]
    fn get(&self, i: usize) -> (&I, Side) {
        (&self.words[i], self.side(i))
    }
    #[inline]
    fn push(&mut self, (word, side): (I, Side)) {
        self.words.push(word);
        self.push_side(side);
    }
    #[inline]
    fn push_ref(&mut self, (word, side): (&I, Side)) {
        self.words.push(word.clone());
        self.push_side(side);
    }
    #[inline]
    fn own((word, side): (&I, Side)) -> (I, Side) {
        (word.clone(), side)
    }
    fn heap_bytes(&self) -> usize {
        self.words.capacity() * size_of::<I>() + self.sides.capacity() * 8
    }
}

// --------------------------------------------------------------------- set

/// A toggle set: membership flips at each boundary. `starts_inside` is the
/// state below the first boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawSet<B> {
    pub(crate) starts_inside: bool,
    pub(crate) bounds: B,
}

impl<B: Boundaries> RawSet<B> {
    /// The empty set.
    pub fn empty() -> Self {
        RawSet {
            starts_inside: false,
            bounds: B::with_capacity(0),
        }
    }
    /// The set of all words.
    pub fn all() -> Self {
        RawSet {
            starts_inside: true,
            bounds: B::with_capacity(0),
        }
    }
    /// Builds from `(lower, upper)` position pairs in any order: sort, sweep.
    pub fn from_spans(mut spans: Vec<RawInterval<B>>) -> Self {
        spans.retain(|(lo, hi)| lo < hi);
        spans.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        let mut out = Self::empty();
        out.bounds = B::with_capacity(spans.len() * 2);
        let mut it = spans.into_iter();
        let Some((mut lo, mut hi)) = it.next() else {
            return out;
        };
        let emit = |out: &mut Self, lo: RawCut<B::Owned>, hi: RawCut<B::Owned>| {
            match lo {
                RawCut::BelowAll => out.starts_inside = true,
                RawCut::At(b) => out.bounds.push(b),
                RawCut::AboveAll => return, // not reachable: `lo < hi` was checked
            }
            if let RawCut::At(b) = hi {
                out.bounds.push(b);
            }
        };
        for (l, h) in it {
            if l <= hi {
                if h > hi {
                    hi = h;
                }
            } else {
                emit(&mut out, lo, hi);
                lo = l;
                hi = h;
            }
        }
        emit(&mut out, lo, hi);
        out
    }

    /// Membership of a value that lies in segment `segment`.
    #[inline]
    pub fn inside(&self, segment: usize) -> bool {
        self.starts_inside ^ (segment & 1 == 1)
    }
    /// `true` if the set has no members.
    pub fn is_empty(&self) -> bool {
        !self.starts_inside && self.bounds.is_empty()
    }
    /// `true` if every word is a member.
    pub fn is_all(&self) -> bool {
        self.starts_inside && self.bounds.is_empty()
    }
    /// The number of maximal disjoint intervals.
    pub fn interval_count(&self) -> usize {
        (self.bounds.len() + usize::from(self.starts_inside)).div_ceil(2)
    }
    #[must_use]
    /// The complement. It flips the membership below the first boundary and keeps the boundaries.
    pub fn complement(&self) -> Self {
        RawSet {
            starts_inside: !self.starts_inside,
            bounds: self.bounds.clone(),
        }
    }

    // The four operations are defined here, not in the typed wrapper, so that
    // their closures do not depend on the value type. Each one is compiled
    // once for each boundary layout.
    #[must_use]
    /// The union of `self` and `o`. One linear pass over both boundary lists.
    pub fn union(&self, o: &Self) -> Self {
        self.merge(o, |a, b| a | b)
    }
    #[must_use]
    /// The intersection of `self` and `o`. One linear pass over both boundary lists.
    pub fn intersection(&self, o: &Self) -> Self {
        self.merge(o, |a, b| a & b)
    }
    #[must_use]
    #[allow(
        clippy::needless_bitwise_bool,
        reason = "branch-free segment rule in the merge loop"
    )]
    /// The members of `self` that are not members of `o`. One linear pass over both boundary lists.
    pub fn difference(&self, o: &Self) -> Self {
        self.merge(o, |a, b| a & !b)
    }
    #[must_use]
    /// The members of exactly one of `self` and `o`. One linear pass over both boundary lists.
    pub fn symmetric_difference(&self, o: &Self) -> Self {
        self.merge(o, |a, b| a ^ b)
    }
    /// Every position of `self` is in `o`. One pass, no allocation.
    #[allow(
        clippy::needless_bitwise_bool,
        reason = "branch-free segment rule in the merge loop"
    )]
    pub fn is_subset(&self, o: &Self) -> bool {
        !self.any_segment(o, |a, b| a & !b)
    }
    /// No position is in both. One pass, no allocation.
    pub fn is_disjoint(&self, o: &Self) -> bool {
        !self.any_segment(o, |a, b| a & b)
    }

    /// One linear pass over both boundary sequences; `f` combines membership.
    #[must_use]
    pub fn merge(&self, o: &Self, f: impl Fn(bool, bool) -> bool) -> Self {
        let (ka, kb) = (&self.bounds, &o.bounds);
        let (la, lb) = (ka.len(), kb.len());
        let (mut ia, mut ib) = (self.starts_inside, o.starts_inside);
        let mut cur = f(ia, ib);
        let mut out = RawSet {
            starts_inside: cur,
            bounds: B::with_capacity(la + lb),
        };
        let (mut i, mut j) = (0, 0);
        // Branch-light: the three-way compare becomes two flags, and the only
        // data-dependent branch is the push.
        while i < la && j < lb {
            let ord = ka.get(i).cmp(&kb.get(j));
            let (take_a, take_b) = (ord != Ordering::Greater, ord != Ordering::Less);
            ia ^= take_a;
            ib ^= take_b;
            if f(ia, ib) != cur {
                out.bounds
                    .push_ref(if take_a { ka.get(i) } else { kb.get(j) });
                cur = !cur;
            }
            i += usize::from(take_a);
            j += usize::from(take_b);
        }
        while i < la {
            ia = !ia;
            if f(ia, ib) != cur {
                out.bounds.push_ref(ka.get(i));
                cur = !cur;
            }
            i += 1;
        }
        while j < lb {
            ib = !ib;
            if f(ia, ib) != cur {
                out.bounds.push_ref(kb.get(j));
                cur = !cur;
            }
            j += 1;
        }
        out
    }

    /// True if `f` holds on some non-empty segment. One pass, no allocation.
    pub fn any_segment(&self, o: &Self, f: impl Fn(bool, bool) -> bool) -> bool {
        let (ka, kb) = (&self.bounds, &o.bounds);
        let (la, lb) = (ka.len(), kb.len());
        let (mut ia, mut ib) = (self.starts_inside, o.starts_inside);
        let (mut i, mut j) = (0, 0);
        loop {
            if f(ia, ib) {
                return true;
            }
            let ord = match (i < la, j < lb) {
                (false, false) => return false,
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (true, true) => ka.get(i).cmp(&kb.get(j)),
            };
            if ord != Ordering::Greater {
                ia = !ia;
                i += 1;
            }
            if ord != Ordering::Less {
                ib = !ib;
                j += 1;
            }
        }
    }
}

// --------------------------------------------------------------------- map

/// A piecewise-constant map: `n` boundaries, `n + 1` segment values.
/// Adjacent segments always differ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawMap<B, T> {
    pub(crate) bounds: B,
    pub(crate) vals: Vec<Option<T>>,
}

/// Appends segments and drops a boundary when the value does not change.
struct Builder<B, T> {
    bounds: B,
    vals: Vec<Option<T>>,
}
impl<B: Boundaries, T: PartialEq> Builder<B, T> {
    fn new(first: Option<T>, cap: usize) -> Self {
        let mut vals = Vec::with_capacity(cap + 1);
        vals.push(first);
        Builder {
            bounds: B::with_capacity(cap),
            vals,
        }
    }
    #[inline]
    fn push_ref(&mut self, b: B::Ref<'_>, v: Option<T>) {
        if self.vals.last() != Some(&v) {
            self.bounds.push_ref(b);
            self.vals.push(v);
        }
    }
    #[inline]
    fn push(&mut self, b: B::Owned, v: Option<T>) {
        if self.vals.last() != Some(&v) {
            self.bounds.push(b);
            self.vals.push(v);
        }
    }
    fn finish(self) -> RawMap<B, T> {
        RawMap {
            bounds: self.bounds,
            vals: self.vals,
        }
    }
}

impl<B: Boundaries, T> RawMap<B, T> {
    /// The empty map.
    pub fn empty() -> Self {
        RawMap {
            bounds: B::with_capacity(0),
            vals: vec![None],
        }
    }
    #[inline]
    /// The value of segment `segment`, or `None` if no value is set there.
    ///
    /// # Panics
    ///
    /// Panics if `segment` is greater than the number of boundaries.
    pub fn value(&self, segment: usize) -> Option<&T> {
        self.vals[segment].as_ref()
    }
    /// `true` if the map has no entries.
    pub fn is_empty(&self) -> bool {
        self.bounds.is_empty() && self.vals[0].is_none()
    }
    /// The number of entries: segments with a value.
    pub fn entry_count(&self) -> usize {
        self.vals.iter().filter(|v| v.is_some()).count()
    }
    /// The set of mapped positions.
    pub fn key_set(&self) -> RawSet<B> {
        let mut bounds = B::with_capacity(self.bounds.len());
        let starts_inside = self.vals[0].is_some();
        let mut inside = starts_inside;
        for i in 0..self.bounds.len() {
            let next = self.vals[i + 1].is_some();
            if next != inside {
                bounds.push_ref(self.bounds.get(i));
                inside = next;
            }
        }
        RawSet {
            starts_inside,
            bounds,
        }
    }
}

impl<B: Boundaries, T: Clone + PartialEq> RawMap<B, T> {
    /// Builds from `(lower, upper, value)` entries. Where entries overlap, the
    /// later entry wins. O(n log n): one sort and one sweep.
    pub fn from_entries(
        entries: impl IntoIterator<Item = (RawCut<B::Owned>, RawCut<B::Owned>, T)>,
    ) -> Self {
        let mut values = Vec::new();
        let mut events: Vec<(RawCut<B::Owned>, bool, usize)> = Vec::new();
        for (lo, hi, t) in entries {
            if lo < hi {
                events.push((lo, true, values.len()));
                events.push((hi, false, values.len()));
                values.push(t);
            }
        }
        events.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        let mut ended = vec![false; values.len()];
        let mut active: BinaryHeap<usize> = BinaryHeap::new();
        let mut out = Builder::<B, T>::new(None, events.len());
        let mut it = events.into_iter().peekable();
        while let Some((pos, start, idx)) = it.next() {
            let mut apply = |start: bool, idx: usize| {
                if start {
                    active.push(idx);
                } else {
                    ended[idx] = true;
                }
            };
            apply(start, idx);
            while let Some((_, s, i)) = it.next_if(|e| e.0 == pos) {
                apply(s, i);
            }
            while active.peek().is_some_and(|&i| ended[i]) {
                active.pop();
            }
            let cur = active.peek().map(|&i| values[i].clone());
            match pos {
                RawCut::BelowAll => out.vals[0] = cur,
                RawCut::At(b) => out.push(b, cur),
                RawCut::AboveAll => {}
            }
        }
        out.finish()
    }

    /// A set as a map: every member maps to `value`.
    pub fn from_set(set: &RawSet<B>, value: &T) -> Self {
        let mut inside = set.starts_inside;
        let mut out = Builder::<B, T>::new(inside.then(|| value.clone()), set.bounds.len());
        for i in 0..set.bounds.len() {
            inside = !inside;
            out.push_ref(set.bounds.get(i), inside.then(|| value.clone()));
        }
        out.finish()
    }

    /// The general binary operation: one linear pass over both maps.
    #[allow(
        clippy::many_single_char_names,
        reason = "merge indices and operands, as in the kernel description"
    )]
    pub fn merge_with<U, W: PartialEq>(
        &self,
        o: &RawMap<B, U>,
        mut f: impl FnMut(Option<&T>, Option<&U>) -> Option<W>,
    ) -> RawMap<B, W> {
        let (ka, kb) = (&self.bounds, &o.bounds);
        let (la, lb) = (ka.len(), kb.len());
        let (mut i, mut j) = (0, 0);
        let mut out = Builder::<B, W>::new(f(self.vals[0].as_ref(), o.vals[0].as_ref()), la + lb);
        while i < la || j < lb {
            let ord = match (i < la, j < lb) {
                (true, true) => ka.get(i).cmp(&kb.get(j)),
                (true, false) => Ordering::Less,
                _ => Ordering::Greater,
            };
            let b = if ord == Ordering::Greater {
                kb.get(j)
            } else {
                ka.get(i)
            };
            if ord != Ordering::Greater {
                i += 1;
            }
            if ord != Ordering::Less {
                j += 1;
            }
            out.push_ref(b, f(self.vals[i].as_ref(), o.vals[j].as_ref()));
        }
        out.finish()
    }

    /// `o` on top of `self`: where both map a value, `o` wins.
    #[must_use]
    pub fn overlay(&self, o: &Self) -> Self {
        self.merge_with(o, |a, b| b.or(a).cloned())
    }
    /// Union with aggregation on overlap.
    #[must_use]
    pub fn union_with(&self, o: &Self, mut f: impl FnMut(&T, &T) -> T) -> Self {
        self.merge_with(o, |a, b| match (a, b) {
            (Some(a), Some(b)) => Some(f(a, b)),
            (a, b) => a.or(b).cloned(),
        })
    }
    /// The part where both maps have a value, combined with `f`.
    pub fn intersection_with<U, W: PartialEq>(
        &self,
        o: &RawMap<B, U>,
        mut f: impl FnMut(&T, &U) -> W,
    ) -> RawMap<B, W> {
        self.merge_with(o, |a, b| Some(f(a?, b?)))
    }
    /// The entries cut down to `set`.
    #[must_use]
    pub fn restrict(&self, set: &RawSet<B>) -> Self {
        self.merge_with(&RawMap::from_set(set, &()), |a, b| b.and(a).cloned())
    }
    /// The entries with `set` removed.
    #[must_use]
    pub fn remove(&self, set: &RawSet<B>) -> Self {
        self.merge_with(&RawMap::from_set(set, &()), |a, b| {
            if b.is_some() { None } else { a.cloned() }
        })
    }

    /// Maps every value; neighbors that become equal are coalesced.
    pub fn map_values<W: PartialEq>(&self, mut f: impl FnMut(&T) -> W) -> RawMap<B, W> {
        let mut out = Builder::<B, W>::new(self.vals[0].as_ref().map(&mut f), self.bounds.len());
        for i in 0..self.bounds.len() {
            out.push_ref(self.bounds.get(i), self.vals[i + 1].as_ref().map(&mut f));
        }
        out.finish()
    }
}

//! Mutable, domain-free kernel: a B+tree of boundaries.
//!
//! [`BoundaryTree<E>`] holds a sorted set of boundaries `E` (a word, or a word
//! and a side bit). Each inner node stores, for each child, the number of
//! boundaries in that child. So the tree gives the rank of a position in
//! one descent, and the rank parity gives membership.
//!
//! The only update is [`BoundaryTree::toggle`]: remove the boundary if it is
//! present, add it if it is absent. Because a set is a toggle set, the
//! symmetric difference of a set and an interval is two toggles. Insert and
//! remove of one value are built on that in the typed layer.
//!
//! Like the frozen kernels, the tree does not know the value type. Every
//! domain with a `u64` word uses the same `BoundaryTree<u64>` code.
//!
//! Nodes live in an arena (`Vec<Node<E>>`) with `u32` links. There is no
//! `unsafe` code.

use alloc::{vec, vec::Vec};
use core::mem;

/// Default number of boundaries in a full leaf.
pub const LEAF: usize = 64;
/// Default number of children in a full inner node.
pub const FANOUT: usize = 64;

#[derive(Clone, Debug)]
enum Node<E> {
    Leaf(Vec<E>),
    Inner(Inner<E>),
}

/// `seps[j]` is a lower bound for the subtree `kids[j + 1]` and an upper
/// bound (exclusive) for `kids[j]`. A separator can be stale after a delete;
/// the bound property still holds, so routing stays correct.
#[derive(Clone, Debug)]
struct Inner<E> {
    seps: Vec<E>,
    kids: Vec<u32>,
    counts: Vec<usize>,
}

enum Change<E> {
    /// One boundary was added. `Some` holds a new right sibling and its separator.
    Grew(Option<(E, u32)>),
    /// One boundary was removed. `true`: the node is under its minimum fill.
    Shrank(bool),
}

/// A sorted set of boundaries with rank queries. See the module docs.
///
/// `L` is the leaf capacity and `F` the inner fan-out. Both must be at least 4.
#[derive(Clone, Debug)]
pub struct BoundaryTree<E, const L: usize = LEAF, const F: usize = FANOUT> {
    nodes: Vec<Node<E>>,
    free: Vec<u32>,
    root: u32,
    len: usize,
}

impl<E: Ord + Clone, const L: usize, const F: usize> Default for BoundaryTree<E, L, F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Ord + Clone, const L: usize, const F: usize> BoundaryTree<E, L, F> {
    const CHECK: () = assert!(
        L >= 4 && F >= 4,
        "leaf capacity and fan-out must be at least 4"
    );

    /// An empty tree.
    ///
    /// The build fails if `L` or `F` is less than 4.
    pub fn new() -> Self {
        let () = Self::CHECK;
        BoundaryTree {
            nodes: vec![Node::Leaf(Vec::new())],
            free: Vec::new(),
            root: 0,
            len: 0,
        }
    }

    /// Builds from strictly increasing boundaries in O(n). Every leaf and
    /// inner node gets at least half of its capacity.
    pub fn from_sorted(keys: impl IntoIterator<Item = E>) -> Self {
        let keys: Vec<E> = keys.into_iter().collect();
        debug_assert!(
            keys.windows(2).all(|w| w[0] < w[1]),
            "boundaries must be strictly increasing"
        );
        let mut t = Self::new();
        t.len = keys.len();
        if keys.len() <= L {
            t.nodes[0] = Node::Leaf(keys);
            return t;
        }
        t.nodes.clear();
        // Level of (node, smallest key, count).
        let mut level: Vec<(u32, E, usize)> = Vec::new();
        let mut it = keys.into_iter();
        for size in even_sizes(t.len, L) {
            let mut leaf = Vec::with_capacity(L + 1);
            leaf.extend(it.by_ref().take(size));
            let min = leaf[0].clone();
            level.push((t.alloc(Node::Leaf(leaf)), min, size));
        }
        while level.len() > 1 {
            let mut next = Vec::new();
            let mut it = level.into_iter();
            for size in even_sizes(it.len(), F) {
                let group: Vec<_> = it.by_ref().take(size).collect();
                let min = group[0].1.clone();
                let total = group.iter().map(|g| g.2).sum();
                let seps = group[1..].iter().map(|g| g.1.clone()).collect();
                let kids = group.iter().map(|g| g.0).collect();
                let counts = group.iter().map(|g| g.2).collect();
                next.push((
                    t.alloc(Node::Inner(Inner { seps, kids, counts })),
                    min,
                    total,
                ));
            }
            level = next;
        }
        t.root = level[0].0;
        t
    }

    /// The number of boundaries.
    pub fn len(&self) -> usize {
        self.len
    }
    /// `true` if the tree has no boundaries.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Number of boundaries `<= key`.
    #[inline]
    pub fn rank_le(&self, key: &E) -> usize {
        self.rank_by(|e| e <= key)
    }

    /// Number of boundaries for which `before` is true. `before` must be
    /// true on a prefix of the sorted boundaries and false after it.
    #[inline]
    pub fn rank_by(&self, before: impl Fn(&E) -> bool) -> usize {
        let mut n = self.root;
        let mut rank = 0;
        loop {
            match &self.nodes[n as usize] {
                Node::Leaf(keys) => return rank + keys.partition_point(&before),
                Node::Inner(inner) => {
                    let i = inner.seps.partition_point(&before);
                    rank += inner.counts[..i].iter().sum::<usize>();
                    n = inner.kids[i];
                }
            }
        }
    }

    /// Removes `e` if present, adds it if absent. Returns `true` if it added.
    pub fn toggle(&mut self, e: E) -> bool {
        let root = self.root;
        match self.toggle_at(root, e) {
            Change::Grew(split) => {
                self.len += 1;
                if let Some((sep, right)) = split {
                    let counts = vec![self.count_of(root), self.count_of(right)];
                    let inner = Inner {
                        seps: vec![sep],
                        kids: vec![root, right],
                        counts,
                    };
                    self.root = self.alloc(Node::Inner(inner));
                }
                true
            }
            Change::Shrank(_) => {
                self.len -= 1;
                if let Node::Inner(inner) = &self.nodes[root as usize]
                    && inner.kids.len() == 1
                {
                    self.root = inner.kids[0];
                    self.release(root);
                }
                false
            }
        }
    }

    /// Calls `f` on every boundary in increasing order.
    pub fn for_each(&self, mut f: impl FnMut(&E)) {
        self.visit(self.root, &mut f);
    }

    /// The boundaries in increasing order.
    pub fn to_vec(&self) -> Vec<E> {
        let mut out = Vec::with_capacity(self.len);
        self.for_each(|e| out.push(e.clone()));
        out
    }

    /// Bytes of heap memory: arena plus node buffers.
    pub fn heap_bytes(&self) -> usize {
        let nodes = self.nodes.capacity() * size_of::<Node<E>>() + self.free.capacity() * 4;
        let bufs: usize = self
            .nodes
            .iter()
            .map(|n| match n {
                Node::Leaf(k) => k.capacity() * size_of::<E>(),
                Node::Inner(i) => {
                    i.seps.capacity() * size_of::<E>()
                        + i.kids.capacity() * 4
                        + i.counts.capacity() * size_of::<usize>()
                }
            })
            .sum();
        nodes + bufs
    }

    /// Height of the tree: 1 for a single leaf.
    pub fn height(&self) -> usize {
        let mut n = self.root;
        let mut h = 1;
        while let Node::Inner(inner) = &self.nodes[n as usize] {
            n = inner.kids[0];
            h += 1;
        }
        h
    }

    /// Checks every structural invariant and panics on the first failure.
    /// For tests; it walks the whole tree.
    ///
    /// # Panics
    ///
    /// Panics if an invariant does not hold. It is a check for tests.
    pub fn check(&self) {
        let mut leaf_depth = None;
        let mut prev: Option<E> = None;
        let mut reached = 0;
        let count = self.check_at(
            self.root,
            1,
            true,
            None,
            None,
            &mut leaf_depth,
            &mut prev,
            &mut reached,
        );
        assert_eq!(
            count, self.len,
            "len does not match the number of boundaries"
        );
        assert_eq!(
            reached + self.free.len(),
            self.nodes.len(),
            "a node is neither reachable nor free"
        );
    }

    // ------------------------------------------------------------ internals

    fn alloc(&mut self, node: Node<E>) -> u32 {
        if let Some(id) = self.free.pop() {
            self.nodes[id as usize] = node;
            id
        } else {
            self.nodes.push(node);
            u32::try_from(self.nodes.len() - 1).expect("at most 2^32 nodes")
        }
    }

    fn release(&mut self, id: u32) {
        self.nodes[id as usize] = Node::Leaf(Vec::new());
        self.free.push(id);
    }

    fn count_of(&self, id: u32) -> usize {
        match &self.nodes[id as usize] {
            Node::Leaf(keys) => keys.len(),
            Node::Inner(inner) => inner.counts.iter().sum(),
        }
    }

    fn inner_mut(&mut self, id: u32) -> &mut Inner<E> {
        match &mut self.nodes[id as usize] {
            Node::Inner(inner) => inner,
            Node::Leaf(_) => unreachable!("expected an inner node"),
        }
    }

    fn toggle_at(&mut self, n: u32, e: E) -> Change<E> {
        let route = match &self.nodes[n as usize] {
            Node::Leaf(_) => None,
            Node::Inner(inner) => {
                let i = inner.seps.partition_point(|s| *s <= e);
                Some((i, inner.kids[i]))
            }
        };
        let Some((i, kid)) = route else {
            return self.toggle_leaf(n, e);
        };
        match self.toggle_at(kid, e) {
            Change::Grew(None) => {
                self.inner_mut(n).counts[i] += 1;
                Change::Grew(None)
            }
            Change::Grew(Some((sep, right))) => {
                let (left_count, right_count) = (self.count_of(kid), self.count_of(right));
                let inner = self.inner_mut(n);
                inner.counts[i] = left_count;
                inner.counts.insert(i + 1, right_count);
                inner.kids.insert(i + 1, right);
                inner.seps.insert(i, sep);
                if inner.kids.len() <= F {
                    return Change::Grew(None);
                }
                let mid = inner.kids.len() / 2;
                let kids = inner.kids.split_off(mid);
                let counts = inner.counts.split_off(mid);
                let mut seps = inner.seps.split_off(mid - 1);
                let up = seps.remove(0);
                let id = self.alloc(Node::Inner(Inner { seps, kids, counts }));
                Change::Grew(Some((up, id)))
            }
            Change::Shrank(under) => {
                self.inner_mut(n).counts[i] -= 1;
                if under {
                    self.fix_underflow(n, i);
                }
                Change::Shrank(self.inner_mut(n).kids.len() < F / 2)
            }
        }
    }

    fn toggle_leaf(&mut self, n: u32, e: E) -> Change<E> {
        let Node::Leaf(keys) = &mut self.nodes[n as usize] else {
            unreachable!("expected a leaf")
        };
        let pos = keys.partition_point(|k| *k < e);
        if pos < keys.len() && keys[pos] == e {
            keys.remove(pos);
            return Change::Shrank(keys.len() < L / 2);
        }
        keys.insert(pos, e);
        if keys.len() <= L {
            return Change::Grew(None);
        }
        let mut right = Vec::with_capacity(L + 1);
        right.extend(keys.drain(keys.len() / 2..));
        let sep = right[0].clone();
        let id = self.alloc(Node::Leaf(right));
        Change::Grew(Some((sep, id)))
    }

    /// Child `i` of inner node `n` is under its minimum fill. Merge it with a
    /// sibling, or share boundaries with the sibling.
    #[allow(
        clippy::many_single_char_names,
        reason = "node, index, left and right siblings, as in B+tree texts"
    )]
    fn fix_underflow(&mut self, n: u32, i: usize) {
        let a = i.saturating_sub(1);
        let (l, r, sep) = {
            let inner = self.inner_mut(n);
            (inner.kids[a], inner.kids[a + 1], inner.seps[a].clone())
        };
        let left = mem::replace(&mut self.nodes[l as usize], Node::Leaf(Vec::new()));
        let right = mem::replace(&mut self.nodes[r as usize], Node::Leaf(Vec::new()));
        match (left, right) {
            (Node::Leaf(mut lk), Node::Leaf(rk)) => {
                lk.extend(rk);
                if lk.len() <= L {
                    let total = lk.len();
                    self.nodes[l as usize] = Node::Leaf(lk);
                    self.release(r);
                    let inner = self.inner_mut(n);
                    inner.seps.remove(a);
                    inner.kids.remove(a + 1);
                    inner.counts.remove(a + 1);
                    inner.counts[a] = total;
                } else {
                    let mut rk = Vec::with_capacity(L + 1);
                    rk.extend(lk.drain(lk.len() / 2..));
                    let (lc, rc, new_sep) = (lk.len(), rk.len(), rk[0].clone());
                    self.nodes[l as usize] = Node::Leaf(lk);
                    self.nodes[r as usize] = Node::Leaf(rk);
                    let inner = self.inner_mut(n);
                    inner.seps[a] = new_sep;
                    inner.counts[a] = lc;
                    inner.counts[a + 1] = rc;
                }
            }
            (Node::Inner(mut li), Node::Inner(ri)) => {
                li.seps.push(sep);
                li.seps.extend(ri.seps);
                li.kids.extend(ri.kids);
                li.counts.extend(ri.counts);
                if li.kids.len() <= F {
                    let total = li.counts.iter().sum();
                    self.nodes[l as usize] = Node::Inner(li);
                    self.release(r);
                    let inner = self.inner_mut(n);
                    inner.seps.remove(a);
                    inner.kids.remove(a + 1);
                    inner.counts.remove(a + 1);
                    inner.counts[a] = total;
                } else {
                    let mid = li.kids.len() / 2;
                    let kids = li.kids.split_off(mid);
                    let counts = li.counts.split_off(mid);
                    let mut seps = li.seps.split_off(mid - 1);
                    let up = seps.remove(0);
                    let (lc, rc) = (li.counts.iter().sum(), counts.iter().sum());
                    self.nodes[l as usize] = Node::Inner(li);
                    self.nodes[r as usize] = Node::Inner(Inner { seps, kids, counts });
                    let inner = self.inner_mut(n);
                    inner.seps[a] = up;
                    inner.counts[a] = lc;
                    inner.counts[a + 1] = rc;
                }
            }
            _ => unreachable!("siblings have the same height"),
        }
    }

    fn visit(&self, n: u32, f: &mut impl FnMut(&E)) {
        match &self.nodes[n as usize] {
            Node::Leaf(keys) => keys.iter().for_each(f),
            Node::Inner(inner) => inner.kids.iter().for_each(|&k| self.visit(k, f)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_at(
        &self,
        n: u32,
        depth: usize,
        is_root: bool,
        lo: Option<&E>,
        hi: Option<&E>,
        leaf_depth: &mut Option<usize>,
        prev: &mut Option<E>,
        reached: &mut usize,
    ) -> usize {
        *reached += 1;
        let in_bounds = |e: &E| lo.is_none_or(|lo| lo <= e) && hi.is_none_or(|hi| e < hi);
        match &self.nodes[n as usize] {
            Node::Leaf(keys) => {
                assert_eq!(
                    *leaf_depth.get_or_insert(depth),
                    depth,
                    "leaves at different depths"
                );
                assert!(
                    is_root || keys.len() >= L / 2,
                    "leaf under minimum fill: {}",
                    keys.len()
                );
                assert!(keys.len() <= L, "leaf over capacity");
                for k in keys {
                    assert!(in_bounds(k), "boundary outside its separators");
                    assert!(
                        prev.as_ref().is_none_or(|p| p < k),
                        "boundaries not strictly increasing"
                    );
                    *prev = Some(k.clone());
                }
                keys.len()
            }
            Node::Inner(inner) => {
                assert_eq!(inner.seps.len() + 1, inner.kids.len(), "separator count");
                assert_eq!(inner.counts.len(), inner.kids.len(), "count array length");
                assert!(inner.kids.len() <= F, "inner node over capacity");
                assert!(
                    if is_root {
                        inner.kids.len() >= 2
                    } else {
                        inner.kids.len() >= F / 2
                    },
                    "inner under minimum fill"
                );
                assert!(
                    inner.seps.windows(2).all(|w| w[0] < w[1]),
                    "separators not increasing"
                );
                assert!(
                    inner.seps.iter().all(in_bounds),
                    "separator outside parent bounds"
                );
                let mut total = 0;
                for (j, &k) in inner.kids.iter().enumerate() {
                    let klo = if j == 0 { lo } else { Some(&inner.seps[j - 1]) };
                    let khi = if j == inner.seps.len() {
                        hi
                    } else {
                        Some(&inner.seps[j])
                    };
                    let c = self.check_at(k, depth + 1, false, klo, khi, leaf_depth, prev, reached);
                    assert_eq!(c, inner.counts[j], "stale child count");
                    total += c;
                }
                total
            }
        }
    }
}

impl<W: Ord + Clone, const L: usize, const F: usize> BoundaryTree<(W, crate::kernel::Side), L, F> {
    /// Dense layout: number of boundaries at or below `Below(word)`.
    #[inline]
    pub fn rank_below(&self, word: &W) -> usize {
        self.rank_by(|(w, side)| w < word || (w == word && *side == crate::kernel::Side::Below))
    }
}

/// Splits `n` items into `ceil(n / cap)` groups whose sizes differ by at most one.
fn even_sizes(n: usize, cap: usize) -> impl Iterator<Item = usize> {
    let groups = n.div_ceil(cap).max(1);
    let (base, extra) = (n / groups, n % groups);
    (0..groups).map(move |g| base + usize::from(g < extra))
}

/// A set of boundaries plus the state below the first one: the mutable
/// counterpart of [`RawSet`](crate::kernel::RawSet).
#[derive(Clone, Debug)]
pub struct RawTreeSet<E, const L: usize = LEAF, const F: usize = FANOUT> {
    pub(crate) starts_inside: bool,
    pub(crate) tree: BoundaryTree<E, L, F>,
}

impl<E: Ord + Clone, const L: usize, const F: usize> RawTreeSet<E, L, F> {
    /// The empty set.
    pub fn empty() -> Self {
        RawTreeSet {
            starts_inside: false,
            tree: BoundaryTree::new(),
        }
    }
    /// The set of all elements.
    pub fn all() -> Self {
        RawTreeSet {
            starts_inside: true,
            tree: BoundaryTree::new(),
        }
    }
    /// Membership of a segment, from the number of boundaries below it.
    #[inline]
    pub fn inside(&self, segment: usize) -> bool {
        self.starts_inside ^ (segment & 1 == 1)
    }
    /// Toggles membership on one side of a position: `BelowAll` flips the whole
    /// set, `AboveAll` changes nothing.
    pub fn toggle(&mut self, at: crate::kernel::RawCut<E>) {
        match at {
            crate::kernel::RawCut::BelowAll => self.starts_inside = !self.starts_inside,
            crate::kernel::RawCut::At(e) => {
                self.tree.toggle(e);
            }
            crate::kernel::RawCut::AboveAll => {}
        }
    }
    /// `true` if the set has no members.
    pub fn is_empty(&self) -> bool {
        !self.starts_inside && self.tree.is_empty()
    }
    /// `true` if every element is a member.
    pub fn is_all(&self) -> bool {
        self.starts_inside && self.tree.is_empty()
    }
    /// The number of maximal disjoint intervals.
    pub fn interval_count(&self) -> usize {
        (self.tree.len() + usize::from(self.starts_inside)).div_ceil(2)
    }
    /// Builds from a frozen kernel in O(n).
    pub fn from_raw_set<B: crate::kernel::Boundaries<Owned = E>>(
        s: &crate::kernel::RawSet<B>,
    ) -> Self {
        let tree = BoundaryTree::from_sorted((0..s.bounds.len()).map(|i| B::own(s.bounds.get(i))));
        RawTreeSet {
            starts_inside: s.starts_inside,
            tree,
        }
    }
    /// Copies into a frozen kernel in O(n).
    pub fn to_raw_set<B: crate::kernel::Boundaries<Owned = E>>(&self) -> crate::kernel::RawSet<B> {
        let mut bounds = B::with_capacity(self.tree.len());
        self.tree.for_each(|e| bounds.push(e.clone()));
        crate::kernel::RawSet {
            starts_inside: self.starts_inside,
            bounds,
        }
    }
}

use core::fmt;
use core::hash::{Hash, Hasher};

use crate::read::{
    Cardinality, DequeRead, ElementType, Elements, Entries, IndexOf, IndexedEntries, Len,
    LookupRef, LookupValue, MapTypes, OrderedElements, OrderedEntries, PriorityRead, RangeElements,
    RangeEntries, SetContains, SetGet, TwoSlices, ValueElements, ValueRef,
};

pub(crate) mod sealed {
    /// Crate-private supertrait: only this crate decides eligibility.
    pub trait Sealed {}
}

/// Backends this crate has audited for use inside [`Frozen<B>`].
///
/// A backend is eligible when no operation reachable through `&B` changes its
/// structure (membership, order, length) and when its `Clone` does not share
/// structure with a mutable alias. `Eligible` is sealed: downstream crates
/// cannot add backends to `Frozen<B>`; they define their own frozen type and
/// implement the open [`read`](mod@crate::read) traits for it.
///
/// Eligibility gates *constructors* that do not start from an owned mutable
/// backend: `Default` and `FromIterator`. Every other construction path is a
/// curated [`FreezeWith`](crate::FreezeWith) implementation.
pub trait Eligible: sealed::Sealed {}

/// Exact-backend frozen wrapper: backend `B` behind a read-only API.
///
/// * **Semantics are the backend's own.** `Frozen<BTreeMap<K, V>>` iterates,
///   compares and deduplicates exactly as `BTreeMap` does, including
///   unspecified behaviour such as which duplicate key `BTreeMap::from_iter`
///   keeps. Family types (`family::FrozenOrderedMap` and friends, available
///   with `alloc`) are where the crate's own contracts apply.
/// * **No `Deref<Target = B>`, no `as_backend()`.** A shared reference to some
///   backends exposes mutation. Contiguous sequences instead deref to `[T]`.
/// * **Leaving is explicit.** [`into_inner`](Self::into_inner) consumes the
///   wrapper and returns the mutable backend.
/// * No `#[repr(transparent)]`: layout is not part of the API.
///
/// Compile-time guarantees (each block must fail to compile):
///
/// ```compile_fail,E0599
/// # extern crate frost_collections_core as frost;
/// // `Default`/`FromIterator` exist only for audited (`Eligible`) backends.
/// #[derive(Default)]
/// struct Mine;
/// let _ = frost::Frozen::<Mine>::default();
/// ```
#[cfg_attr(
    feature = "std",
    doc = r"
```compile_fail,E0614
# extern crate frost_collections_core as frost;
use frost::prelude::*;
use std::collections::HashMap;
let f = HashMap::from([(1, 2)]).seal();
let _: &HashMap<i32, i32> = &*f; // no Deref to the backend
```

```compile_fail,E0599
# extern crate frost_collections_core as frost;
use frost::prelude::*;
use std::collections::HashMap;
let mut f = HashMap::from([(1, 2)]).seal();
let _ = f.get_mut(&1); // no `&mut V`
```
"
)]
#[cfg_attr(
    feature = "alloc",
    doc = r"
```compile_fail,E0277
# extern crate frost_collections_core as frost;
use frost::write::Push;
fn push_any<C: Push<Item = i32>>(c: &mut C) { c.push(1) }
let mut f = frost::FreezeExt::seal(vec![1]);
push_any(&mut f); // mutation traits are not forwarded
```
"
)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frozen<B> {
    inner: B,
}

impl<B> Frozen<B> {
    /// Crate-internal constructor used by audited freeze plans only.
    #[inline]
    pub(crate) const fn new_unchecked(inner: B) -> Self {
        Self { inner }
    }

    /// Crate-internal shared access for backend modules.
    #[inline]
    pub(crate) const fn backend(&self) -> &B {
        &self.inner
    }

    /// Consume the frozen wrapper and return the backend, mutable again.
    /// No allocation or rebuilding. Only `B` itself moves; for an inline
    /// backend such as an array, that move includes the inline elements.
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> B {
        self.inner
    }

    /// Number of elements or entries.
    #[inline]
    pub fn len(&self) -> usize
    where
        B: Len,
    {
        self.inner.len()
    }

    /// `true` when empty.
    #[inline]
    pub fn is_empty(&self) -> bool
    where
        B: Len,
    {
        self.inner.is_empty()
    }

    // Lookup methods (`get`, `contains`, …) are inherent *per backend* in
    // `crate::backends`, not generic here: a generic `get<Q>` would shadow
    // `<[T]>::get(usize)` on slice-deref'ing sequences, because method
    // resolution does not fall back past an inherent method whose where-clause
    // fails.

    /// Heap priority element.
    #[inline]
    pub fn peek(&self) -> Option<&B::Item>
    where
        B: PriorityRead,
    {
        self.inner.peek()
    }
}

impl<B: fmt::Debug> fmt::Debug for Frozen<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Frozen").field(&self.inner).finish()
    }
}

// `Hash` is forwarded explicitly so it exists exactly when `B: Hash`.
impl<B: Hash> Hash for Frozen<B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<B: Eligible + Default> Default for Frozen<B> {
    fn default() -> Self {
        Self::new_unchecked(B::default())
    }
}

/// Builds the backend with **its own** `FromIterator` semantics.
impl<B, A> FromIterator<A> for Frozen<B>
where
    B: Eligible + FromIterator<A>,
{
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
        Self::new_unchecked(B::from_iter(iter))
    }
}

impl<B: IntoIterator> IntoIterator for Frozen<B> {
    type Item = B::Item;
    type IntoIter = B::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

#[allow(
    clippy::into_iter_without_iter,
    reason = "`iter()` is inherent per backend; a generic one would collide"
)]
impl<'a, B> IntoIterator for &'a Frozen<B>
where
    &'a B: IntoIterator,
{
    type Item = <&'a B as IntoIterator>::Item;
    type IntoIter = <&'a B as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        (&self.inner).into_iter()
    }
}

// ---------------------------------------------------------------------------
// Read capabilities forward unconditionally: a `Frozen<B>` only exists for
// audited `B`, and every read trait is read-only by construction. Mutation
// traits (`crate::write`) are never forwarded.
// ---------------------------------------------------------------------------

impl<B: Len> Len for Frozen<B> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<B: Cardinality> Cardinality for Frozen<B> {
    fn cardinality(&self) -> u64 {
        self.inner.cardinality()
    }
}

impl<B: ElementType> ElementType for Frozen<B> {
    type Item = B::Item;
}

impl<B: Elements> Elements for Frozen<B> {
    type Iter<'a>
        = B::Iter<'a>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.inner.elements()
    }
}

impl<B: ValueElements> ValueElements for Frozen<B> {
    type Values<'a>
        = B::Values<'a>
    where
        Self: 'a;

    fn values(&self) -> Self::Values<'_> {
        self.inner.values()
    }
}

impl<B: ValueRef<Q>, Q: ?Sized> ValueRef<Q> for Frozen<B> {
    type Value = B::Value;

    fn get_ref(&self, query: &Q) -> Option<&Self::Value> {
        self.inner.get_ref(query)
    }
}

impl<B: LookupValue<Q>, Q: ?Sized> LookupValue<Q> for Frozen<B> {
    type Value = B::Value;

    fn get_value(&self, query: &Q) -> Option<Self::Value> {
        self.inner.get_value(query)
    }
}

impl<B: MapTypes> MapTypes for Frozen<B> {
    type Key = B::Key;
    type Value = B::Value;
}

impl<B: LookupRef<Q>, Q: ?Sized> LookupRef<Q> for Frozen<B> {
    fn get_key_value(&self, query: &Q) -> Option<(&B::Key, &B::Value)> {
        self.inner.get_key_value(query)
    }
    fn get(&self, query: &Q) -> Option<&B::Value> {
        self.inner.get(query)
    }
}

impl<B: Entries> Entries for Frozen<B> {
    type Iter<'a>
        = B::Iter<'a>
    where
        Self: 'a;
    fn entries(&self) -> Self::Iter<'_> {
        self.inner.entries()
    }
}

impl<B: OrderedEntries> OrderedEntries for Frozen<B> {
    fn first_key_value(&self) -> Option<(&B::Key, &B::Value)> {
        self.inner.first_key_value()
    }
    fn last_key_value(&self) -> Option<(&B::Key, &B::Value)> {
        self.inner.last_key_value()
    }
}

impl<B: RangeEntries<Q>, Q: ?Sized> RangeEntries<Q> for Frozen<B> {
    type Range<'a>
        = B::Range<'a>
    where
        Self: 'a;
    fn range_entries<R: core::ops::RangeBounds<Q>>(&self, range: R) -> Self::Range<'_> {
        self.inner.range_entries(range)
    }
}

impl<B: SetContains<Q>, Q: ?Sized> SetContains<Q> for Frozen<B> {
    fn contains(&self, query: &Q) -> bool {
        self.inner.contains(query)
    }
}

impl<B: SetGet<Q>, Q: ?Sized> SetGet<Q> for Frozen<B> {
    fn get(&self, query: &Q) -> Option<&B::Item> {
        self.inner.get(query)
    }
}

impl<B: OrderedElements> OrderedElements for Frozen<B> {
    fn first_element(&self) -> Option<&B::Item> {
        self.inner.first_element()
    }
    fn last_element(&self) -> Option<&B::Item> {
        self.inner.last_element()
    }
}

impl<B: RangeElements<Q>, Q: ?Sized> RangeElements<Q> for Frozen<B> {
    type Range<'a>
        = B::Range<'a>
    where
        Self: 'a;
    fn range_elements<R: core::ops::RangeBounds<Q>>(&self, range: R) -> Self::Range<'_> {
        self.inner.range_elements(range)
    }
}

impl<B: DequeRead> DequeRead for Frozen<B> {
    fn front(&self) -> Option<&B::Item> {
        self.inner.front()
    }
    fn back(&self) -> Option<&B::Item> {
        self.inner.back()
    }
}

impl<B: TwoSlices> TwoSlices for Frozen<B> {
    fn as_slices(&self) -> (&[B::Item], &[B::Item]) {
        self.inner.as_slices()
    }
}

impl<B: PriorityRead> PriorityRead for Frozen<B> {
    fn peek(&self) -> Option<&B::Item> {
        self.inner.peek()
    }
}

impl<B: IndexedEntries> IndexedEntries for Frozen<B> {
    fn get_index(&self, index: usize) -> Option<(&B::Key, &B::Value)> {
        self.inner.get_index(index)
    }
}

impl<B: IndexOf<Q>, Q: ?Sized> IndexOf<Q> for Frozen<B> {
    fn get_index_of(&self, query: &Q) -> Option<usize> {
        self.inner.get_index_of(query)
    }
}

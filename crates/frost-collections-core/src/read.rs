//! Read capabilities shared by mutable backends, exact frozen wrappers and
//! family types.
//!
//! Each trait names one capability. Algorithms bound on the capability they
//! use, never on "a frozen collection" or on a universal collection trait:
//!
//! ```
//! # extern crate frost_collections_core as frost;
//! # #[cfg(feature = "std")]
//! # {
//! use frost::prelude::*;
//! use std::collections::{BTreeMap, HashMap};
//!
//! fn total<M>(m: &M, keys: &[&str]) -> u32
//! where
//!     M: LookupRef<str, Value = u32>,
//! {
//!     keys.iter().filter_map(|k| m.get(k)).sum()
//! }
//!
//! let h: HashMap<String, u32> = [("a".into(), 1), ("b".into(), 2)].into();
//! let b: BTreeMap<String, u32> = [("a".into(), 1), ("b".into(), 2)].into();
//! assert_eq!(total(&h, &["a", "b", "z"]), 3);          // mutable backend
//! assert_eq!(total(&b.compact(), &["a", "b", "z"]), 3); // frozen family
//! # }
//! ```
//!
//! Contiguous sequences use the standard `AsRef<[T]>` / `Deref<Target = [T]>`
//! rather than a crate-specific slice trait.
//!
//! # Dispatch
//! Traversal traits carry generic associated iterator types so callers can
//! name and bound them (`for<'a> C::Iter<'a>: ExactSizeIterator` is *not*
//! required; add it at the use site). GAT-bearing traits are not
//! dyn-compatible. Lookup traits ([`LookupRef`], [`SetContains`], [`SetGet`],
//! [`PriorityRead`], [`IndexedEntries`], [`IndexOf`]) have no GATs and no
//! generic methods, so they *are* dyn-compatible:
//!
//! ```
//! # extern crate frost_collections_core as frost;
//! # #[cfg(feature = "std")]
//! # {
//! use frost::prelude::*;
//! use std::collections::HashMap;
//! let m: FrozenHashMap<String, u32> = HashMap::from([("a".to_owned(), 1)]).compact();
//! let erased: &dyn LookupRef<str, Key = String, Value = u32> = &m;
//! assert_eq!(erased.get("a"), Some(&1));
//! # }
//! ```

use core::ops::RangeBounds;

/// Number of stored elements or entries, counted as `usize`.
///
/// Representations whose natural cardinality is not a `usize` (for example a
/// compressed `u64` integer set) deliberately do not implement this trait; they
/// expose their own cardinality capability instead.
pub trait Len {
    /// Number of elements or entries.
    fn len(&self) -> usize;
    /// `true` when [`len`](Self::len) is zero.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Cardinality for collections whose element count is naturally a `u64`.
///
/// This counts members, never the size of a bitset's addressable universe.
/// A compressed set may contain more members than fit in a target's `usize`;
/// implementations must not truncate its cardinality to implement [`Len`].
pub trait Cardinality {
    /// Number of members. Consult the backend for the cost of counting.
    fn cardinality(&self) -> u64;
}

/// Traversal that yields values, as used by compressed integer sets.
///
/// Unlike [`Elements`], this makes no claim that an element is stored at an
/// address that can be borrowed. Iterator capabilities and order remain those
/// of the backend.
pub trait ValueElements: ElementType {
    /// Iterator of reconstructed or copied values.
    type Values<'a>: Iterator<Item = Self::Item>
    where
        Self: 'a;
    /// Iterate the member values.
    fn values(&self) -> Self::Values<'_>;
}

/// Borrow a value when a backend cannot also lend a stored key representative.
///
/// Generational maps synthesize their keys from slots and generations. They
/// can implement this trait without pretending to satisfy [`LookupRef`].
pub trait ValueRef<Q: ?Sized> {
    /// Value stored under a key.
    type Value: ?Sized;
    /// Borrow the value associated with `query`.
    fn get_ref(&self, query: &Q) -> Option<&Self::Value>;
}

/// Lookup that returns a value, for example one decoded from compressed bytes.
///
/// This is deliberately independent of [`LookupRef`]: an FST's integer result
/// need not exist as an addressable integer in its serialized representation.
pub trait LookupValue<Q: ?Sized> {
    /// Returned value.
    type Value;
    /// Retrieve the decoded value associated with `query`.
    fn get_value(&self, query: &Q) -> Option<Self::Value>;
}

/// Borrowed traversal of stored elements in the collection's documented order.
///
/// No `DoubleEndedIterator`, `ExactSizeIterator` or `FusedIterator` promise is
/// made here; concrete iterator types may provide them.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not provide borrowed traversal of stored elements",
    note = "maps implement `Entries`; bit and integer sets yield values, not references"
)]
pub trait Elements: ElementType {
    /// Borrowing iterator.
    type Iter<'a>: Iterator<Item = &'a Self::Item>
    where
        Self: 'a;
    /// Iterate stored elements.
    fn elements(&self) -> Self::Iter<'_>;
}

/// Element type of a sequence, set or heap.
///
/// Kept separate from [`Elements`] so that element-typed capabilities without
/// GATs ([`SetGet`], [`DequeRead`], [`PriorityRead`], …) stay dyn-compatible.
pub trait ElementType {
    /// Stored element type.
    type Item;
}

/// Key and value types of a map-like collection.
pub trait MapTypes {
    /// Stored key type.
    type Key;
    /// Stored value type.
    type Value;
}

/// Lookup by a borrowed query `Q`, returning references to stored data.
///
/// The relation between `Q` and [`MapTypes::Key`] (`Borrow` + `Hash`/`Eq`,
/// `Borrow` + `Ord`, `Equivalent`, …) is a bound on each *implementation*, never
/// on the trait. Implementations must agree with the backend's own lookup.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be queried by `&{Q}`",
    note = "check the backend's key relation: `K: Borrow<{Q}>` with `Hash + Eq` (hash maps), `Ord` (ordered maps), or the backend's `Equivalent` trait"
)]
pub trait LookupRef<Q: ?Sized>: MapTypes {
    /// Stored key and value for `query`. The returned key is the *stored
    /// representative*, which may differ observably from `query`.
    fn get_key_value(&self, query: &Q) -> Option<(&Self::Key, &Self::Value)>;

    /// Stored value for `query`.
    fn get(&self, query: &Q) -> Option<&Self::Value> {
        self.get_key_value(query).map(|(_, v)| v)
    }

    /// `true` if an entry equivalent to `query` is stored.
    fn contains_key(&self, query: &Q) -> bool {
        self.get_key_value(query).is_some()
    }
}

/// Borrowed traversal of `(key, value)` entries in the documented order.
pub trait Entries: MapTypes {
    /// Borrowing entry iterator.
    type Iter<'a>: Iterator<Item = (&'a Self::Key, &'a Self::Value)>
    where
        Self: 'a;
    /// Iterate stored entries.
    fn entries(&self) -> Self::Iter<'_>;
}

/// Entries in ascending key order with access to both ends.
///
/// Implementors that also implement [`Entries`] promise ascending key order.
/// Not a subtrait of [`Entries`]: that keeps it dyn-compatible, and avoids a
/// `for<'a> Self::Iter<'a>: DoubleEndedIterator` bound (higher-ranked bounds
/// over `where Self: 'a` GATs currently force `Self: 'static`).
pub trait OrderedEntries: MapTypes {
    /// Entry with the least key. Named like `BTreeMap::first_key_value`
    /// (`&self`) so the trait never outranks std's `first_entry(&mut self)`.
    fn first_key_value(&self) -> Option<(&Self::Key, &Self::Value)>;
    /// Entry with the greatest key.
    fn last_key_value(&self) -> Option<(&Self::Key, &Self::Value)>;
}

/// Bounded traversal of an ordered map by borrowed bounds.
///
/// # Panics
/// Implementations panic on the same inverted or empty-excluded bounds that
/// `BTreeMap::range` rejects, so every ordered representation behaves alike.
pub trait RangeEntries<Q: ?Sized>: MapTypes {
    /// Iterator over the entries within a range.
    type Range<'a>: DoubleEndedIterator<Item = (&'a Self::Key, &'a Self::Value)>
    where
        Self: 'a;
    /// Entries whose keys fall in `range`, ascending.
    fn range_entries<R: RangeBounds<Q>>(&self, range: R) -> Self::Range<'_>;
}

/// Membership test that promises no reference to a stored element.
///
/// This is the capability a compressed or computed set can honestly offer.
pub trait SetContains<Q: ?Sized> {
    /// `true` if an element equivalent to `query` is a member.
    fn contains(&self, query: &Q) -> bool;
}

/// Retrieve the stored representative element for a query.
pub trait SetGet<Q: ?Sized>: SetContains<Q> + ElementType {
    /// Stored element equivalent to `query`.
    fn get(&self, query: &Q) -> Option<&Self::Item>;
}

/// Ordered sets: least and greatest element.
pub trait OrderedElements: ElementType {
    /// Least element.
    fn first_element(&self) -> Option<&Self::Item>;
    /// Greatest element.
    fn last_element(&self) -> Option<&Self::Item>;
}

/// Bounded traversal of an ordered set.
///
/// # Panics
/// Same bound validation as `BTreeSet::range`.
pub trait RangeElements<Q: ?Sized>: Elements {
    /// Iterator over elements within a range.
    type Range<'a>: DoubleEndedIterator<Item = &'a Self::Item>
    where
        Self: 'a;
    /// Elements in `range`, ascending.
    fn range_elements<R: RangeBounds<Q>>(&self, range: R) -> Self::Range<'_>;
}

/// Logical front/back of a double-ended sequence.
pub trait DequeRead: ElementType {
    /// First element in logical order.
    fn front(&self) -> Option<&Self::Item>;
    /// Last element in logical order.
    fn back(&self) -> Option<&Self::Item>;
}

/// Ring storage exposed as two borrowed slices in logical order.
///
/// Not implemented for contiguous types: obtaining one slice there is
/// `AsRef<[T]>`. Not emulated by copying.
pub trait TwoSlices: ElementType {
    /// `(head, tail)`; concatenated they are the logical sequence.
    fn as_slices(&self) -> (&[Self::Item], &[Self::Item]);
}

/// The priority element of a heap. Heap traversal order is storage order, not
/// priority order; this trait is the only priority-aware read.
pub trait PriorityRead: ElementType {
    /// Greatest element under the heap's ordering.
    fn peek(&self) -> Option<&Self::Item>;
}

/// Positional access to entries of an insertion-ordered map.
pub trait IndexedEntries: MapTypes {
    /// Entry at `index` in insertion (or explicitly reordered) order.
    fn get_index(&self, index: usize) -> Option<(&Self::Key, &Self::Value)>;
}

/// Position of a key in an insertion-ordered map.
pub trait IndexOf<Q: ?Sized>: IndexedEntries {
    /// Position of the entry equivalent to `query`.
    fn get_index_of(&self, query: &Q) -> Option<usize>;
}

// ---------------------------------------------------------------------------
// Slices and arrays participate in `core`-only builds.
// ---------------------------------------------------------------------------

impl<T> Len for [T] {
    fn len(&self) -> usize {
        <[T]>::len(self)
    }
}

impl<T> ElementType for [T] {
    type Item = T;
}

impl<T> Elements for [T] {
    type Iter<'a>
        = core::slice::Iter<'a, T>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

impl<T, const N: usize> Len for [T; N] {
    fn len(&self) -> usize {
        N
    }
}

impl<T, const N: usize> ElementType for [T; N] {
    type Item = T;
}

impl<T, const N: usize> Elements for [T; N] {
    type Iter<'a>
        = core::slice::Iter<'a, T>
    where
        Self: 'a;
    fn elements(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

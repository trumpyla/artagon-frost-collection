use alloc::sync::Arc;
use core::fmt;
use core::ops::Deref;

mod sealed {
    pub trait Sealed {}
}

/// A collection whose structure remains frozen while shared.
///
/// This sealed marker covers [`crate::Frozen`] with an eligible backend and
/// Frost's frozen collection families. It prevents publishing a live
/// concurrent collection or an ordinary mutable backend as a snapshot.
/// Freezing is structural: stored values may still use interior mutability.
/// Their contents do not gain a stronger immutability guarantee here.
///
/// Downstream crates cannot expand the supported snapshot payloads:
///
/// ```compile_fail
/// use frost::concurrent::SnapshotValue;
/// struct PretendFrozen;
/// impl SnapshotValue for PretendFrozen {}
/// ```
///
/// Mutable families and profiles must be frozen before publication:
///
/// ```compile_fail,E0277
/// use frost::concurrent::SnapshotStore;
/// use frost::family::MutableVec;
/// let store = SnapshotStore::new(MutableVec::<u8>::new());
/// ```
///
/// ```compile_fail,E0277
/// use frost::concurrent::SnapshotStore;
/// use frost::adaptive::InlineVec;
/// let store = SnapshotStore::new(InlineVec::<u8, 4>::new());
/// ```
///
/// ```compile_fail,E0277
/// use frost::concurrent::SnapshotStore;
/// use frost::adaptive::AdaptiveHashMap;
/// let store = SnapshotStore::new(AdaptiveHashMap::<u8, u8>::new());
/// ```
///
/// ```compile_fail,E0277
/// use frost::concurrent::SnapshotStore;
/// use frost::adaptive::AdaptiveOrderedMap;
/// let store = SnapshotStore::new(AdaptiveOrderedMap::<u8, u8>::new());
/// ```
#[cfg_attr(
    feature = "dashmap-6",
    doc = r"
Live concurrent maps are not frozen snapshots:

```compile_fail,E0277
use frost::backend::dashmap::DashMap;
use frost::concurrent::SnapshotStore;
let store = SnapshotStore::new(DashMap::<u8, u8>::new());
```
"
)]
pub trait SnapshotValue: sealed::Sealed {}

impl<B: crate::Eligible> sealed::Sealed for crate::Frozen<B> {}
impl<B: crate::Eligible> SnapshotValue for crate::Frozen<B> {}

impl<T> sealed::Sealed for crate::family::FrozenVec<T> {}
impl<T> SnapshotValue for crate::family::FrozenVec<T> {}
impl<K, V> sealed::Sealed for crate::family::FrozenOrderedMap<K, V> {}
impl<K, V> SnapshotValue for crate::family::FrozenOrderedMap<K, V> {}
impl<T> sealed::Sealed for crate::family::FrozenOrderedSet<T> {}
impl<T> SnapshotValue for crate::family::FrozenOrderedSet<T> {}
#[cfg(feature = "int-set")]
impl<K: crate::integer::IntKey> sealed::Sealed for crate::integer::FrozenIntSet<K> {}
#[cfg(feature = "int-set")]
impl<K: crate::integer::IntKey> SnapshotValue for crate::integer::FrozenIntSet<K> {}
impl<K, V, S> sealed::Sealed for crate::family::FrozenHashMap<K, V, S> {}
impl<K, V, S> SnapshotValue for crate::family::FrozenHashMap<K, V, S> {}
impl<T, S> sealed::Sealed for crate::family::FrozenHashSet<T, S> {}
impl<T, S> SnapshotValue for crate::family::FrozenHashSet<T, S> {}
impl<T, const N: usize> sealed::Sealed for crate::adaptive::InlineFrozenVec<T, N> {}
impl<T, const N: usize> SnapshotValue for crate::adaptive::InlineFrozenVec<T, N> {}
impl<K, V, S> sealed::Sealed for crate::adaptive::AdaptiveFrozenHashMap<K, V, S> {}
impl<K, V, S> SnapshotValue for crate::adaptive::AdaptiveFrozenHashMap<K, V, S> {}

/// A protected view of one complete frozen version.
///
/// Dereferencing the guard borrows its collection. Publications do not alter
/// that collection's structure, and the guard keeps it alive even after its
/// store is dropped. Stored values can still contain shared interior state.
///
/// Prefer a guard for short synchronous reads. Long-lived guards can occupy
/// the underlying publisher's limited fast-read slots; use
/// [`into_owned`](Self::into_owned) or [`SnapshotStore::load`] when retaining a
/// version in a data structure or across an asynchronous suspension point.
/// Guarded loads can fall back to reference-counted ownership, and publication
/// can also convert an existing guard to ownership. This API does not promise
/// that every guarded read avoids reference-count operations.
///
/// ```
/// use frost::concurrent::SnapshotStore;
/// use frost::family::FrozenVec;
/// let store = SnapshotStore::new(FrozenVec::from([1, 2]));
/// let guard = store.load_guard();
/// store.publish(FrozenVec::from([3, 4]));
/// assert_eq!(guard.as_slice(), &[1, 2]);
/// let retained = guard.into_owned();
/// drop(store);
/// assert_eq!(retained.as_slice(), &[1, 2]);
/// ```
#[must_use = "a snapshot guard keeps one version alive only while it is retained"]
pub struct SnapshotGuard<F: SnapshotValue> {
    inner: arc_swap::Guard<Arc<F>>,
}

impl<F: SnapshotValue> SnapshotGuard<F> {
    /// Retain this exact version in an owned `Arc` and release the guard.
    #[must_use]
    pub fn into_owned(self) -> Arc<F> {
        arc_swap::Guard::into_inner(self.inner)
    }
}

impl<F: SnapshotValue> Deref for SnapshotGuard<F> {
    type Target = F;

    fn deref(&self) -> &F {
        &self.inner
    }
}

impl<F: SnapshotValue> AsRef<F> for SnapshotGuard<F> {
    fn as_ref(&self) -> &F {
        self
    }
}

impl<F: SnapshotValue + fmt::Debug> fmt::Debug for SnapshotGuard<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SnapshotGuard")
            .field(&self.as_ref())
            .finish()
    }
}

/// Atomically publish complete frozen collections and retain stable versions.
///
/// Call [`load_guard`](Self::load_guard) or [`load`](Self::load) once and reuse
/// that version for queries that must see the same collection structure.
/// Later publications do not alter the loaded version or invalidate its
/// references. Choose [`peek`](Self::peek) for a short scoped read and `load`
/// for an owned `Arc` that can be retained without occupying a guard slot.
///
/// Multiple writers may publish concurrently. The current value follows the
/// atomic swaps' modification order; overlapping calls may return out of
/// order. [`publish`](Self::publish) unconditionally replaces the current
/// version. To derive a replacement from its predecessor without silently
/// overwriting an intervening publication, use
/// [`publish_if_current`](Self::publish_if_current) or [`update`](Self::update).
/// A successful conditional publication takes effect at its atomic pointer
/// comparison and replacement. Native `ArcSwap` operations remain available
/// via [`crate::backend::arc_swap`] for applications needing other policies.
///
/// Share the store itself with `Arc<SnapshotStore<_>>`. It is `Send` and
/// `Sync` exactly when its payload permits that, with no manual auto-trait
/// implementations.
///
/// ```
/// use frost::concurrent::SnapshotStore;
/// use frost::family::FrozenVec;
/// let store = SnapshotStore::new(FrozenVec::from([1, 2]));
/// let old = store.load();
/// let replaced = store.publish(FrozenVec::from([3, 4]));
/// assert_eq!(old.as_slice(), &[1, 2]);
/// assert!(std::sync::Arc::ptr_eq(&old, &replaced));
/// assert_eq!(store.load().as_slice(), &[3, 4]);
/// ```
///
/// Mutable collections are not accepted as snapshot payloads:
///
/// ```compile_fail
/// use frost::concurrent::SnapshotStore;
/// let store = SnapshotStore::new(vec![1, 2, 3]);
/// ```
pub struct SnapshotStore<F: SnapshotValue> {
    current: arc_swap::ArcSwap<F>,
}

impl<F: SnapshotValue> SnapshotStore<F> {
    /// Create a store containing its first frozen version.
    pub fn new(value: F) -> Self {
        Self {
            current: arc_swap::ArcSwap::from_pointee(value),
        }
    }

    /// Obtain an owned reference to one complete version.
    ///
    /// Use this when keeping a version for later, sending it to another task,
    /// or passing it to an API that needs an `Arc`. Short synchronous queries
    /// can use [`load_guard`](Self::load_guard) or [`peek`](Self::peek) to take
    /// advantage of the underlying publisher's guarded-read fast path.
    #[must_use]
    pub fn load(&self) -> Arc<F> {
        self.current.load_full()
    }

    /// Protect one complete version for short synchronous reads.
    ///
    /// The returned guard dereferences to `F` and keeps that exact version
    /// alive independently of later publications. Guarded reads often avoid
    /// acquiring an owned `Arc`, but fast-read slots are limited and a fallback
    /// or concurrent publication can require reference-count operations.
    /// See [`SnapshotGuard`] for retention guidance.
    pub fn load_guard(&self) -> SnapshotGuard<F> {
        SnapshotGuard {
            inner: self.current.load(),
        }
    }

    /// Inspect one complete version while holding a temporary guard.
    ///
    /// `read` runs exactly once. It may return owned data, but references
    /// borrowed from the version cannot escape. A panic releases the guard
    /// during unwinding. Keep the callback short to avoid retaining old
    /// versions and occupying a fast-read slot unnecessarily.
    ///
    /// ```
    /// use frost::concurrent::SnapshotStore;
    /// use frost::family::FrozenVec;
    /// let store = SnapshotStore::new(FrozenVec::from([1, 2, 3]));
    /// assert_eq!(store.peek(|version| version.as_slice().iter().sum::<i32>()), 6);
    /// ```
    ///
    /// ```compile_fail
    /// use frost::concurrent::SnapshotStore;
    /// use frost::family::FrozenVec;
    /// let store = SnapshotStore::new(FrozenVec::from([1, 2, 3]));
    /// let escaped: &i32 = store.peek(|version| &version.as_slice()[0]);
    /// println!("{escaped}");
    /// ```
    pub fn peek<R>(&self, read: impl FnOnce(&F) -> R) -> R {
        read(&self.load_guard())
    }

    /// Publish `value` atomically and return the previous owned version.
    ///
    /// Existing readers retain the previous version for as long as needed.
    /// The returned version is the one replaced by this particular swap,
    /// even if another writer publishes before this method returns.
    pub fn publish(&self, value: F) -> Arc<F> {
        self.current.swap(Arc::new(value))
    }

    /// Publish `value` only if `expected` is still the current version.
    ///
    /// The comparison uses `Arc` allocation identity, not collection contents.
    /// On success, return the exact version replaced by this operation.
    ///
    /// No cloning bound is required on `F`. Each publication uses a fresh
    /// outer `Arc`, and retaining `expected` keeps its allocation alive for
    /// the identity comparison.
    ///
    /// # Errors
    ///
    /// If `expected` does not match, return the original unconsumed `value`
    /// and an owned version observed during the comparison. That observed
    /// version may already have been replaced when this method returns.
    ///
    /// # Examples
    ///
    /// ```
    /// use frost::concurrent::SnapshotStore;
    /// use frost::family::FrozenVec;
    /// let store = SnapshotStore::new(FrozenVec::from([0]));
    /// let expected = store.load();
    /// let previous = store.publish_if_current(&expected, FrozenVec::from([1])).unwrap();
    /// assert!(std::sync::Arc::ptr_eq(&previous, &expected));
    /// let (rejected, observed) = store
    ///     .publish_if_current(&expected, FrozenVec::from([2]))
    ///     .unwrap_err();
    /// assert_eq!(rejected.as_slice(), &[2]);
    /// assert_eq!(observed.as_slice(), &[1]);
    /// ```
    pub fn publish_if_current(&self, expected: &Arc<F>, value: F) -> Result<Arc<F>, (F, Arc<F>)> {
        // Keep a private owner so rejection can recover F without cloning it.
        let next = Arc::new(value);
        let previous = self.current.compare_and_swap(expected, Arc::clone(&next));
        let previous = arc_swap::Guard::into_inner(previous);
        if Arc::ptr_eq(&previous, expected) {
            Ok(previous)
        } else {
            // ArcSwap drops its candidate on failure; it never publishes that
            // candidate. No reader can have acquired an owner, so this private
            // Arc is unique. This relies on ArcSwap's CAS ownership contract.
            let value = Arc::try_unwrap(next)
                .unwrap_or_else(|_| unreachable!("a rejected version has one owner"));
            Err((value, previous))
        }
    }

    /// Derive and conditionally publish a replacement, retrying on conflict.
    ///
    /// Return the exact version displaced by the successful publication,
    /// consistent with [`publish`](Self::publish). Each retry derives from
    /// the version observed by the previous failed comparison. The successful
    /// pointer comparison is this update's publication point: a replacement
    /// commits only while the version passed to that derivation is current.
    /// An unconditional `publish` can still replace any version by design.
    ///
    /// `derive` may run more than once. Keep it retry-safe: externally visible
    /// side effects can repeat. Prepare expensive inputs outside the callback
    /// when possible, and avoid publishing to this store from inside `derive`.
    /// A callback panic does not publish a version from that invocation; it
    /// does not roll back interior mutations or other writers' publications.
    ///
    /// Contention can cause arbitrarily many retries and starvation of an
    /// individual writer. Allocation, user callbacks, and destruction of
    /// rejected versions are part of this operation; this API promises no
    /// bounded completion time or unconditional lock-free progress.
    ///
    /// ```
    /// use frost::concurrent::SnapshotStore;
    /// use frost::family::FrozenVec;
    /// let store = SnapshotStore::new(FrozenVec::from([0_usize]));
    /// let previous = store.update(|version| FrozenVec::from([version.as_slice()[0] + 1]));
    /// assert_eq!(previous.as_slice(), &[0]);
    /// assert_eq!(store.peek(|version| version.as_slice()[0]), 1);
    /// ```
    pub fn update(&self, derive: impl FnMut(&F) -> F) -> Arc<F> {
        super::publication::update(self.load(), derive, |expected, next| {
            self.publish_if_current(expected, next)
        })
    }

    /// Consume the store and recover its current owned version.
    #[must_use]
    pub fn into_inner(self) -> Arc<F> {
        self.current.into_inner()
    }
}

impl<F: SnapshotValue + Default> Default for SnapshotStore<F> {
    fn default() -> Self {
        Self::new(F::default())
    }
}

impl<F: SnapshotValue + fmt::Debug> fmt::Debug for SnapshotStore<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnapshotStore")
            .field("current", &self.load_guard().as_ref())
            .finish()
    }
}

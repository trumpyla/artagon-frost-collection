//! Freeze plans: the *type* of the plan selects the output type and names the
//! cost contract.
//!
//! | Plan | Output | Cost contract |
//! |------|--------|---------------|
//! | [`Retain`] | `Frozen<B>` over the same representation | no allocation or rebuilding; moves the backend, including any inline storage |
//! | [`Compact`] | the family's compact representation | may allocate, reallocate or move elements; preserves the family contract |
//! | [`Sorted`] | sorted sequence from a heap | O(n log n), in place |
//! | [`ExactLen<N>`] | `Frozen<[T; N]>` | fallible, source returned on mismatch |
//! | `adaptive::Inline<N>` (`alloc`) | `InlineFrozenVec<T, N>` | moves ≤ N elements inline, else boxes |
//! | `adaptive::AdaptiveHash` (`std`) | `AdaptiveFrozenHashMap` | linear table below a threshold, else shrinks the hash table |
//! | `integer::IntSetPolicy` (`int-set`, fallible) | `FrozenIntSet<K>` | validate canonical keys, normalize once, select bounded storage; use `try_freeze_with` |
//!
//! A plan that is *not* implemented for a backend is a compile error, not a
//! silent fallback. `BinaryHeap` deliberately has no `Compact` plan because
//! "compact" is ambiguous between storage order and priority order:
//!
#![cfg_attr(
    feature = "alloc",
    doc = r"
```compile_fail,E0277
use frost::prelude::*;
let h = std::collections::BinaryHeap::from([3, 1, 2]);
let _ = h.compact();
```
"
)]

use core::fmt;

/// Seal the existing representation (no rebuild).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Retain;

/// Convert to the family's compact representation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Compact;

/// Heap → ascending sorted sequence (priority order made explicit).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sorted;

/// Freeze into a fixed-length array; fails unless `len == N`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ExactLen<const N: usize>;

/// Infallible consuming conversion selected by plan `P`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no `{P}` freeze plan",
    note = "plans are explicit per backend; see the table in `frost::plan`"
)]
pub trait FreezeWith<P>: Sized {
    /// The final public frozen type.
    type Output;
    /// Perform the conversion.
    fn freeze_with(self, plan: P) -> Self::Output;
}

/// Fallible consuming conversion selected by plan `P`.
///
/// Whether the source is recoverable is visible in `Error`: plans that can
/// validate before committing use [`Rejected<Self, E>`] and give the source
/// back intact. Panics and allocation aborts are never reported as `Err`.
#[diagnostic::on_unimplemented(message = "`{Self}` has no fallible `{P}` freeze plan")]
pub trait TryFreezeWith<P>: Sized {
    /// The final public frozen type.
    type Output;
    /// Failure type.
    type Error;
    /// Attempt the conversion.
    ///
    /// # Errors
    /// The plan's validation failed.
    fn try_freeze_with(self, plan: P) -> Result<Self::Output, Self::Error>;
}

/// A refused conversion that returns its unchanged input.
///
/// The field is `input`, not `source`, so it cannot be confused with
/// `Error::source()` (which returns `error`).
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Rejected<S, E> {
    /// The input, as it was before the attempt.
    pub input: S,
    /// Why the conversion was refused.
    pub error: E,
}

impl<S, E> Rejected<S, E> {
    /// Recover the input.
    #[must_use]
    pub fn into_input(self) -> S {
        self.input
    }
    /// Split into input and reason.
    #[must_use]
    pub fn into_parts(self) -> (S, E) {
        (self.input, self.error)
    }
}

impl<S, E: fmt::Debug> fmt::Debug for Rejected<S, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rejected")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<S, E: fmt::Display> fmt::Display for Rejected<S, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}

impl<S, E: core::error::Error + 'static> core::error::Error for Rejected<S, E> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Length did not match [`ExactLen<N>`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct LengthMismatch {
    /// Required length.
    pub expected: usize,
    /// Actual length.
    pub actual: usize,
}

impl fmt::Display for LengthMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "expected exactly {} elements, found {}",
            self.expected, self.actual
        )
    }
}

impl core::error::Error for LengthMismatch {}

/// Method-call sugar for the common plans. Blanket-implemented; each method is
/// available only when the matching [`FreezeWith`] impl exists.
///
/// **Stability promise:** this trait's method set is closed (`seal`,
/// `compact`). New plans are reached through `freeze_with`, never by adding
/// methods here, because a new blanket method could collide with a
/// downstream extension trait. Code that wants no blanket methods in scope
/// can import [`FreezeWith`] alone instead of the prelude.
pub trait FreezeExt: Sized {
    /// `freeze_with(Retain)`: same representation and a read-only API,
    /// without allocating or rebuilding. Moving an inline backend also moves
    /// its inline storage.
    #[inline]
    fn seal(self) -> <Self as FreezeWith<Retain>>::Output
    where
        Self: FreezeWith<Retain>,
    {
        self.freeze_with(Retain)
    }

    /// `freeze_with(Compact)`: compact family representation; may reallocate.
    #[inline]
    fn compact(self) -> <Self as FreezeWith<Compact>>::Output
    where
        Self: FreezeWith<Compact>,
    {
        self.freeze_with(Compact)
    }
}

impl<T> FreezeExt for T {}

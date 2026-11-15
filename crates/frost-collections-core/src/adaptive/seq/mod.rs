//! *Sequence* sub-category profiles: inline storage with heap fallback.
//!
//! One enum, two variants (`Inline`, `Heap`), one contract: logical element
//! order with `[T]` equality, ordering and hashing. Mutable [`InlineVec`] and
//! frozen [`InlineFrozenVec`] share the inline buffer, so moving between them
//! never allocates for the inline variant.

mod buf;
mod frozen;
mod mutable;

pub use frozen::{Inline, InlineFrozenVec};
pub use mutable::{InlineVec, IntoIter};

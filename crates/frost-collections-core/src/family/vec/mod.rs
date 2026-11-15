//! Contiguous-sequence families: [`MutableVec`] ⇄ [`FrozenVec`].

mod frozen;
mod mutable;

pub use frozen::FrozenVec;
pub use mutable::{Drain, IntoIter, MutableVec};

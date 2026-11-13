//! Finite formal models of the collection-algebra laws in `docs/design/algebra`.
//!
//! Each module owns one chapter. Laws are checked by exhausting a small stated
//! domain against reference definitions written here, not imported from Frost.
//! Where Frost implements the operation (frozen set algebra, frozen interior
//! mutability), the same inputs also run through the real types.
//!
//! Each law checker is a function of the operation it checks. Tests apply it to
//! the reference definition, which must pass, and to deliberately wrong
//! definitions, which must fail; that shows the check pins the definition.
//!
//! A passing model covers only its finite domain. It is not a proof and says
//! nothing about operations Frost does not implement yet.
#![cfg(feature = "std")]

mod bags;
mod bimaps;
mod common;
mod computed;
mod domains;
mod frozen;
mod maps;
mod relations;
mod sequences;
mod sets;

//! Framework adapters, one module per integration, each behind a feature
//! that carries the integration's semver-major (`serde-1`, `rayon-1`, …).
//!
//! Rules every adapter follows:
//! * **Logical encoding.** Wire formats see the logical collection (a
//!   sequence, or a map in the family's traversal order), never a
//!   representation tag or table layout. Where traversal order is defined
//!   (sequences, ordered families) the bytes are identical for every
//!   representation; for hash families serde order is unspecified, as for
//!   `HashMap`, while borsh family encodings use ascending member/key order.
//!   User-defined element encodings retain their own determinism contract.
//! * **Construction goes through the family contract.** Family decoding builds via
//!   the family builder (first key representative, last value) or a strict
//!   validating constructor — never via a backend bulk constructor whose
//!   duplicate policy differs.
//! * **Frozen eligibility holds.** `Frozen<B>` decodes only for audited
//!   (`Eligible`) backends and preserves those backends' decoding semantics.
//! * **No representation leaks.** Parallel iterators are crate-owned
//!   newtypes, like the sequential ones.
//!
//! | Feature | Module | Provides |
//! |---|---|---|
//! | `serde-1` | `serde` | `Serialize`/`Deserialize`, strict `deserialize_with` helpers |
//! | `rayon-1` | `rayon` | `into_par_iter`/`par_iter`/`par_iter_mut`, `FromParallelIterator`, `ParallelExtend` |
//! | `arbitrary-1` | `arbitrary` (impls only) | `Arbitrary` for fuzzing, exercising every representation |
//! | `borsh-1` | `borsh` (impls only) | canonical `BorshSerialize`/`BorshDeserialize`; non-canonical input rejected |
//! | `schemars-1` | `schemars` (impls only) | `JsonSchema` matching the serde encoding |

#[cfg(feature = "serde-1")]
pub mod serde;

#[cfg(feature = "rayon-1")]
pub mod rayon;

#[cfg(feature = "arbitrary-1")]
mod arbitrary;

#[cfg(feature = "borsh-1")]
mod borsh;

#[cfg(feature = "schemars-1")]
mod schemars;

//! Test and benchmark support shared by the Frost packages.
//!
//! [`workload`] generates deterministic set workloads as plain data, with
//! independent oracles. [`mod@bench`] is the dependency-free timing and listing
//! code every benchmark target uses. Nothing here depends on a Frost package,
//! and this package is only ever a dev-dependency.
#![forbid(unsafe_code)]

pub mod bench;
pub mod workload;

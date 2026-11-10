//! Shared integration-test support. Not a test target: Cargo discovers only
//! top-level `tests/*.rs` files and `tests/<dir>/main.rs`.

#![allow(dead_code)]

pub(crate) mod workload;

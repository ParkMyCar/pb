//! Rust bindings for the `pb` rules WASM sandbox.
//!
//! `pb` defines the interface for rules with WASM Interface Types in the
//! [`pb-wit`](https://github.com/ParkMyCar/pb-wit) repository. This crate
//! provides idomatic Rust wrappers around this interface.

pub mod futures;
pub mod logging;
pub mod resolver;
pub mod rules;

wit_bindgen::generate!({
    world: "rule-set",
    path: "pb-wit/wit"
});

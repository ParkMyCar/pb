//! Defines a logger that can be used for `pb` rules and target resolvers that are written in Rust.

use crate::wit::pb::rules as wit;
use crate::HostState;

impl wit::logging::Host for HostState {
    fn event(
        &mut self,
        level: wit::logging::Level,
        message: wasmtime::component::__internal::String,
        location: wit::logging::Location,
        fields: wasmtime::component::__internal::Vec<wit::logging::Field>,
    ) -> () {
        println!("{level:?} --> {message}");
    }
}

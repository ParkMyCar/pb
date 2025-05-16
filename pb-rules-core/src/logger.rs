//! Defines a logger that can be used for `pb` rules and target resolvers that are written in Rust.

use super::wit;

#[derive(Default)]
pub struct Logger;

impl Logger {
    pub fn add_to_linker<T, U>(
        linker: &mut wasmtime::component::Linker<T>,
        get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    ) -> wasmtime::Result<()>
    where
        U: wit::pb::rules::logging::Host,
    {
        wit::pb::rules::logging::add_to_linker(linker, get)
    }
}

impl wit::pb::rules::logging::Host for Logger {
    fn event(
        &mut self,
        level: wit::pb::rules::logging::Level,
        message: wasmtime::component::__internal::String,
        location: wit::pb::rules::logging::Location,
        fields: wasmtime::component::__internal::Vec<wit::pb::rules::logging::Field>,
    ) -> () {
        println!("{level:?} --> {message}");
    }
}

//! Defines a logger that can be used for `pb` rules and target resolvers that are written in Rust.

use wasmtime::component::bindgen;

bindgen!({ path: "pb-wit/wit" });

#[derive(Default)]
pub struct Logger;

impl Logger {
    pub fn add_to_linker<T, U>(
        linker: &mut wasmtime::component::Linker<T>,
        get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    ) -> wasmtime::Result<()>
    where
        U: pb::rules::logging::Host,
    {
        pb::rules::logging::add_to_linker(linker, get)
    }
}

impl pb::rules::logging::Host for Logger {
    fn event(
        &mut self,
        level: pb::rules::logging::Level,
        message: wasmtime::component::__internal::String,
        location: pb::rules::logging::Location,
        fields: wasmtime::component::__internal::Vec<pb::rules::logging::Field>,
    ) -> () {
        println!("{level:?} --> {message}");
    }
}

// wit_bindgen::generate!({
//     world: "logger",
//     path: "wit/core",
// });

// pub struct Logger;

// impl exports::pb::core::logging::Guest for Logger {
//     fn event(
//         level: exports::pb::core::logging::Level,
//         message: _rt::String,
//         _location: exports::pb::core::logging::Location,
//         _fields: _rt::Vec<exports::pb::core::logging::Field>,
//     ) -> () {
//         // TODO(parkmycar): Construct real tracing events.

//         use exports::pb::core::logging::Level as WitLevel;
//         match level {
//             WitLevel::Trace => tracing::trace!("{message}"),
//             WitLevel::Debug => tracing::debug!("{message}"),
//             WitLevel::Info => tracing::info!("{message}"),
//             WitLevel::Warn => tracing::warn!("{message}"),
//             WitLevel::Error => tracing::error!("{message}"),
//         }
//     }

//     fn add(a: u32, b: u32) -> u32 {
//         a + b
//     }
// }

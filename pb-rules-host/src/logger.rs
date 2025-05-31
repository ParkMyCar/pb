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
        let fields: Vec<_> = fields
            .into_iter()
            .map(|field| (field.name, field.value))
            .collect();

        match level {
            wit::logging::Level::Trace => tracing::trace!(target: "", ?fields, "{message}"),
            wit::logging::Level::Debug => tracing::debug!(target: "", ?fields, "{message}"),
            wit::logging::Level::Info => tracing::info!(target: "", ?fields, "{message}"),
            wit::logging::Level::Warn => tracing::warn!(target: "", ?fields, "{message}"),
            wit::logging::Level::Error => tracing::error!(target: "", ?fields, "{message}"),
        };
    }
}

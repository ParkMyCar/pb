//! Defines a logger that can be used for `pb` rules and target resolvers that are written in Rust.

wit_bindgen::generate!({
    world: "logger",
    path: "wit/core",
});

pub struct Logger;

impl exports::pb::core::logging::Guest for Logger {
    fn event(
        level: exports::pb::core::logging::Level,
        message: _rt::String,
        _location: exports::pb::core::logging::Location,
        _fields: _rt::Vec<exports::pb::core::logging::Field>,
    ) -> () {
        // TODO(parkmycar): Construct real tracing events.

        use exports::pb::core::logging::Level as WitLevel;
        match level {
            WitLevel::Trace => tracing::trace!("{message}"),
            WitLevel::Debug => tracing::debug!("{message}"),
            WitLevel::Info => tracing::info!("{message}"),
            WitLevel::Warn => tracing::warn!("{message}"),
            WitLevel::Error => tracing::error!("{message}"),
        }
    }
}

export!(Logger);

use std::collections::BTreeMap;

use tracing::Subscriber;
use tracing::field::Visit;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

/// [`tracing`] reports the log message a field instead of a separate field.
static MESSAGE_FIELD_NAME: &str = "message";

/// Executes the provided closure within the scope of a [`WasmLoggingLayer`].
pub fn with_logging<F, T>(closure: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = tracing_subscriber::registry()
        .with(WasmLoggingLayer)
        .set_default();
    (closure)()
}

/// A [`tracing_subscriber::Layer`] that reports to the WASM host's provided
/// [`event`] method.
///
/// [`event`]: super::pb::rules::logging::event
pub struct WasmLoggingLayer;

impl<S> Layer<S> for WasmLoggingLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let level = match *event.metadata().level() {
            tracing::Level::TRACE => crate::pb::rules::logging::Level::Trace,
            tracing::Level::DEBUG => crate::pb::rules::logging::Level::Debug,
            tracing::Level::INFO => crate::pb::rules::logging::Level::Info,
            tracing::Level::WARN => crate::pb::rules::logging::Level::Warn,
            tracing::Level::ERROR => crate::pb::rules::logging::Level::Error,
        };
        let location = crate::pb::rules::logging::Location {
            file_path: event.metadata().file().map(|p| p.to_string()),
            target: Some(event.metadata().target().to_string()),
            line: event.metadata().line(),
        };
        let mut fields = BTreeMap::default();
        let mut collector = FieldCollector::new(&mut fields);
        event.record(&mut collector);
        let message = collector.extract_message().unwrap_or_default();
        let fields: Vec<_> = fields
            .into_iter()
            .map(|(name, value)| crate::pb::rules::logging::Field { name, value })
            .collect();

        super::pb::rules::logging::event(level, &message, &location, &fields[..]);
    }
}

struct FieldCollector<'a> {
    fields: &'a mut BTreeMap<String, String>,
}

impl<'a> FieldCollector<'a> {
    fn new(fields: &'a mut BTreeMap<String, String>) -> Self {
        FieldCollector { fields }
    }

    /// Remove the field named [`MESSAGE_FIELD_NAME`] if it exists.
    fn extract_message(&mut self) -> Option<String> {
        self.fields.remove(MESSAGE_FIELD_NAME)
    }
}

impl<'a> Visit for FieldCollector<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let key = field.name().to_string();
        let value = format!("{:?}", value);
        self.fields.insert(key, value);
    }
}

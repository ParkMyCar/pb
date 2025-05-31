//! Defines a logger that can be used for `pb` rules and target resolvers that are written in Rust.

use crate::wit::pb::rules as wit;
use crate::HostState;

use std::borrow::Cow;
use std::fmt;

/// Format for logs emitted from WebAssembly.
#[derive(Debug)]
pub struct LoggingFormat {
    /// Whether or not ANSI color codes or other control sequences are supported.
    ///
    /// See: <https://no-color.org/>.
    ansi: bool,
}

impl LoggingFormat {
    pub fn from_env() -> Self {
        let ansi = !pb_ore::env::is_truthy("NO_COLOR");
        LoggingFormat { ansi }
    }
}

impl wit::logging::Host for HostState {
    fn event(
        &mut self,
        level: wit::logging::Level,
        message: wasmtime::component::__internal::String,
        location: wit::logging::Location,
        fields: wasmtime::component::__internal::Vec<wit::logging::Field>,
    ) -> () {
        // The `tracing` crate doesn't support manually constructing events, so we turn off as many
        // defaults as possible and use our own Display formatting to get decent looking logs.

        let fmted = LoggingMessage {
            location: &location,
            fields: &fields[..],
            message: &message,
            format: &self.logging_format,
        };

        match level {
            wit::logging::Level::Trace => tracing::trace!(name: "", target: "wasm", "{fmted}"),
            wit::logging::Level::Debug => tracing::debug!(name: "", target: "wasm", "{fmted}"),
            wit::logging::Level::Info => tracing::info!(name: "", target: "wasm", "{fmted}"),
            wit::logging::Level::Warn => tracing::warn!(name: "", target: "wasm", "{fmted}"),
            wit::logging::Level::Error => tracing::error!(name: "", target: "wasm", "{fmted}"),
        };
    }
}

#[derive(Debug)]
struct LoggingMessage<'a> {
    location: &'a wit::logging::Location,
    fields: &'a [wit::logging::Field],
    message: &'a str,

    /// Specifics of how our log messages should be formatted.
    format: &'a LoggingFormat,
}

impl<'a> fmt::Display for LoggingMessage<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(target) = self.location.target.as_ref() {
            if self.format.ansi {
                let formatted = ansi_term::Style::new()
                    .dimmed()
                    .paint(Cow::Borrowed(target.as_str()));
                write!(f, "{formatted}")?;
            } else {
                write!(f, "{target}")?;
            }
        }

        if let Some(file_path) = self.location.file_path.as_ref() {
            if self.location.target.is_some() {
                write!(f, "::")?;
            }

            if self.format.ansi {
                let formatted = ansi_term::Style::new()
                    .dimmed()
                    .paint(Cow::Borrowed(file_path.as_str()));
                write!(f, "{formatted}")?;
            } else {
                write!(f, "{file_path}")?;
            }

            if let Some(line_number) = self.location.line {
                if self.format.ansi {
                    let dimmed = ansi_term::Style::new().dimmed();
                    write!(f, "{}#L{}{}", dimmed.prefix(), line_number, dimmed.suffix())?;
                } else {
                    write!(f, "#L{line_number}")?;
                }
            }

            write!(f, " ")?;
        }

        write!(f, "{} ", self.message)?;

        for field in self.fields {
            if self.format.ansi {
                let name = ansi_term::Style::new()
                    .italic()
                    .paint(Cow::Borrowed(field.name.as_str()));
                write!(f, "{name}={} ", field.value)?;
            } else {
                write!(f, "{}={} ", field.name, field.value)?;
            }
        }

        Ok(())
    }
}

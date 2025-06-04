//! Ideas:
//!
//! 1. A "target discovery" phase that allows rules to read their own spec files and tell the build
//!    system what to do. For example, the Rust rules could read Cargo.toml files and return targets
//!    for those.
//!    1a. First-class support for listing targets. For example, I had no idea that zstd supported
//!        a separate multi-threaded build, it would be great to make that more discoverable.
//!
//! 2. First-class support for rules to add subcommands, e.g. `cargo install`-like
//!
//! 3. First-class support for documenting targets. e.g. a `--help` like output where a library
//!    author can document how one target differs from another.
//!
//! 4. Define a logging spec so process wrappers can communicate debugging into with the build
//!    system, most emit messages over a file descriptor.
//!

use defs::WORKSPACE_FILENAME;
use pb_cfg::ConfigSetBuilder;

pub mod cfgs;
pub mod defs;
pub mod engine;
pub mod rules;

pub use engine::{Engine, EngineConfig};

/// Register all of the [`Config`]s for this crate.
///
/// [`Config`]: pb_cfg::Config
pub fn register_configs(set: &mut ConfigSetBuilder) {
    set.register(&WORKSPACE_FILENAME);
}

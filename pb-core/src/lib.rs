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

use pb_filesystem::filesystem::Filesystem;
use pb_filesystem::path::PbPath;

pub struct Workspace {
    /// Root directory of the workspace, where the user's files live.
    root_dir: PbPath,
    /// Directory where we can stash metadata for this workspace.
    metadata_dir: PbPath,
    /// Our interface to the filesystem.
    filesystem: Filesystem,
}

impl Workspace {
    pub fn new(root: PbPath, metadata_root: PbPath) -> Self {
        // Store the metadata for this workspace at a deterministic location that is unlikely
        // to conflict with other workspaces.
        let metadata_filename = blake3::hash(root.inner.as_bytes());
        let metadata_dir = format!("{}/{metadata_filename}", metadata_root.inner);
        let metadata_dir = PbPath::new(metadata_dir).expect("known valid");
        let filesystem = Filesystem::new(4, 1024);

        Workspace {
            root_dir: root,
            metadata_dir,
            filesystem,
        }
    }
}

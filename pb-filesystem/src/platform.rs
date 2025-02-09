//! Abstract interface for a specific platform, e.g. `darwin`, `unix`, etc.

use crate::{Error, FileMetadata};

mod todo;

/// Platform specific filesystem operations.
pub trait Platform {
    fn stat(path: String) -> Result<FileMetadata, Error>;
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "macos")] {
        mod darwin;
        pub use darwin::DarwinPlatform as FilesystemPlatform;
    } else {
        pub use todo::TodoPlatform as FilesystemPlatform;
    }
}

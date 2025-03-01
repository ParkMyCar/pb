//! Abstract interface for a specific platform, e.g. `darwin`, `unix`, etc.

use bitflags::bitflags;
use std::fmt::Debug;

use crate::{DirectoryEntry, Error, FileStat};

mod todo;

bitflags! {
    #[derive(Debug)]
    pub struct OpenOptions: u32 {
        const READ_ONLY = 0b0000_0001;
        const READ_WRITE = 0b0000_0010;

        const APPEND = 0b0000_0100;
        const CREATE = 0b0000_1000;
        const EXCLUSIVE = 0b0001_0000;
        const TRUNCATE = 0b0010_0000;

        /// Restrict opening to just directories.
        const DIRECTORY = 0b0100_0000;
    }
}

impl Default for OpenOptions {
    fn default() -> Self {
        OpenOptions::READ_ONLY
    }
}

/// Platform specific filesystem operations.
pub trait Platform {
    type Path: PlatformPath;
    type Filename: PlatformFilename;

    type Handle: Debug + Clone;
    type DirStream: Debug + Clone;
    type FileStream: Debug + Clone;

    fn open(path: Self::Path, options: OpenOptions) -> Result<Self::Handle, Error>;
    fn openat(
        handle: Self::Handle,
        filename: Self::Filename,
        options: OpenOptions,
    ) -> Result<Self::Handle, Error>;
    fn close(handle: Self::Handle) -> Result<(), Error>;

    fn stat(path: Self::Path) -> Result<FileStat, Error>;
    fn fstat(handle: Self::Handle) -> Result<FileStat, Error>;

    fn fsync(handle: Self::Handle) -> Result<(), Error>;

    fn listdir(handle: Self::Handle) -> Result<Vec<DirectoryEntry>, Error>;

    fn open_filestream(handle: Self::Handle) -> Result<Self::FileStream, Error>;
    fn close_filestream(handle: Self::FileStream) -> Result<(), Error>;
    fn read(stream: &mut Self::FileStream, buf: &mut [u8]) -> Result<usize, Error>;

    fn file_handle_max() -> Result<usize, Error>;
}

pub trait PlatformPath: Debug + Clone {
    fn try_new(val: String) -> Result<Self, crate::Error>;
}

pub trait PlatformFilename: Debug + Clone {
    fn try_new(val: String) -> Result<Self, crate::Error>;
}

/// Type alias for the [`Platform::Handle`] associated type for the current [`FilesystemPlatform`].
pub type PlatformHandleType = <FilesystemPlatform as Platform>::Handle;
/// Type alias for the [`Platform::Path`] associated type for the current [`FilesystemPlatform`].
pub type PlatformPathType = <FilesystemPlatform as Platform>::Path;
/// Type alias for the [`Platform::Filename`] associated type for the current [`FilesystemPlatform`].
pub type PlatformFilenameType = <FilesystemPlatform as Platform>::Filename;
/// TODO
pub type PlatformFileStreamType = <FilesystemPlatform as Platform>::FileStream;

cfg_if::cfg_if! {
    if #[cfg(target_os = "macos")] {
        mod darwin;
        pub use darwin::DarwinPlatform as FilesystemPlatform;
    } else {
        pub use todo::TodoPlatform as FilesystemPlatform;
    }
}

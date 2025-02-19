//! Placeholder Platform that uses `todo!(...)` for all implementations.

use crate::platform::{OpenOptions, Platform, PlatformFilename, PlatformPath};
use crate::DirectoryEntry;

pub struct TodoPlatform;

impl Platform for TodoPlatform {
    type Path = String;
    type Filename = String;

    type Handle = u64;
    type DirStream = Self::Handle;

    fn open(_path: String, _options: OpenOptions) -> Result<Self::Handle, crate::Error> {
        todo!("open")
    }
    fn openat(
        _handle: Self::Handle,
        _filename: Self::Filename,
        _options: OpenOptions,
    ) -> Result<Self::Handle, crate::Error> {
        todo!("openat")
    }
    fn close(_handle: Self::Handle) -> Result<(), crate::Error> {
        todo!("close")
    }

    fn stat(_path: String) -> Result<crate::FileMetadata, crate::Error> {
        todo!("stat")
    }

    fn fstat(_handle: Self::Handle) -> Result<crate::FileMetadata, crate::Error> {
        todo!("fstat")
    }

    fn fsync(_handle: Self::Handle) -> Result<(), crate::Error> {
        todo!("fsync")
    }

    fn listdir(_handle: Self::Handle) -> Result<Vec<DirectoryEntry>, crate::Error> {
        todo!("listdir")
    }
}

impl PlatformPath for String {
    fn try_new(val: String) -> Result<Self, crate::Error> {
        Ok(val)
    }
}

impl PlatformFilename for String {
    fn try_new(val: String) -> Result<Self, crate::Error> {
        Ok(val)
    }
}

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

    fn mkdir(_path: Self::Path) -> Result<(), crate::Error> {
        todo!("mkdir")
    }
    fn mkdirat(_handle: Self::Handle, _filename: Self::Filename) -> Result<(), crate::Error> {
        todo!("mkdirat")
    }

    fn stat(_path: String) -> Result<crate::FileStat, crate::Error> {
        todo!("stat")
    }

    fn fstat(_handle: Self::Handle) -> Result<crate::FileStat, crate::Error> {
        todo!("fstat")
    }

    fn fsync(_handle: Self::Handle) -> Result<(), crate::Error> {
        todo!("fsync")
    }

    fn listdir(_handle: Self::Handle) -> Result<Vec<DirectoryEntry>, crate::Error> {
        todo!("listdir")
    }

    fn read(_stream: Self::Handle, _buf: &mut [u8], _offset: usize) -> Result<usize, crate::Error> {
        todo!("read")
    }

    fn write(_handle: Self::Handle, _data: &[u8], _offset: usize) -> Result<usize, crate::Error> {
        todo!("write")
    }

    fn rename(_from: Self::Path, _to: Self::Path) -> Result<(), crate::Error> {
        todo!("rename")
    }

    fn renameat(
        _from_handle: Self::Handle,
        _from_filename: Self::Filename,
        _to_handle: Self::Handle,
        _to_filename: Self::Filename,
    ) -> Result<(), crate::Error> {
        todo!("renameat")
    }

    fn fsetxattr(
        _handle: Self::Handle,
        _name: Self::Filename,
        _data: &[u8],
    ) -> Result<(), crate::Error> {
        todo!("fsetxattr")
    }
    fn fgetxattr(
        _handle: Self::Handle,
        _name: Self::Filename,
        _buf: &mut [u8],
    ) -> Result<usize, crate::Error> {
        todo!("fgetxattr")
    }

    fn fgetpath(_handle: Self::Handle) -> Result<Self::Path, crate::Error> {
        todo!("fgetpath")
    }

    fn file_handle_max() -> Result<usize, crate::Error> {
        todo!("file_handle_max")
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

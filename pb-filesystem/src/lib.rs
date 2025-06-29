#![allow(dead_code)]

pub mod filesystem;
pub mod handle;
pub mod locations;
pub mod platform;
pub mod tree;

#[cfg(test)]
mod tests;

use pb_types::Timespec;

/// Errors that can be returned from filesystem operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Operation is not permitted")]
    PermissionDenied,
    #[error("No such file or directory")]
    NotFound,
    #[error("No such process")]
    NoProcess,
    #[error("Invalid or unexpected data was returned: {0}")]
    InvalidData(Box<str>),
    #[error("Attempted to open a resource as a file, that wasn't a file")]
    NotAFile(Box<str>),
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Metadata about a file that is used to detect changes.
#[derive(Debug, Copy, Clone)]
pub struct FileStat {
    /// Size of a file in bytes.
    pub size: u64,
    /// Type of the file.
    pub kind: FileType,
    /// Inode number of the file.
    pub inode: u64,
    /// File mode/permissions.
    pub mode: u32,
    /// User ID of the file owner.
    pub user: u32,
    /// Group ID of the file owner.
    pub group: u32,
    /// File modified time.
    ///
    /// Generally changes when the file content changes.
    pub mtime: Timespec,
    /// Attribute change time.
    ///
    /// Changes whenever file ownership, size, or link count changes.
    pub ctime: Timespec,
    /// Optimal blocksize for I/O, if available.
    pub optimal_blocksize: Option<usize>,
}

/// Kind of object on the filesystem.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
    Symlink,
}

/// Information returned from an individual entry when listing a directory.
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    /// Inode number of the file.
    pub inode: u64,
    /// Name of the entry.
    pub name: String,
    /// Kind of entry.
    pub kind: FileType,
}

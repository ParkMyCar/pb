pub mod platform;
pub mod filesystem;

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
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Metadata about a file that is used to detect changes.
#[derive(Debug, Copy, Clone)]
pub struct FileMetadata {
    /// Size of a file in bytes.
    size: u64,
    /// Inode number of the file.
    inode: u64,
    /// File mode/permissions.
    mode: u32,
    /// User ID of the file owner.
    user: u32,
    /// Group ID of the file owner.
    group: u32,
    /// File modified time.
    ///
    /// Generally changes when the file content changes.
    mtime: Timespec,
    /// Attribute change time.
    ///
    /// Changes whenever file ownership, size, or link count changes.
    ctime: Timespec,
}

/// Time info returned from a `stat` call.
#[derive(Debug, Copy, Clone)]
pub struct Timespec {
    /// Seconds.
    secs: i64,
    /// Nanoseconds.
    ///
    /// Not all filesystems provide this, thus often it will be 0.
    nanos: i64,
}

//! Types used throughout `pb`.
//!
//! The goal of this crate is to be very lightweight, so take care with adding dependencies.

/// Metadata we track for a file to determine when it's changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    /// Size of the file in bytes.
    size: u64,
    /// Last modified time of the file.
    mtime: Timespec,
    /// Inode of the file.
    inode: u64,
    /// File mode/permissions.
    mode: u32,
    /// User ID of the file owner.
    user: u32,
    /// Group ID of the file owner.
    group: u32,
    /// Fingerprint of the file contents, generally a hash.
    fingerprint: [u8; 32],
}

/// Time info returned from a `stat` call.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Timespec {
    /// Seconds.
    pub secs: i64,
    /// Nanoseconds.
    ///
    /// Not all filesystems provide this, thus often it will be 0.
    pub nanos: i64,
}

//! Types used throughout `pb`.
//!
//! The goal of this crate is to be very lightweight, so take care with adding dependencies.

/// Metadata we track for a file to determine when it's changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata<T> {
    /// Size of the file in bytes.
    size: u64,
    /// Last modified time of the file.
    mtime: Timespec,
    /// Inode of the file.
    inode: u64,
    /// File mode/permissions.
    mode: u32,
    /// Fingerprint of the file contents, generally a hash.
    fingerprint: T,
}

pub type FileMetadataXx64 = FileMetadata<Xxh64Hash>;
pub type FileMetadataXx128 = FileMetadata<Xxh128Hash>;

/// Hash from xxh64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Xxh64Hash(u64);

impl Xxh64Hash {
    pub fn new(val: u64) -> Self {
        Xxh64Hash(val)
    }
}

/// Hash from xxh128.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Xxh128Hash(u128);

impl Xxh128Hash {
    pub fn new(val: u128) -> Self {
        Xxh128Hash(val)
    }
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

impl Timespec {
    /// Create a [`Timespec`] from the number of milliseconds since the epoch.
    pub fn from_epoch_millis(millis: u64) -> Self {
        let secs = millis / 1000;
        let nanos = (millis % 1000) * 10u64.pow(6);

        Timespec {
            secs: secs.try_into().expect("overlowed timespec"),
            nanos: nanos.try_into().expect("overlowed timespec"),
        }
    }
}

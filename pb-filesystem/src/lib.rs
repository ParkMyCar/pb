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

#[derive(Debug, Copy, Clone)]
pub struct Timespec {
    /// Seconds.
    secs: i64,
    /// Nanoseconds.
    ///
    /// Not all filesystems provide this, thus often it will be 0.
    nanos: i64,
}

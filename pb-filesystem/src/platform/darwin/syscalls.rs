//! Syscalls used for the Darwin platform.

use crate::platform::darwin::types::rlimit;

use super::types::{self, c_char, c_int, c_uint, dir_stream, dirent, file_descriptor};

unsafe extern "C" {
    /// Open the file at `path` with the provided flags.
    ///
    /// When creating a file we require an additional `mode` argument.
    pub unsafe fn open(path: *const c_char, flags: types::c_int, ...) -> c_int;
    /// Open the file at the path relative to the provided file descriptor.
    ///
    /// When creating a file we require an additional `mode` argument.
    pub unsafe fn openat(
        fildes: file_descriptor,
        path: *const c_char,
        flags: types::c_int,
        ...
    ) -> c_int;
    /// Close a file handle.
    pub unsafe fn close(fildes: file_descriptor) -> c_int;

    /// Make a directory at the specified path.
    pub unsafe fn mkdir(path: *const c_char, mode: u16) -> c_int;
    /// Make a directory at the specified path relative to the provided file descriptor.
    pub unsafe fn mkdirat(fildes: file_descriptor, path: *const c_char, mode: u16) -> c_int;

    /// Read `nbytes` from the provided file descriptor into `buf`.
    pub unsafe fn pread(fildes: file_descriptor, buf: *mut u8, nbytes: usize, offset: i64)
        -> isize;
    /// Write `nbytes` to the provided file descriptor.
    pub unsafe fn pwrite(
        fildes: file_descriptor,
        buf: *const u8,
        nbytes: usize,
        offset: i64,
    ) -> isize;

    /// Rename the link at `old` to `new`.
    pub unsafe fn rename(old: *const c_char, new: *const c_char) -> c_int;
    /// Rename the link at `old` relative to `oldfd`, to `new` relative to `newfd`.
    pub unsafe fn renameat(
        oldfd: file_descriptor,
        old: *const c_char,
        newfd: file_descriptor,
        new: *const c_char,
    ) -> c_int;

    pub unsafe fn renameatx_np(
        oldfd: file_descriptor,
        old: *const c_char,
        newfd: file_descriptor,
        new: *const c_char,
        flags: c_uint,
    ) -> c_int;

    /// Get an extended attribute value.
    pub unsafe fn fgetxattr(
        fildes: file_descriptor,
        name: *const c_char,
        value: *const u8,
        size: types::c_int,
        position: u32,
        options: types::c_int,
    ) -> isize;
    /// Set an extended attribute value for the provided file descriptor.
    pub unsafe fn fsetxattr(
        fildes: file_descriptor,
        name: *const c_char,
        value: *const u8,
        size: types::c_int,
        position: u32,
        options: types::c_int,
    ) -> i32;

    /// Returns statistics about the file at `path`.
    pub unsafe fn stat(path: *const c_char, buf: *mut types::stat) -> c_int;
    /// Returns statistics about the file open with the provided file descriptor.
    pub unsafe fn fstat(fildes: file_descriptor, buf: *mut types::stat) -> c_int;
    /// Returns statistics about the file at the path relative to the provided file descriptor.
    ///
    /// The value for `flag` can be bitwise OR of the following:
    /// 1. [`AT_SYMLINK_NOFOLLOW`]
    /// 2. [`AT_SYMLINK_NOFOLLOW_ANY`], if the path contains a symbolic link the status of the
    ///    link will be returned.
    ///
    /// [`AT_SYMLINK_NOFOLLOW`]: super::types::flags::AT_SYMLINK_NOFOLLOW
    /// [`AT_SYMLINK_NOFOLLOW_ANY`]: super::types::flags::AT_SYMLINK_NOFOLLOW_ANY
    pub unsafe fn fstatat(
        fildes: file_descriptor,
        path: *const c_char,
        buf: *mut types::stat,
        flag: c_int,
    ) -> c_int;

    /// Sync the buffered content of a file to disk.
    ///
    /// Note: This does not guarantee that the disk flushes the content to permanent
    /// storage, just that the data has been moved out of kernel buffers and onto a disk.
    /// Internally the disk may have it's own in-memory buffers. To guarantee a file is
    /// made durable see [`fcntl`].
    pub unsafe fn fsync(fildes: file_descriptor) -> c_int;
    /// File control.
    pub unsafe fn fcntl(fildes: file_descriptor, cmd: c_int, ...) -> c_int;
    /// Duplicate a file descriptor.
    pub unsafe fn dup(fildes: file_descriptor) -> file_descriptor;

    /// Open a directory stream for reading from a file descriptor.
    pub unsafe fn fdopendir(fildes: file_descriptor) -> dir_stream;
    /// Return the next entry in the directory.
    pub unsafe fn readdir(dirp: dir_stream) -> *const dirent;
    /// Close the directory stream and the associated file descriptor.
    pub unsafe fn closedir(dirp: dir_stream) -> c_int;

    /// Get resource limits for the current process.
    pub unsafe fn getrlimit(resource: c_int, limits: *mut rlimit) -> c_int;
}

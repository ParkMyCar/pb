//! Syscalls used for the Darwin platform.

use super::types::{self, c_char, c_int, file_descriptor};

unsafe extern "C" {
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
}

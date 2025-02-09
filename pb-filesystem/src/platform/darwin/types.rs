#![allow(non_camel_case_types)]

//! Types used by the Darwin platform.

pub(crate) type c_char = i8;
pub(crate) type c_int = i32;

pub(crate) type file_descriptor = c_int;

pub(crate) mod flags {
    use super::*;

    /// Act on the symlink itself, do not follow it.
    pub const AT_SYMLINK_NOFOLLOW: c_int = 0x0020;
    /// Act on the target of the symlink.
    pub const AT_SYMLINK_FOLLOW: c_int = 0x0040;
    /// Path should not contain any symlinks.
    pub const AT_SYMLINK_NOFOLLOW_ANY: c_int = 0x0800;
}

/// Data returned by calls to the `stat` family of functions.
///
/// Note: On versions of MacOS < 10.5 a 32-bit integer was used to represent
/// inode numbers, and thus this struct is incorrect.
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct stat {
    pub st_dev: i32,
    pub st_mode: u16,
    pub st_nlink: u16,
    pub st_ino: u64,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: i32,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
    pub st_birthtime: i64,
    pub st_birthtime_nsec: i64,
    pub st_size: i64,
    pub st_blocks: i64,
    pub st_blksize: i32,
    pub st_flags: u32,
    pub st_gen: u32,
    pub st_lspare: i32,
    pub st_qspare: [i64; 2],
}

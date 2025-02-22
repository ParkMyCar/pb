#![allow(non_camel_case_types)]

//! Types used by the Darwin platform.

pub(crate) type c_char = i8;
pub(crate) type c_int = i32;

#[derive(Debug, Copy, Clone)]
pub struct DarwinHandle {
    inner: file_descriptor,
}
pub(crate) type file_descriptor = c_int;

impl DarwinHandle {
    pub fn from_raw(val: file_descriptor) -> Self {
        DarwinHandle { inner: val }
    }

    pub fn into_raw(self) -> file_descriptor {
        self.inner
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DarwinFileStream {
    inner: file_descriptor,
}

impl DarwinFileStream {
    pub fn from_raw(val: file_descriptor) -> Self {
        DarwinFileStream { inner: val }
    }

    pub fn into_raw(self) -> file_descriptor {
        self.inner
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DarwinDirStream {
    pub(crate) inner: dir_stream,
}
pub(crate) type dir_stream = *const ();

pub(crate) mod flags {
    use super::*;

    /// Open for reading only.
    pub const O_RDONLY: c_int = 0x0000;
    /// Open for writing only.
    pub const O_WRONLY: c_int = 0x0001;
    /// Open for reading and writing.
    pub const O_RDWR: c_int = 0x0002;
    /// Mask for the above modes.
    pub const O_ACCMODE: c_int = 0x0003;

    /// Open the file for execute only.
    pub const O_EXEC: c_int = 0x40000000;
    /// Restrict opening to just directories.
    pub const O_DIRECTORY: c_int = 0x00100000;
    /// Open the directory for searching only.
    pub const O_SEARCH: c_int = O_EXEC | O_DIRECTORY;

    /// Act on the symlink itself, do not follow it.
    pub const AT_SYMLINK_NOFOLLOW: c_int = 0x0020;
    /// Act on the target of the symlink.
    pub const AT_SYMLINK_FOLLOW: c_int = 0x0040;
    /// Path should not contain any symlinks.
    pub const AT_SYMLINK_NOFOLLOW_ANY: c_int = 0x0800;

    /// Mask for `st_mode` that contains filetype information.
    pub const S_IFMT: u16 = 0xF000;

    /// Named pipe (FIFO).
    pub const S_IFIFO: u16 = 0x1000;
    /// Character special.
    pub const S_IFCHR: u16 = 0x2000;
    /// Directory.
    pub const S_IFDIR: u16 = 0x4000;
    /// Block special.
    pub const S_IFBLK: u16 = 0x6000;
    /// Regular file.
    pub const S_IFREG: u16 = 0x8000;
    /// Symbolic link.
    pub const S_IFLNK: u16 = 0xA000;
    /// Socket.
    pub const S_IFSOCK: u16 = 0xC000;

    /// Unknown filetype, from `readdir`.
    pub const DT_UNKNOWN: u8 = 0;
    /// Named pipe (FIFO), from `readdir`.
    pub const DT_FIFO: u8 = 1;
    /// Character special, from `readdir`.
    pub const DT_CHR: u8 = 2;
    /// Directory, from `readdir`.
    pub const DT_DIR: u8 = 4;
    /// Block special, from `readdir`.
    pub const DT_BLK: u8 = 6;
    /// Regular file, from `readdir`.
    pub const DT_REG: u8 = 8;
    /// Symbolic link, from `readdir`.
    pub const DT_LNK: u8 = 10;
    /// Socker, from `readdir`.
    pub const DT_SOCK: u8 = 12;

    // CPU time per process.
    pub const RLIMIT_CPU: c_int = 0;
    // File size.
    pub const RLIMIT_FSIZE: c_int = 1;
    // Data segment size.
    pub const RLIMIT_DATA: c_int = 2;
    // Stack size.
    pub const RLIMIT_STACK: c_int = 3;
    // Core file size.
    pub const RLIMIT_CORE: c_int = 4;
    // Address space (resident set size).
    pub const RLIMIT_AS: c_int = 5;
    // Locked-in-memory address space.
    pub const RLIMIT_MEMLOCK: c_int = 6;
    // Number of processes.
    pub const RLIMIT_NPROC: c_int = 7;
    // Number of open files.
    pub const RLIMIT_NOFILE: c_int = 8;
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

/// According to <dirent.h>.
const DARWIN_MAXPATHLEN: usize = 1024;

/// Directory entry returned from the `readdir` family of functions.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct dirent {
    pub d_ino: u64,
    pub d_seekoff: u64,
    pub d_reclen: u16,
    pub d_namlen: u16,
    pub d_type: u8,
    pub d_name: [u8; DARWIN_MAXPATHLEN],
}

impl Default for dirent {
    fn default() -> Self {
        dirent {
            d_ino: 0,
            d_seekoff: 0,
            d_reclen: 0,
            d_namlen: 0,
            d_type: 0,
            d_name: [0; DARWIN_MAXPATHLEN],
        }
    }
}

pub type rlim_t = u64;

/// Limits returned from `getrlimit`.
#[repr(C)]
#[derive(Default, Debug, Copy, Clone)]
pub struct rlimit {
    /// Current (soft) limit.
    pub(crate) rlim_cur: rlim_t,
    pub(crate) rlim_max: rlim_t,
}

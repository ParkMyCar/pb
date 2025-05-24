use pb_ore::cast::CastFrom;
use pb_types::Timespec;
use std::ffi::{c_uint, CStr, CString};

use crate::path::PbFilename;
use crate::platform::darwin::path::DarwinFilename;
use crate::platform::darwin::types::{rlimit, DarwinDirStream, DarwinHandle};
use crate::platform::{OpenOptions, Platform, PlatformPath};
use crate::{DirectoryEntry, FileStat, FileType};

mod path;
mod syscalls;
mod types;

#[cfg(test)]
mod tests;

pub use path::DarwinPath;

pub struct DarwinPlatform;

fn check_result(val: types::c_int) -> Result<types::c_int, crate::Error> {
    if val == -1 {
        // TODO: Maybe read errno directly.
        let err = std::io::Error::last_os_error().raw_os_error();
        Err(crate::Error::from_darwin_sys(err.unwrap_or(-1)))
    } else {
        Ok(val)
    }
}

impl Platform for DarwinPlatform {
    type Path = DarwinPath;
    type Filename = DarwinFilename;

    type Handle = DarwinHandle;
    type DirStream = DarwinDirStream;

    fn open(path: Self::Path, options: OpenOptions) -> Result<Self::Handle, crate::Error> {
        let path = CString::from(path);

        let mut flags = types::flags::O_RDONLY;

        // TODO(parkmycar): Handle the remaining flags here.
        if options.contains(OpenOptions::READ_WRITE) {
            flags |= types::flags::O_RDWR;
        } else if options.contains(OpenOptions::DIRECTORY) {
            flags |= types::flags::O_DIRECTORY;
        } else if options.contains(OpenOptions::CREATE) {
            flags |= types::flags::O_CREAT;
            flags |= types::flags::O_RDWR;
        } else if options.contains(OpenOptions::TRUNCATE) {
            flags |= types::flags::O_TRUNC;
            flags |= types::flags::O_RDWR;
        }

        // If we're creating a file make sure it's writeable.
        let mode = if (flags & types::flags::O_CREAT) > 0 {
            types::mode::DEFAULT_FILE_MODE as c_uint
        } else {
            0
        };

        let result = if mode != 0 {
            unsafe { syscalls::open(path.into_raw(), flags, mode) }
        } else {
            unsafe { syscalls::open(path.into_raw(), flags) }
        };
        let fd = check_result(result)?;
        let handle = DarwinHandle::from_raw(fd);

        Ok(handle)
    }

    fn openat(
        handle: Self::Handle,
        filename: Self::Filename,
        options: OpenOptions,
    ) -> Result<Self::Handle, crate::Error> {
        let filename = CString::from(filename);

        let mut flags = types::flags::O_RDONLY;

        // TODO(parkmycar): Handle the remaining flags here.
        if options.contains(OpenOptions::READ_WRITE) {
            flags |= types::flags::O_RDWR;
        } else if options.contains(OpenOptions::DIRECTORY) {
            flags |= types::flags::O_DIRECTORY;
        } else if options.contains(OpenOptions::CREATE) {
            flags |= types::flags::O_CREAT;
            flags |= types::flags::O_RDWR;
        } else if options.contains(OpenOptions::TRUNCATE) {
            flags |= types::flags::O_TRUNC;
            flags |= types::flags::O_RDWR;
        }

        // If we're creating a file make sure it's writeable.
        let mode = if (flags & types::flags::O_CREAT) > 0 {
            types::mode::DEFAULT_FILE_MODE as c_uint
        } else {
            0
        };

        let result = if mode != 0 {
            unsafe { syscalls::openat(handle.into_raw(), filename.into_raw(), flags, mode) }
        } else {
            unsafe { syscalls::openat(handle.into_raw(), filename.into_raw(), flags) }
        };
        let fd = check_result(result)?;
        let handle = DarwinHandle::from_raw(fd);

        Ok(handle)
    }

    fn close(handle: Self::Handle) -> Result<(), crate::Error> {
        let result = unsafe { syscalls::close(handle.into_raw()) };
        check_result(result)?;
        Ok(())
    }

    fn mkdir(path: Self::Path) -> Result<(), crate::Error> {
        let path = CString::from(path);
        let result = unsafe { syscalls::mkdir(path.into_raw(), types::mode::DEFAULT_DIR_MODE) };
        check_result(result)?;
        Ok(())
    }

    fn mkdirat(handle: Self::Handle, filename: Self::Filename) -> Result<(), crate::Error> {
        let filename = CString::from(filename);
        let result = unsafe {
            syscalls::mkdirat(
                handle.into_raw(),
                filename.into_raw(),
                types::mode::DEFAULT_DIR_MODE,
            )
        };
        check_result(result)?;
        Ok(())
    }

    fn stat(path: Self::Path) -> Result<FileStat, crate::Error> {
        let path = CString::from(path);
        let mut raw_stat = types::stat::default();

        let result = unsafe { syscalls::stat(path.into_raw(), &mut raw_stat as *mut _) };
        check_result(result)?;

        let metadata = FileStat::try_from(raw_stat)?;
        Ok(metadata)
    }

    fn fstat(handle: Self::Handle) -> Result<FileStat, crate::Error> {
        let mut raw_stat = types::stat::default();

        let result = unsafe { syscalls::fstat(handle.into_raw(), &mut raw_stat as *mut _) };
        check_result(result)?;

        let metadata = FileStat::try_from(raw_stat)?;
        Ok(metadata)
    }

    fn fsync(handle: Self::Handle) -> Result<(), crate::Error> {
        let result = unsafe { syscalls::fsync(handle.into_raw()) };
        check_result(result)?;
        Ok(())
    }

    fn listdir(handle: Self::Handle) -> Result<Vec<DirectoryEntry>, crate::Error> {
        // Duplicate the file handle because `fopendir` moves ownership of the
        // handle to the system.
        let result = unsafe { syscalls::dup(handle.into_raw()) };
        let dup_handle = check_result(result)?;

        // Create a directory stream.
        let dir_stream = unsafe { syscalls::fdopendir(dup_handle) };
        if dir_stream.is_null() {
            return Err(crate::Error::Unknown("failed to open directory".into()));
        }

        let mut entries = Vec::new();
        let mut dirent = unsafe { syscalls::readdir(dir_stream) };

        while !dirent.is_null() {
            let entry = DirectoryEntry::try_from(unsafe { *dirent })?;
            entries.push(entry);

            dirent = unsafe { syscalls::readdir(dir_stream) };
        }

        // Done listing! Close the directory stream.
        unsafe { syscalls::closedir(dir_stream) };

        Ok(entries)
    }

    fn read(handle: Self::Handle, buf: &mut [u8], offset: usize) -> Result<usize, crate::Error> {
        let buf_ptr = buf.as_mut_ptr();
        let buf_len = buf.len();
        let offset = offset.try_into().expect("TODO");

        let result = unsafe { syscalls::pread(handle.into_raw(), buf_ptr, buf_len, offset) };
        if result < 0 {
            Err(crate::Error::Unknown("TODO".to_string()))
        } else {
            let bytes_read = result.try_into().expect("checked that we're positive");
            Ok(bytes_read)
        }
    }

    fn write(handle: Self::Handle, data: &[u8], offset: usize) -> Result<usize, crate::Error> {
        let data_ptr = data.as_ptr();
        let data_len = data.len();
        let offset = offset.try_into().expect("TODO");

        let result = unsafe { syscalls::pwrite(handle.into_raw(), data_ptr, data_len, offset) };
        if result < 0 {
            Err(crate::Error::Unknown("TODO".to_string()))
        } else {
            let bytes_written = result.try_into().expect("checked that we're positive");
            Ok(bytes_written)
        }
    }

    fn rename(from: Self::Path, to: Self::Path) -> Result<(), crate::Error> {
        let from = CString::from(from);
        let to = CString::from(to);

        let result = unsafe { syscalls::rename(from.as_ptr(), to.as_ptr()) };
        check_result(result)?;
        Ok(())
    }

    fn renameat(
        from_handle: Self::Handle,
        from_filename: Self::Filename,
        to_handle: Self::Handle,
        to_filename: Self::Filename,
    ) -> Result<(), crate::Error> {
        let from = CString::from(from_filename);
        let to = CString::from(to_filename);

        let result = unsafe {
            syscalls::renameat(
                from_handle.into_raw(),
                from.as_ptr(),
                to_handle.into_raw(),
                to.as_ptr(),
            )
        };
        check_result(result)?;
        Ok(())
    }

    fn fsetxattr(
        handle: Self::Handle,
        name: Self::Filename,
        data: &[u8],
    ) -> Result<(), crate::Error> {
        /// The current man page for fsetxattr specifies that "only the resource fork extended
        /// attribute makes use of [the position] argument. For all others, position is reserved
        /// and should be set to zero."
        const POSITION: u32 = 0;

        let name = CString::from(name);
        let data_len: i32 = data
            .len()
            .try_into()
            .map_err(|err: std::num::TryFromIntError| crate::Error::Unknown(err.to_string()))?;
        let data_ptr = data.as_ptr();

        // TODO: expose these options.
        let options = 0;

        let result = unsafe {
            syscalls::fsetxattr(
                handle.into_raw(),
                name.into_raw(),
                data_ptr,
                data_len,
                POSITION,
                options,
            )
        };
        check_result(result)?;

        Ok(())
    }

    fn fgetxattr(
        handle: Self::Handle,
        name: Self::Filename,
        buf: &mut [u8],
    ) -> Result<usize, crate::Error> {
        /// The current man page for fgetxattr specifies that "only the resource fork extended
        /// attribute makes use of [the position] argument. For all others, position is reserved
        /// and should be set to zero."
        const POSITION: u32 = 0;

        let name = CString::from(name);

        // Note: If this buffer cannot fit the xattr then we get back error 34 "result too large".
        let buf_len: i32 = buf
            .len()
            .try_into()
            .map_err(|err: std::num::TryFromIntError| crate::Error::Unknown(err.to_string()))?;
        let buf_ptr = buf.as_ptr();

        // TODO: expose these options.
        let options = 0;

        let result = unsafe {
            syscalls::fgetxattr(
                handle.into_raw(),
                name.into_raw(),
                buf_ptr,
                buf_len,
                POSITION,
                options,
            )
        };
        let bytes_read = check_result(result.try_into().expect("TODO"))?;

        Ok(bytes_read.try_into().expect("known positive"))
    }

    fn fgetpath(handle: Self::Handle) -> Result<Self::Path, crate::Error> {
        let buffer = vec![0u8; types::constants::MAXPATHLEN * 4];
        let result =
            unsafe { syscalls::fcntl(handle.into_raw(), types::flags::F_GETPATH, buffer.as_ptr()) };
        check_result(result)?;

        let path = CStr::from_bytes_until_nul(&buffer[..])
            .expect("TODO")
            .to_string_lossy()
            .to_string();
        let path = <Self::Path as PlatformPath>::try_new(path).expect("TODO");

        Ok(path)
    }

    fn file_handle_max() -> Result<usize, crate::Error> {
        let mut limits = rlimit::default();
        let result =
            unsafe { syscalls::getrlimit(types::flags::RLIMIT_NPROC, &mut limits as *mut _) };
        check_result(result)?;

        Ok(usize::cast_from(limits.rlim_cur))
    }
}

impl TryFrom<types::stat> for FileStat {
    type Error = crate::Error;

    fn try_from(stat: types::stat) -> Result<Self, Self::Error> {
        let size = u64::try_from(stat.st_size).map_err(|_| {
            let msg = format!("negative file size: {}", stat.st_size).into();
            crate::Error::InvalidData(msg)
        })?;

        let mtime = Timespec {
            secs: stat.st_mtime,
            nanos: stat.st_mtime_nsec,
        };
        let ctime = Timespec {
            secs: stat.st_ctime,
            nanos: stat.st_ctime_nsec,
        };

        let masked_kind = stat.st_mode & types::flags::S_IFMT;
        let kind = if masked_kind == types::flags::S_IFLNK {
            FileType::Symlink
        } else if masked_kind == types::flags::S_IFDIR {
            FileType::Directory
        } else if masked_kind == types::flags::S_IFREG {
            FileType::File
        } else {
            tracing::warn!(?masked_kind, "falling back to file");
            FileType::File
        };

        let optimal_blocksize = match stat.st_blksize {
            ..0 => None,
            x => {
                let optimal: usize = x.try_into().expect("checked above that we're non-negative");
                Some(optimal)
            }
        };

        let metadata = FileStat {
            size,
            kind,
            inode: stat.st_ino,
            mode: u32::cast_from(stat.st_mode),
            user: stat.st_uid,
            group: stat.st_gid,
            mtime,
            ctime,
            optimal_blocksize,
        };
        Ok(metadata)
    }
}

impl TryFrom<types::dirent> for DirectoryEntry {
    type Error = crate::Error;

    fn try_from(dirent: types::dirent) -> Result<Self, Self::Error> {
        let filename_len = dirent.d_namlen;
        let filename_buf = &dirent.d_name;

        assert!(filename_len <= 1024);

        let filename_len = usize::cast_from(filename_len);
        let filename = std::str::from_utf8(&filename_buf[..filename_len])
            .expect("invalid UTF-8 found with filename");
        let filename = PbFilename::new(filename.to_string())?;

        let kind = match dirent.d_type {
            types::flags::DT_DIR => FileType::Directory,
            types::flags::DT_LNK => FileType::Symlink,
            types::flags::DT_REG => FileType::File,
            kind => {
                tracing::warn!(kind, "falling back to file");
                FileType::File
            }
        };

        Ok(DirectoryEntry {
            inode: dirent.d_ino,
            name: filename,
            kind,
        })
    }
}

impl crate::Error {
    /// Create an [`Error`] from the value returned by a system call.
    ///
    /// Derived from `sys/errno.h` on MacOS.
    pub fn from_darwin_sys(val: types::c_int) -> Self {
        match val {
            1 => crate::Error::PermissionDenied,
            2 => crate::Error::NotFound,
            3 => crate::Error::NoProcess,
            x => crate::Error::Unknown(x.to_string()),
        }
    }
}

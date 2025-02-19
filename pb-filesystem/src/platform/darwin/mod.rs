use pb_ore::cast::CastFrom;
use std::ffi::CString;

use crate::path::PbFilename;
use crate::platform::darwin::path::DarwinFilename;
use crate::platform::darwin::types::{DarwinDirStream, DarwinHandle};
use crate::platform::{OpenOptions, Platform};
use crate::{DirectoryEntry, FileMetadata, FileType, Timespec};

mod path;
mod syscalls;
mod types;

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
        }

        let result = unsafe { syscalls::open(path.into_raw(), flags, 0) };
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
        }

        let result = unsafe { syscalls::openat(handle.into_raw(), filename.into_raw(), flags, 0) };
        let fd = check_result(result)?;
        let handle = DarwinHandle::from_raw(fd);

        Ok(handle)
    }

    fn close(handle: Self::Handle) -> Result<(), crate::Error> {
        let result = unsafe { syscalls::close(handle.into_raw()) };
        check_result(result)?;
        Ok(())
    }

    fn stat(path: Self::Path) -> Result<FileMetadata, crate::Error> {
        let path = CString::from(path);
        let mut raw_stat = types::stat::default();

        let result = unsafe { syscalls::stat(path.into_raw(), &mut raw_stat as *mut _) };
        check_result(result)?;

        let metadata = FileMetadata::try_from(raw_stat)?;
        Ok(metadata)
    }

    fn fstat(handle: Self::Handle) -> Result<FileMetadata, crate::Error> {
        let mut raw_stat = types::stat::default();

        let result = unsafe { syscalls::fstat(handle.into_raw(), &mut raw_stat as *mut _) };
        check_result(result)?;

        let metadata = FileMetadata::try_from(raw_stat)?;
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
}

impl TryFrom<types::stat> for FileMetadata {
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

        let metadata = FileMetadata {
            size,
            kind,
            inode: stat.st_ino,
            mode: u32::cast_from(stat.st_mode),
            user: stat.st_uid,
            group: stat.st_gid,
            mtime,
            ctime,
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

use pb_ore::cast::CastFrom;
use std::ffi::CString;

use crate::platform::Platform;
use crate::{FileMetadata, Timespec};

mod syscalls;
mod types;

pub struct DarwinPlatform;

fn check_result(val: types::c_int) -> Result<(), crate::Error> {
    if val == 0 {
        Ok(())
    } else {
        Err(crate::Error::from_darwin_sys(val))
    }
}

impl Platform for DarwinPlatform {
    fn stat(path: String) -> Result<FileMetadata, crate::Error> {
        let path = CString::new(path).expect("invalid C String");
        let mut raw_stat = types::stat::default();

        let result = unsafe { syscalls::stat(path.into_raw(), &mut raw_stat as *mut _) };
        check_result(result)?;

        let metadata = FileMetadata::try_from(raw_stat)?;
        Ok(metadata)
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

        let metadata = FileMetadata {
            size,
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

//! Darwin specific paths.

use std::{ffi::CString, path::PathBuf};

use crate::platform::{PlatformFilename, PlatformPath};

/// Paths for common Darwin filesystems, i.e. HFS+ and APFS.
///
/// ### HFS+
/// * Defaults to case insensitive, but preserving.
/// * Normalizes all file paths to Unicode NFD.
/// * Based on Unicode 3.2
/// * Internally stored as UTF-16.
///
/// ### APFS
/// * Defaults to case insensitive, but preserving.
/// * Normalization insensitive. Internally APFS normalizes filenames to NFD, then
///   converts UTF-8 to UTF-32, and finally hashes this representation.
/// * Based on Unicode 9.0
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DarwinPath {
    inner: String,
}

impl DarwinPath {
    pub(crate) fn into_inner(self) -> String {
        self.inner
    }
}

impl PlatformPath for DarwinPath {
    fn try_new(val: PathBuf) -> Result<Self, crate::Error> {
        // TODO: Don't go through String here.
        let inner = val.to_str().expect("non UTF-8 path").to_string();
        Ok(DarwinPath { inner })
    }
}

impl From<DarwinPath> for CString {
    fn from(path: DarwinPath) -> Self {
        CString::new(path.inner).expect("UTF-8 is always valid")
    }
}

/// Individual component of a [`DarwinPath`].
///
/// See documentation on [`DarwinPath`] for the specifics.
#[derive(Debug, Clone)]
pub struct DarwinFilename {
    inner: String,
}

impl PlatformFilename for DarwinFilename {
    fn try_new(val: String) -> Result<Self, crate::Error> {
        Ok(DarwinFilename { inner: val })
    }
}

impl From<DarwinFilename> for CString {
    fn from(filename: DarwinFilename) -> Self {
        CString::new(filename.inner).expect("UTF-8 is always valid")
    }
}

//! Darwin specific paths.

use std::ffi::CString;

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
#[derive(Debug, Clone)]
pub struct DarwinPath {
    inner: String,
}

impl PlatformPath for DarwinPath {
    fn try_new(val: String) -> Result<Self, crate::Error> {
        Ok(DarwinPath { inner: val })
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

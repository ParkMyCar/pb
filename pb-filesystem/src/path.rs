//! PB filename and path structures.

/// Filesystem path used interally throughout PB.
///
/// ### Specification
/// * NFC encoded
///
/// ### Why not use [`PathBuf`]?
/// **tl;dr** unicode normalization
///
/// Certain characters have multiple valid encodings, e.g. the character "A with a ring
/// above" can be encoded as a single unicode character `U+00C5` ('Å', NFC) or two
/// characters `U+0041` + `U+030A` ('A' + '◌̊', NFD). Different filesystems use different
/// normalization formats, so we unify them at the application level with [`PbPath`] to
/// ensure we're consistent across platforms with how we refer to files.
///
/// [`PathBuf`]: std::path::PathBuf
#[derive(Debug, Clone)]
pub struct PbPath {
    pub inner: String,
}

impl PbPath {
    pub fn new(val: String) -> Result<Self, crate::Error> {
        Ok(PbPath { inner: val })
    }
}

/// Filename component of a [`PbPath`].
///
/// See the docs comment on [`PbPath`] for specifics of this type.
#[derive(Debug, Clone)]
pub struct PbFilename {
    pub inner: String,
}

impl PbFilename {
    pub fn new(val: String) -> Result<Self, crate::Error> {
        Ok(PbFilename { inner: val })
    }
}

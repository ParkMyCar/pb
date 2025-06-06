use std::{path::PathBuf, sync::Arc};

use crate::{filesystem::Filesystem, handle::DirectoryHandle};

static DELETE_DIRECTORY_NAME: &str = "trash";

/// A "trash" directory that we can move files into such that they get
/// asynchronously deleted.
pub struct TrashDirectory {
    /// Root of the trash directory.
    root_path: PathBuf,
    /// Handle to the root of the trash directory.
    root_handle: Arc<DirectoryHandle>,
    /// Handle to our filesystem abstraction.
    filesystem: Filesystem,
}

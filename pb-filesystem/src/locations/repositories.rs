use std::sync::Arc;

use crate::filesystem::Filesystem;
use crate::handle::DirectoryHandle;
use crate::path::PbPath;

static REPOSITORY_DIRECTORY_NAME: &str = "repositories";

/// The "repositories" directory is where external resources get placed after
/// downloading.
///
/// For example, most languages have a package registry that hosts libraries. A
/// build rule may require downloading an external library (e.g. a crates from
/// crates.io) which we will locally store in the [`RepositoryDirectory`].
///
/// See the [`ScratchDirectory`] API for creating files.
///
/// [`ScratchDirectory`]: crate::locations::scratch::ScratchDirectory
#[derive(Clone)]
pub struct RepositoryDirectory {
    /// Handle to the repositories directory.
    root_handle: Arc<DirectoryHandle>,
    /// Handle to our filesystem abstraction.
    filesystem: Filesystem,
}

impl RepositoryDirectory {
    /// Create a new [`RepositoryDirectory`] as `root_path /`[`REPOSITORY_DIRECTORY_NAME`].
    pub async fn new(root: PbPath, filesystem: Filesystem) -> Result<Self, crate::Error> {
        tracing::info!(?root, "starting Repository Directory");

        let root = filesystem.open(root.inner).as_directory().await?;
        // Create the repository directory if it doesn't exist.
        //
        // TODO: Scan/index for existing repositories on startup.
        let root_handle = root
            .openat(REPOSITORY_DIRECTORY_NAME.to_string())
            .as_directory()
            .await?;

        Ok(RepositoryDirectory {
            root_handle: Arc::new(root_handle),
            filesystem,
        })
    }

    /// Handle to the root of the directory.
    pub fn root_directory(&self) -> Arc<DirectoryHandle> {
        Arc::clone(&self.root_handle)
    }
}

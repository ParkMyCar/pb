use pb_filesystem::filesystem::Filesystem;
use pb_filesystem::path::PbPath;

pub struct Workspace {
    /// Root directory of the workspace, where the user's files live.
    root_dir: PbPath,
    /// Directory where we can stash metadata for this workspace.
    metadata_dir: PbPath,
    /// Our interface to the filesystem.
    filesystem: Filesystem,
}

impl Workspace {
    pub fn new(root: PbPath, metadata_root: PbPath) -> Self {
        // Store the metadata for this workspace at a deterministic location that is unlikely
        // to conflict with other workspaces.
        let metadata_filename = blake3::hash(root.inner.as_bytes());
        let metadata_dir = format!("{}/{metadata_filename}", metadata_root.inner);
        let metadata_dir = PbPath::new(metadata_dir).expect("known valid");

        let filesystem = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            Filesystem::new_tokio(handle, 1024)
        } else {
            todo!("support a non-tokio Filesystem")
        };

        Workspace {
            root_dir: root,
            metadata_dir,
            filesystem,
        }
    }
}

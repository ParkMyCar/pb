use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use derivative::Derivative;

use crate::filesystem::Filesystem;
use crate::handle::{DirectoryHandle, DirectoryKind, FileKind};
use crate::path::PbPath;
use crate::platform::{FilesystemPlatform, Platform, PlatformFilename};

static SCRATCH_DIRECTORY_NAME: &str = ".pb_scratch";

/// Name for the extended attribute to describe the rule set that created this scratch file.
static SCRATCH_XATTR_TAG_RULESET_NAME: &str = "org.pb.scratch.rule_set";
/// Name for the extended attribute that includes a general comment about this scratch file.
static SCRATCH_XATTR_TAG_COMMENT_NAME: &str = "org.pb.scratch.comment";

/// A "scratch" directory that can be used to store transient files.
///
/// A common use-case for a [`ScratchDirectory`] is to download a file into the
/// scratch space and once it's complete, move it to the final location. This
/// way if the download only partially completes we're not left with a
/// corrupted file.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct ScratchDirectory {
    /// Root of the scratch directory.
    root_path: PbPath,
    /// Handle to the root of the scratch directory.
    #[derivative(Debug = "ignore")]
    root_handle: Arc<DirectoryHandle>,
    /// Handle to our filesystem abstraction.
    #[derivative(Debug = "ignore")]
    filesystem: Filesystem,
}

impl ScratchDirectory {
    /// Create a new [`ScratchDirectory`] at `root_path /`[`SCRATCH_DIRECTORY_NAME`].
    pub async fn new(root: PbPath, filesystem: Filesystem) -> Result<Self, crate::Error> {
        let root_path = format!("{}/{SCRATCH_DIRECTORY_NAME}", root.inner);
        tracing::info!(?root_path, "starting Scratch Directory");

        // TODO: Implement automatic cleanup.
        let root_handle = filesystem.open(root_path.clone()).as_directory().await?;
        let root_path = PbPath::new(root_path).expect("known good");

        Ok(ScratchDirectory {
            root_path,
            root_handle: Arc::new(root_handle),
            filesystem,
        })
    }

    /// Create a new file in the scratch space with a random name.
    pub async fn file(&self) -> Result<ScratchFileHandle, crate::Error> {
        let filename = uuid::Uuid::new_v4().to_string();
        tracing::debug!(?filename, "creating new scratch file");

        let (inner, _stat) = self
            .root_handle
            .openat(filename.clone())
            .as_file()
            .with_create()
            .await?;
        Ok(ScratchHandle {
            inner,
            root_handle: Arc::clone(&self.root_handle),
            filename,
        })
    }

    /// Create a new directory in the scratch space with a random name.
    pub async fn directory(&self) -> Result<ScratchDirectoryHandle, crate::Error> {
        let filename = uuid::Uuid::new_v4().to_string();
        tracing::debug!(?filename, "creating new scratch directory");

        let inner = self
            .root_handle
            .openat(filename.clone())
            .as_directory()
            .with_create()
            .await?;
        Ok(ScratchHandle {
            inner,
            root_handle: Arc::clone(&self.root_handle),
            filename,
        })
    }
}

/// A resource in the [`ScratchDirectory`].
pub struct ScratchHandle<Kind> {
    /// Handle to the resource in the scratch directory.
    inner: crate::handle::Handle<Kind>,
    /// Handle to the root of the [`ScratchDirectory`].
    root_handle: Arc<DirectoryHandle>,
    /// Name of this resource.
    filename: String,
}

impl<K> ScratchHandle<K> {
    /// Tag this [`ScratchHandle`] with the ruleset that created it.
    pub async fn tag_ruleset(&self, name: &str) -> Result<(), crate::Error> {
        tracing::debug!(filename = ?self.filename, ?name, "tagging scratch file with ruleset");
        self.inner
            .setxattr(
                SCRATCH_XATTR_TAG_RULESET_NAME.to_string(),
                name.as_bytes().to_vec(),
            )
            .await?;
        Ok(())
    }

    /// Tag this [`ScratchHandle`] with a general comment.
    pub async fn tag_comment(&self, comment: &str) -> Result<(), crate::Error> {
        tracing::debug!(filename = ?self.filename, ?comment, "tagging scratch file with comment");
        self.inner
            .setxattr(
                SCRATCH_XATTR_TAG_COMMENT_NAME.to_string(),
                comment.as_bytes().to_vec(),
            )
            .await?;
        Ok(())
    }

    /// Durably persist a resource in the [`ScratchDirectory`] by moving it
    /// outside the scratch space.
    pub async fn persistat(
        self,
        to_handle: DirectoryHandle,
        to_filename: String,
    ) -> Result<crate::handle::Handle<K>, crate::Error> {
        let ScratchHandle {
            inner,
            root_handle,
            filename: from_filename,
        } = self;

        let from_filename = PlatformFilename::try_new(from_filename)?;
        let to_filename = PlatformFilename::try_new(to_filename)?;
        tracing::debug!(
            ?from_filename,
            ?to_filename,
            "durably persist a scratch resource"
        );

        inner
            .worker
            .run(move || {
                FilesystemPlatform::renameat(
                    root_handle.to_inner(),
                    from_filename,
                    to_handle.to_inner(),
                    to_filename,
                )
            })
            .await?;

        Ok(inner)
    }
}

impl<K> Deref for ScratchHandle<K> {
    type Target = crate::handle::Handle<K>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<K> DerefMut for ScratchHandle<K> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub type ScratchFileHandle = ScratchHandle<FileKind>;
pub type ScratchDirectoryHandle = ScratchHandle<DirectoryKind>;

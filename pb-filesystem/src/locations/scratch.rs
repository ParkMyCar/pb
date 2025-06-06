use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;

use derivative::Derivative;

use crate::filesystem::Filesystem;
use crate::handle::{DirectoryHandle, DirectoryKind, FileKind};
use crate::platform::{FilesystemPlatform, Platform, PlatformFilename};

static SCRATCH_DIRECTORY_NAME: &str = "scratch";

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
///
/// TODO: Add automatic tracking of leaked scratch files.
#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub struct ScratchDirectory {
    /// Root of the scratch directory.
    root_path: PathBuf,
    /// Handle to the root of the scratch directory.
    #[derivative(Debug = "ignore")]
    root_handle: Arc<DirectoryHandle>,
    /// Handle to our filesystem abstraction.
    #[derivative(Debug = "ignore")]
    filesystem: Filesystem,
}

impl ScratchDirectory {
    /// Create a new [`ScratchDirectory`] at `root_path /`[`SCRATCH_DIRECTORY_NAME`].
    pub async fn new(root: PathBuf, filesystem: Filesystem) -> Result<Self, crate::Error> {
        let root_path = root.join(SCRATCH_DIRECTORY_NAME);
        tracing::info!(?root_path, "starting Scratch Directory");

        // TODO: Implement automatic cleanup.
        let root_handle = filesystem.open(root_path.clone()).as_directory().await?;

        Ok(ScratchDirectory {
            root_path,
            root_handle: Arc::new(root_handle),
            filesystem,
        })
    }

    /// Create a new file in the scratch space with a random name.
    pub fn file(&self) -> impl Future<Output = Result<ScratchFileHandle, crate::Error>> + 'static {
        let filename = uuid::Uuid::new_v4().to_string();
        let builder = self
            .root_handle
            .openat(filename.clone())
            .as_file()
            .with_create();
        let root_handle = Arc::clone(&self.root_handle);

        async move {
            tracing::debug!(?filename, "creating new scratch file");
            let (inner, _stat) = builder.await?;
            Ok(ScratchHandle {
                inner,
                root_handle,
                filename,
            })
        }
    }

    /// Create a new directory in the scratch space with a random name.
    pub fn directory(
        &self,
    ) -> impl Future<Output = Result<ScratchDirectoryHandle, crate::Error>> + 'static {
        let filename = uuid::Uuid::new_v4().to_string();
        let builder = self
            .root_handle
            .openat(filename.clone())
            .as_directory()
            .with_create();
        let root_handle = Arc::clone(&self.root_handle);

        async move {
            tracing::debug!(?filename, "creating new scratch directory");
            Ok(ScratchHandle {
                inner: builder.await?,
                root_handle,
                filename,
            })
        }
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
    /// Returns a mutable reference to the inner handle.
    pub fn inner_mut(&mut self) -> &mut crate::handle::Handle<K> {
        &mut self.inner
    }

    pub fn into_inner(self) -> crate::handle::Handle<K> {
        self.inner
    }

    /// Tag this [`ScratchHandle`] with the ruleset that created it.
    pub async fn tag_ruleset(&mut self, name: &str) -> Result<(), crate::Error> {
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
    pub async fn tag_comment(&mut self, comment: &str) -> Result<(), crate::Error> {
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
        to_handle: &DirectoryHandle,
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

        let to_handle = to_handle.to_inner();
        inner
            .worker
            .run(move || {
                FilesystemPlatform::renameat(
                    root_handle.to_inner(),
                    from_filename,
                    to_handle,
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

//! Module that defines a strongly typed filesystem handle.

use futures::channel::mpsc::UnboundedSender;
use futures::future::{Future, TryFutureExt};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use std::borrow::Cow;
use std::future::IntoFuture;
use std::pin::Pin;
use std::sync::Arc;

use crate::platform::{OpenOptions, PlatformFilenameType, PlatformPathType};
use crate::DirectoryEntry;

use super::filesystem::FilesystemWorker;
use super::platform::{
    FilesystemPlatform, Platform, PlatformFilename, PlatformHandleType, PlatformPath,
};
use super::FileMetadata;

/// [`Handle`] to a file.
pub type FileHandle = Handle<FileKind>;
/// [`Handle`] to a directory.
pub type DirectoryHandle = Handle<DirectoryKind>;

/// Type level marker for a handle whose kind is not yet known.
pub struct UnknownKind;
/// Type level marker for a handle to a file.
pub struct FileKind;
/// Type level marker for a handle to a directory.
pub struct DirectoryKind {
    /// Global limiter of open file handles.
    permits: Arc<Semaphore>,
}

/// Opened handle to an object on the filesystem.
#[derive(Debug)]
pub struct Handle<Kind = UnknownKind> {
    /// Actual platform handle, generally a file descriptor.
    pub(crate) inner: Option<PlatformHandleType>,
    /// Permit from the [`Filesystem`] abstraction which rate limits resources.
    pub(crate) permit: Option<OwnedSemaphorePermit>,
    /// Worker that runs I/O operations.
    pub(crate) worker: FilesystemWorker,
    /// Sending side of a queue to close dropped [`Handle`]s.
    pub(crate) drops_tx: UnboundedSender<DroppedHandle>,
    /// Reason this [`Handle`] was opened.
    pub(crate) diagnostics: Option<Cow<'static, str>>,

    /// Type-level flag for what kind of object this handle references.
    pub(crate) kind: Kind,
}

impl<A> Handle<A> {
    /// Get metadata about this handle.
    pub async fn stat(&self) -> Result<FileMetadata, crate::Error> {
        let inner = self.to_inner();
        let result = self
            .worker
            .run(move || FilesystemPlatform::fstat(inner))
            .await?;
        Ok(result)
    }

    /// Flush any buffered state of this file out to disk.
    pub async fn fsync(&self) -> Result<(), crate::Error> {
        let inner = self.to_inner();
        let () = self
            .worker
            .run(move || FilesystemPlatform::fsync(inner))
            .await?;
        Ok(())
    }

    /// Close the filesystem handle, releasing its resources.
    pub async fn close(mut self) -> Result<(), crate::Error> {
        let inner = self
            .inner
            .take()
            .expect("programming error, handle dropped?");
        let permit = self
            .permit
            .take()
            .expect("programming error, handle dropped?");
        let () = self
            .worker
            .run(move || FilesystemPlatform::close(inner))
            .await?;
        drop(permit);

        Ok(())
    }

    /// Attach some diagnostic information to a [`Handle`] for easier debugging.
    pub fn diagnostics<T: Into<Cow<'static, str>>>(&mut self, reason: T) {
        self.diagnostics = Some(reason.into());
    }

    pub(crate) fn to_inner(&self) -> PlatformHandleType {
        self.inner
            .expect("programming error, handle dropped?")
            .clone()
    }

    pub(crate) fn into_parts(mut self) -> (PlatformHandleType, OwnedSemaphorePermit) {
        let handle = self
            .inner
            .take()
            .expect("programming error, handle dropped?");
        let permit = self
            .permit
            .take()
            .expect("programming error, handle dropped?");
        (handle, permit)
    }
}

impl Handle<DirectoryKind> {
    /// List all of the files in the directory.
    pub async fn list(&self) -> Result<Vec<DirectoryEntry>, crate::Error> {
        let inner = self.to_inner();
        let files = self
            .worker
            .run(move || FilesystemPlatform::listdir(inner))
            .await?;
        Ok(files)
    }

    pub async fn openat(&self, filename: String) -> Result<Handle, crate::Error> {
        let permit = Semaphore::acquire_owned(Arc::clone(&self.kind.permits))
            .await
            .expect("filesystem shutting down");
        let inner = self.to_inner();
        let filename = PlatformFilenameType::try_new(filename)?;
        let options = OpenOptions::READ_ONLY;
        let handle = self
            .worker
            .run(move || FilesystemPlatform::openat(inner, filename, options))
            .await?;

        Ok(Handle {
            inner: Some(handle),
            permit: Some(permit),
            worker: self.worker.clone(),
            drops_tx: self.drops_tx.clone(),
            // TODO(parkmycar): Maybe tag these diagnostics with openat context?
            diagnostics: self.diagnostics.clone(),
            kind: UnknownKind,
        })
    }
}

impl<K> Drop for Handle<K> {
    fn drop(&mut self) {
        let handle = self.inner.take();
        let permit = self.permit.take();
        let diagnostics = self.diagnostics.take();

        match (handle, permit) {
            (Some(handle), Some(permit)) => {
                let dropped_handle = DroppedHandle {
                    inner: handle,
                    permit,
                    diagnostics,
                };
                self.drops_tx
                    .unbounded_send(dropped_handle)
                    .expect("filesystem shutting down");
            }
            (None, None) => (),
            (Some(_), None) | (None, Some(_)) => panic!("Handle partially cleaned up?"),
        }
    }
}

/// Extension trait for [`Handle`].
pub trait HandleExt {
    type Return;

    /// Attach some diagnostic information to a [`Handle`] for easier debugging.
    fn diagnostics<T: Into<Cow<'static, str>>>(self, reason: T) -> Self::Return;
}

impl<E, F: Future<Output = Result<Handle, E>>> HandleExt for F {
    type Return = futures::future::MapOk<Self, Box<dyn FnOnce(Handle) -> Handle + Send + Sync>>;

    fn diagnostics<T: Into<Cow<'static, str>>>(self, reason: T) -> Self::Return {
        let reason = reason.into();
        let closure = Box::new(|mut handle: Handle| {
            handle.diagnostics(reason);
            handle
        });
        self.map_ok(closure)
    }
}

#[derive(Debug)]
pub struct UnknownDetails;

#[derive(Debug, Default)]
pub struct FileDetails {
    flags: OpenOptions,
}

#[derive(Debug)]
pub struct DirectoryDetails;

/// Builder struct for a [`Handle`].
pub struct HandleBuilder<Details = UnknownDetails> {
    /// Worker that runs I/O operations.
    pub(crate) worker: FilesystemWorker,
    /// Sending side of a queue to close dropped [`Handle`]s.
    pub(crate) drops_tx: UnboundedSender<DroppedHandle>,
    /// Global sempahore limiting all open filesystem handles.
    pub(crate) permits: Arc<Semaphore>,
    /// Reason this [`Handle`] was opened.
    pub(crate) diagnostics: Option<Cow<'static, str>>,

    /// Path we're opening.
    pub(crate) path: String,
    /// Details for opening a specific kind of file handle.
    pub(crate) details: Details,
}

impl HandleBuilder<UnknownDetails> {
    pub(crate) fn new(
        worker: FilesystemWorker,
        drops_tx: UnboundedSender<DroppedHandle>,
        permits: Arc<Semaphore>,
        path: String,
    ) -> HandleBuilder<UnknownDetails> {
        HandleBuilder {
            worker,
            drops_tx,
            permits,
            diagnostics: None,
            path,
            details: UnknownDetails,
        }
    }
}

impl<D> HandleBuilder<D> {
    /// Tag this [`Handle`] with the reason we're opening it.
    pub fn diagnostics<T: Into<Cow<'static, str>>>(mut self, reason: T) -> Self {
        self.diagnostics = Some(reason.into());
        self
    }

    /// Open a file with this [`HandleBuilder`].
    pub fn as_file(self) -> HandleBuilder<FileDetails> {
        HandleBuilder {
            worker: self.worker,
            drops_tx: self.drops_tx,
            permits: self.permits,
            diagnostics: self.diagnostics,
            path: self.path,
            details: FileDetails::default(),
        }
    }

    /// Open a directory with this [`HandleBuilder`].
    pub fn as_directory(self) -> HandleBuilder<DirectoryDetails> {
        HandleBuilder {
            worker: self.worker,
            drops_tx: self.drops_tx,
            permits: self.permits,
            diagnostics: self.diagnostics,
            path: self.path,
            details: DirectoryDetails,
        }
    }
}

impl HandleBuilder<FileDetails> {
    /// Append to the file when writing.
    pub fn with_append(mut self) -> Self {
        self.details.flags |= OpenOptions::APPEND;
        self
    }

    /// Create the file if it doesn't exist.
    pub fn with_create(mut self) -> Self {
        self.details.flags |= OpenOptions::CREATE;
        self
    }

    /// Error if [`HandleBuilder::with_create`] is specified and the file already exists.
    pub fn with_exclusive(mut self) -> Self {
        self.details.flags |= OpenOptions::EXCLUSIVE;
        self
    }

    /// Truncate the file when opening.
    pub fn with_truncate(mut self) -> Self {
        self.details.flags |= OpenOptions::TRUNCATE;
        self
    }
}

impl IntoFuture for HandleBuilder {
    type Output = Result<Handle<UnknownKind>, crate::Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        let fut = async move {
            let permit = Semaphore::acquire_owned(self.permits)
                .await
                .expect("failed to acquire permit");

            // Open this handle with just read only perms.
            let options = OpenOptions::READ_ONLY;
            let path = PlatformPathType::try_new(self.path)?;
            let handle = self
                .worker
                .run(move || FilesystemPlatform::open(path, options))
                .await?;

            let handle = Handle {
                inner: Some(handle),
                permit: Some(permit),
                worker: self.worker,
                drops_tx: self.drops_tx,
                diagnostics: self.diagnostics,
                kind: UnknownKind,
            };

            Ok(handle)
        };
        Box::pin(fut)
    }
}

impl IntoFuture for HandleBuilder<FileDetails> {
    type Output = Result<Handle<FileKind>, crate::Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        let fut = async move {
            let permit = Semaphore::acquire_owned(self.permits)
                .await
                .expect("failed to acquire permit");
            let path = PlatformPathType::try_new(self.path)?;
            let handle = self
                .worker
                .run(move || FilesystemPlatform::open(path, self.details.flags))
                .await?;

            let handle = Handle {
                inner: Some(handle),
                permit: Some(permit),
                worker: self.worker,
                drops_tx: self.drops_tx,
                diagnostics: self.diagnostics,
                kind: FileKind,
            };

            Ok(handle)
        };
        Box::pin(fut)
    }
}

impl IntoFuture for HandleBuilder<DirectoryDetails> {
    type Output = Result<Handle<DirectoryKind>, crate::Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        let fut = async move {
            let kind = DirectoryKind {
                permits: Arc::clone(&self.permits),
            };
            let permit = Semaphore::acquire_owned(self.permits)
                .await
                .expect("failed to acquire permit");

            let options = OpenOptions::DIRECTORY;
            let path = PlatformPathType::try_new(self.path)?;
            let handle = self
                .worker
                .run(move || FilesystemPlatform::open(path, options))
                .await?;

            let handle = Handle {
                inner: Some(handle),
                permit: Some(permit),
                worker: self.worker,
                drops_tx: self.drops_tx,
                diagnostics: self.diagnostics,
                kind,
            };

            Ok(handle)
        };
        Box::pin(fut)
    }
}

/// A [`Handle`] that has been [`Drop`]-ed but not yet closed.
pub(crate) struct DroppedHandle {
    /// The platform specific file handle.
    pub(crate) inner: PlatformHandleType,
    /// Permit we keep open for the life of the handle for resource management.
    pub(crate) permit: OwnedSemaphorePermit,
    /// Diagnostics from the original handle.
    pub(crate) diagnostics: Option<Cow<'static, str>>,
}

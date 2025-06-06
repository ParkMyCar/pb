//! Module that defines a strongly typed filesystem handle.

use futures::future::{Future, TryFutureExt};
use pb_types::Timespec;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use std::borrow::Cow;
use std::future::IntoFuture;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use crate::filesystem::BlockPool;
use crate::platform::{OpenOptions, PlatformFilenameType, PlatformPathType};
use crate::{DirectoryEntry, FileType};

use super::filesystem::FilesystemWorker;
use super::platform::{
    FilesystemPlatform, Platform, PlatformFilename, PlatformHandleType, PlatformPath,
};
use super::FileStat;

/// [`Handle`] to a file.
pub type FileHandle = Handle<FileKind>;
/// [`Handle`] to a directory.
pub type DirectoryHandle = Handle<DirectoryKind>;

/// Enum wrapper around all the different kinds of handles.
pub enum HandleKind {
    File(FileHandle),
    Directory(DirectoryHandle),
}

/// Type level marker for a handle whose kind is not yet known.
pub struct UnknownKind;

/// Type level marker for a handle to a file.
pub struct FileKind {
    /// Optimal blocksize for I/O.
    pub(crate) optimal_blocksize: Option<usize>,
}

/// Type level marker for a handle to a directory.
pub struct DirectoryKind {
    /// Global limiter of open file handles.
    pub(crate) permits: Arc<Semaphore>,
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
    pub(crate) drops_tx: crossbeam::channel::Sender<DroppedHandle>,
    /// Reason this [`Handle`] was opened.
    pub(crate) diagnostics: Option<Cow<'static, str>>,

    /// Type-level flag for what kind of object this handle references.
    pub(crate) kind: Kind,
}

impl<A> Handle<A> {
    /// Get metadata about this handle.
    pub async fn stat(&self) -> Result<FileStat, crate::Error> {
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

    /// Set the specified xattr on the file.
    pub async fn setxattr(&mut self, name: String, data: Vec<u8>) -> Result<(), crate::Error> {
        let inner = self.to_inner();
        let name = PlatformFilenameType::try_new(name)?;
        let () = self
            .worker
            .run(move || FilesystemPlatform::fsetxattr(inner, name, &data[..]))
            .await?;
        Ok(())
    }

    /// Set the mtime on the file.
    pub async fn setmtime(&mut self, _time: Timespec) -> Result<(), crate::Error> {
        todo!()
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

    /// Open the file relative to this directory.
    pub fn openat(&self, filename: String) -> HandleBuilder {
        let directory = self.to_inner();
        HandleBuilder::new(
            self.worker.clone(),
            self.drops_tx.clone(),
            Arc::clone(&self.kind.permits),
            HandleLocation::At {
                directory,
                filename: filename.into(),
            },
        )
    }

    /// Stat the file relative to this directory.
    pub async fn fstatat(&self, filename: String) -> Result<FileStat, crate::Error> {
        let inner = self.to_inner();
        let name = PlatformFilenameType::try_new(filename)?;
        let stat = self
            .worker
            .run(move || FilesystemPlatform::fstatat(inner, name))
            .await?;
        Ok(stat)
    }
}

impl Handle<FileKind> {
    /// Write the provided data to the file.
    pub async fn write(&mut self, data: Vec<u8>, offset: usize) -> Result<(), crate::Error> {
        let inner = self.to_inner();
        let _result = self
            .worker
            .run(move || FilesystemPlatform::write(inner, &data[..], offset))
            .await?;
        Ok(())
    }

    /// Read some bytes from the file into the provided buffer, in a blocking fashion.
    pub fn read_blocking(&self, buf: &mut [u8], offset: usize) -> Result<usize, crate::Error> {
        let inner = self.to_inner();
        FilesystemPlatform::read(inner, buf, offset)
    }

    /// Read the contents of the file executing some work on the worker's thread pool.
    pub async fn read_with<'a, R, F>(&self, work: F) -> Result<R, crate::Error>
    where
        R: Send + 'static,
        F: FnOnce(internal::ReadIterator) -> Result<R, crate::Error> + Send + 'static,
    {
        let inner = self.to_inner();
        // Most filesystems have a block size of 4096.
        //
        // TODO: Consider using a multiple of the block size if the file requires more
        // than 1 block to read.
        let block_size = self
            .kind
            .optimal_blocksize
            .unwrap_or(4096)
            .saturating_mul(8);

        self.worker
            .run(move || {
                let result = BlockPool::BLOCK_POOL.with_borrow_mut(|pool| {
                    let block = pool.get_block(block_size);
                    let byte_iter = internal::ReadIterator::new(inner, block);
                    work(byte_iter)
                });
                result
            })
            .await
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
                    .send(dropped_handle)
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
pub struct DirectoryDetails {
    /// Should we make a directory or not.
    create: bool,
}

#[derive(Debug)]
pub enum HandleLocation {
    /// Opening a path directly.
    Path(PathBuf),
    /// Opening relative to a parent directory.
    At {
        directory: PlatformHandleType,
        filename: String,
    },
}

/// Builder struct for a [`Handle`].
pub struct HandleBuilder<Details = UnknownDetails> {
    /// Worker that runs I/O operations.
    pub(crate) worker: FilesystemWorker,
    /// Sending side of a queue to close dropped [`Handle`]s.
    pub(crate) drops_tx: crossbeam::channel::Sender<DroppedHandle>,
    /// Global sempahore limiting all open filesystem handles.
    pub(crate) permits: Arc<Semaphore>,
    /// Reason this [`Handle`] was opened.
    pub(crate) diagnostics: Option<Cow<'static, str>>,

    /// Location we're opening.
    pub(crate) location: HandleLocation,
    /// Details for opening a specific kind of file handle.
    pub(crate) details: Details,
}

impl HandleBuilder<UnknownDetails> {
    pub(crate) fn new(
        worker: FilesystemWorker,
        drops_tx: crossbeam::channel::Sender<DroppedHandle>,
        permits: Arc<Semaphore>,
        location: HandleLocation,
    ) -> HandleBuilder<UnknownDetails> {
        HandleBuilder {
            worker,
            drops_tx,
            permits,
            diagnostics: None,
            location,
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
            location: self.location,
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
            location: self.location,
            details: DirectoryDetails { create: false },
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

impl HandleBuilder<DirectoryDetails> {
    /// Create the directory if it doesn't exist.
    pub fn with_create(mut self) -> Self {
        self.details.create = true;
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
            let handle = match self.location {
                HandleLocation::Path(path) => {
                    let path = PlatformPathType::try_new(path)?;
                    let handle = self
                        .worker
                        .run(move || FilesystemPlatform::open(path, options))
                        .await?;
                    handle
                }
                HandleLocation::At {
                    directory,
                    filename,
                } => {
                    let filename = PlatformFilenameType::try_new(filename)?;
                    let handle = self
                        .worker
                        .run(move || FilesystemPlatform::openat(directory, filename, options))
                        .await?;
                    handle
                }
            };

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
    type Output = Result<(Handle<FileKind>, FileStat), crate::Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        let fut = async move {
            let permit = Semaphore::acquire_owned(self.permits)
                .await
                .expect("failed to acquire permit");

            let (handle, stat) = match self.location {
                HandleLocation::Path(path) => {
                    let path = PlatformPathType::try_new(path)?;
                    self.worker
                        .run(move || {
                            let handle = FilesystemPlatform::open(path, self.details.flags)?;
                            // TODO(parkmycar): Always stating a file when opening feels wasteful?
                            let stat = FilesystemPlatform::fstat(handle.clone())?;
                            Ok((handle, stat))
                        })
                        .await?
                }
                HandleLocation::At {
                    directory,
                    filename,
                } => {
                    let filename = PlatformFilenameType::try_new(filename)?;
                    self.worker
                        .run(move || {
                            let handle = FilesystemPlatform::openat(
                                directory,
                                filename,
                                self.details.flags,
                            )?;
                            // TODO(parkmycar): Always stating a file when opening feels wasteful?
                            let stat = FilesystemPlatform::fstat(handle.clone())?;
                            Ok((handle, stat))
                        })
                        .await?
                }
            };

            if stat.kind != FileType::File {
                Err(crate::Error::NotAFile("todo".into()))
            } else {
                let kind = FileKind {
                    optimal_blocksize: stat.optimal_blocksize,
                };
                let handle = Handle {
                    inner: Some(handle),
                    permit: Some(permit),
                    worker: self.worker,
                    drops_tx: self.drops_tx,
                    diagnostics: self.diagnostics,
                    kind,
                };
                Ok((handle, stat))
            }
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

            // First create the directory.
            if self.details.create {
                match &self.location {
                    HandleLocation::Path(path) => {
                        let path = PlatformPathType::try_new(path.clone())?;
                        self.worker
                            .run(move || FilesystemPlatform::mkdir(path))
                            .await?;
                    }
                    HandleLocation::At {
                        directory,
                        filename,
                    } => {
                        let directory = directory.clone();
                        let filename = PlatformFilenameType::try_new(filename.clone())?;
                        self.worker
                            .run(move || FilesystemPlatform::mkdirat(directory, filename))
                            .await?;
                    }
                }
            }

            // Then open a handle to it.
            let options = OpenOptions::DIRECTORY;
            let handle = match self.location {
                HandleLocation::Path(path) => {
                    let path = PlatformPathType::try_new(path)?;
                    let handle = self
                        .worker
                        .run(move || FilesystemPlatform::open(path, options))
                        .await?;
                    handle
                }
                HandleLocation::At {
                    directory,
                    filename,
                } => {
                    let filename = PlatformFilenameType::try_new(filename)?;
                    let handle = self
                        .worker
                        .run(move || FilesystemPlatform::openat(directory, filename, options))
                        .await?;
                    handle
                }
            };

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

pub mod internal {
    use crate::filesystem::Block;
    use crate::platform::{FilesystemPlatform, Platform, PlatformHandleType};
    use pb_ore::iter::LendingIterator;

    /// A [`LendingIterator`] that reads from a [`Handle`] and returns byte slices.
    ///
    /// [`Handle`]: crate::handle::Handle
    pub struct ReadIterator<'a> {
        /// Stream of the file we're reading from.
        handle: PlatformHandleType,
        /// Re-usable block of memory for I/O.
        block: &'a mut Block,
        /// Current offset into the file that we're reading from.
        offset: usize,
        /// Is the iterator complete.
        done: bool,
    }

    impl<'a> ReadIterator<'a> {
        pub fn new(handle: PlatformHandleType, block: &'a mut Block) -> Self {
            ReadIterator {
                handle,
                block,
                offset: 0,
                done: false,
            }
        }
    }

    impl<'r> LendingIterator for ReadIterator<'r> {
        type Item<'a>
            = Result<&'a [u8], crate::Error>
        where
            Self: 'a,
            'r: 'a;

        fn next(&mut self) -> Option<Self::Item<'_>> {
            // If we previously errored, or read 0 bytes, don't yield again.
            if self.done {
                return None;
            }

            // We re-use the block for each iteration so make sure it's cleared.
            // self.block.clear();

            // Read the next chunk.
            let block_size = self.block.size();
            match FilesystemPlatform::read(self.handle.clone(), self.block.as_mut(), self.offset) {
                // Read less bytes than the size of the buffer, we're done!
                Ok(bytes_read) if bytes_read < block_size => {
                    self.done = true;
                    self.offset = self
                        .offset
                        .checked_add(bytes_read)
                        .expect("read more than usize bytes?");
                    Some(Ok(&self.block.as_ref()[..bytes_read]))
                }
                // Errored, so stop reading here.
                Err(e) => {
                    self.done = true;
                    Some(Err(e))
                }
                // Yield the bytes we just read!
                Ok(bytes_read) => {
                    self.offset = self
                        .offset
                        .checked_add(bytes_read)
                        .expect("read more than usize bytes?");
                    Some(Ok(&self.block.as_ref()[..bytes_read]))
                }
            }
        }
    }
}

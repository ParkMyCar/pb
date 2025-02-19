use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::{FutureExt, StreamExt};
use std::fmt;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::handle::HandleBuilder;
use crate::platform::PlatformPathType;

use super::handle::{DroppedHandle, Handle};
use super::platform::{FilesystemPlatform, Platform, PlatformPath};
use super::FileMetadata;

/// A safe Filesystem abstraction.
///
/// The goal of this type is to abstract over platform specific implementations for
/// filesystem operations, provide automatic cleanup and management of resources, as well
/// as helpers to attach debug information to filesystem [`Handle`]s.
pub struct Filesystem {
    /// Pool to spawn blocking work on.
    worker: FilesystemWorker,
    /// The number of file system handles that are allowed to be open at once.
    permits: Arc<Semaphore>,
    /// Queue of handles that have been dropped but not yet closed.
    drops_tx: UnboundedSender<DroppedHandle>,
}

impl Filesystem {
    pub fn new_tokio(worker: tokio::runtime::Handle, max_handles: usize) -> Self {
        let (drops_tx, drops_rx) = futures::channel::mpsc::unbounded();
        Filesystem {
            worker: FilesystemWorker::new_tokio(worker, drops_rx),
            permits: Arc::new(Semaphore::new(max_handles)),
            drops_tx,
        }
    }

    pub fn open(&self, path: String) -> HandleBuilder {
        HandleBuilder::new(
            self.worker.clone(),
            self.drops_tx.clone(),
            Arc::clone(&self.permits),
            path,
        )
    }

    pub async fn close(&self, handle: Handle) -> Result<(), crate::Error> {
        let (handle, permit) = handle.into_parts();
        self.worker
            .run(move || FilesystemPlatform::close(handle))
            .await?;
        drop(permit);
        Ok(())
    }

    pub async fn stat(&self, path: String) -> Result<FileMetadata, crate::Error> {
        let path = PlatformPathType::try_new(path)?;
        let result = self.worker.run(|| FilesystemPlatform::stat(path)).await?;
        Ok(result)
    }
}

/// Worker for handling filesystem operations.
///
/// Most filesystem operations are not truly asynchronous, so instead we spawn a
/// thread-pool and run the blocking operations there.
#[derive(Clone)]
pub struct FilesystemWorker {
    /// Thread pool for spawning I/O.
    pool: Arc<WorkerPool>,
}

impl FilesystemWorker {
    fn new_tokio(
        runtime: tokio::runtime::Handle,
        mut drops_rx: UnboundedReceiver<DroppedHandle>,
    ) -> Self {
        let task = runtime.spawn(async move {
            while let Some(drop_handle) = drops_rx.next().await {
                let DroppedHandle {
                    inner,
                    permit,
                    diagnostics,
                } = drop_handle;

                // Close the handle.
                let result = FilesystemPlatform::close(inner);
                // Drop our permit.
                drop(permit);

                match result {
                    Ok(()) => tracing::info!("async closed handle for: {diagnostics:?}"),
                    Err(err) => tracing::warn!(
                        "failed to async close handle for: {diagnostics:?}, err: {err}"
                    ),
                }
            }
        });
        let pool = WorkerPool::Tokio {
            runtime,
            _drop_task: task,
        };

        FilesystemWorker {
            pool: Arc::new(pool),
        }
    }

    pub fn run<T, W>(&self, work: W) -> impl Future<Output = T> + 'static
    where
        T: Send + 'static,
        W: FnOnce() -> T + Send + 'static,
    {
        let (tx, rx) = futures::channel::oneshot::channel();
        match &*self.pool {
            WorkerPool::Tokio { runtime, .. } => {
                runtime.spawn_blocking(|| {
                    let result = work();
                    // We don't care about the sender going away.
                    let _ = tx.send(result);
                });
            }
        }

        rx.map(|result| result.expect("worker pool shutting down"))
    }
}

impl fmt::Debug for FilesystemWorker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilesystemWorker").finish()
    }
}

#[derive(Debug)]
enum WorkerPool {
    Tokio {
        runtime: tokio::runtime::Handle,
        /// Task that closes [`DroppedHandle`]s.
        _drop_task: tokio::task::JoinHandle<()>,
    },
}

use futures::FutureExt;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::handle::{HandleBuilder, HandleLocation};
use crate::platform::PlatformPathType;

use super::handle::{DroppedHandle, Handle};
use super::platform::{FilesystemPlatform, Platform, PlatformPath};
use super::FileStat;

/// A safe Filesystem abstraction.
///
/// The goal of this type is to abstract over platform specific implementations for
/// filesystem operations, provide automatic cleanup and management of resources, as well
/// as helpers to attach debug information to filesystem [`Handle`]s.
#[derive(Clone)]
pub struct Filesystem {
    /// Pool to spawn blocking work on.
    worker: FilesystemWorker,
    /// The number of file system handles that are allowed to be open at once.
    permits: Arc<Semaphore>,
    /// Queue of handles that have been dropped but not yet closed.
    drops_tx: crossbeam::channel::Sender<DroppedHandle>,
}

impl Filesystem {
    pub fn new(num_threads: usize, max_handles: usize) -> Self {
        let (drops_tx, drops_rx) = crossbeam::channel::unbounded();
        Filesystem {
            worker: FilesystemWorker::new(num_threads, drops_rx),
            permits: Arc::new(Semaphore::new(max_handles)),
            drops_tx,
        }
    }

    pub fn available_permits(&self) -> usize {
        self.permits.available_permits()
    }

    pub fn open(&self, path: String) -> HandleBuilder {
        HandleBuilder::new(
            self.worker.clone(),
            self.drops_tx.clone(),
            Arc::clone(&self.permits),
            HandleLocation::Path(path),
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

    pub async fn stat(&self, path: String) -> Result<FileStat, crate::Error> {
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
    fn new(size: usize, drops_rx: crossbeam::channel::Receiver<DroppedHandle>) -> Self {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(size)
            .build()
            .expect("failed to create threadpool");

        thread_pool.spawn(move || {
            let mut handles = Vec::new();

            loop {
                // Block until there is a dropped handle.
                match drops_rx.recv() {
                    Ok(dropped_handle) => handles.push(dropped_handle),
                    Err(notice) => {
                        tracing::info!(?notice, "drops sender went away, shutting down");
                        return;
                    }
                }

                // Collect all of the currently queued handles, if any.
                handles.extend(drops_rx.try_iter());

                // Drop all of the handles.
                for dropped_handle in handles.drain(..) {
                    let DroppedHandle {
                        inner,
                        permit,
                        diagnostics,
                    } = dropped_handle;

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
            }
        });

        let pool = WorkerPool::Rayon { pool: thread_pool };
        FilesystemWorker {
            pool: Arc::new(pool),
        }
    }

    pub fn run<T, W>(&self, work: W) -> impl Future<Output = T> + 'static
    where
        T: Send + 'static,
        W: FnOnce() -> T + Send + 'static,
    {
        self.run_typed(work)
            .map(|result| result.expect("worker pool shutting down"))
    }

    /// TODO document why this exists, and why it's nice to be able to name our return type.
    pub fn run_typed<T, W>(&self, work: W) -> tokio::sync::oneshot::Receiver<T>
    where
        T: Send + 'static,
        W: FnOnce() -> T + Send + 'static,
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        match &*self.pool {
            WorkerPool::Tokio { runtime, .. } => {
                runtime.spawn_blocking(|| {
                    let result = work();
                    // We don't care about the sender going away.
                    let _ = tx.send(result);
                });
            }
            WorkerPool::Rayon { pool } => {
                pool.spawn(|| {
                    let result = work();
                    // We don't care about the sender going away.
                    let _ = tx.send(result);
                });
            }
        }
        rx
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
    Rayon {
        pool: rayon::ThreadPool,
    },
}

/// Pool of [`Block`]s used when reading files.
#[derive(Debug, Default)]
pub struct BlockPool {
    blocks: HashMap<usize, Block>,
}

impl BlockPool {
    // Thread local variables for each worker in the pool.
    std::thread_local! {
        /// A pool of reusable memory [`Block`]s that can to read into when doing I/O.
        pub(crate) static BLOCK_POOL: RefCell<BlockPool> = RefCell::new(BlockPool::default());
    }

    /// Gets a block of the specified size, lazily creating one if it doesn't exist.
    pub fn get_block(&mut self, size: usize) -> &mut Block {
        self.blocks.entry(size).or_insert_with(|| Block::new(size))
    }
}

/// Pre-allocated and reusable block of memory for reading the contents of a file.
#[derive(Debug)]
pub struct Block {
    inner: Vec<u8>,
}

impl Block {
    /// Creates a new block of the specified size with 0's
    pub fn new(size: usize) -> Block {
        let inner = vec![0; size];
        Block { inner }
    }

    pub fn size(&self) -> usize {
        self.inner.len()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn as_ref(&self) -> &[u8] {
        &self.inner[..]
    }

    pub fn as_mut(&mut self) -> &mut [u8] {
        &mut self.inner[..]
    }
}

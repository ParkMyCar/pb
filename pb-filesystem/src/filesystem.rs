use futures::FutureExt;
use std::future::Future;

use crate::platform::{FilesystemPlatform, Platform};
use crate::FileMetadata;

pub struct Filesystem {
    /// Pool to spawn blocking work on.
    worker: WorkerPool,
    /// Semaphore which limits how many handles are open at once.
    handle_limiter: tokio::sync::Semaphore,
}

impl Filesystem {
    pub fn new_tokio(worker: tokio::runtime::Handle, max_handles: usize) -> Self {
        Filesystem {
            worker: WorkerPool::Tokio(worker),
            handle_limiter: tokio::sync::Semaphore::new(max_handles),
        }
    }

    pub async fn stat(&self, path: String) -> Result<FileMetadata, crate::Error> {
        let permit = self.handle_limiter.acquire().await.expect("shutting down");
        let result = self.worker.run(|| FilesystemPlatform::stat(path)).await?;

        // Stat doesn't keep a file descriptor open so we can drop our permit.
        drop(permit);

        Ok(result)
    }
}

#[derive(Debug)]
enum WorkerPool {
    Tokio(tokio::runtime::Handle),
}

impl WorkerPool {
    pub fn run<T, W>(&self, work: W) -> impl Future<Output = T> + 'static
    where
        T: Send + 'static,
        W: FnOnce() -> T + Send + 'static,
    {
        let (tx, rx) = futures::channel::oneshot::channel();
        match self {
            WorkerPool::Tokio(handle) => {
                handle.spawn_blocking(|| {
                    let result = work();
                    // We don't care about the sender going away.
                    let _ = tx.send(result);
                });
            }
        }

        rx.map(|result| result.expect("worker pool shutting down"))
    }
}

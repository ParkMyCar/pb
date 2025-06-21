use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::future::{Future, IntoFuture};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use futures::future::{LocalBoxFuture, TryFutureExt};
use futures::FutureExt;
use pb_trie::{TrieMap, TrieNode};
use pb_types::InternedPath;
use tokio::sync::Semaphore;

use crate::handle::internal::ReadIterator;
use crate::handle::{DirectoryHandle, DirectoryKind, FileKind, Handle};
use crate::platform::{FilesystemPlatform, OpenOptions, Platform, PlatformPath, PlatformPathType};
use crate::{FileStat, FileType};

/// Tree description of an object in the filesystem.
#[derive(Debug)]
pub struct MetadataTree<T: Clone> {
    /// Where this tree is rooted at.
    root_path: PathBuf,
    /// Entries in the tree.
    trie: pb_trie::TrieMap<InternedPath, (), T>,
    /// The ignore set this tree was created with.
    ignore: Option<globset::GlobSet>,
    /// Interned strings.
    strings: lasso::Rodeo,
}

impl<T: Clone> MetadataTree<T> {
    /// Returns if the provided path is ignored by the [`MetadataTree`]'s initial globset.
    pub fn ignored<P: AsRef<Path>>(&self, path: P) -> bool {
        let Some(globset) = self.ignore.as_ref() else {
            return false;
        };
        globset.is_match(path.as_ref())
    }
}

impl<T: Clone> fmt::Display for MetadataTree<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pretty_trie = self.trie.pretty(|f, component| {
            let name = self.strings.resolve(component);
            f.write_all(name.as_bytes())
        });
        write!(f, "{pretty_trie}")?;
        Ok(())
    }
}

impl<K> Handle<K> {
    /// Get the absolute path that corresponds to this file handle.
    ///
    /// TODO: How does this interact when a single file has multiple hard links?
    async fn fullpath(&self) -> Result<PathBuf, crate::Error> {
        let inner = self.to_inner();
        let path = self
            .worker
            .run(move || FilesystemPlatform::fgetpath(inner))
            .await?;
        let path = PathBuf::from(path.into_inner());
        Ok(path)
    }
}

impl DirectoryHandle {
    /// Recursively walk this handle returning a [`MetadataTree`] that describes
    /// everything underneath it.
    ///
    /// TODO: Currently this implementation works on paths. It might be nice
    /// rewrite this to use "openat semantics", but that is tricky from a resource
    /// perspective because it requires keeping many file handles open.
    pub fn tree(&self) -> TreeBuilder<(), FileStat> {
        TreeBuilder::new(self)
    }
}

pub struct TreeBuilder<'a, T, S>
where
    T: Clone,
    S: TreeFileMetadata<Value = T>,
{
    /// Directory we will begin iterating from.
    root_directory: &'a DirectoryHandle,

    /// Closure that will be called with the contents of every closure.
    file_work: Option<
        Arc<
            dyn for<'d> Fn(&'d FileStat, ReadIterator<'d>) -> Result<T, crate::Error>
                + Send
                + Sync
                + 'static,
        >,
    >,
    /// Globset of files to ignore.
    ignore: Option<globset::GlobSet>,

    _file_stat: std::marker::PhantomData<fn() -> S>,
}

impl<'a> TreeBuilder<'a, (), FileStat> {
    pub fn new(root_directory: &'a DirectoryHandle) -> Self {
        TreeBuilder {
            root_directory,
            file_work: None,
            ignore: None,
            _file_stat: std::marker::PhantomData::default(),
        }
    }

    pub fn with_data<T, W>(self, work: W) -> TreeBuilder<'a, T, (FileStat, T)>
    where
        T: Clone + Send + 'static,
        for<'d> W:
            Fn(&'d FileStat, ReadIterator<'d>) -> Result<T, crate::Error> + Send + Sync + 'static,
    {
        TreeBuilder {
            root_directory: self.root_directory,
            file_work: Some(Arc::new(work)),
            ignore: self.ignore,
            _file_stat: std::marker::PhantomData::default(),
        }
    }
}

impl<'a, T, S> TreeBuilder<'a, T, S>
where
    T: Clone,
    S: TreeFileMetadata<Value = T>,
{
    pub fn ignore(mut self, glob_set: globset::GlobSet) -> Self {
        self.ignore = Some(glob_set);
        self
    }
}

impl<'a, T, S> IntoFuture for TreeBuilder<'a, T, S>
where
    T: Clone + Send + 'static,
    S: TreeFileMetadata<Value = T>,
{
    type Output = Result<MetadataTree<S>, crate::Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        let handle_dir = |path: PathBuf| {
            let worker_ = self.root_directory.worker.clone();
            let drops_tx_ = self.root_directory.drops_tx.clone();
            let permits_ = Arc::clone(&self.root_directory.kind.permits);

            async move {
                let path = PlatformPathType::try_new(path).expect("known valid");
                let permit = Semaphore::acquire_owned(permits_.clone())
                    .await
                    .expect("failed to acquire permit");
                let handle = worker_
                    .run(|| FilesystemPlatform::open(path, OpenOptions::DIRECTORY))
                    .await?;
                let handle = Handle {
                    inner: Some(handle),
                    permit: Some(permit),
                    worker: worker_.clone(),
                    drops_tx: drops_tx_.clone(),
                    diagnostics: Some(Cow::Borrowed("tree")),
                    kind: DirectoryKind {
                        permits: Arc::clone(&permits_),
                    },
                };
                Ok::<_, crate::Error>(handle)
            }
        };

        let handle_file = move |path: PathBuf| {
            let worker_ = self.root_directory.worker.clone();
            let drops_tx_ = self.root_directory.drops_tx.clone();
            let permits_ = Arc::clone(&self.root_directory.kind.permits);
            let maybe_work_fn_ = match &self.file_work {
                None => None,
                Some(closure) => Some(Arc::clone(closure)),
            };

            async move {
                // Open a handle to our path.
                let path = PlatformPathType::try_new(path).expect("known valid");
                let (stat, value) = match maybe_work_fn_.as_ref() {
                    None => {
                        let stat = worker_.run(|| FilesystemPlatform::stat(path)).await?;
                        (stat, None)
                    }
                    Some(work_fn) => {
                        let permit = Semaphore::acquire_owned(permits_.clone())
                            .await
                            .expect("failed to acquire permit");
                        let (handle, stat) = worker_
                            .run(|| {
                                let handle =
                                    FilesystemPlatform::open(path, OpenOptions::READ_ONLY)?;
                                let stat = FilesystemPlatform::fstat(handle)?;
                                Ok((handle, stat))
                            })
                            .await?;
                        let handle = Handle {
                            inner: Some(handle),
                            permit: Some(permit),
                            worker: worker_.clone(),
                            drops_tx: drops_tx_.clone(),
                            diagnostics: Some(Cow::Borrowed("tree-file")),
                            kind: FileKind {
                                optimal_blocksize: stat.optimal_blocksize,
                            },
                        };
                        let work_fn_ = Arc::clone(work_fn);
                        let value = handle
                            .read_with(move |reader| work_fn_(&stat, reader))
                            .await?;
                        handle.close().await?;

                        (stat, Some(value))
                    }
                };

                let output = S::from_parts(stat, value);
                Ok::<_, crate::Error>(output)
            }
        };

        async move {
            let strings = Rc::new(RefCell::new(lasso::Rodeo::new()));
            let start_path = self.root_directory.fullpath().await?;
            let children = walk_directory(
                start_path.clone(),
                self.ignore.as_ref(),
                &handle_dir,
                &handle_file,
                strings.clone(),
            )
            .await?;
            // All of the futures have completed by now so this is safe.
            let strings = strings.take();

            Ok(MetadataTree {
                root_path: start_path,
                trie: TrieMap::from_node(TrieNode::Edge { children, data: () }),
                ignore: self.ignore,
                strings,
            })
        }
        .boxed_local()
    }
}

/// Recursively walk a directory.
fn walk_directory<'a, D, W, S, F1, F2>(
    path: PathBuf,
    ignore: Option<&'a globset::GlobSet>,
    open_dir: &'a D,
    process_file: &'a W,
    strings: Rc<RefCell<lasso::Rodeo>>,
) -> LocalBoxFuture<'a, Result<BTreeMap<lasso::Spur, TrieNode<InternedPath, (), S>>, crate::Error>>
where
    S: TreeFileMetadata,
    F1: Future<Output = Result<DirectoryHandle, crate::Error>> + Send,
    F2: Future<Output = Result<S, crate::Error>> + Send,
    D: Fn(PathBuf) -> F1 + Sync,
    W: Fn(PathBuf) -> F2 + Sync,
{
    enum ProcessResult<S_: TreeFileMetadata> {
        Directory(BTreeMap<lasso::Spur, TrieNode<InternedPath, (), S_>>),
        File(S_),
    }

    async move {
        tracing::trace!(?path, "processing directory");
        let handle = open_dir(path.clone()).await?;
        let entries = handle.list().await?;

        let mut children = BTreeMap::default();
        let mut futures = Vec::new();

        for entry in entries {
            let new_path = path.join(&entry.name);
            if let Some(ignore_glob_set) = ignore.as_ref() {
                if ignore_glob_set.is_match(&new_path) {
                    continue;
                }
            }

            match entry.kind {
                FileType::File => {
                    // Drive all of the file futures in parallel.
                    let future = process_file(new_path)
                        .map_ok(|val| (ProcessResult::File(val), entry.name))
                        .boxed_local();
                    futures.push(future);
                }
                FileType::Directory => {
                    // Drive all of the directory futures in parallel.
                    let future = walk_directory(
                        new_path,
                        ignore,
                        open_dir,
                        process_file,
                        Rc::clone(&strings),
                    )
                    .map_ok(|result| (ProcessResult::Directory(result), entry.name))
                    .boxed_local();
                    futures.push(future);
                }
                FileType::Symlink => (),
            }
        }

        // Close our handle to make sure we free resources as quickly as possible.
        handle.close().await?;

        // Drive all of the child directories in parallel.
        for result in futures::future::join_all(futures).await {
            let (process_result, filename) = result?;
            let name = strings.borrow_mut().get_or_intern(filename);
            let node = match process_result {
                ProcessResult::Directory(recursive_children) => TrieNode::Edge {
                    children: recursive_children,
                    data: (),
                },
                ProcessResult::File(data) => TrieNode::Leaf { data },
            };
            children.insert(name, node);
        }

        Ok(children)
    }
    .boxed_local()
}

pub trait TreeFileMetadata: Clone + Send + 'static {
    type Value: Clone + Send + 'static;

    fn from_parts(stat: FileStat, other: Option<Self::Value>) -> Self;
}

impl TreeFileMetadata for FileStat {
    type Value = ();

    fn from_parts(stat: FileStat, other: Option<()>) -> Self {
        assert!(other.is_none());
        stat
    }
}

impl<T: Clone + Send + 'static> TreeFileMetadata for (FileStat, T) {
    type Value = T;

    fn from_parts(stat: FileStat, other: Option<T>) -> Self {
        let other = other.expect("should always be provided something!");
        (stat, other)
    }
}

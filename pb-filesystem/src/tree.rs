use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::sync::Arc;

use futures::future::BoxFuture;
use futures::FutureExt;
use tokio::sync::Semaphore;

use crate::handle::{DirectoryHandle, DirectoryKind, FileHandle, Handle};
use crate::path::PbPath;
use crate::platform::{FilesystemPlatform, OpenOptions, Platform, PlatformPath, PlatformPathType};
use crate::{FileStat, FileType};

/// Tree description of an object in the filesystem.
#[derive(Debug)]
pub struct MetadataTree {
    /// Where this tree is rooted at.
    root_path: PbPath,
    /// Entries in the tree.
    root_node: TreeNode,
}

impl fmt::Display for MetadataTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: This isn't optimal at all, just threw enough code at this to make it work.
        let tree_node = TreeNodeWithName(self.root_path.inner.clone(), self.root_node.clone());
        let mut buf = Vec::new();
        ptree::write_tree(&tree_node, &mut buf).expect("TODO");
        let buf = String::from_utf8_lossy(&buf[..]);
        write!(f, "{buf}")?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
enum TreeNode {
    File {
        stat: FileStat,
    },
    Directory {
        children: BTreeMap<String, TreeNode>,
    },
}

#[derive(Debug, Clone)]
struct TreeNodeWithName(String, TreeNode);

impl ptree::TreeItem for TreeNodeWithName {
    type Child = TreeNodeWithName;

    fn write_self<W: std::io::Write>(
        &self,
        f: &mut W,
        _style: &ptree::Style,
    ) -> std::io::Result<()> {
        match self.1 {
            TreeNode::File { stat } => write!(f, "{} --- {}B", self.0, stat.size),
            TreeNode::Directory { .. } => write!(f, "{}", self.0),
        }
    }

    fn children(&self) -> Cow<[Self::Child]> {
        match &self.1 {
            TreeNode::File { .. } => Cow::Owned(vec![]),
            TreeNode::Directory { children } => {
                let children: Vec<_> = children
                    .iter()
                    .map(|(name, node)| TreeNodeWithName(name.clone(), node.clone()))
                    .collect();
                Cow::Owned(children)
            }
        }
    }
}

impl<K> Handle<K> {
    /// Get the absolute path that corresponds to this file handle.
    ///
    /// TODO: How does this interact when a single file has multiple hard links?
    async fn fullpath(&self) -> Result<PbPath, crate::Error> {
        let inner = self.to_inner();
        let path = self
            .worker
            .run(move || FilesystemPlatform::fgetpath(inner))
            .await?;
        let path = PbPath::new(path.into_inner())?;
        Ok(path)
    }
}

impl FileHandle {
    /// Recursively walk this handle returning a [`MetadataTree`] that describes
    /// everything underneath it.
    pub async fn tree(&self) -> Result<MetadataTree, crate::Error> {
        let stat = self.stat().await?;
        Ok(MetadataTree {
            root_path: self.fullpath().await?,
            root_node: TreeNode::File { stat },
        })
    }
}

impl DirectoryHandle {
    /// Recursively walk this handle returning a [`MetadataTree`] that describes
    /// everything underneath it.
    ///
    /// TODO: Currently this implementation works on paths. It might be nice
    /// rewrite this to use "openat semantics", but that is tricky from a resource
    /// perspective because it requires keeping many file handles open.
    pub async fn tree(&self) -> Result<MetadataTree, crate::Error> {
        fn process_directory<O, F>(
            path: PbPath,
            open: &O,
        ) -> BoxFuture<'_, Result<BTreeMap<String, TreeNode>, crate::Error>>
        where
            F: Future<Output = Result<DirectoryHandle, crate::Error>> + Send,
            O: Fn(PbPath) -> F + Sync,
        {
            async move {
                tracing::trace!(?path, "processing directory");
                let handle = open(path.clone()).await?;
                let entries = handle.list().await?;
                let mut children = BTreeMap::default();

                for entry in entries {
                    match entry.kind {
                        FileType::File => {
                            let stat = handle.fstatat(entry.name.inner.clone()).await?;
                            children.insert(entry.name.inner, TreeNode::File { stat });
                        }
                        FileType::Directory => {
                            let new_path = format!("{}/{}", &path.inner, &entry.name.inner);
                            let new_path = PbPath::new(new_path).expect("known valid");
                            let recursive_children = process_directory(new_path, open).await?;
                            let node = TreeNode::Directory {
                                children: recursive_children,
                            };
                            children.insert(entry.name.inner, node);
                        }
                        FileType::Symlink => (),
                    }
                }

                Ok(children)
            }
            .boxed()
        }

        let open_dir = |path: PbPath| {
            let worker_ = self.worker.clone();
            let drops_tx_ = self.drops_tx.clone();
            let permits_ = Arc::clone(&self.kind.permits);

            async move {
                let path = PlatformPathType::try_new(path.inner).expect("known valid");
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

        let start_path = self.fullpath().await?;
        let children = process_directory(start_path.clone(), &open_dir).await?;

        Ok(MetadataTree {
            root_path: start_path,
            root_node: TreeNode::Directory { children },
        })
    }
}

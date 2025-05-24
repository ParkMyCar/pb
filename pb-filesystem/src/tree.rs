use std::collections::{BTreeMap, VecDeque};

use crate::handle::{DirectoryHandle, FileHandle, Handle};
use crate::path::{PbFilename, PbPath};
use crate::platform::{FilesystemPlatform, Platform};

/// Tree description of an object in the filesystem.
#[derive(Debug)]
pub struct MetadataTree {
    /// Where this tree is rooted at.
    root_path: PbPath,
    /// Entries in the tree.
    root_node: TreeNode,
}

#[derive(Debug)]
enum TreeNode {
    File,
    Directory {
        children: BTreeMap<String, TreeNode>,
    },
}

impl<K> Handle<K> {
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
        Ok(MetadataTree {
            root_path: self.fullpath().await?,
            root_node: TreeNode::File,
        })
    }
}

impl DirectoryHandle {
    /// Recursively walk this handle returning a [`MetadataTree`] that describes
    /// everything underneath it.
    pub async fn tree(&self) -> Result<MetadataTree, crate::Error> {
        todo!()
    }
}

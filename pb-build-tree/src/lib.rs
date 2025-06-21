use std::{
    collections::BTreeMap,
    fmt::Display,
    path::{Path, PathBuf},
};

use compact_str::CompactString;
use pb_ore::{assert_none, id_gen::Gen};
use pb_trie::TrieMap;
use pb_types::{BuildTarget, BuildTargetPath, FileMetadataXx64, InternedPath, SourceDependency};
use smallvec::SmallVec;

#[derive(Debug)]
pub struct BuildTree {
    /// Locations of all the files in our workspace.
    file_locations: TrieMap<InternedPath, (), FileId>,
    /// Map of [`FileId`] to [`FileNode`].
    files: BTreeMap<FileId, FileNode>,

    /// Relative path were a glob is defined.
    glob_locations: TrieMap<InternedPath, (), GlobId>,
    /// Map of [`GlobId`] to [`GlobNode`].
    globs: BTreeMap<GlobId, GlobNode>,

    /// Map of [`DynamicSourcesId`] to [`DynamicSourcesNode`].
    dynamic_sources: BTreeMap<DynamicSourcesId, DynamicSourcesNode>,

    /// Locations of all our build targets.
    build_target_locations: TrieMap<InternedPath, (), BuildTargetId>,
    /// Map of [`BuildTargetId`] to [`BuildTarget`].
    build_targets: BTreeMap<BuildTargetId, BuildTargetNode>,

    /// String interner.
    strings: lasso::Rodeo,
    /// ID generator for all the nodes in our build tree.
    id_gen: Gen<u64>,
}

impl BuildTree {
    /// Create a new [`BuildTree`].
    pub fn new() -> Self {
        BuildTree {
            file_locations: TrieMap::new(),
            files: BTreeMap::default(),
            glob_locations: TrieMap::new(),
            globs: BTreeMap::default(),
            dynamic_sources: BTreeMap::default(),
            build_target_locations: TrieMap::new(),
            build_targets: BTreeMap::default(),
            strings: lasso::Rodeo::new(),
            id_gen: Gen::default(),
        }
    }

    /// Insert the provided [`FileMetadataXx64`] into the tree.
    ///
    /// # Errors
    ///
    /// * If the provided path contains a non-directory that is not the final component.
    pub fn insert_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        metadata: FileMetadataXx64,
    ) -> Result<(), anyhow::Error> {
        // Insert this target.
        let id = self.gen_file_id();
        let node = FileNode {
            metadata,
            build_dependents: SmallVec::new(),
        };
        let prev = self.files.insert(id, node);
        assert_none!(prev);

        // Add the path mapping.
        let path = self.intern_file_path(path);
        self.file_locations.insert_leaf(path, id)?;

        Ok(())
    }

    ///
    pub fn update_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        file: FileMetadataXx64,
    ) -> Result<impl Iterator<Item = BuildTargetId>, anyhow::Error> {
        let node = self
            .lookup_file_path(path)
            .and_then(|path| self.file_locations.get_leaf(path))
            .and_then(|id| self.files.get_mut(id))
            .ok_or_else(|| anyhow::anyhow!("file does not exist"))?;

        // Update the metadata.
        node.metadata = file;
        // Return all of the build targets that depend on this file.
        Ok(node.build_dependents.iter().copied())
    }

    /// Get the [`FileMetadataXx64`] associated with the provided path, if it exists.
    pub fn get_file(&self, path: &PathBuf) -> Option<&FileMetadataXx64> {
        let path = self.lookup_file_path(path)?;
        self.file_locations
            .get_leaf(path)
            .and_then(|id| self.files.get(id))
            .map(|node| &node.metadata)
    }

    /// Insert a new [`BuildTarget`] into our [`BuildTree`].
    pub fn insert_build_target(
        &mut self,
        path: &BuildTargetPath,
        target: BuildTarget,
    ) -> Result<(), anyhow::Error> {
        // Lookup our dependencies.
        let source_deps: Vec<_> = target
            .source_deps
            .into_iter()
            .map(|dep| {
                let dep = match dep {
                    SourceDependency::File(path) => self
                        .lookup_file_path(&path)
                        .and_then(|path| self.file_locations.get_leaf(path).copied())
                        .map(|file_id| SourceDependencyId::File(file_id))
                        .ok_or_else(|| anyhow::anyhow!("depends on non-existent file {path:?}"))?,
                    SourceDependency::Glob(glob) => todo!(),
                    SourceDependency::Rule(rule) => self
                        .lookup_build_path(&rule)
                        .and_then(|path| self.build_target_locations.get_leaf(path).copied())
                        .map(|rule_id| SourceDependencyId::Rule(rule_id))
                        .ok_or_else(|| {
                            anyhow::anyhow!("depends on non-existent target {rule:?}")
                        })?,
                };
                Ok::<_, anyhow::Error>(dep)
            })
            .collect::<Result<_, _>>()?;
        let build_deps = target
            .build_deps
            .into_iter()
            .map(|path| {
                let dep = self
                    .lookup_build_path(&path)
                    .and_then(|path| self.build_target_locations.get_leaf(path).copied())
                    .ok_or_else(|| anyhow::anyhow!("depends on non-existent target {path:?}"))?;
                Ok::<_, anyhow::Error>(dep)
            })
            .collect::<Result<_, _>>()?;

        let id = self.gen_build_target_id();
        // Update our source dependencies so we know what build rules depend on them.
        for source_dep in &source_deps {
            match source_dep {
                SourceDependencyId::File(file_dep) => {
                    let file = self.files.get_mut(&file_dep).expect("file should exist");
                    file.build_dependents.push(id);
                }
                SourceDependencyId::Glob(glob_dep) => todo!(),
                SourceDependencyId::Rule(rule_dep) => todo!(),
            }
        }

        // Create the node from the provided build target.
        let rule = self.strings.get_or_intern(&target.rule);
        let tree_path = self.intern_build_path(path);
        let node = BuildTargetNode {
            name: path.name.clone(),
            rule,
            source_deps,
            build_deps,
            path: tree_path.clone(),
        };

        // Insert this node.
        let prev = self.build_targets.insert(id, node);
        assert_none!(prev);

        // Add the path mapping.
        self.build_target_locations.insert_leaf(tree_path, id)?;

        Ok(())
    }

    /// Return a pretty version of the file tree that can be displayed.
    pub fn pretty_file_tree(&self) -> impl Display {
        self.file_locations
            .pretty(|f, name| f.write_all(self.strings.resolve(name).as_bytes()))
    }

    /// Intern a [`PathBuf`].
    fn intern_file_path<P: AsRef<Path>>(&mut self, path: P) -> InternedPath {
        let path = path.as_ref();
        let mut components = SmallVec::default();

        // Add the relative path.
        for component in path.components() {
            let s = component.as_os_str().to_str().expect("non UTF-8 path");
            let component = self.strings.get_or_intern(s);
            components.push(component);
        }

        InternedPath(components)
    }

    /// Get the [`InternedPath`] for this [`PathBuf`], if one exists.
    fn lookup_file_path<P: AsRef<Path>>(&self, path: P) -> Option<InternedPath> {
        let path = path.as_ref();
        let mut components = SmallVec::default();

        // Add the relative path.
        for component in path.components() {
            let s = component.as_os_str().to_str().expect("non UTF-8 path");
            let component = self.strings.get(s)?;
            components.push(component);
        }

        Some(InternedPath(components))
    }

    /// Construct a [`PathBuf`] from the provided [`InternedPath`];
    fn resolve_file_path(&self, path: &InternedPath) -> PathBuf {
        let mut pathbuf = PathBuf::new();
        for component in &path.0 {
            pathbuf.push(self.strings.resolve(component));
        }
        pathbuf
    }

    /// Intern a [`BuildTargetPath`].
    fn intern_build_path(&mut self, path: &BuildTargetPath) -> InternedPath {
        let mut components = SmallVec::default();

        // Add the repository.
        let repository = self.strings.get_or_intern(&path.repository);
        components.push(repository);

        // Add the relative path.
        let parent = self.intern_file_path(&path.parents);
        components.extend_from_slice(&parent.0[..]);

        // Add the target name.
        let name = self.strings.get_or_intern(&path.name);
        components.push(name);

        InternedPath(components)
    }

    /// Construct a [`BuildTargetPath`] from the provided [`InternedPath`];
    fn resolve_build_path(&self, path: &InternedPath) -> BuildTargetPath {
        let repository = self.strings.resolve(&path.0[0]);
        let mut parents = PathBuf::new();
        for component in &path.0[1..path.0.len() - 1] {
            parents.push(self.strings.resolve(component));
        }
        let name = self.strings.resolve(&path.0[path.0.len() - 1]);

        BuildTargetPath {
            repository: CompactString::new(repository),
            parents,
            name: CompactString::new(name),
        }
    }

    /// Get the [`InternedPath`] for this [`BuildTargetPath`], if one exists.
    fn lookup_build_path(&self, path: &BuildTargetPath) -> Option<InternedPath> {
        let mut components = SmallVec::default();

        // Add the repository.
        let repository = self.strings.get(&path.repository)?;
        components.push(repository);

        // Add the relative path.
        let parent = self.lookup_file_path(&path.parents)?;
        components.extend_from_slice(&parent.0[..]);

        // Add the target name.
        let name = self.strings.get(&path.name)?;
        components.push(name);

        Some(InternedPath(components))
    }

    fn gen_file_id(&mut self) -> FileId {
        FileId(self.id_gen.next())
    }

    fn gen_build_target_id(&mut self) -> BuildTargetId {
        BuildTargetId(self.id_gen.next())
    }
}

/// A single build target within the tree.
///
/// Externally we interface with [`BuildTarget`]s, but in a [`BuildTree`] we store this type.
#[derive(Clone, Debug)]
struct BuildTargetNode {
    /// Name of the build target.
    name: CompactString,
    /// Name of the rule associated with this target.
    rule: lasso::Spur,

    /// Other dependencies in our build graph that this target depends on.
    build_deps: Vec<BuildTargetId>,
    /// Source files that this build target directly depends on.
    source_deps: Vec<SourceDependencyId>,

    /// The path this node is located at.
    path: InternedPath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SourceDependencyId {
    File(FileId),
    Glob(GlobId),
    Rule(BuildTargetId),
}

/// A single file within the tree.
#[derive(Clone, Debug)]
struct FileNode {
    /// Metadata for this file.
    metadata: FileMetadataXx64,
    /// The [`BuildTarget`]s that depend on this file.
    build_dependents: SmallVec<[BuildTargetId; 2]>,
}

/// ID for a file in our graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileId(u64);

impl From<u64> for FileId {
    fn from(value: u64) -> Self {
        FileId(value)
    }
}

/// A glob in the tree that needs to be tracked.
#[derive(Clone, Debug)]
struct GlobNode {
    /// The pattern for this glob.
    pattern: CompactString,
    /// Compiled glob, stored as a performance optimization.
    globset: globset::GlobSet,
    /// The [`BuildTarget`]s that depend on this glob.
    build_dependents: SmallVec<[BuildTargetId; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlobId(u64);

impl From<u64> for GlobId {
    fn from(value: u64) -> Self {
        GlobId(value)
    }
}

/// Outputs from a build rule which power the _sources_ of another.
#[derive(Clone, Debug)]
struct DynamicSourcesNode {
    /// The files we resolved the last time the build target was run.
    files: Option<Box<[FileId]>>,
    /// Build target responsible for determining these sources.
    build_target: BuildTargetId,
}

/// ID for a [`DynamicSourcesNode`] in our graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DynamicSourcesId(u64);

impl From<u64> for DynamicSourcesId {
    fn from(value: u64) -> Self {
        DynamicSourcesId(value)
    }
}

/// ID for a [`BuildTarget`] in our graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuildTargetId(u64);

impl From<u64> for BuildTargetId {
    fn from(value: u64) -> Self {
        BuildTargetId(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoketest_add_files() {
        let mut build_tree = BuildTree::new();
        let mut rng = rand::rng();

        build_tree
            .insert_file(
                "library_a/srcs/lib.rs",
                FileMetadataXx64::test_rand(&mut rng),
            )
            .unwrap();
        build_tree
            .insert_file(
                "library_b/srcs/lib.rs",
                FileMetadataXx64::test_rand(&mut rng),
            )
            .unwrap();

        // let lib_a_srcs_glob = BuildTarget {
        //     rule: "std.glob".into(),
        //     build_deps: Vec::default(),
        //     // file_deps:
        // };

        // build_tree.insert_build_target(path, target)

        println!("{}", build_tree.pretty_file_tree());
    }
}

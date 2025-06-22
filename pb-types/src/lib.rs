//! Types used throughout `pb`.
//!
//! The goal of this crate is to be very lightweight, so take care with adding dependencies.

use std::path::PathBuf;

use compact_str::CompactString;
use smallvec::SmallVec;

/// Metadata we track for a file to determine when it's changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata<T> {
    /// Size of the file in bytes.
    pub size: u64,
    /// Last modified time of the file.
    pub mtime: Timespec,
    /// Inode of the file.
    pub inode: u64,
    /// File mode/permissions.
    pub mode: u32,
    /// Fingerprint of the file contents, generally a hash.
    pub fingerprint: T,
}

pub type FileMetadataXx64 = FileMetadata<Xxh64Hash>;
pub type FileMetadataXx128 = FileMetadata<Xxh128Hash>;

impl FileMetadataXx64 {
    pub fn test_rand(rng: &mut impl rand::Rng) -> Self {
        FileMetadata {
            size: rng.random(),
            mtime: Timespec::test_rand(rng),
            inode: rng.random(),
            mode: rng.random(),
            fingerprint: Xxh64Hash(rng.random()),
        }
    }
}

/// Hash from xxh64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Xxh64Hash(u64);

impl Xxh64Hash {
    pub fn new(val: u64) -> Self {
        Xxh64Hash(val)
    }
}

/// Hash from xxh128.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Xxh128Hash(u128);

impl Xxh128Hash {
    pub fn new(val: u128) -> Self {
        Xxh128Hash(val)
    }
}

/// Time info returned from a `stat` call.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Timespec {
    /// Seconds.
    pub secs: i64,
    /// Nanoseconds.
    ///
    /// Not all filesystems provide this, thus often it will be 0.
    pub nanos: i64,
}

impl Timespec {
    /// Create a [`Timespec`] from the number of milliseconds since the epoch.
    pub fn from_epoch_millis(millis: u64) -> Self {
        let secs = millis / 1000;
        let nanos = (millis % 1000) * 10u64.pow(6);

        Timespec {
            secs: secs.try_into().expect("overlowed timespec"),
            nanos: nanos.try_into().expect("overlowed timespec"),
        }
    }

    pub fn test_rand(rng: &mut impl rand::Rng) -> Self {
        Timespec {
            secs: rng.random(),
            nanos: rng.random(),
        }
    }
}

/// Location of a [`BuildTarget`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildTargetPath {
    /// The repository we're located in. `None` indicates the root workspace.
    pub repository: CompactString,
    /// Path to the directory containing the manifest file.
    pub parents: PathBuf,
    /// Name of the target.
    pub name: CompactString,
}

/// A single target in our build graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildTarget {
    /// Name of the rule this target uses.
    pub rule: CompactString,

    /// Dependencies on other build targets.
    pub build_deps: Vec<BuildTargetPath>,
    /// Dependencies on source files.
    pub source_deps: Vec<SourceDependency>,
}

/// Types of source file dependencies that a [`BuildTarget`] can have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceDependency {
    /// A single file.
    File(PathBuf),
    /// A glob of files.
    Glob(CompactString),
    /// Outputs of another build rule.
    Rule(BuildTargetPath),
}

/// A path whose components are in a [`lasso::Rodeo`].
#[derive(Clone, Debug)]
pub struct InternedPath(pub SmallVec<[InternedComponent; 8]>);

/// A single component within an [`InternedPath`].
pub type InternedComponent = lasso::Spur;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum BuildKey {
    /// User-defined target (from BUILD.pb or manifest).
    Target {
        /// Where it's defined (e.g., "//src/lib/BUILD.pb").
        location: PathBuf,
        /// Target name within that file.
        name: String,
        /// Build configuration.
        config: CompileConfig,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CompileConfig {
    /// Name of the compiler.
    compiler: CompactString,
    /// Version of the compiler.
    version: semver::Version,
    /// Target we're building for.
    target_triple: target_lexicon::Triple,
    /// Optimization level we're compiling for.
    opt_level: OptimizationLevel,
    /// Compilation flags that effect the output.
    flags: Vec<CompactString>,
}

/// Represents compiler optimization levels across different compilers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptimizationLevel {
    /// No optimization - fastest compilation, slowest execution, best debugging.
    None,
    /// Basic optimization - minimal performance improvements.
    Basic,
    /// Standard optimization - good balance of performance and compilation time.
    Standard,
    /// Apply All optimizations - maximum performance, slower compilation.
    All,
    /// Optimize for binary size over speed.
    Size,
    /// Aggressively optimize for smallest binary size.
    MinSize,

    /// Maximum performance, may break standards compliance.
    MaxPerformence,
    /// Debug-friendly optimization - some performance with preserved debugging.
    Debug,
}

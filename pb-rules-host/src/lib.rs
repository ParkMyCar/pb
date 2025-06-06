//! Host implementations for the `pb` rules WASM sandbox.
//!
//! `pb` defines the interface for rules with WASM Interface Types in the
//! [`pb-wit`](https://github.com/ParkMyCar/pb-wit) repository. As part of the
//! interface we define what functionality the "host" provides to the
//! sandbox, like filesystem and network access.
//!
//! This crate contains the host implementations for our WIT interfaces.

use pb_cfg::ConfigSet;
use pb_filesystem::locations::{repositories::RepositoryDirectory, scratch::ScratchDirectory};
use wasmtime::component::ResourceTable;

use crate::wit::pb::rules::context::WriteClient;

pub mod wit {
    wasmtime::component::bindgen!({
        path: "pb-wit/wit",
        with: {
            "pb:rules/read-filesystem@0.1.0/file": crate::filesystem::FileHandle,
            "pb:rules/write-filesystem@0.1.0/write-client": crate::filesystem::WriteClient,
            "pb:rules/write-filesystem@0.1.0/write-file": crate::filesystem::WriteFileHandle,
            "pb:rules/write-filesystem@0.1.0/create-file-future": crate::filesystem::CreateFileFuture,
            "pb:rules/write-filesystem@0.1.0/write-directory": crate::filesystem::WriteDirectoryHandle,
            "pb:rules/write-filesystem@0.1.0/create-directory-future": crate::filesystem::CreateDirectoryFuture,
            "pb:rules/types@0.1.0/failable-future": crate::types::FailableFuture,
            "pb:rules/types@0.1.0/bytes-stream": crate::types::BytesStream,
            "pb:rules/types@0.1.0/waker": crate::types::HostWaker,
            "pb:rules/types@0.1.0/provider-dict": crate::types::Provider,
            "pb:rules/http@0.1.0/client": crate::http::Client,
            "pb:rules/http@0.1.0/response": crate::http::Response,
            "pb:rules/http@0.1.0/response-future": crate::http::ResponseFuture,
            "pb:rules/context@0.1.0/ctx": crate::context::Context,
            "pb:rules/context@0.1.0/actions": crate::context::Actions,
        }
    });
}

pub mod context;
pub mod filesystem;
pub mod http;
pub mod logger;
pub mod types;

pub struct HostState {
    /// Interface for making HTTP requests.
    pub(crate) http_client: reqwest::Client,
    /// Interface for the underlying filesystem.
    pub(crate) filesystem: pb_filesystem::filesystem::Filesystem,

    /// Scratch directory that we can create files and directories in.
    pub(crate) scratch_space: pb_filesystem::locations::scratch::ScratchDirectory,
    /// Directory for externally downloaded repositories.
    pub(crate) repositories: pb_filesystem::locations::repositories::RepositoryDirectory,
    /// TODO: Is this needed?
    pub(crate) write_filesystem: crate::filesystem::WriteClient,

    /// Format for logs emitted from WebAssembly.
    pub(crate) logging_format: crate::logger::LoggingFormat,

    /// Resources handed to WASM.
    pub resources: ResourceTable,
}

impl Clone for HostState {
    fn clone(&self) -> Self {
        HostState {
            http_client: self.http_client.clone(),
            filesystem: self.filesystem.clone(),
            scratch_space: self.scratch_space.clone(),
            repositories: self.repositories.clone(),
            write_filesystem: self.write_filesystem.clone(),
            logging_format: self.logging_format.clone(),
            resources: ResourceTable::new(),
        }
    }
}

impl HostState {
    pub async fn new(
        _configs: &ConfigSet,
        http_client: reqwest::Client,
        filesystem: pb_filesystem::filesystem::Filesystem,
        scratch_space: ScratchDirectory,
        repositories: RepositoryDirectory,
    ) -> Result<Self, anyhow::Error> {
        let logging_format = crate::logger::LoggingFormat::from_env();

        Ok(HostState {
            http_client,
            filesystem,
            scratch_space,
            repositories,
            write_filesystem: WriteClient::default(),
            logging_format,
            resources: ResourceTable::new(),
        })
    }

    pub fn add_to_linker<T, U>(
        linker: &mut wasmtime::component::Linker<T>,
        get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    ) -> wasmtime::Result<()>
    where
        U: wit::pb::rules::logging::Host
            + wit::pb::rules::read_filesystem::Host
            + wit::pb::rules::write_filesystem::Host
            + wit::pb::rules::types::Host
            + wit::pb::rules::context::Host
            + wit::pb::rules::http::Host,
    {
        wit::pb::rules::logging::add_to_linker(linker, get)?;
        wit::pb::rules::read_filesystem::add_to_linker(linker, get)?;
        wit::pb::rules::types::add_to_linker(linker, get)?;
        wit::pb::rules::http::add_to_linker(linker, get)?;
        wit::pb::rules::context::add_to_linker(linker, get)?;
        wit::pb::rules::write_filesystem::add_to_linker(linker, get)?;
        Ok(())
    }

    pub fn context(
        &mut self,
        rule_set: &str,
        rule_name: &str,
        rule_version: &str,
        target_name: &str,
    ) -> wasmtime::component::Resource<crate::context::Context> {
        let context = crate::context::Context::new(rule_set, rule_name, rule_version, target_name);
        self.resources.push(context).unwrap()
    }
}

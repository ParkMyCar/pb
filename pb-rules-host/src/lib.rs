//! Host implementations for the `pb` rules WASM sandbox.
//!
//! `pb` defines the interface for rules with WASM Interface Types in the
//! [`pb-wit`](https://github.com/ParkMyCar/pb-wit) repository. As part of the
//! interface we define what functionality the "host" provides to the
//! sandbox, like filesystem and network access.
//!
//! This crate contains the host implementations for our WIT interfaces.

use wasmtime::component::ResourceTable;

use crate::wit::pb::rules::context::WriteClient;

pub mod wit {
    wasmtime::component::bindgen!({
        path: "pb-wit/wit",
        with: {
            "pb:rules/read-filesystem@0.1.0/file": crate::filesystem::FileHandle,
            "pb:rules/write-filesystem@0.1.0/write-client": crate::filesystem::WriteClient,
            "pb:rules/write-filesystem@0.1.0/write-file": crate::filesystem::FileHandle,
            "pb:rules/types@0.1.0/bytes-stream": crate::types::BytesStream,
            "pb:rules/types@0.1.0/bytes-sink": crate::types::BytesSink,
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
    pub(crate) http_client: reqwest::Client,
    pub(crate) filesystem: pb_filesystem::filesystem::Filesystem,

    pub(crate) write_filesystem: crate::filesystem::WriteClient,

    /// Resources handed to WASM.
    pub resources: ResourceTable,
}

impl HostState {
    pub fn new(handle: tokio::runtime::Handle) -> Self {
        let filesystem = pb_filesystem::filesystem::Filesystem::new_tokio(handle, 128);

        HostState {
            http_client: reqwest::Client::default(),
            filesystem,
            write_filesystem: WriteClient::default(),
            resources: ResourceTable::new(),
        }
    }

    pub fn add_to_linker<T, U>(
        linker: &mut wasmtime::component::Linker<T>,
        get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    ) -> wasmtime::Result<()>
    where
        U: wit::pb::rules::logging::Host
            + wit::pb::rules::read_filesystem::Host
            + wit::pb::rules::types::Host
            + wit::pb::rules::context::Host
            + wit::pb::rules::http::Host,
    {
        wit::pb::rules::logging::add_to_linker(linker, get)?;
        wit::pb::rules::read_filesystem::add_to_linker(linker, get)?;
        wit::pb::rules::types::add_to_linker(linker, get)?;
        wit::pb::rules::http::add_to_linker(linker, get)?;
        wit::pb::rules::context::add_to_linker(linker, get)?;
        Ok(())
    }

    pub fn context(&mut self) -> wasmtime::component::Resource<crate::context::Context> {
        let context = crate::context::Context::default();
        self.resources.push(context).unwrap()
    }
}

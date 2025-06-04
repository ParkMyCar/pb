//! The main event loop for the `pb` build system.

use std::io::Read;
use std::path::PathBuf;

use derivative::Derivative;
use futures::FutureExt;
use pb_cfg::ConfigSet;
use pb_filesystem::locations::repositories::RepositoryDirectory;
use pb_filesystem::path::PbPath;
use pb_filesystem::{filesystem::Filesystem, locations::scratch::ScratchDirectory};
use pb_rules_host::HostState;

use crate::defs::{WorkspaceSpec, WORKSPACE_FILENAME};
use crate::rules::StdRules;

/// Name of the 'std' rule set.
static STD_RULES_NAME: &str = "std";

/// Configuration for creating a [`Engine`].
pub struct EngineConfig {
    /// Root directory for `pb` metadata.
    pub pb_root_dir: PbPath,
    /// Root directory of the workspace, where the user's files live.
    pub workspace_dir: PbPath,
    /// Dynamic configs for the build system.
    pub configs: ConfigSet,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Engine {
    /// Root directory for `pb` metadata.
    pb_root_dir: PbPath,
    /// Root directory of the workspace, where the user's files live.
    workspace_dir: PbPath,
    /// Specification for the rules, repositories, toolchains, and more for this workspace.
    spec: WorkspaceSpec,

    /// Client for making HTTP requests.
    http_client: reqwest::Client,
    /// Our interface to the filesystem.
    #[derivative(Debug = "ignore")]
    filesystem: Filesystem,
    /// A "scratch" directory that we download and create files in.
    #[derivative(Debug = "ignore")]
    scratch_dir: ScratchDirectory,
    /// The location of all of our externally downloaded repositories.
    #[derivative(Debug = "ignore")]
    repositories_dir: RepositoryDirectory,

    /// Dynamic configs for the build system.
    configs: ConfigSet,

    /// The WASM engine for executing rules.
    pub wasm_engine: wasmtime::Engine,
    /// WASM linker used to instantiate components.
    #[derivative(Debug = "ignore")]
    pub wasm_linker: wasmtime::component::Linker<HostState>,
    /// State that gets provided to WASM guest functions.
    #[derivative(Debug = "ignore")]
    pub host_state: HostState,
}

impl Engine {
    pub async fn new(config: EngineConfig) -> Result<Self, anyhow::Error> {
        let EngineConfig {
            workspace_dir,
            pb_root_dir,
            configs,
        } = config;

        let http_client = reqwest::Client::new();
        let filesystem = Filesystem::new(4, 1024);

        let spec = {
            let filename = WORKSPACE_FILENAME.read(&configs);
            let path = PathBuf::from(workspace_dir.inner.clone()).join(filename);
            tracing::info!(?path, "reading Workspace spec");
            let mut file = std::fs::File::open(path)?;

            let mut buffer = String::new();
            file.read_to_string(&mut buffer)?;

            WorkspaceSpec::from_toml(&buffer)?
        };

        let wasm_engine = {
            // Modules can be compiled through either the text or binary format
            let mut config = wasmtime::Config::new();
            config.wasm_component_model(true).wasm_multi_memory(true);
            tracing::info!(?config, "initializing WASM engine");
            let engine = wasmtime::Engine::new(&config)?;
            engine
        };
        let wasm_linker = {
            let mut linker = wasmtime::component::Linker::new(&wasm_engine);
            pb_rules_host::HostState::add_to_linker(
                &mut linker,
                |state: &mut pb_rules_host::HostState| state,
            )?;
            linker
        };

        let scratch_dir_fut =
            ScratchDirectory::new(pb_root_dir.clone(), filesystem.clone()).boxed();
        let repositories_dir_fut =
            RepositoryDirectory::new(pb_root_dir.clone(), filesystem.clone()).boxed();

        let (scratch_dir, repositories_dir) = futures::join!(scratch_dir_fut, repositories_dir_fut);
        let scratch_dir = scratch_dir?;
        let repositories_dir = repositories_dir?;

        // Create the host state required for running WASM guest functions.
        let host_state = HostState::new(
            &configs,
            http_client.clone(),
            filesystem.clone(),
            scratch_dir.clone(),
            repositories_dir.clone(),
        )
        .await?;

        Ok(Engine {
            pb_root_dir,
            workspace_dir,
            spec,
            configs,
            http_client,
            filesystem,
            scratch_dir,
            repositories_dir,
            wasm_engine,
            wasm_linker,
            host_state,
        })
    }

    pub async fn load_rules(&self) -> Result<StdRules, anyhow::Error> {
        // First we load the `std` rules so we have a way to make HTTP requests.
        let Some(std_rules_spec) = self.spec.rules.get(STD_RULES_NAME) else {
            anyhow::bail!("std rules not found");
        };
        let std_rules = StdRules::try_load(
            std_rules_spec,
            &self.wasm_linker,
            &self.wasm_engine,
            &self.host_state,
        )?;

        Ok(std_rules)
    }
}

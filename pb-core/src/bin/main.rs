use std::io::Read;

use pb_cfg::ConfigSet;
use pb_filesystem::path::PbPath;
use pb_rules_host::{wit::pb::rules::types::Attribute, HostState};
use tracing::Level;
use tracing_subscriber::EnvFilter;
use wasmtime::*;

#[tokio::main(flavor = "current_thread")]
async fn main2() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 2);

    let workspace_root = &args[1];
    tracing::info!(?workspace_root, "starting at");

    // Register all of our dynamic configs.
    let mut configs = ConfigSet::builder();
    pb_core::cfgs::all_cfgs(&mut configs);
    let configs = configs.build();

    // Create our Workspace instance.
    let engine_config = pb_core::EngineConfig {
        pb_root_dir: PbPath::new("/Users/parker/.pb".to_string()).unwrap(),
        workspace_dir: PbPath::new(workspace_root.to_string()).unwrap(),
        configs,
    };
    let engine = pb_core::Engine::new(engine_config).await?;
    let std_rules = engine.load_rules().await?;

    let result = std_rules.http_repository(
        &engine.wasm_engine,
        &engine.host_state,
        "darwin_aarch64".to_string(),
        "https://github.com/MaterializeInc/toolchains/releases/download/clang-19.1.6-2/darwin_aarch64.tar.zst".to_string(),
    ).await;

    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    let mut contents = String::new();
    let mut file = std::fs::File::open("/Users/parker/Development/pb/pb/misc/examples/pb.toml")?;
    file.read_to_string(&mut contents)?;

    let value: toml::Value = toml::from_str(&contents)?;
    println!("{value:?}");

    Ok(())
}

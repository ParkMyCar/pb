use std::collections::BTreeMap;

use pb_cfg::Config;
use serde::Deserialize;

pub static WORKSPACE_FILENAME: Config<&'static str> = Config::new(
    "workspace_filename",
    "The filename for what defines the root of the workspace.",
    "WORKSPACE.pb.toml",
);

/// Definition of [`Workspace`], parsed from a [`WORKSPACE_FILENAME`].
///
/// [`Workspace`]: crate::Workspace
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceSpec {
    /// The rules imported into this workspace.
    pub rules: BTreeMap<String, RuleSpec>,
}

impl WorkspaceSpec {
    pub fn from_toml(raw: &str) -> Result<Self, anyhow::Error> {
        let workspace = toml::from_str(raw)?;
        Ok(workspace)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RuleSpec {
    Version(String),
    Remote {
        url: String,
        integrity: Option<String>,
        hash: Option<String>,
        algo: Option<String>,
    },
    Local {
        path: String,
    },
}

use std::sync::Arc;

use crate::wit::pb::rules as wit;
use crate::HostState;

impl crate::wit::pb::rules::context::Host for HostState {}

pub struct Context {
    rule_set: Arc<str>,
    rule_name: Arc<str>,
    rule_version: Arc<str>,
    target_name: Arc<str>,
}

impl Context {
    pub fn new(rule_set: &str, rule_name: &str, rule_version: &str, target_name: &str) -> Self {
        Context {
            rule_set: rule_set.into(),
            rule_name: rule_name.into(),
            rule_version: rule_version.into(),
            target_name: target_name.into(),
        }
    }
}

impl crate::wit::pb::rules::context::HostCtx for HostState {
    fn actions(
        &mut self,
        self_: wasmtime::component::Resource<wit::context::Ctx>,
    ) -> wasmtime::component::Resource<wit::context::Actions> {
        self.resources.push(Actions::new(&self)).unwrap()
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<crate::wit::pb::rules::context::Ctx>,
    ) -> wasmtime::Result<()> {
        self.resources.delete(rep).unwrap();
        Ok(())
    }
}

#[derive(Default)]
pub struct Actions {
    client: reqwest::Client,
    write_filesystem: crate::filesystem::WriteClient,
}

impl Actions {
    fn new(state: &HostState) -> Self {
        Actions {
            client: state.http_client.clone(),
            write_filesystem: state.write_filesystem.clone(),
        }
    }
}

impl wit::context::HostActions for HostState {
    fn http(
        &mut self,
        self_: wasmtime::component::Resource<Actions>,
    ) -> wasmtime::component::Resource<wit::context::Client> {
        // TODO: The client should be a child of the actions.
        let actions = self.resources.get(&self_).unwrap();
        let client = crate::http::Client {
            inner: actions.client.clone(),
        };
        self.resources.push(client).unwrap()
    }

    fn write_filesystem(
        &mut self,
        self_: wasmtime::component::Resource<Actions>,
    ) -> wasmtime::component::Resource<wit::context::WriteClient> {
        let actions = self.resources.get(&self_).unwrap();
        let client = actions.write_filesystem.clone();
        self.resources.push(client).unwrap()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Actions>) -> wasmtime::Result<()> {
        self.resources.delete(rep).unwrap();
        Ok(())
    }
}

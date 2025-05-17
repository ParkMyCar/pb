use crate::wit::pb::rules as wit;
use crate::HostState;

impl crate::wit::pb::rules::context::Host for HostState {}

#[derive(Default)]
pub struct Context;

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
}

impl Actions {
    fn new(state: &HostState) -> Self {
        Actions {
            client: state.http_client.clone(),
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

    fn drop(&mut self, rep: wasmtime::component::Resource<Actions>) -> wasmtime::Result<()> {
        self.resources.delete(rep).unwrap();
        Ok(())
    }
}

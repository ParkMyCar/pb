use filesystem::FileNamespace;
use logger::Logger;

pub mod wit {
    wasmtime::component::bindgen!({
        path: "pb-wit/wit",
        with: {
            "pb:rules/read-filesystem@0.1.0/file": crate::filesystem::FileHandle,
            "pb:rules/types@0.1.0/waker": crate::MyWaker,
            "pb:rules/types@0.1.0/provider-dict": crate::MyProvider,
        }
    });
}

pub mod filesystem;
pub mod logger;

#[derive(Default)]
pub struct HostStuff {
    logger: Logger,
    filesystem: FileNamespace,
    waker: MyWaker,
    provider: MyProvider,
}

impl HostStuff {
    pub fn add_to_linker<T, U>(
        linker: &mut wasmtime::component::Linker<T>,
        get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    ) -> wasmtime::Result<()>
    where
        U: wit::pb::rules::logging::Host
            + wit::pb::rules::read_filesystem::Host
            + wit::pb::rules::types::Host,
    {
        wit::pb::rules::logging::add_to_linker(linker, get)?;
        wit::pb::rules::read_filesystem::add_to_linker(linker, get)?;
        wit::pb::rules::types::add_to_linker(linker, get)?;
        Ok(())
    }
}

impl wit::pb::rules::logging::Host for HostStuff {
    fn event(
        &mut self,
        level: wit::pb::rules::logging::Level,
        message: wasmtime::component::__internal::String,
        location: wit::pb::rules::logging::Location,
        fields: wasmtime::component::__internal::Vec<wit::pb::rules::logging::Field>,
    ) -> () {
        self.logger.event(level, message, location, fields)
    }
}

impl wit::pb::rules::read_filesystem::HostFile for HostStuff {
    fn name(
        &mut self,
        self_: wasmtime::component::Resource<wit::pb::rules::read_filesystem::File>,
    ) -> wasmtime::component::__internal::String {
        self.filesystem.name(self_)
    }

    fn read(
        &mut self,
        self_: wasmtime::component::Resource<wit::pb::rules::read_filesystem::File>,
    ) -> wasmtime::component::__internal::Vec<u8> {
        self.filesystem.read(self_)
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<wit::pb::rules::read_filesystem::File>,
    ) -> wasmtime::Result<()> {
        self.filesystem.drop(rep)
    }
}
impl wit::pb::rules::read_filesystem::Host for HostStuff {}

#[derive(Default)]
pub struct MyWaker;

impl wit::pb::rules::types::HostWaker for MyWaker {
    fn wake(&mut self, self_: wasmtime::component::Resource<wit::pb::rules::types::Waker>) -> () {}

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<wit::pb::rules::types::Waker>,
    ) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl wit::pb::rules::types::HostWaker for HostStuff {
    fn wake(&mut self, self_: wasmtime::component::Resource<MyWaker>) -> () {
        self.waker.wake(self_);
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<MyWaker>) -> wasmtime::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct MyProvider;

impl wit::pb::rules::types::HostProviderDict for MyProvider {
    fn get(
        &mut self,
        self_: wasmtime::component::Resource<wit::pb::rules::types::ProviderDict>,
        key: wasmtime::component::__internal::String,
    ) -> wit::pb::rules::types::ProviderValue {
        wit::pb::rules::types::ProviderValue::Text("FOOBAR".into())
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<wit::pb::rules::types::ProviderDict>,
    ) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl wit::pb::rules::types::HostProviderDict for HostStuff {
    fn get(
        &mut self,
        self_: wasmtime::component::Resource<MyProvider>,
        key: wasmtime::component::__internal::String,
    ) -> wit::pb::rules::types::ProviderValue {
        self.provider.get(self_, key)
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<MyProvider>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl wit::pb::rules::types::Host for HostStuff {}

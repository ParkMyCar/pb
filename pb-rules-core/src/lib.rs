use filesystem::{pb::rules::read_filesystem::HostFile, FileNamespace};
use logger::{pb::rules::logging::Host, Logger};

pub mod filesystem;
pub mod logger;

pub use logger::TargetResolver;

#[derive(Default)]
pub struct HostStuff {
    logger: Logger,
    filesystem: FileNamespace,
}

impl HostStuff {
    pub fn add_to_linker<T, U>(
        linker: &mut wasmtime::component::Linker<T>,
        get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    ) -> wasmtime::Result<()>
    where
        U: logger::pb::rules::logging::Host + filesystem::pb::rules::read_filesystem::Host,
    {
        logger::pb::rules::logging::add_to_linker(linker, get)?;
        filesystem::pb::rules::read_filesystem::add_to_linker(linker, get)?;
        Ok(())
    }
}

impl logger::pb::rules::logging::Host for HostStuff {
    fn event(
        &mut self,
        level: logger::pb::rules::logging::Level,
        message: wasmtime::component::__internal::String,
        location: logger::pb::rules::logging::Location,
        fields: wasmtime::component::__internal::Vec<logger::pb::rules::logging::Field>,
    ) -> () {
        self.logger.event(level, message, location, fields)
    }
}

impl filesystem::pb::rules::read_filesystem::HostFile for HostStuff {
    fn name(
        &mut self,
        self_: wasmtime::component::Resource<filesystem::pb::rules::read_filesystem::File>,
    ) -> wasmtime::component::__internal::String {
        self.filesystem.name(self_)
    }

    fn read(
        &mut self,
        self_: wasmtime::component::Resource<filesystem::pb::rules::read_filesystem::File>,
    ) -> wasmtime::component::__internal::Vec<u8> {
        self.filesystem.read(self_)
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<filesystem::pb::rules::read_filesystem::File>,
    ) -> wasmtime::Result<()> {
        self.filesystem.drop(rep)
    }
}
impl filesystem::pb::rules::read_filesystem::Host for HostStuff {}

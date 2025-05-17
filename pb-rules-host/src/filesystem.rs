use crate::wit::pb::rules as wit;
use crate::HostState;

impl wit::read_filesystem::Host for HostState {}

#[derive(Default)]
pub struct FileHandle;

impl wit::read_filesystem::HostFile for HostState {
    fn name(
        &mut self,
        self_: wasmtime::component::Resource<wit::read_filesystem::File>,
    ) -> wasmtime::component::__internal::String {
        "TODO filename".into()
    }

    fn read(
        &mut self,
        self_: wasmtime::component::Resource<wit::read_filesystem::File>,
    ) -> wasmtime::component::__internal::Vec<u8> {
        vec![42u8; 10].into()
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<wit::read_filesystem::File>,
    ) -> wasmtime::Result<()> {
        Ok(())
    }
}

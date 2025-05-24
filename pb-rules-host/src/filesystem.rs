use crate::wit::pb::rules as wit;
use crate::HostState;

impl wit::read_filesystem::Host for HostState {}

/// A client that can be used to write files.
#[derive(Default, Debug, Clone)]
pub struct WriteClient {}

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

    fn read_stream(
        &mut self,
        _: wasmtime::component::Resource<FileHandle>,
    ) -> wasmtime::component::Resource<wit::types::BytesStream> {
        todo!()
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<wit::read_filesystem::File>,
    ) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl wit::write_filesystem::HostWriteClient for HostState {
    fn create_file(
        &mut self,
        self_: wasmtime::component::Resource<WriteClient>,
        name: wasmtime::component::__internal::String,
    ) -> wasmtime::component::Resource<wit::write_filesystem::WriteFile> {
        let file = FileHandle::default();
        self.resources.push(file).unwrap()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<WriteClient>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl wit::write_filesystem::HostWriteFile for HostState {
    fn write_sink(
        &mut self,
        self_: wasmtime::component::Resource<wit::write_filesystem::WriteFile>,
    ) -> wasmtime::component::Resource<wit::write_filesystem::BytesSink> {
        todo!()
    }

    fn write_xattr(
        &mut self,
        self_: wasmtime::component::Resource<wit::write_filesystem::WriteFile>,
        name: wasmtime::component::__internal::String,
    ) -> wasmtime::component::Resource<wit::write_filesystem::BytesSink> {
        todo!()
    }

    fn set_mtime(
        &mut self,
        self_: wasmtime::component::Resource<wit::write_filesystem::WriteFile>,
        millis: u64,
    ) -> () {
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<wit::write_filesystem::WriteFile>,
    ) -> wasmtime::Result<()> {
        Ok(())
    }
}

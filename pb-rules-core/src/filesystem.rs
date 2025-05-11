use wasmtime::component::bindgen;

bindgen!({ path: "pb-wit/wit" });

#[derive(Default)]
pub struct FileNamespace;

impl FileNamespace {
    pub fn add_to_linker<T, U>(
        linker: &mut wasmtime::component::Linker<T>,
        get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    ) -> wasmtime::Result<()>
    where
        U: pb::rules::read_filesystem::Host,
    {
        pb::rules::read_filesystem::add_to_linker(linker, get)
    }
}

impl pb::rules::read_filesystem::HostFile for FileNamespace {
    fn name(
        &mut self,
        self_: wasmtime::component::Resource<pb::rules::read_filesystem::File>,
    ) -> wasmtime::component::__internal::String {
        "TODO filename".into()
    }

    fn read(
        &mut self,
        self_: wasmtime::component::Resource<pb::rules::read_filesystem::File>,
    ) -> wasmtime::component::__internal::Vec<u8> {
        vec![42u8; 10].into()
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<pb::rules::read_filesystem::File>,
    ) -> wasmtime::Result<()> {
        Ok(())
    }
}
impl pb::rules::read_filesystem::Host for FileNamespace {}

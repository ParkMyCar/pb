wit_bindgen::generate!({
    world: "filesystem",
    path: "wit/core",
});

pub struct FileHandle;

impl exports::pb::core::read_filesystem::GuestFile for FileHandle {
    fn name(&self) -> _rt::String {
        "TODO FileHandle::name".to_string()
    }

    fn read(&self) -> _rt::Vec<u8> {
        vec![42]
    }
}

pub struct Filesystem;

impl exports::pb::core::read_filesystem::Guest for Filesystem {
    type File = FileHandle;
}

export!(Filesystem);

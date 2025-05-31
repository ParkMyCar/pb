use std::sync::Arc;

use futures::future::BoxFuture;
use futures::FutureExt;
use pb_filesystem::locations::scratch::{ScratchDirectoryHandle, ScratchFileHandle};
use pb_types::Timespec;

use crate::types::HostWaker;
use crate::wit::pb::rules as wit;
use crate::wit::pb::rules::write_filesystem::FailableFuture;
use crate::HostState;

impl wit::read_filesystem::Host for HostState {}

/// A client that can be used to write files.
#[derive(Default, Debug, Clone)]
pub struct WriteClient {}

pub struct FileHandle {
    /// Name of the file.
    name: String,
    /// Open filesystem resource.
    inner: pb_filesystem::handle::FileHandle,
}

impl wit::read_filesystem::HostFile for HostState {
    fn name(
        &mut self,
        self_: wasmtime::component::Resource<wit::read_filesystem::File>,
    ) -> wasmtime::component::__internal::String {
        let handle = self.resources.get(&self_).unwrap();
        handle.name.clone().into()
    }

    fn read(
        &mut self,
        self_: wasmtime::component::Resource<wit::read_filesystem::File>,
    ) -> wasmtime::component::__internal::Vec<u8> {
        vec![42u8; 10].into()
    }

    fn read_stream(
        &mut self,
        self_: wasmtime::component::Resource<wit::read_filesystem::File>,
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

pub struct CreateFileFuture {
    pub(crate) inner: BoxFuture<'static, Result<WriteFileHandleInner, String>>,
}

impl CreateFileFuture {
    fn new(inner: BoxFuture<'static, Result<WriteFileHandleInner, String>>) -> Self {
        CreateFileFuture { inner }
    }
}

impl wit::write_filesystem::HostCreateFileFuture for HostState {
    fn poll(
        &mut self,
        self_: wasmtime::component::Resource<CreateFileFuture>,
        waker: wasmtime::component::Resource<HostWaker>,
    ) -> wit::write_filesystem::CreateFilePoll {
        let waker = self.resources.get(&waker).unwrap().clone();
        let resource = self.resources.get_mut(&self_).unwrap();
        let mut context = std::task::Context::from_waker(waker.waker());

        match resource.inner.poll_unpin(&mut context) {
            std::task::Poll::Pending => wit::write_filesystem::CreateFilePoll::Pending,
            std::task::Poll::Ready(result) => {
                let result = match result {
                    Ok(inner) => {
                        let handle = WriteFileHandle::new(inner);
                        Ok(self.resources.push(handle).unwrap())
                    }
                    Err(e) => Err(e),
                };
                wit::write_filesystem::CreateFilePoll::Ready(result)
            }
        }
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<CreateFileFuture>,
    ) -> wasmtime::Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct WriteFileHandle {
    state: Arc<tokio::sync::Mutex<WriteFileHandleInner>>,
}

impl WriteFileHandle {
    fn new(inner: WriteFileHandleInner) -> Self {
        WriteFileHandle {
            state: Arc::new(tokio::sync::Mutex::new(inner)),
        }
    }
}

pub enum WriteFileHandleInner {
    /// File this is a direct child of the scratch directory.
    Root {
        /// Handle to a file resource.
        file: ScratchFileHandle,
        /// Desired name for this file at the final destination.
        desired_name: String,
        /// The offset that we've written to thus far.
        offset: usize,
    },
    /// File nested within the scratch directory, it's ancestor will get moved into place.
    Child {
        /// Handle ro a file resource.
        file: pb_filesystem::handle::FileHandle,
        /// The offset that we've written to thus far.
        offset: usize,
    },
    Closed,
}

impl WriteFileHandleInner {
    fn try_inner(&mut self) -> Result<&mut pb_filesystem::handle::FileHandle, String> {
        match self {
            WriteFileHandleInner::Root { file, .. } => Ok(file.inner_mut()),
            WriteFileHandleInner::Child { file, .. } => Ok(file),
            WriteFileHandleInner::Closed => Err("file closed".to_string()),
        }
    }

    fn try_offset(&mut self) -> Result<&mut usize, String> {
        match self {
            WriteFileHandleInner::Root { offset, .. }
            | WriteFileHandleInner::Child { offset, .. } => Ok(offset),
            WriteFileHandleInner::Closed => Err("file closed".to_string()),
        }
    }
}

impl wit::write_filesystem::HostWriteClient for HostState {
    fn create_file(
        &mut self,
        _self: wasmtime::component::Resource<WriteClient>,
        name: wasmtime::component::__internal::String,
    ) -> wasmtime::component::Resource<CreateFileFuture> {
        let create_file_fut = self.scratch_space.file();
        let future = async move {
            let root_file_handle = create_file_fut.await.map_err(|err| err.to_string())?;
            Ok::<_, String>(WriteFileHandleInner::Root {
                file: root_file_handle,
                desired_name: name,
                offset: 0,
            })
        }
        .boxed();
        self.resources.push(CreateFileFuture::new(future)).unwrap()
    }

    fn create_directory(
        &mut self,
        _self: wasmtime::component::Resource<WriteClient>,
        name: wasmtime::component::__internal::String,
    ) -> wasmtime::component::Resource<wit::write_filesystem::CreateDirectoryFuture> {
        let create_dir_fut = self.scratch_space.directory();
        let future = async move {
            let root_dir_handle = create_dir_fut.await.map_err(|err| err.to_string())?;
            Ok::<_, String>(WriteDirectoryInner::Root {
                dir: root_dir_handle,
                desired_name: name,
            })
        }
        .boxed();
        self.resources
            .push(CreateDirectoryFuture::new(future))
            .unwrap()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<WriteClient>) -> wasmtime::Result<()> {
        self.resources.delete(rep)?;
        Ok(())
    }
}

impl wit::write_filesystem::HostWriteFile for HostState {
    fn append(
        &mut self,
        self_: wasmtime::component::Resource<WriteFileHandle>,
        data: wasmtime::component::__internal::Vec<u8>,
    ) -> wasmtime::component::Resource<wit::write_filesystem::FailableFuture> {
        let scratch_file = self.resources.get(&self_).unwrap().clone();

        let future = async move {
            let mut scratch_file = scratch_file.state.lock().await;
            let cur_offset = *scratch_file.try_offset()?;
            let to_write = data.len();

            // Write data at our last offset.
            scratch_file
                .try_inner()?
                .write(data, cur_offset)
                .await
                .map_err(|err| err.to_string())?;

            // Update the offset for the next time that we write.
            let cur_offset = scratch_file.try_offset()?;
            *cur_offset = cur_offset
                .checked_add(to_write)
                .expect("overflowed offset when writing");

            Ok::<_, String>(())
        }
        .boxed();

        self.resources.push(FailableFuture::new(future)).unwrap()
    }

    fn write_xattr(
        &mut self,
        self_: wasmtime::component::Resource<WriteFileHandle>,
        name: wasmtime::component::__internal::String,
        data: wasmtime::component::__internal::Vec<u8>,
    ) -> wasmtime::component::Resource<wit::write_filesystem::FailableFuture> {
        let scratch_file = self.resources.get(&self_).unwrap().clone();

        let future = async move {
            let mut scratch_file = scratch_file.state.lock().await;

            scratch_file
                .try_inner()?
                .setxattr(name.into(), data.into())
                .await
                .map_err(|err| err.to_string())?;

            Ok::<_, String>(())
        }
        .boxed();

        self.resources.push(FailableFuture::new(future)).unwrap()
    }

    fn set_mtime(
        &mut self,
        self_: wasmtime::component::Resource<wit::write_filesystem::WriteFile>,
        millis: u64,
    ) -> wasmtime::component::Resource<wit::write_filesystem::FailableFuture> {
        let scratch_file = self.resources.get(&self_).unwrap().clone();

        let future = async move {
            let mut scratch_file = scratch_file.state.lock().await;

            let timespec = Timespec::from_epoch_millis(millis);
            scratch_file
                .try_inner()?
                .setmtime(timespec)
                .await
                .map_err(|err| err.to_string())?;

            Ok::<_, String>(())
        }
        .boxed();

        self.resources.push(FailableFuture::new(future)).unwrap()
    }

    fn close(
        &mut self,
        self_: wasmtime::component::Resource<WriteFileHandle>,
    ) -> wasmtime::component::Resource<FailableFuture> {
        let scratch_file = self.resources.get(&self_).unwrap().clone();

        // TODO: Configure where this file gets placed.
        let repositories_dir = self.repositories.root_directory();

        let future = async move {
            let mut scratch_file = scratch_file.state.lock().await;

            // Mark the file as closed.
            let prev_state = std::mem::replace(&mut *scratch_file, WriteFileHandleInner::Closed);

            match prev_state {
                WriteFileHandleInner::Root {
                    file, desired_name, ..
                } => {
                    file.fsync().await.map_err(|err| err.to_string())?;
                    file.persistat(&*repositories_dir, desired_name)
                        .await
                        .map_err(|err| err.to_string())?;
                }
                WriteFileHandleInner::Child { file, .. } => {
                    file.fsync().await.map_err(|err| err.to_string())?;
                    file.close().await.map_err(|err| err.to_string())?;
                }
                WriteFileHandleInner::Closed => return Err("file closed".to_string()),
            }

            Ok::<_, String>(())
        }
        .boxed();

        self.resources.push(FailableFuture::new(future)).unwrap()
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<wit::write_filesystem::WriteFile>,
    ) -> wasmtime::Result<()> {
        self.resources.delete(rep)?;
        Ok(())
    }
}

pub struct CreateDirectoryFuture {
    pub(crate) inner: BoxFuture<'static, Result<WriteDirectoryInner, String>>,
}

impl CreateDirectoryFuture {
    fn new(inner: BoxFuture<'static, Result<WriteDirectoryInner, String>>) -> Self {
        CreateDirectoryFuture { inner }
    }
}

impl wit::write_filesystem::HostCreateDirectoryFuture for HostState {
    fn poll(
        &mut self,
        self_: wasmtime::component::Resource<CreateDirectoryFuture>,
        waker: wasmtime::component::Resource<HostWaker>,
    ) -> wit::write_filesystem::CreateDirectoryPoll {
        let waker = self.resources.get(&waker).unwrap().clone();
        let resource = self.resources.get_mut(&self_).unwrap();
        let mut context = std::task::Context::from_waker(waker.waker());

        match resource.inner.poll_unpin(&mut context) {
            std::task::Poll::Pending => wit::write_filesystem::CreateDirectoryPoll::Pending,
            std::task::Poll::Ready(result) => {
                let result = match result {
                    Ok(inner) => {
                        let handle = WriteDirectoryHandle::new(inner);
                        Ok(self.resources.push(handle).unwrap())
                    }
                    Err(e) => Err(e),
                };
                wit::write_filesystem::CreateDirectoryPoll::Ready(result)
            }
        }
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<CreateDirectoryFuture>,
    ) -> wasmtime::Result<()> {
        self.resources.delete(rep)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct WriteDirectoryHandle {
    state: Arc<tokio::sync::Mutex<WriteDirectoryInner>>,
}

impl WriteDirectoryHandle {
    fn new(inner: WriteDirectoryInner) -> Self {
        WriteDirectoryHandle {
            state: Arc::new(tokio::sync::Mutex::new(inner)),
        }
    }
}

pub enum WriteDirectoryInner {
    Root {
        dir: ScratchDirectoryHandle,
        desired_name: String,
    },
    Child {
        dir: pb_filesystem::handle::DirectoryHandle,
    },
    Closed,
}

impl WriteDirectoryInner {
    fn try_inner(&mut self) -> Result<&mut pb_filesystem::handle::DirectoryHandle, String> {
        match self {
            WriteDirectoryInner::Root { dir, .. } => Ok(dir.inner_mut()),
            WriteDirectoryInner::Child { dir } => Ok(dir),
            WriteDirectoryInner::Closed => Err("file closed".to_string()),
        }
    }
}

impl wit::write_filesystem::HostWriteDirectory for HostState {
    fn create_directory(
        &mut self,
        self_: wasmtime::component::Resource<WriteDirectoryHandle>,
        name: wasmtime::component::__internal::String,
    ) -> wasmtime::component::Resource<CreateDirectoryFuture> {
        let parent = self.resources.get(&self_).unwrap().clone();
        let future = async move {
            let mut parent = parent.state.lock().await;
            let child = parent
                .try_inner()?
                .openat(name)
                .as_directory()
                .with_create()
                .await
                .map_err(|err| err.to_string())?;
            Ok(WriteDirectoryInner::Child { dir: child })
        }
        .boxed();
        self.resources
            .push(CreateDirectoryFuture::new(future))
            .unwrap()
    }

    fn create_file(
        &mut self,
        self_: wasmtime::component::Resource<WriteDirectoryHandle>,
        name: wasmtime::component::__internal::String,
    ) -> wasmtime::component::Resource<CreateFileFuture> {
        let parent = self.resources.get(&self_).unwrap().clone();
        let future = async move {
            let mut parent = parent.state.lock().await;
            let (child, _stat) = parent
                .try_inner()?
                .openat(name)
                .as_file()
                .with_create()
                .await
                .map_err(|err| err.to_string())?;
            Ok(WriteFileHandleInner::Child {
                file: child,
                offset: 0,
            })
        }
        .boxed();
        self.resources.push(CreateFileFuture::new(future)).unwrap()
    }

    fn write_xattr(
        &mut self,
        self_: wasmtime::component::Resource<WriteDirectoryHandle>,
        name: wasmtime::component::__internal::String,
        data: wasmtime::component::__internal::Vec<u8>,
    ) -> wasmtime::component::Resource<FailableFuture> {
        let handle = self.resources.get(&self_).unwrap().clone();
        let future = async move {
            let mut handle = handle.state.lock().await;
            handle
                .try_inner()?
                .setxattr(name.into(), data.into())
                .await
                .map_err(|err| err.to_string())?;
            Ok(())
        }
        .boxed();
        self.resources.push(FailableFuture::new(future)).unwrap()
    }

    fn set_mtime(
        &mut self,
        self_: wasmtime::component::Resource<WriteDirectoryHandle>,
        millis: u64,
    ) -> wasmtime::component::Resource<FailableFuture> {
        let handle = self.resources.get(&self_).unwrap().clone();
        let future = async move {
            let mut handle = handle.state.lock().await;
            let timespec = Timespec::from_epoch_millis(millis);
            handle
                .try_inner()?
                .setmtime(timespec)
                .await
                .map_err(|err| err.to_string())?;
            Ok(())
        }
        .boxed();
        self.resources.push(FailableFuture::new(future)).unwrap()
    }

    fn close(
        &mut self,
        self_: wasmtime::component::Resource<WriteDirectoryHandle>,
    ) -> wasmtime::component::Resource<FailableFuture> {
        let scratch_dir = self.resources.get(&self_).unwrap().clone();
        // TODO: Configure where this file gets placed.
        let repositories_dir = self.repositories.root_directory();

        let future = async move {
            let mut scratch_dir = scratch_dir.state.lock().await;

            // Mark the file as closed.
            let prev_state = std::mem::replace(&mut *scratch_dir, WriteDirectoryInner::Closed);

            match prev_state {
                WriteDirectoryInner::Root { dir, desired_name } => {
                    dir.fsync().await.map_err(|err| err.to_string())?;
                    dir.persistat(&*repositories_dir, desired_name)
                        .await
                        .map_err(|err| err.to_string())?;
                }
                WriteDirectoryInner::Child { dir } => {
                    dir.fsync().await.map_err(|err| err.to_string())?;
                    dir.close().await.map_err(|err| err.to_string())?;
                }
                WriteDirectoryInner::Closed => return Err("directory closed".to_string()),
            }

            Ok::<_, String>(())
        }
        .boxed();

        self.resources.push(FailableFuture::new(future)).unwrap()
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<WriteDirectoryHandle>,
    ) -> wasmtime::Result<()> {
        self.resources.delete(rep)?;
        Ok(())
    }
}

impl wit::write_filesystem::Host for HostState {}

use std::collections::BTreeMap;
use std::pin::Pin;

use futures::stream::BoxStream;
use futures::{Sink, SinkExt, StreamExt};

use crate::http::Response;
use crate::wit::pb::rules as wit;
use crate::HostState;

impl wit::types::Host for HostState {}

#[derive(Clone)]
pub struct HostWaker {
    inner: std::task::Waker,
}

impl HostWaker {
    pub fn new(waker: std::task::Waker) -> HostWaker {
        HostWaker { inner: waker }
    }

    pub(crate) fn waker(&self) -> &std::task::Waker {
        &self.inner
    }
}

impl wit::types::HostWaker for HostState {
    fn wake(&mut self, self_: wasmtime::component::Resource<wit::types::Waker>) -> () {
        let waker = self.resources.get(&self_).expect("waker doesn't exist");
        waker.inner.wake_by_ref();
    }

    fn clone(
        &mut self,
        self_: wasmtime::component::Resource<wit::types::Waker>,
    ) -> wasmtime::component::Resource<wit::types::Waker> {
        let waker = self.resources.get(&self_).expect("waker doesn't exist");
        self.resources.push(waker.clone()).expect("out of space?")
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<wit::types::Waker>,
    ) -> wasmtime::Result<()> {
        self.resources.delete(rep)?;
        Ok(())
    }
}

pub struct Provider {
    inner: BTreeMap<String, wit::types::ProviderValue>,
}

impl wit::types::HostProviderDict for HostState {
    fn get(
        &mut self,
        self_: wasmtime::component::Resource<Provider>,
        key: wasmtime::component::__internal::String,
    ) -> wit::types::ProviderValue {
        let provider = self.resources.get(&self_).unwrap();
        let value = provider.inner.get(&key).expect("key does not exist");

        match value {
            wit::types::ProviderValue::File(val) => wit::types::ProviderValue::File(val.clone()),
            wit::types::ProviderValue::Text(val) => wit::types::ProviderValue::Text(val.clone()),
            wit::types::ProviderValue::Nested(_) => todo!(),
        }
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Provider>) -> wasmtime::Result<()> {
        self.resources.delete(rep)?;
        Ok(())
    }
}

/// An asynchronous iterator of bytes from the Host.
pub struct BytesStream {
    pub(crate) stream: BoxStream<'static, Vec<u8>>,
}

impl wit::types::HostBytesStream for HostState {
    fn poll_next(
        &mut self,
        self_: wasmtime::component::Resource<BytesStream>,
        waker: wasmtime::component::Resource<crate::types::HostWaker>,
    ) -> wit::types::BytesPoll {
        let waker = self.resources.get(&waker).unwrap().clone();
        let resource = self.resources.get_mut(&self_).unwrap();
        let mut context = std::task::Context::from_waker(waker.waker());

        match resource.stream.poll_next_unpin(&mut context) {
            std::task::Poll::Pending => wit::types::BytesPoll::Pending,
            std::task::Poll::Ready(result) => wit::types::BytesPoll::Ready(result),
        }
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<BytesStream>) -> wasmtime::Result<()> {
        // self.resources.delete(rep).unwrap();
        Ok(())
    }
}

///
pub struct BytesSink {
    pub(crate) inner: Pin<Box<dyn Sink<Vec<u8>, Error = String>>>,
}

impl wit::types::HostBytesSink for HostState {
    fn start_send(
        &mut self,
        self_: wasmtime::component::Resource<wit::types::BytesSink>,
        bytes: wasmtime::component::__internal::Vec<u8>,
    ) {
        let sink = self.resources.get_mut(&self_).unwrap();
        sink.inner
            .start_send_unpin(bytes.into())
            .expect("TODO we should take errors");
    }

    fn poll_flush(
        &mut self,
        self_: wasmtime::component::Resource<wit::types::BytesSink>,
        waker: wasmtime::component::Resource<HostWaker>,
    ) -> wit::types::FlushPoll {
        let waker = self.resources.get(&waker).unwrap().clone();
        let sink = self.resources.get_mut(&self_).unwrap();
        let mut context = std::task::Context::from_waker(waker.waker());

        match sink.inner.poll_flush_unpin(&mut context) {
            std::task::Poll::Pending => wit::types::FlushPoll::Pending,
            std::task::Poll::Ready(_) => wit::types::FlushPoll::Ready,
        }
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<wit::types::BytesSink>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

use std::str::FromStr;

use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::header::{HeaderName, HeaderValue};

use crate::wit::pb::rules as wit;
use crate::HostState;

impl wit::http::Host for HostState {}

/// Client to make HTTP requests.
#[derive(Default, Clone)]
pub struct Client {
    pub inner: reqwest::Client,
}

impl wit::http::HostClient for HostState {
    fn get(
        &mut self,
        self_: wasmtime::component::Resource<Client>,
        request: wit::http::Request,
    ) -> wasmtime::component::Resource<crate::http::Response> {
        let client = self.resources.get(&self_).unwrap();

        let headers = request
            .headers
            .into_iter()
            .map(|(name, val)| {
                let name = HeaderName::from_str(&name).expect("invalid header name");
                let val = HeaderValue::from_str(&val).expect("invalid header val");
                (name, val)
            })
            .collect();
        let request = client.inner.get(&request.url).headers(headers).send();

        let response = Response {
            inner: Some(Box::pin(request)),
        };
        self.resources.push(response).unwrap()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Client>) -> wasmtime::Result<()> {
        // self.resources.delete(rep)?;
        Ok(())
    }
}

/// Response to an HTTP request.
pub struct Response {
    inner: Option<BoxFuture<'static, Result<reqwest::Response, reqwest::Error>>>,
}

impl Response {
    fn headers(&self) -> &reqwest::header::HeaderMap {
        todo!()
    }
}

impl wit::http::HostResponse for HostState {
    fn headers(
        &mut self,
        self_: wasmtime::component::Resource<Response>,
    ) -> wasmtime::component::__internal::Vec<(
        wasmtime::component::__internal::String,
        wasmtime::component::__internal::String,
    )> {
        let response = self.resources.get(&self_).unwrap();
        response
            .headers()
            .iter()
            .map(|(name, val)| {
                let name = name.to_string();
                let val = val.to_str().unwrap().to_string();
                (name, val)
            })
            .collect()
    }

    fn body(
        &mut self,
        self_: wasmtime::component::Resource<Response>,
    ) -> wasmtime::component::Resource<ResponseBodyStream> {
        println!("calling body {self_:?}");
        let response = self.resources.get_mut(&self_).unwrap();
        let stream = ResponseBodyStream::new(response);
        self.resources.push(stream).unwrap()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Response>) -> wasmtime::Result<()> {
        // TODO: Check if the Response existed, it might not if we turned it
        // into a body stream.
        // self.resources.delete(rep);
        Ok(())
    }
}

/// Stream for the body of a [`Response`].
pub struct ResponseBodyStream {
    stream: BoxStream<'static, Vec<u8>>,
}

impl ResponseBodyStream {
    pub fn new(response: &mut Response) -> Self {
        let response = response.inner.take();
        let work = async move {
            let result = response.unwrap().await.unwrap();
            futures::stream::unfold(result, |mut result| async move {
                result
                    .chunk()
                    .await
                    .unwrap()
                    .map(|val| (val.to_vec(), result))
            })
        };
        ResponseBodyStream {
            stream: futures::stream::once(work).flatten().boxed(),
        }
    }
}

impl wit::http::HostBodyStream for HostState {
    fn poll_next(
        &mut self,
        self_: wasmtime::component::Resource<ResponseBodyStream>,
        waker: wasmtime::component::Resource<crate::types::HostWaker>,
    ) -> wit::http::BodyPoll {
        let waker = self.resources.get(&waker).unwrap().clone();
        let resource = self.resources.get_mut(&self_).unwrap();
        let mut context = std::task::Context::from_waker(waker.waker());

        match resource.stream.poll_next_unpin(&mut context) {
            std::task::Poll::Pending => wit::http::BodyPoll::Pending,
            std::task::Poll::Ready(result) => wit::http::BodyPoll::Ready(result),
        }
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<ResponseBodyStream>,
    ) -> wasmtime::Result<()> {
        // self.resources.delete(rep).unwrap();
        Ok(())
    }
}

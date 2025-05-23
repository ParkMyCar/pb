use std::str::FromStr;

use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt};
use reqwest::header::{HeaderName, HeaderValue};

use crate::wit::pb::rules as wit;
use crate::wit::pb::rules::http::BytesStream;
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
    ) -> wasmtime::component::Resource<crate::http::ResponseFuture> {
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
        let response = client.inner.get(&request.url).headers(headers).send();

        let response = ResponseFuture {
            inner: response.boxed(),
        };
        self.resources.push(response).unwrap()
    }

    fn drop(&mut self, _rep: wasmtime::component::Resource<Client>) -> wasmtime::Result<()> {
        // self.resources.delete(rep)?;
        Ok(())
    }
}

pub struct ResponseFuture {
    inner: BoxFuture<'static, Result<reqwest::Response, reqwest::Error>>,
}

impl wit::http::HostResponseFuture for HostState {
    fn poll(
        &mut self,
        self_: wasmtime::component::Resource<wit::http::ResponseFuture>,
        waker: wasmtime::component::Resource<wit::http::Waker>,
    ) -> wit::http::ResponsePoll {
        let waker = self.resources.get(&waker).unwrap().clone();
        let resource = self.resources.get_mut(&self_).unwrap();
        let mut context = std::task::Context::from_waker(waker.waker());

        match resource.inner.poll_unpin(&mut context) {
            std::task::Poll::Pending => wit::http::ResponsePoll::Pending,
            std::task::Poll::Ready(response) => {
                let response = self
                    .resources
                    .push(Response {
                        inner: Some(response),
                    })
                    .unwrap();
                wit::http::ResponsePoll::Ready(response)
            }
        }
    }

    fn drop(
        &mut self,
        _rep: wasmtime::component::Resource<wit::http::ResponseFuture>,
    ) -> wasmtime::Result<()> {
        Ok(())
    }
}

/// Response to an HTTP request.
pub struct Response {
    pub(crate) inner: Option<Result<reqwest::Response, reqwest::Error>>,
}

impl Response {
    fn status(&self) -> u16 {
        let response = self
            .inner
            .as_ref()
            .expect("response was already taken, maybe turned into a bytes stream?");
        let response = response
            .as_ref()
            .expect("TODO make the HTTP Get API failable");
        response.status().as_u16()
    }

    fn headers(&self) -> &reqwest::header::HeaderMap {
        let response = self
            .inner
            .as_ref()
            .expect("response was already taken, maybe turned into a bytes stream?");
        let response = response
            .as_ref()
            .expect("TODO make the HTTP Get API failable");
        response.headers()
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

    fn status(&mut self, self_: wasmtime::component::Resource<Response>) -> u16 {
        let response = self.resources.get(&self_).unwrap();
        response.status()
    }

    fn body(
        &mut self,
        self_: wasmtime::component::Resource<Response>,
    ) -> wasmtime::component::Resource<BytesStream> {
        println!("calling body {self_:?}");
        let response = self.resources.get_mut(&self_).unwrap();
        let stream = BytesStream::from(response);
        self.resources.push(stream).unwrap()
    }

    fn drop(&mut self, _rep: wasmtime::component::Resource<Response>) -> wasmtime::Result<()> {
        // TODO: Check if the Response existed, it might not if we turned it
        // into a body stream.
        // self.resources.delete(rep);
        Ok(())
    }
}

impl From<&mut Response> for BytesStream {
    fn from(response: &mut Response) -> Self {
        let response = response.inner.take();
        let work = async move {
            let result = response.unwrap().unwrap();
            futures::stream::unfold(result, |mut result| async move {
                result
                    .chunk()
                    .await
                    .unwrap()
                    .map(|val| (val.to_vec(), result))
            })
        };

        BytesStream {
            stream: futures::stream::once(work).flatten().boxed(),
        }
    }
}

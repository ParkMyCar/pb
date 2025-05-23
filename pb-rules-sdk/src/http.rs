//! Type wrappers around the WIT defined HTTP client.

use futures::{FutureExt, future::BoxFuture};

use crate::futures::FutureCompat2;

pub struct HostFutureAdapter {
    inner: crate::pb::rules::http::ResponseFuture,
}

impl Future for HostFutureAdapter {
    type Output = crate::pb::rules::http::Response;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        crate::logging::with_logging(|| {
            let waker = cx.waker().data() as *const ();
            let waker = waker as *const crate::exports::pb::rules::rules::Waker;
            let waker = unsafe { &*waker };
            let waker = waker.clone();

            match self.as_ref().inner.poll(waker) {
                crate::pb::rules::http::ResponsePoll::Pending => std::task::Poll::Pending,
                crate::pb::rules::http::ResponsePoll::Ready(val) => std::task::Poll::Ready(val),
            }
        })
    }
}

impl FutureCompat2<crate::pb::rules::http::Response> for crate::pb::rules::http::ResponseFuture {
    fn compat(self) -> BoxFuture<'static, crate::pb::rules::http::Response> {
        HostFutureAdapter { inner: self }.boxed()
    }
}

//! Types and adapters so we can write async friendly rules and execute them in
//! an async friendly way in the host.

use futures::Stream;
use futures::future::{BoxFuture, FutureExt, LocalBoxFuture};
use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable};

pub trait FutureCompat2<T> {
    fn compat(self) -> BoxFuture<'static, T>;
}

pub struct HostFailableFutureAdapter {
    inner: crate::pb::rules::types::FailableFuture,
}

impl Future for HostFailableFutureAdapter {
    type Output = Result<(), String>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        crate::logging::with_logging(|| {
            let waker = cx.waker().data() as *const ();
            let waker = waker as *const crate::exports::pb::rules::rules::Waker;
            let waker = unsafe { &*waker };
            let waker = waker.clone();

            match self.as_ref().inner.poll(waker) {
                crate::pb::rules::types::FailablePoll::Pending => std::task::Poll::Pending,
                crate::pb::rules::types::FailablePoll::Ready(val) => std::task::Poll::Ready(val),
            }
        })
    }
}

impl FutureCompat2<Result<(), String>> for crate::pb::rules::types::FailableFuture {
    fn compat(self) -> BoxFuture<'static, Result<(), String>> {
        HostFailableFutureAdapter { inner: self }.boxed()
    }
}

/// Rule implementations (i.e. WASM Guest functions) are provided a "waker"
/// that we define in WIT. This type adapts a regular Rust future to one that
/// can be polled by the WASM host.
pub struct GuestFutureAdapter<T> {
    inner: RefCell<LocalBoxFuture<'static, T>>,
}

impl<T> GuestFutureAdapter<T> {
    pub fn new(fut: LocalBoxFuture<'static, T>) -> Self {
        GuestFutureAdapter {
            inner: RefCell::new(fut),
        }
    }
}

impl<T: 'static> GuestFutureAdapter<T> {
    pub fn poll(&self, waker: crate::exports::pb::rules::rules::Waker) -> std::task::Poll<T> {
        let waker = WakerAdapter2::new(waker).waker();
        let mut context = std::task::Context::from_waker(&waker);
        let mut inner = self.inner.borrow_mut();
        inner.as_mut().poll(&mut context)
    }
}

pub struct ByteStreamWrapper {
    inner: crate::pb::rules::types::BytesStream,
}

impl ByteStreamWrapper {
    pub fn new(inner: crate::pb::rules::types::BytesStream) -> Self {
        ByteStreamWrapper { inner }
    }
}

impl Stream for ByteStreamWrapper {
    type Item = Vec<u8>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        crate::logging::with_logging(|| {
            let waker = cx.waker().data() as *const ();
            let waker = waker as *const crate::exports::pb::rules::rules::Waker;
            let waker = unsafe { &*waker };
            let waker = waker.clone();

            match self.as_ref().inner.poll_next(waker) {
                crate::pb::rules::types::BytesPoll::Pending => std::task::Poll::Pending,
                crate::pb::rules::types::BytesPoll::Ready(val) => std::task::Poll::Ready(val),
            }
        })
    }
}

static ADAPTER_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    raw_waker_adapter_clone,
    raw_waker_adapter_wake,
    raw_waker_adapter_wake_by_ref,
    raw_waker_adapter_drop,
);

pub struct WakerAdapter2 {
    inner: Arc<crate::exports::pb::rules::rules::Waker>,
}

impl WakerAdapter2 {
    pub fn new(waker: crate::exports::pb::rules::rules::Waker) -> Self {
        WakerAdapter2 {
            inner: Arc::new(waker),
        }
    }

    pub fn waker(self) -> std::task::Waker {
        let waker = Arc::into_raw(self.inner) as *const ();
        unsafe { std::task::Waker::new(waker, &ADAPTER_WAKER_VTABLE) }
    }
}

unsafe fn raw_waker_adapter_clone(waker: *const ()) -> RawWaker {
    unsafe {
        Arc::increment_strong_count(waker as *const crate::exports::pb::rules::rules::Waker);
    }
    RawWaker::new(waker as *const (), &ADAPTER_WAKER_VTABLE)
}

unsafe fn raw_waker_adapter_wake(waker: *const ()) {
    let waker = unsafe { Arc::from_raw(waker as *const crate::exports::pb::rules::rules::Waker) };
    waker.wake();
}

unsafe fn raw_waker_adapter_wake_by_ref(waker: *const ()) {
    let waker = unsafe {
        ManuallyDrop::new(Arc::from_raw(
            waker as *const crate::exports::pb::rules::rules::Waker,
        ))
    };
    waker.wake();
}

unsafe fn raw_waker_adapter_drop(waker: *const ()) {
    unsafe {
        Arc::decrement_strong_count(waker as *const crate::exports::pb::rules::rules::Waker);
    }
}

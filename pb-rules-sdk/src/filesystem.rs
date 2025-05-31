use futures::FutureExt;
use pb_ore::cast::CastFrom;
use std::pin::Pin;

use crate::futures::FutureCompat2;

pub struct FileReader {
    file: crate::pb::rules::read_filesystem::File,
    offset: usize,
}

impl crate::pb::rules::read_filesystem::File {
    pub fn into_reader(self) -> FileReader {
        FileReader {
            file: self,
            offset: 0,
        }
    }
}

impl std::io::Read for FileReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes = crate::pb::rules::read_filesystem::File::read(
            &self.file,
            u64::cast_from(buf.len()),
            u64::cast_from(self.offset),
        );
        let num_bytes = bytes.len();

        // TODO: A zero copy API would be way better.
        buf[..num_bytes].copy_from_slice(&bytes[..]);
        self.offset = self
            .offset
            .checked_add(bytes.len())
            .expect("overflowed offset when reading");

        Ok(num_bytes)
    }
}

pub struct HostCreateFileFutureAdapter {
    inner: crate::pb::rules::write_filesystem::CreateFileFuture,
}

impl Future for HostCreateFileFutureAdapter {
    type Output = Result<crate::pb::rules::write_filesystem::WriteFile, String>;

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
                crate::pb::rules::write_filesystem::CreateFilePoll::Pending => {
                    std::task::Poll::Pending
                }
                crate::pb::rules::write_filesystem::CreateFilePoll::Ready(val) => {
                    std::task::Poll::Ready(val)
                }
            }
        })
    }
}

impl FutureCompat2<Result<crate::pb::rules::write_filesystem::WriteFile, String>>
    for crate::pb::rules::write_filesystem::CreateFileFuture
{
    fn compat(
        self,
    ) -> futures::future::BoxFuture<
        'static,
        Result<crate::pb::rules::write_filesystem::WriteFile, String>,
    > {
        HostCreateFileFutureAdapter { inner: self }.boxed()
    }
}

pub struct HostCreateDirectoryFutureAdapter {
    inner: crate::pb::rules::write_filesystem::CreateDirectoryFuture,
}

impl Future for HostCreateDirectoryFutureAdapter {
    type Output = Result<crate::pb::rules::write_filesystem::WriteDirectory, String>;

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
                crate::pb::rules::write_filesystem::CreateDirectoryPoll::Pending => {
                    std::task::Poll::Pending
                }
                crate::pb::rules::write_filesystem::CreateDirectoryPoll::Ready(val) => {
                    std::task::Poll::Ready(val)
                }
            }
        })
    }
}

impl FutureCompat2<Result<crate::pb::rules::write_filesystem::WriteDirectory, String>>
    for crate::pb::rules::write_filesystem::CreateDirectoryFuture
{
    fn compat(
        self,
    ) -> futures::future::BoxFuture<
        'static,
        Result<crate::pb::rules::write_filesystem::WriteDirectory, String>,
    > {
        HostCreateDirectoryFutureAdapter { inner: self }.boxed()
    }
}

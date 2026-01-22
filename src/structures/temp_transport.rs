use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub struct TempTransport<'a, T: AsyncRead + AsyncWrite + Unpin + Send + Sync> {
    base_transport: &'a mut T,
}

impl<'a, T: AsyncRead + AsyncWrite + Unpin + Send + Sync> TempTransport<'a, T> {
    pub fn new(transport: &'a mut T) -> Self {
        TempTransport {
            base_transport: transport,
        }
    }
}

impl<'a, T: AsyncRead + AsyncWrite + Unpin + Send + Sync> AsyncRead for TempTransport<'a, T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut *self.base_transport).poll_read(cx, buf)
    }
}

impl<'a, T: AsyncRead + AsyncWrite + Unpin + Send + Sync> AsyncWrite for TempTransport<'a, T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut *self.base_transport).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut *self.base_transport).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut *self.base_transport).poll_shutdown(cx)
    }
}

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use crate::structures::transport::Transport;

pub struct TempTransport<'a> {
    base_transport: &'a mut Transport,
}

impl<'a> TempTransport<'a> {
    pub fn new(transport: &'a mut Transport) -> Self {
        TempTransport { base_transport: transport }
    }
}

impl<'a> AsyncRead for TempTransport<'a> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut *self.base_transport).poll_read(cx, buf)
    }
}

impl<'a> AsyncWrite for TempTransport<'a> {
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
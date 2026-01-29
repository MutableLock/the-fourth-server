use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_rustls::{client::TlsStream as ClientTlsStream, server::TlsStream as ServerTlsStream};

/// Unified transport wrapper, for different types of streams
pub struct Transport {
    inner: Box<dyn AsyncReadWrite>,
}

/// Trait object to unify AsyncRead + AsyncWrite
pub trait AsyncReadWrite: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static {}
impl<T: AsyncRead + AsyncWrite + ?Sized + Send + Sync + Unpin + 'static> AsyncReadWrite for T {}

impl Transport {
    /// Wrap a plain TcpStream
    pub fn plain(stream: TcpStream) -> Self {
        Self {
            inner: Box::new(stream),
        }
    }

    /// Wrap a server-side TLS stream
    pub fn tls_server(stream: ServerTlsStream<TcpStream>) -> Self {
        Self {
            inner: Box::new(stream),
        }
    }

    /// Wrap a client-side TLS stream
    pub fn tls_client(stream: ClientTlsStream<TcpStream>) -> Self {
        Self {
            inner: Box::new(stream),
        }
    }

    /// Optionally expose inner (if needed)
    pub fn inner(&mut self) -> &mut dyn AsyncReadWrite {
        &mut *self.inner
    }
}

impl AsyncRead for Transport {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut *self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for Transport {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut *self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut *self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut *self.inner).poll_shutdown(cx)
    }
}

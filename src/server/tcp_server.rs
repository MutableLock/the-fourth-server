use crate::server::server_router::TcpServerRouter;
use crate::structures::s_type;
use crate::structures::s_type::ServerErrorEn::InternalError;
use crate::structures::s_type::{PacketMeta, ServerErrorEn};
use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering::Relaxed},
};
use std::sync::atomic::Ordering;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::Mutex;

use futures_util::SinkExt;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::yield_now;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

/// TcpServer, responsible for listening and processing network traffic
///
#[derive(Clone)]
struct StreamData {
    stream: Arc<Mutex<Framed<TcpStream, LengthDelimitedCodec>>>,
    in_handle: Arc<AtomicBool>,
}

pub struct TcpServer {
    router: Arc<TcpServerRouter>,
    socket: Arc<TcpListener>,
    socket_in_handle: Arc<Mutex<HashMap<SocketAddr, StreamData>>>,
    shutdown_sig: Arc<AtomicBool>,
}

impl TcpServer {
    pub async fn new(bind_address: String, router: Arc<TcpServerRouter>) -> Self {
        Self {
            router,
            socket: Arc::new(
                TcpListener::bind(&bind_address)
                    .await
                    .expect("Failed to bind to address"),
            ),
            socket_in_handle: Arc::new(Mutex::new(HashMap::new())),
            shutdown_sig: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(self_ref: Arc<Mutex<Self>>) {
        let (listener, handle_sockets, router, shutdown_sig) = {
            let s = self_ref.lock().await;
            (
                s.socket.clone(),
                s.socket_in_handle.clone(),
                s.router.clone(),
                s.shutdown_sig.clone(),
            )
        };

        tokio::spawn(async move {
            loop {
                if shutdown_sig.load(Relaxed) {
                    break;
                }

                let _ = Self::accept_one_connection(&listener, &handle_sockets).await;
                let sockets = handle_sockets.clone();
                let router = router.clone();

                if sockets.lock().await.is_empty() {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }

                let to_spawn: Vec<_> = {
                    let mut sockets = sockets.lock().await;
                    sockets
                        .iter_mut()
                        .filter(|(_, data)| !data.in_handle.load(Relaxed))
                        .map(|(addr, data)| {
                            data.in_handle.store(true, Ordering::SeqCst);
                            (*addr, data.stream.clone(), data.in_handle.clone())
                        })
                        .collect()
                };


                for (addr, socket, flag) in to_spawn {
                    let router_handle = router.clone();
                    let sockets_handle = sockets.clone();
                    tokio::spawn(async move { Self::handle_connection(addr, socket, flag, router_handle.as_ref(), &sockets_handle).await });
                }

                for route in router.get_routes().iter() {
                    let req = route.1.lock().await.request_to_move_stream();
                    match req {
                        None => {}
                        Some(req) => {
                            let mut res = Vec::new();
                            for x in req.iter() {
                                let found_route = sockets.lock().await.remove(x);
                                if found_route.is_some() {
                                    let found_route = found_route.unwrap();

                                    res.push(found_route.stream);
                                }
                            }
                            if !res.is_empty() {
                                route.1.lock().await.accept_stream(res);
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn send_stop(&self) {
        self.shutdown_sig.store(true, Relaxed);
    }

    pub async fn accept_one_connection(
        listener: &TcpListener,
        connections: &Arc<Mutex<HashMap<SocketAddr, StreamData>>>,
    ) -> io::Result<()> {
        struct AcceptFuture<'a> {
            listener: &'a TcpListener,
        }
        impl<'a> std::future::Future for AcceptFuture<'a> {
            type Output = io::Result<Option<(tokio::net::TcpStream, SocketAddr)>>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                match self.listener.poll_accept(cx) {
                    Poll::Ready(Ok((stream, addr))) => Poll::Ready(Ok(Some((stream, addr)))),
                    Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                    Poll::Pending => Poll::Ready(Ok(None)), // no connection ready
                }
            }
        }

        // Create the future
        let fut = AcceptFuture { listener };

        // Await it first
        let result = fut.await?;

        // Then match
        match result {
            Some((stream, addr)) => {
                // New connection exists, register it
                stream.set_nodelay(true)?;

                let framed = Framed::new(stream, LengthDelimitedCodec::new());
                let stream_data = StreamData {
                    stream: Arc::new(Mutex::new(framed)),
                    in_handle: Arc::new(AtomicBool::new(false)),
                };

                connections.lock().await.insert(addr, stream_data);
            }
            None => {
                // No connection ready — just continue
                tokio::task::yield_now().await;
            }
        }

        Ok(())
    }

    /*
    async fn accept_new_connections(
        listener: &Arc<Mutex<TcpListener>>,
        connections: &Arc<Mutex<HashMap<SocketAddr, StreamData>>>,
    ) {

        while let Ok((stream, addr)) = listener.lock().await.accept().await {
            stream.set_nodelay(true).unwrap();
            let framed = Framed::new(stream, LengthDelimitedCodec::new());
            let stream_data = StreamData {
                stream: Arc::new(Mutex::new(framed)),
                in_handle: Arc::new(AtomicBool::new(false)),
            };
            connections.lock().await.insert(addr, stream_data);
        }
    }

     */

    async fn handle_connection(
        addr: SocketAddr,
        stream: Arc<Mutex<Framed<TcpStream, LengthDelimitedCodec>>>,
        active_flag: Arc<AtomicBool>,
        router: &TcpServerRouter,
        connections: &Arc<Mutex<HashMap<SocketAddr, StreamData>>>,
    ) {
        use futures_util::SinkExt;
        let mut stream = stream.lock().await;
        // Step 1: Receive meta
        let meta_data: Result<Option<BytesMut>, ()> =
            receive_message(addr.clone(), &mut stream, connections).await;
        if meta_data.is_err() {
            return;
        }
        let meta_data = meta_data.unwrap();
        if meta_data.is_none() {
            active_flag.store(false, Relaxed);
            return;
        }

        let meta_data = meta_data.unwrap();
        let has_payload = match s_type::from_slice::<PacketMeta>(meta_data.deref()) {
            Ok(meta) => meta.has_payload,
            Err(_) => false,
        };

        let mut payload: BytesMut= BytesMut::new();
        if has_payload {
            let payload_res = receive_message(addr.clone(), &mut stream, connections).await;
            if payload_res.is_err() {
                active_flag.store(false, Relaxed);
                return;
            }
            let payload_opt = payload_res.unwrap();
            if payload_opt.is_none() {
                let _ = stream.close().await;
                connections.lock().await.remove(&addr);
                return;
            }
            payload = payload_opt.unwrap();
        }
        let res = router.serve_packet(meta_data, payload, addr).await;
        let message = res.unwrap_or_else(|err|Bytes::from(s_type::to_vec(&err).unwrap()));
        let res = stream.send(message).await;
        match res {
            Err(_) => {
                let _ = stream.close();
                connections.lock().await.remove(&addr);
                return;
            }
            _ => {}
        }
        active_flag.store(false, Relaxed);
    }
}


async fn receive_message(
    addr: SocketAddr,
    stream: &mut Framed<TcpStream, LengthDelimitedCodec>,
    connections: &Arc<Mutex<HashMap<SocketAddr, StreamData>>>,
) -> Result<Option<BytesMut>, ()> {
    use futures_util::{StreamExt};
    match stream.next().await {
        Some(data) => match data {
            Ok(data) => {
                return Ok(Some(data));
            }
            Err(e) => {
                // This is where codec-level decoding errors happen
                match e.kind() {
                    // IO errors usually mean the connection is broken
                    std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::UnexpectedEof => {
                       let conn = connections.lock().await.remove(&addr);
                        let _ = conn.unwrap().stream.lock().await.close().await;
                        println!("Client disconnected");
                        return Err(());
                    }

                    // Frame too large (if you set max_frame_length)
                    std::io::ErrorKind::InvalidData => {
                        eprintln!("Frame exceeded maximum size: {e}");
                        return Err(());
                    }

                    // Other IO errors
                    _ => {
                        eprintln!("IO error while reading frame: {e}");
                        return Err(());

                    }
                }
            }
        },
        None => {
            return Ok(None);
        }
    }
}

// Custom Error Display
impl fmt::Display for ServerErrorEn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerErrorEn::MalformedMetaInfo(Some(msg)) => {
                write!(f, "Malformed meta info: {}", msg)
            }
            ServerErrorEn::MalformedMetaInfo(None) => write!(f, "Malformed meta info!"),
            ServerErrorEn::NoSuchHandler(Some(msg)) => write!(f, "No such handler: {}", msg),
            ServerErrorEn::NoSuchHandler(None) => write!(f, "No such handler!"),
            InternalError(Some(data)) => {
                write!(
                    f,
                    "{}",
                    String::from_utf8(data.clone())
                        .unwrap_or_else(|_| "Internal server error!".to_owned())
                )
            }
            InternalError(None) => {
                write!(f, "Internal server error!")
            }
            ServerErrorEn::PayloadLost => {
                write!(f, "Payload lost!")
            }
        }
    }
}

impl std::error::Error for ServerErrorEn {}

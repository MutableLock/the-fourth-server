use crate::server::server_router::TcpServerRouter;
use crate::structures::s_type;
use crate::structures::s_type::ServerErrorEn::InternalError;
use crate::structures::s_type::{PacketMeta, ServerErrorEn};
use std::collections::HashMap;
use std::fmt;
use std::io::{Error, Read, Write};
use std::net::SocketAddr;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering::Relaxed},
};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::{Mutex, Notify};

use crate::server::handler::Handler;
use futures_util::{SinkExt, StreamExt};
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

/*
#[derive(Clone)]
struct StreamData {
    stream: Arc<Mutex<Framed<TcpStream, LengthDelimitedCodec>>>,
    in_handle: Arc<AtomicBool>,
    processors: Arc<TrafficProcessorHolder>,
}

impl StreamData {
    pub async fn send(&self, message: BytesMut) {
        let message = self.processors.post_process_traffic(message).await.freeze();
        self.stream.lock().await.send(message).await.unwrap();
    }
    pub async fn next(&self) -> Option<Result<BytesMut, Error>> {
        let msg = self.stream.lock().await.next().await?;
        if msg.is_err() {
            return Some(Err(msg.unwrap_err()));
        }
        let msg = msg.unwrap();
        Some(Ok(self.processors.pre_process_traffic(msg).await))
    }
}


pub trait TrafficProcess {
    fn initial_connect(&mut self, source: Arc<Mutex<TcpStream>>);
    fn post_process_traffic(&mut self, data: BytesMut) -> BytesMut;
    fn pre_process_traffic(&mut self, data: BytesMut) -> BytesMut;
}

struct TrafficProcessorHolder {
    processors: Vec<Arc<Mutex<dyn TrafficProcess>>>,
    base_stream: Arc<Mutex<TcpStream>>,
}

impl TrafficProcessorHolder {
    fn initial_connect(&mut self, source: Arc<Mutex<TcpStream>>) {
        todo!()
    }

    pub async fn post_process_traffic(&self, mut data: BytesMut) -> BytesMut {
        for proc in self.processors.iter() {
            data = proc.lock().await.post_process_traffic(data);
        }
        data
    }

    pub async fn pre_process_traffic(&self, mut data: BytesMut) -> BytesMut {
        for proc in self.processors.iter() {
            data = proc.lock().await.pre_process_traffic(data);
        }
        data
    }
}
*/

pub type RequestChannel = (
    Sender<Arc<Mutex<dyn Handler>>>,
    Receiver<Arc<Mutex<dyn Handler>>>,
);

pub struct TcpServer {
    router: Arc<TcpServerRouter>,
    socket: Arc<TcpListener>,
    shutdown_sig: Arc<Notify>,
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
            shutdown_sig: Arc::new(Notify::new()),
        }
    }

    pub async fn start(self_ref: Arc<Self>) {
        let (listener, router, shutdown_sig) = {
            let s = self_ref;
            (
                s.socket.clone(),
                s.router.clone(),
                s.shutdown_sig.clone(),
            )
        };

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    res = listener.accept() => { if res.is_ok() {
                                    let stream = res.unwrap();
                                    let framed = Framed::new(stream.0, LengthDelimitedCodec::new());
                                let router = router.clone();
                                tokio::spawn(async move {
                                    Self::handle_connection(stream.1, framed, router.as_ref()).await;
                                });
                                } }
                    _ = shutdown_sig.notified() => break,
                }
            }
        });
    }

    pub fn send_stop(&self) {
        self.shutdown_sig.notify_one();
    }

    async fn handle_connection(
        addr: SocketAddr,
        mut stream: Framed<TcpStream, LengthDelimitedCodec>,
        router: &TcpServerRouter,
    ) {
        use futures_util::SinkExt;
        let mut move_sig = tokio::sync::oneshot::channel::<Arc<Mutex<dyn Handler>>>();
        let mut move_sig = (Some(move_sig.0), move_sig.1);
        loop {
            let meta_data: Result<Option<BytesMut>, bool> =
                receive_message(addr.clone(), &mut stream).await;
            if meta_data.is_err() {
                if meta_data.unwrap_err() {
                    stream.close().await.unwrap();
                    return;
                }
                continue;
            }
            let meta_data = meta_data.unwrap();
            if meta_data.is_none() {
                continue;
            }

            let meta_data = meta_data.unwrap();
            let has_payload = match s_type::from_slice::<PacketMeta>(meta_data.deref()) {
                Ok(meta) => meta.has_payload,
                Err(_) => false,
            };

            let mut payload: BytesMut = BytesMut::new();
            if has_payload {
                let payload_res = receive_message(addr.clone(), &mut stream).await;
                if payload_res.is_err() {
                    if payload_res.unwrap_err() {
                        stream.close().await.unwrap();
                        return;
                    }
                    continue;
                }
                let payload_opt = payload_res.unwrap();
                if payload_opt.is_none() {
                    let _ = stream.close().await;
                    return;
                }
                payload = payload_opt.unwrap();
            }
            let res = router
                .serve_packet(meta_data, payload, (addr, &mut move_sig.0))
                .await;

            let message = res.unwrap_or_else(|err| Bytes::from(s_type::to_vec(&err).unwrap()));
            let res = stream.send(message).await;

            if let Ok(requester) = move_sig.1.try_recv() {
                requester.lock().await.accept_stream(addr, stream);
                return;
            }

            match res {
                Err(_) => {
                    let _ = stream.close();
                    return;
                }
                _ => {}
            }
        }
    }
}

async fn receive_message(
    addr: SocketAddr,
    stream: &mut Framed<TcpStream, LengthDelimitedCodec>,
) -> Result<Option<BytesMut>, bool> {
    use futures_util::StreamExt;
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
                        println!("Client disconnected");
                        return Err(true);
                    }

                    // Frame too large (if you set max_frame_length)
                    std::io::ErrorKind::InvalidData => {
                        eprintln!("Frame exceeded maximum size: {e}");
                        return Err(false);
                    }

                    // Other IO errors
                    _ => {
                        eprintln!("IO error while reading frame: {e}");
                        return Err(false);
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

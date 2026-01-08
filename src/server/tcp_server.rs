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
use crate::structures::traffic_proc::TrafficProcessorHolder;
use futures_util::{SinkExt, StreamExt};
use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub type RequestChannel = (
    Sender<Arc<Mutex<dyn Handler>>>,
    Receiver<Arc<Mutex<dyn Handler>>>,
);

pub struct TcpServer {
    router: Arc<TcpServerRouter>,
    socket: Arc<TcpListener>,
    shutdown_sig: Arc<Notify>,
    processor: Option<TrafficProcessorHolder>,
}

impl TcpServer {
    pub async fn new(
        bind_address: String,
        router: Arc<TcpServerRouter>,
        processor: Option<TrafficProcessorHolder>,
    ) -> Self {
        Self {
            router,
            socket: Arc::new(
                TcpListener::bind(&bind_address)
                    .await
                    .expect("Failed to bind to address"),
            ),
            shutdown_sig: Arc::new(Notify::new()),
            processor,
        }
    }

    pub async fn start(&mut self) {
        let (listener, router, shutdown_sig) = {
            (
                self.socket.clone(),
                self.router.clone(),
                self.shutdown_sig.clone(),
            )
        };
        let mut processor = if let Some(proc) = self.processor.take() {
            proc
        } else {
            TrafficProcessorHolder::new()
        };
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    res = listener.accept() => { if res.is_ok() {
                                    let mut stream = res.unwrap();
                                    if processor.initial_connect(&mut stream.0).await {
                            let framed = Framed::new(stream.0, LengthDelimitedCodec::new());
                                    let router = router.clone();
                                    let prc_clone = processor.clone();
                                    tokio::spawn(async move {
                                        Self::handle_connection(stream.1, framed, router.as_ref(), prc_clone).await;
                                    });
                        } else {
                            stream.0.shutdown().await;
                        }

                                }
                    }
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
        mut processor: TrafficProcessorHolder,
    ) {
        use futures_util::SinkExt;
        let mut move_sig = tokio::sync::oneshot::channel::<Arc<Mutex<dyn Handler>>>();
        let mut move_sig = (Some(move_sig.0), move_sig.1);
        loop {
            let meta_data: Result<Option<BytesMut>, bool> =
                receive_message(addr.clone(), &mut stream, &mut processor).await;
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
                let payload_res = receive_message(addr.clone(), &mut stream, &mut processor).await;
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

            let message = res.unwrap_or_else(|err| s_type::to_vec(&err).unwrap());
            let res = send_message(&mut stream, message, &mut processor).await;
            if let Ok(requester) = move_sig.1.try_recv() {
                requester
                    .lock()
                    .await
                    .accept_stream(addr, (stream, processor.clone()));
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

async fn send_message(
    stream: &mut Framed<TcpStream, LengthDelimitedCodec>,
    message: Vec<u8>,
    processor: &mut TrafficProcessorHolder,
) -> Result<(), io::Error> {
    let message = Bytes::from(processor.post_process_traffic(message).await);
    stream.send(message).await
}

async fn receive_message(
    addr: SocketAddr,
    stream: &mut Framed<TcpStream, LengthDelimitedCodec>,
    processor: &mut TrafficProcessorHolder,
) -> Result<Option<BytesMut>, bool> {
    use futures_util::StreamExt;
    match stream.next().await {
        Some(data) => match data {
            Ok(mut data) => {
                data = processor.pre_process_traffic(data).await;
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

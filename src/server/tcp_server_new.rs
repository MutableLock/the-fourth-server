use crate::server::server_router::TcpServerRouter;
use crate::structures::s_type;
use crate::structures::s_type::ServerErrorEn::InternalError;
use crate::structures::s_type::{PacketMeta, ServerErrorEn};
use crate::util::rand_utils::generate_random_u8_vec;
use crate::util::thread_pool::ThreadPool;
use std::collections::HashMap;
use std::fmt;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::ops::Deref;
use std::sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicBool, Ordering::Relaxed},
};
use tungstenite::{Bytes, Error, Message, WebSocket};

/// TcpServer, responsible for listening and processing network traffic
///

struct StreamData {
    stream: Arc<Mutex<WebSocket<TcpStream>>>,
    in_handle: Arc<Mutex<AtomicBool>>,
}

pub struct TcpServer {
    router: Arc<TcpServerRouter>,
    socket: Arc<Mutex<TcpListener>>,
    socket_in_handle: Arc<Mutex<HashMap<SocketAddr, StreamData>>>,
    work_group: Arc<Mutex<ThreadPool>>,
    shutdown_sig: Arc<Mutex<AtomicBool>>,
}

impl TcpServer {
    pub fn new(
        bind_address: String,
        router: Arc<TcpServerRouter>,
        work_group: ThreadPool,
    ) -> Self {
        Self {
            router,
            socket: Arc::new(Mutex::new(
                TcpListener::bind(&bind_address).expect("Failed to bind to address"),
            )),
            socket_in_handle: Arc::new(Mutex::new(HashMap::new())),
            work_group: Arc::new(Mutex::new(work_group)),
            shutdown_sig: Arc::new(Mutex::new(AtomicBool::new(false))),
        }
    }

    pub fn start(self_ref: Arc<Mutex<Self>>) {
        let listener = self_ref.lock().unwrap().socket.clone();
        listener.lock().unwrap().set_nonblocking(true).unwrap();

        let handle_sockets = self_ref.lock().unwrap().socket_in_handle.clone();
        let router = self_ref.lock().unwrap().router.clone();
        let work_group = self_ref.lock().unwrap().work_group.clone();
        let shutdown_sig = self_ref.lock().unwrap().shutdown_sig.clone();
        let thread_pool = work_group.clone();

        work_group.lock().unwrap().execute(move || {
            loop {
                if shutdown_sig.lock().unwrap().load(Relaxed) {
                    break;
                }

                Self::accept_new_connections(&listener, &handle_sockets);

                let sockets = handle_sockets.clone();
                let router = router.clone();

                sockets.lock().unwrap().retain(|addr, data| {
                    if data.in_handle.lock().unwrap().load(Relaxed) {
                        return true; // skip if already being handled
                    }

                    let socket = data.stream.clone();
                    let flag = data.in_handle.clone();
                    flag.lock().unwrap().store(true, Relaxed);

                    let thread_pool = thread_pool.clone();
                    let router = router.clone();
                    let sockets = sockets.clone();
                    let addr = *addr;

                    thread_pool.lock().unwrap().execute(move || {
                        Self::handle_connection(addr, socket, flag, &router, &sockets);
                    });
                    true
                });
                router.get_routes().iter().for_each(|route| {
                    let req = route.1.lock().unwrap().request_to_move_stream();
                    match req {
                        None => {}
                        Some(req) => {
                            let mut res = Vec::new();
                            req.iter().for_each(|x| {
                                let found_route = sockets.lock().unwrap().remove(x);
                                if found_route.is_some() {
                                    res.push(found_route.unwrap().stream);
                                }
                            });
                            if !res.is_empty() {
                                route.1.lock().unwrap().accept_stream(res);
                            }
                        }
                    }
                })
            }
        });
    }

    pub fn send_stop(&self) {
        self.shutdown_sig.lock().unwrap().store(true, Relaxed);
    }

    fn accept_new_connections(
        listener: &Arc<Mutex<TcpListener>>,
        connections: &Arc<Mutex<HashMap<SocketAddr, StreamData>>>,
    ) {
        while let Ok((stream, addr)) = listener.lock().unwrap().accept() {
            stream.set_nodelay(true).unwrap();
            let stream = tungstenite::accept(stream);
            if stream.is_ok() {
                let stream = stream.unwrap();
                let stream_data = StreamData {
                    stream: Arc::new(Mutex::new(stream)),
                    in_handle: Arc::new(Mutex::new(AtomicBool::new(false))),
                };
                connections.lock().unwrap().insert(addr, stream_data);
            }

        }
    }

    fn handle_connection(
        addr: SocketAddr,
        stream: Arc<Mutex<WebSocket<TcpStream>>>,
        active_flag: Arc<Mutex<AtomicBool>>,
        router: &TcpServerRouter,
        connections: &Arc<Mutex<HashMap<SocketAddr, StreamData>>>,
    ) {
        let mut stream = stream.lock().unwrap();
        // Step 1: Receive meta
        let meta_data: Result<Option<Vec<u8>>, ()> =
            receive_message(addr.clone(), &mut stream, connections);
        if meta_data.is_err() {
            return;
        }
        let meta_data = meta_data.unwrap();
        if meta_data.is_none() {
            active_flag.lock().unwrap().store(false, Relaxed);
            return;
        }

        let meta_data = meta_data.unwrap();
        let has_payload = match s_type::from_slice::<PacketMeta>(meta_data.deref()) {
            Ok(meta) => meta.has_payload,
            Err(_) => false,
        };

        let mut payload: Vec<u8> = Vec::new();
        if has_payload {
            let payload_res = receive_message(addr.clone(), &mut stream, connections);
            if payload_res.is_err() {
                active_flag.lock().unwrap().store(false, Relaxed);
                return;
            }
            let payload_opt = payload_res.unwrap();
            if payload_opt.is_none() {
                let _ = stream.close(None);
                connections.lock().unwrap().remove(&addr);
                return;
            }
            payload = payload_opt.unwrap();
        }
        let res = router.serve_packet(meta_data, payload, addr);

        let message = match res {
            Ok(data) => Message::Binary(Bytes::from(data)),
            Err(err) => Message::Binary(Bytes::from(s_type::to_vec(&err).unwrap())),
        };
        let res = stream.send(message);
        match res {
            Err(_) => {
                let _ = stream.close(None);
                connections.lock().unwrap().remove(&addr);
                return;
            }
            _ => {}
        }
        active_flag.lock().unwrap().store(false, Relaxed);
    }
}

fn bytes_into_vec(b: tungstenite::Bytes) -> Vec<u8> {
    match b.try_into() {
        Ok(vec) => vec, // zero-copy if unique
        _ => Vec::new(),
    }
}

fn receive_message(
    addr: SocketAddr,
    stream: &mut MutexGuard<WebSocket<TcpStream>>,
    connections: &Arc<Mutex<HashMap<SocketAddr, StreamData>>>,
) -> Result<Option<Vec<u8>>, ()> {
    match stream.read() {
        Ok(data) => match data {
            Message::Text(_) => {
                let _ = stream.close(None);
                connections.lock().unwrap().remove(&addr);
                return Err(());
            }
            Message::Binary(data) => {
                return Ok(Some(bytes_into_vec(data)));
            }
            Message::Ping(_) => {
                let len = rand::random_range(1..124);
                let res = stream.send(Message::Pong(Bytes::from(generate_random_u8_vec(len))));
                return match res {
                    Ok(_) => {
                        Ok(None)
                    }
                    Err(_) => {
                        let _ = stream.close(None);
                        connections.lock().unwrap().remove(&addr);
                        Err(())
                    }
                };
            }
            Message::Pong(_) => {
                let len = rand::random_range(1..124);
                let res = stream.send(Message::Ping(Bytes::from(generate_random_u8_vec(len))));
                return match res {
                    Ok(_) => {
                        Ok(None)
                    }
                    Err(_) => {
                        let _ = stream.close(None);
                        connections.lock().unwrap().remove(&addr);
                        Err(())
                    }
                };
            }
            Message::Close(_) => {
                let _ = stream.close(None);
                connections.lock().unwrap().remove(&addr);
                return Err(());
            }
            Message::Frame(_) => {
                let _ = stream.close(None);
                connections.lock().unwrap().remove(&addr);
                return Err(());
            }
        },
        Err(e) => {
            return match e {
                Error::ConnectionClosed => {
                    let _ = stream.close(None);
                    connections.lock().unwrap().remove(&addr);
                    Err(())
                }
                Error::AlreadyClosed => {
                    let _ = stream.close(None);
                    connections.lock().unwrap().remove(&addr);
                    Err(())
                }
                Error::Io(_) => Ok(None),
                Error::Tls(_) => {
                    let _ = stream.close(None);
                    connections.lock().unwrap().remove(&addr);
                    Err(())
                }
                Error::Capacity(_) => {
                    let _ = stream.close(None);
                    connections.lock().unwrap().remove(&addr);
                    Err(())
                }
                Error::Protocol(_) => {
                    let _ = stream.close(None);
                    connections.lock().unwrap().remove(&addr);
                    Err(())
                }
                Error::WriteBufferFull(_) => {
                    let _ = stream.close(None);
                    connections.lock().unwrap().remove(&addr);
                    Err(())
                }
                Error::Utf8(_) => {
                    let _ = stream.close(None);
                    connections.lock().unwrap().remove(&addr);
                    Err(())
                }
                Error::AttackAttempt => {
                    let _ = stream.close(None);
                    connections.lock().unwrap().remove(&addr);
                    Err(())
                }
                Error::Url(_) => {
                    let _ = stream.close(None);
                    connections.lock().unwrap().remove(&addr);
                    Err(())
                }
                Error::Http(_) => {
                    let _ = stream.close(None);
                    connections.lock().unwrap().remove(&addr);
                    Err(())
                }
                Error::HttpFormat(_) => {
                    let _ = stream.close(None);
                    connections.lock().unwrap().remove(&addr);
                    Err(())
                }
            };
        }
    };
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

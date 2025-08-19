//Pretty shitty implementations of client at this time, if u have ideas feel free to contact me

use crate::structures::s_type;
use crate::structures::s_type::{
    HandlerMetaAns, HandlerMetaReq, PacketMeta, StructureType, SystemSType,
};
use std::collections::HashMap;
use tungstenite::Bytes;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Relaxed, Release};
use std::thread::{spawn, JoinHandle};
use tungstenite::{Message, WebSocket};
use tungstenite::stream::MaybeTlsStream;

pub struct ClientConnection {
    socket: Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>,
    receivers: Arc<Vec<Arc<Mutex<dyn Receiver>>>>,
    running: Arc<Mutex<AtomicBool>>,
    current_thread: Option<JoinHandle<()>>,
}

pub trait Receiver: Send + Sync {
    fn get_handler_name(&self) -> String;
    fn get_request(&mut self) -> Option<(Vec<u8>, Box<dyn StructureType>)>;
    fn receive_response(&mut self, response: Vec<u8>);
}

impl ClientConnection {
    /**
    @param receivers - every receiver must have unique handler name,
    if there is two or more receivers with the same handler name,
    the requested packets will be routed only at one receiver!!!
    */
    pub fn new(connection_dest: String, receivers: Vec<Arc<Mutex<dyn Receiver>>>) -> Self {
        let socket = tungstenite::connect(&connection_dest).unwrap().0;

        Self {
            socket: Arc::new(Mutex::new(socket)),
            receivers: Arc::new(receivers),
            running: Arc::new(Mutex::new(AtomicBool::new(true))),
            current_thread: None,
        }
    }

    pub fn start(&mut self) {
        let receivers = self.receivers.clone();
        let socket = self.socket.clone();
        let running = self.running.clone();
        self.current_thread = Some(spawn(move || {
            let mut receivers_mapped: HashMap<u64, Arc<Mutex<dyn Receiver>>> = HashMap::new();

            loop {
                if !running.lock().unwrap().load(Relaxed){
                    break;
                }
                if receivers_mapped.is_empty() {
                    Self::register_receivers(&receivers, &socket, &mut receivers_mapped);
                }

                Self::process_requests(&receivers_mapped, &socket);
            }
        }));
    }

    pub fn stop(&mut self) {
        self.running.lock().unwrap().store(false, Release);
    }

    pub fn stop_and_move_stream(&mut self) -> Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>{
        self.running.lock().unwrap().store(false, Release);
        self.current_thread.take().unwrap().join().unwrap();
        self.socket.clone()
    }

    fn register_receivers(
        receivers: &Arc<Vec<Arc<Mutex<dyn Receiver>>>>,
        socket: &Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>,
        receivers_mapped: &mut HashMap<u64, Arc<Mutex<dyn Receiver>>>,
    ) {
        for receiver in receivers.iter() {
            let meta_req = HandlerMetaReq {
                s_type: SystemSType::HandlerMetaReq,
                handler_name: receiver.lock().unwrap().get_handler_name(),
            };

            let data = s_type::to_vec(&meta_req).unwrap();
            socket.lock().unwrap().send(Message::Binary(Bytes::from(data))).unwrap();

            let data = Self::wait_for_data(socket);
            let meta_ans = s_type::from_slice::<HandlerMetaAns>(&data).unwrap();

            receivers_mapped.insert(meta_ans.id, receiver.clone());
        }
    }

    fn process_requests(
        receivers_mapped: &HashMap<u64, Arc<Mutex<dyn Receiver>>>,
        socket: &Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>,
    ) {
        for (handler_id, receiver) in receivers_mapped.iter() {
            let mut receiver_lock = receiver.lock().unwrap();
            if let Some((payload, structure)) = receiver_lock.get_request() {
                let meta = PacketMeta {
                    s_type: SystemSType::PacketMeta,
                    s_type_req: structure.get_serialize_function()(structure.clone_unique()),
                    handler_id: *handler_id,
                    has_payload: !payload.is_empty(),
                };

                let meta_bytes = s_type::to_vec(&meta).unwrap();
                socket.lock().unwrap().send(Message::Binary(Bytes::from(meta_bytes))).unwrap();
                println!("{}", payload.len());
                socket.lock().unwrap().send(Message::Binary(Bytes::from(payload))).unwrap();
                let response = Self::wait_for_data(socket);
                receiver_lock.receive_response(response);
            }
        }
    }

    fn wait_for_data(
        socket: &Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>,
    ) -> Vec<u8> {
        let mut res = socket.lock().unwrap().read();
        while res.is_err() {
            let err = res.unwrap_err();
            match err {
                tungstenite::Error::Io(_) => {}
                _ => {
                    panic!("Unexpected error reading from socket");
                }
            }
            res = socket.lock().unwrap().read();
        }
        res.unwrap().into_data().to_vec()
    }
}
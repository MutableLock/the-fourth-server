//Pretty shitty implementations of client at this time, if u have ideas feel free to contact me
use crate::structures::s_type;
use crate::structures::s_type::{
    HandlerMetaAns, HandlerMetaReq, PacketMeta, StructureType, SystemSType,
};
use crate::structures::traffic_proc::TrafficProcessorHolder;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Relaxed, Release};
use std::time::Duration;
use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_rustls::rustls::{ClientConfig, ClientConnection, StreamOwned};
use tokio_rustls::TlsConnector;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder, Framed, LengthDelimitedCodec};
use crate::structures::transport::Transport;

pub struct ClientConnect<C>
where
    C: Encoder<Bytes, Error = io::Error>
    + Decoder<Item = BytesMut, Error = io::Error>
    + Clone
    + Send
    + Sync
    + 'static, {
    socket: Arc<Mutex<Framed<Transport, C>>>,
    receivers: Arc<Vec<Arc<Mutex<dyn Receiver>>>>,
    running: Arc<AtomicBool>,
    current_thread: Option<JoinHandle<()>>,
    processor: Option<TrafficProcessorHolder<C>>,
    
}
#[async_trait]
pub trait Receiver: Send + Sync {
    async fn get_handler_name(&self) -> String;
    async fn get_request(&mut self) -> Option<(Vec<u8>, Box<dyn StructureType>)>;
    async fn receive_response(&mut self, response: BytesMut);
}

impl<C> ClientConnect<C>
where
    C: Encoder<Bytes, Error = io::Error>
    + Decoder<Item = BytesMut, Error = io::Error>
    + Clone
    + Send
    + Sync
    + 'static, {
    /**
    @param receivers - every receiver must have unique handler name,
    if there is two or more receivers with the same handler name,
    the requested packets will be routed only at one receiver!!!
    */
    pub async fn new(
        server_name: String,
        connection_dest: String,
        receivers: Vec<Arc<Mutex<dyn Receiver>>>,
        mut processor: Option<TrafficProcessorHolder<C>>,
        codec: C,
        client_config: Option<ClientConfig>
    ) -> Self {
        let mut socket = TcpStream::connect(connection_dest).await.unwrap();
        socket.set_nodelay(true).unwrap();
        let mut socket = if let Some(client_config) = client_config {
            let connector = TlsConnector::from(Arc::new(client_config));
            let res = connector.connect(server_name.try_into().unwrap(), socket).await.unwrap();
            Transport::TlsClient(res)
        } else {
            Transport::Plain(socket)
        };


        if processor.is_some() {
            if !processor
                .as_mut()
                .unwrap()
                .initial_connect(&mut socket)
                .await
            {
                panic!("Failed to connect to processor");
            }
        }
        let mut socket = Framed::new(socket, codec);
        if processor.is_some() {
            if !processor
                .as_mut()
                .unwrap()
                .initial_framed_connect(&mut socket)
                .await
            {
                panic!("Failed to connect to processor");
            }
        }
        Self {
            socket: Arc::new(Mutex::new(socket)),
            receivers: Arc::new(receivers),
            running: Arc::new(AtomicBool::new(true)),
            current_thread: None,
            processor,
        }
    }

    pub async fn start(&mut self) {
        let receivers = self.receivers.clone();
        let socket = self.socket.clone();
        let running = self.running.clone();
        let mut processor = if let Some(processor) = self.processor.take() {
            processor
        } else {
            TrafficProcessorHolder::new()
        };
        self.current_thread = Some(tokio::spawn(async move {
            let mut receivers_mapped: HashMap<u64, Arc<Mutex<dyn Receiver>>> = HashMap::new();

            loop {
                if !running.load(Relaxed) {
                    break;
                }
                if receivers_mapped.is_empty() {
                    Self::register_receivers(
                        &receivers,
                        &socket,
                        &mut receivers_mapped,
                        &mut processor,
                    )
                    .await;
                } else {
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                }

                Self::process_requests(&receivers_mapped, &socket, &mut processor).await;
            }
        }));
    }

    pub fn stop(&mut self) {
        self.running.store(false, Release);
    }

    pub fn stop_and_move_stream(&mut self) -> Arc<Mutex<Framed<Transport, C>>> {
        self.running.store(false, Release);
        self.socket.clone()
    }

    async fn register_receivers(
        receivers: &Arc<Vec<Arc<Mutex<dyn Receiver>>>>,
        socket: &Arc<Mutex<Framed<Transport, C>>>,
        receivers_mapped: &mut HashMap<u64, Arc<Mutex<dyn Receiver>>>,
        processor: &mut TrafficProcessorHolder<C>,
    ) {
        use futures_util::SinkExt;

        for receiver in receivers.iter() {
            let meta_req = HandlerMetaReq {
                s_type: SystemSType::HandlerMetaReq,
                handler_name: receiver.lock().await.get_handler_name().await,
            };

            let data = s_type::to_vec(&meta_req).unwrap();
            let data = processor.post_process_traffic(data).await;

            socket
                .lock()
                .await
                .send(Bytes::from(data))
                .await
                .expect("Failed to send");

            let mut data = processor
                .pre_process_traffic(Self::wait_for_data(socket).await)
                .await;

            let meta_ans = s_type::from_slice::<HandlerMetaAns>(data.as_mut()).unwrap();

            receivers_mapped.insert(meta_ans.id, receiver.clone());
        }
    }

    async fn process_requests(
        receivers_mapped: &HashMap<u64, Arc<Mutex<dyn Receiver>>>,
        socket: &Arc<Mutex<Framed<Transport, C>>>,
        processor: &mut TrafficProcessorHolder<C>,
    ) {
        use futures_util::SinkExt;

        for (handler_id, receiver) in receivers_mapped.iter() {
            let mut receiver_lock = receiver.lock().await;
            if let Some((payload, structure)) = receiver_lock.get_request().await {
                let meta = PacketMeta {
                    s_type: SystemSType::PacketMeta,
                    s_type_req: structure.get_serialize_function()(structure.clone_unique()),
                    handler_id: *handler_id,
                    has_payload: !payload.is_empty(),
                };

                let meta_bytes = processor
                    .post_process_traffic(s_type::to_vec(&meta).unwrap())
                    .await;
                let payload = processor.post_process_traffic(payload).await;
                socket
                    .lock()
                    .await
                    .send(Bytes::from(meta_bytes))
                    .await
                    .unwrap();
                println!("{}", payload.len());
                socket
                    .lock()
                    .await
                    .send(Bytes::from(payload))
                    .await
                    .unwrap();
                let response = processor
                    .pre_process_traffic(Self::wait_for_data(socket).await)
                    .await;

                receiver_lock.receive_response(response).await;
            }
        }
    }

    async fn wait_for_data(
        socket: &Arc<Mutex<Framed<Transport, C>>>,
    ) -> BytesMut {
        use futures_util::StreamExt;

        let mut res = socket.lock().await.next().await;
        while res.is_none() {
            res = socket.lock().await.next().await;
        }
        res.unwrap().unwrap()
    }
}

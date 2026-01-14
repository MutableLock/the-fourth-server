use crate::structures::s_type::StructureType;
use crate::structures::traffic_proc::TrafficProcessorHolder;
use crate::structures::transport::Transport;
use async_trait::async_trait;
use std::io;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{Mutex, mpsc};
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::ClientConfig;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

pub struct ClientConnect {}
#[async_trait]
pub trait DataConsumer {
    async fn response_received(
        &mut self,
        request: DataRequest,
        handler_id: u64,
        response: BytesMut,
    );
}

pub struct HandlerInfo {
    id: Option<u64>,
    named: Option<String>,
}

impl HandlerInfo {
    pub fn new_named(name: String) -> Self {
        Self {
            id: None,
            named: Some(name),
        }
    }

    pub fn new_id(id: u64) -> Self {
        Self {
            id: Some(id),
            named: None,
        }
    }

    pub fn id(&self) -> Option<u64> {
        self.id
    }

    pub fn named(&self) -> &Option<String> {
        &self.named
    }
}

pub struct DataRequest {
    pub handler_info: HandlerInfo,
    pub data: Vec<u8>,
    pub s_type: Box<dyn StructureType>,
}

struct ClientRequest {
    req: DataRequest,
    consumer: Arc<Mutex<dyn DataConsumer>>,
}

impl ClientConnect {
    pub async fn new<
        C: Encoder<Bytes, Error = io::Error>
            + Decoder<Item = BytesMut, Error = io::Error>
            + Clone
            + Send
            + Sync
            + 'static,
    >(
        server_name: String,
        connection_dest: String,
        mut processor: Option<TrafficProcessorHolder<C>>,
        codec: C,
        client_config: Option<ClientConfig>,
        max_request_in_time: usize,
    ) -> Self {
        let mut socket = TcpStream::connect(connection_dest).await.unwrap();
        socket.set_nodelay(true).unwrap();
        let mut socket = if let Some(client_config) = client_config {
            let connector = TlsConnector::from(Arc::new(client_config));
            let res = connector
                .connect(server_name.try_into().unwrap(), socket)
                .await
                .unwrap();
            Transport::tls_client(res)
        } else {
            Transport::plain(socket)
        };
        let (tx, rx) = mpsc::channel::<ClientRequest>(max_request_in_time);
    }

    async fn connection_main<
        C: Encoder<Bytes, Error = io::Error>
            + Decoder<Item = BytesMut, Error = io::Error>
            + Clone
            + Send
            + Sync
            + 'static,
    >(
        socket: Transport,
        processor: Option<TrafficProcessorHolder<C>>,
        mut rx: Receiver<ClientRequest>,
        shutdown_sig: tokio::sync::oneshot::Sender<()>,
    ) {
        let processor = processor.unwrap_or(TrafficProcessorHolder::new());
        tokio::spawn(async move {
            loop{
                while let Some(request) = rx.recv().await {}
            }
        });
    }
}

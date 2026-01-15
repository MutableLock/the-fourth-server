pub mod target_router;

use crate::client::target_router::TargetRouter;
use crate::structures::s_type;
use crate::structures::s_type::{PacketMeta, StructureType, SystemSType};
use crate::structures::traffic_proc::TrafficProcessorHolder;
use crate::structures::transport::Transport;
use async_trait::async_trait;
use futures_util::SinkExt;
use std::io;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{Mutex, mpsc};
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::ClientConfig;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder, Framed};

pub struct ClientConnect {
    tx: Sender<ClientRequest>,
}
#[async_trait]
pub trait DataConsumer: Send + Sync {
    async fn response_received(&mut self, handler_id: u64, response: BytesMut);
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

pub struct ClientRequest {
    pub req: DataRequest,
    pub consumer: Arc<Mutex<dyn DataConsumer>>,
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
        let mut socket = Framed::new(
            if let Some(client_config) = client_config {
                let connector = TlsConnector::from(Arc::new(client_config));
                let res = connector
                    .connect(server_name.try_into().unwrap(), socket)
                    .await
                    .unwrap();
                Transport::tls_client(res)
            } else {
                Transport::plain(socket)
            },
            codec,
        );
        let (tx, rx) = mpsc::channel::<ClientRequest>(max_request_in_time);
        Self::connection_main(socket, processor, rx).await;
        Self { tx }
    }

    pub async fn dispatch_request(&self, request: ClientRequest) {
        self.tx.send(request).await.unwrap();
    }

    async fn connection_main<
        C: Encoder<Bytes, Error = io::Error>
            + Decoder<Item = BytesMut, Error = io::Error>
            + Clone
            + Send
            + Sync
            + 'static,
    >(
        mut socket: Framed<Transport, C>,
        processor: Option<TrafficProcessorHolder<C>>,
        mut rx: Receiver<ClientRequest>,
    ) {
        let mut processor = processor.unwrap_or(TrafficProcessorHolder::new());
        let mut router = TargetRouter::new();
        tokio::spawn(async move {
            loop {
                while let Some(request) = rx.recv().await {
                    Self::process_request(request, &mut socket, &mut processor, &mut router).await;
                }
            }
        });
    }

    async fn process_request<
        C: Encoder<Bytes, Error = io::Error>
            + Decoder<Item = BytesMut, Error = io::Error>
            + Clone
            + Send
            + Sync
            + 'static,
    >(
        request: ClientRequest,
        socket: &mut Framed<Transport, C>,
        processor: &mut TrafficProcessorHolder<C>,
        target_router: &mut TargetRouter,
    ) {
        let handler_id = if let Some(id) = request.req.handler_info.id() {
            id
        } else {
            let named = request.req.handler_info.named.unwrap();
            target_router
                .request_route(named.as_str(), socket, processor)
                .await
                .unwrap()
        };
        let meta = PacketMeta {
            s_type: SystemSType::PacketMeta,
            s_type_req: request.req.s_type.get_serialize_function()(request.req.s_type),
            handler_id,
            has_payload: !request.req.data.is_empty(),
        };
        let meta_bytes = processor
            .post_process_traffic(s_type::to_vec(&meta).unwrap())
            .await;
        let payload = processor.post_process_traffic(request.req.data).await;
        socket.send(Bytes::from(meta_bytes)).await.unwrap();
        socket.send(Bytes::from(payload)).await.unwrap();
        let response = processor
            .pre_process_traffic(wait_for_data(socket).await)
            .await;
        request
            .consumer
            .lock()
            .await
            .response_received(handler_id, response)
            .await;
    }
}

pub async fn wait_for_data<
    C: Encoder<Bytes, Error = io::Error>
        + Decoder<Item = BytesMut, Error = io::Error>
        + Clone
        + Send
        + Sync
        + 'static,
>(
    socket: &mut Framed<Transport, C>,
) -> BytesMut {
    use futures_util::StreamExt;

    let mut res = socket.next().await;
    while res.is_none() {
        res = socket.next().await;
    }
    res.unwrap().unwrap()
}

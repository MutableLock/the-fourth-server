use std::net::SocketAddr;
use std::sync::{Arc};
use async_trait::async_trait;
use tokio::io;
use tokio::sync::{Mutex};
use tokio::net::TcpStream;
use tokio::sync::oneshot::Sender;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder, Framed, LengthDelimitedCodec};
use crate::codec::codec_trait::TfCodec;
use crate::structures::s_type::StructureType;
use crate::structures::traffic_proc::TrafficProcessorHolder;
use crate::structures::transport::Transport;

#[async_trait]
pub trait Handler: Send + Sync {
    type Codec: Encoder<Bytes, Error = io::Error> + Decoder<Item = BytesMut, Error = io::Error> + Clone + Send  + Sync + 'static + TfCodec;
    async fn serve_route(
        &mut self,
        /*If request needed, call take() on this option
        *if let Some(tx) = meta.1.take(){
        *            tx.send(...).unwrap();
        *        }
        */
        client_meta: (SocketAddr,  &mut Option<Sender<Arc<Mutex<dyn Handler<Codec = Self::Codec>>>>>),
        s_type: Box<dyn StructureType>,
        data: BytesMut,
    ) -> Result<Vec<u8>, Vec<u8>>;
    
    async fn accept_stream(&mut self, add: SocketAddr, stream: (Framed<Transport, Self::Codec>, TrafficProcessorHolder<Self::Codec>));

}

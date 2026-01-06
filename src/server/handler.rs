use std::net::SocketAddr;
use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::net::TcpStream;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use crate::structures::s_type::StructureType;

pub trait Handler: Send + Sync {
    fn serve_route(
        &mut self,
        client_meta: SocketAddr,
        s_type: Box<dyn StructureType>,
        data: BytesMut,
    ) -> Result<Bytes, Bytes>;

    /**
    This function is needed to request the connected stream to handle it manually,
    if the requested stream exists and not disconnect, the result will be passed into accept_stream
     */
    fn request_to_move_stream(&self) -> Option<Vec<SocketAddr>>;

    fn accept_stream(&mut self, stream: Vec<Arc<Mutex<Framed<TcpStream, LengthDelimitedCodec>>>>);

}

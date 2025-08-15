use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use tungstenite::WebSocket;
use crate::structures::s_type::StructureType;

pub trait Handler: Send + Sync {
    fn serve_route(
        &mut self,
        client_meta: SocketAddr,
        s_type: Box<dyn StructureType>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, Vec<u8>>;

    /**
    This function is needed to request the connected stream to handle it manually,
    if the requested stream exists and not disconnect, the result will be passed into accept_stream
     */
    fn request_to_move_stream(&self) -> Option<Vec<SocketAddr>>;

    fn accept_stream(&mut self, stream: Vec<Arc<Mutex<WebSocket<TcpStream>>>>);

}

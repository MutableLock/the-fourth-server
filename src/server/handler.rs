use std::net::SocketAddr;
use std::sync::{Arc};
use tokio::sync::{Mutex, Notify};
use tokio::net::TcpStream;
use tokio::sync::oneshot::Sender;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use crate::structures::s_type::StructureType;

pub trait Handler: Send + Sync {
    fn serve_route(
        &mut self,
        /*If request needed, call take() on this option
        *if let Some(tx) = meta.1.take(){
        *            tx.send(...).unwrap();
        *        }
        */
        client_meta: (SocketAddr,  &mut Option<Sender<Arc<Mutex<dyn Handler>>>>),
        s_type: Box<dyn StructureType>,
        data: BytesMut,
    ) -> Result<Bytes, Bytes>;
    
    fn accept_stream(&mut self, add: SocketAddr, tream: Framed<TcpStream, LengthDelimitedCodec>);

}

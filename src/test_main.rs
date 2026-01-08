pub mod util;
pub mod structures;
pub mod server;
pub mod client;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
pub use openssl;
pub use bincode;
pub use sha2;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::sync::oneshot::Sender;
use tokio::time::sleep;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use crate::server::handler::Handler;
use crate::server::server_router::TcpServerRouter;
use crate::server::tcp_server::TcpServer;
use crate::structures::s_type;
use crate::structures::s_type::StructureType;
use crate::testing::test_client::init_client;
use crate::testing::test_s_type::{InitialRequest, InitialResponse, PayloadRequest, PayloadResponse, TestError, TestStructureType};

mod testing;

struct TestHandler{
    moved_streams: Vec<Framed<TcpStream, LengthDelimitedCodec>>,
}


impl Handler for TestHandler {
    fn serve_route(
        &mut self,
        _: (SocketAddr,  &mut Option<Sender<Arc<Mutex<dyn Handler>>>>),
        s_type: Box<dyn StructureType>,
        mut data: BytesMut
    ) -> Result<Bytes, Bytes> {
        let base_s_type = s_type.as_any().downcast_ref::<TestStructureType>().unwrap();
        if data.is_empty() {
            let test_error = TestError {
                s_type: TestStructureType::TestError,
                error: "no meta!".to_string(),
            };
            println!("No meta server");
            return Err(s_type::to_vec(&test_error).unwrap().into());
        }

        match base_s_type {
            TestStructureType::InitialRequest => {
                let request = s_type::from_slice::<InitialRequest>(data.as_mut()).unwrap();
                let response = InitialResponse {
                    s_type: TestStructureType::InitialResponse,
                    response: request.request * 5,
                };
                println!("Success server");
                return Ok(s_type::to_vec(&response).unwrap().into());
            }
            TestStructureType::PayloadRequest => {
                let request = s_type::from_slice::<PayloadRequest>(data.as_mut()).unwrap();
                let mut response = PayloadResponse {
                    s_type: TestStructureType::PayloadResponse,
                    medium_payload: request.medium_payload.clone(),
                    request: request.request,
                };
                response.medium_payload.sort();
                println!("Success server");
                return Ok(s_type::to_vec(&response).unwrap().into());
            }
            TestStructureType::HighPayloadRequest => {
                //let request = s_type::from_slice::<HighPayloadRequest>(data.as_slice()).unwrap();

                let test_error = TestError {
                    s_type: TestStructureType::TestError,
                    error: "TestError".to_string(),
                };
                println!("Success server");
                return Err(s_type::to_vec(&test_error).unwrap().into());
            }
            _ => {
                let test_error = TestError {
                    s_type: TestStructureType::TestError,
                    error: "TestError".to_string(),
                };
                println!("Success server");
                return Err(s_type::to_vec(&test_error).unwrap().into());
            }
        }
    }

    fn accept_stream(&mut self, add: SocketAddr, stream: Framed<TcpStream, LengthDelimitedCodec>) {
        self.moved_streams.push(stream);
    }
}
#[tokio::main]
pub async fn main() {
    let mut router = TcpServerRouter::new(Box::new(TestStructureType::HighPayloadRequest));
    router.add_route(
        Arc::new(Mutex::new(TestHandler {moved_streams: Vec::new()})),
        "TestHandler".to_string(),
        vec![
            Box::new(TestStructureType::InitialRequest),
            Box::new(TestStructureType::PayloadRequest),
            Box::new(TestStructureType::HighPayloadRequest),
        ],
    );
    router.commit_routes();
    let router = Arc::new(router);


    let server = Arc::new(TcpServer::new(
        "127.0.0.1:3333".to_string(),
        router,
    ).await);

    TcpServer::start(server.clone()).await;
    let mut client = init_client().await;
    client.start().await;

    sleep(Duration::from_millis(1500)).await;
    server.send_stop();
    client.stop();
    println!("sended stop waiting before exit");
    sleep(Duration::from_millis(1500)).await;
    println!("now the process will need to shutdown, if not this is trouble");
}
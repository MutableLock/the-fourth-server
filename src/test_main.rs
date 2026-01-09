pub mod client;
pub mod server;
pub mod structures;
pub mod util;

use crate::server::handler::Handler;
use crate::server::server_router::TcpServerRouter;
use crate::server::tcp_server::{TcpServer};
use crate::structures::s_type;
use crate::structures::s_type::StructureType;
use crate::testing::test_s_type::{
    InitialRequest, InitialResponse, PayloadRequest, PayloadResponse, TestError, TestStructureType,
};
pub use bincode;
pub use openssl;
pub use sha2;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::sync::oneshot::Sender;
use tokio::time::sleep;
use tokio_util::bytes::{BytesMut};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use crate::structures::traffic_proc::TrafficProcessorHolder;
use crate::testing::test_proc::TestProcessor;

mod testing;

struct TestHandler {
    moved_streams: Vec<Framed<TcpStream, LengthDelimitedCodec>>,
}
#[async_trait]
impl Handler for TestHandler {
    async fn serve_route(
        &mut self,
        _: (SocketAddr, &mut Option<Sender<Arc<Mutex<dyn Handler>>>>),
        s_type: Box<dyn StructureType>,
        mut data: BytesMut,
    ) -> Result<Vec<u8>, Vec<u8>> {
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
                return Ok(s_type::to_vec(&response).unwrap());
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
                return Ok(s_type::to_vec(&response).unwrap());
            }
            TestStructureType::HighPayloadRequest => {
                //let request = s_type::from_slice::<HighPayloadRequest>(data.as_slice()).unwrap();

                let test_error = TestError {
                    s_type: TestStructureType::TestError,
                    error: "TestError".to_string(),
                };
                println!("Success server");
                return Err(s_type::to_vec(&test_error).unwrap());
            }
            _ => {
                let test_error = TestError {
                    s_type: TestStructureType::TestError,
                    error: "TestError".to_string(),
                };
                println!("Success server");
                return Err(s_type::to_vec(&test_error).unwrap());
            }
        }
    }

    async fn accept_stream(
        &mut self,
        _: SocketAddr,
        stream: (
            Framed<tokio::net::TcpStream, LengthDelimitedCodec>,
            TrafficProcessorHolder,
        ),
    ) {
        self.moved_streams.push(stream.0);
    }
}
#[tokio::main]
pub async fn main() {
    let mut router = TcpServerRouter::new(Box::new(TestStructureType::HighPayloadRequest));
    router.add_route(
        Arc::new(Mutex::new(TestHandler {
            moved_streams: Vec::new(),
        })),
        "TestHandler".to_string(),
        vec![
            Box::new(TestStructureType::InitialRequest),
            Box::new(TestStructureType::PayloadRequest),
            Box::new(TestStructureType::HighPayloadRequest),
        ],
    );
    router.commit_routes();
    let router = Arc::new(router);

    let mut proc_holder = TrafficProcessorHolder::new();
    proc_holder.register_processor(Box::new(TestProcessor::new()));
    let mut server = TcpServer::new("127.0.0.1:3333".to_string(), router, Some(proc_holder)).await;

    server.start().await;
   

    sleep(Duration::from_millis(600000)).await;
    server.send_stop();
    println!("sended stop waiting before exit");
    sleep(Duration::from_millis(1500)).await;
    println!("now the process will need to shutdown, if not this is trouble");
}

pub mod util;
pub mod structures;
pub mod server;
pub mod client;

mod testing;

use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use tungstenite::WebSocket;
use crate::server::handler::Handler;
use crate::server::server_router::TcpServerRouter;
use crate::server::tcp_server_new::TcpServer;
use crate::structures::s_type;
use crate::structures::s_type::{ServerErrorEn, StructureType};
use crate::testing::test_client::init_client;
use crate::testing::test_s_type::{HighPayloadRequest, InitialRequest, InitialResponse, PayloadRequest, PayloadResponse, TestError, TestStructureType};
use crate::util::thread_pool::ThreadPool;

struct TestHandler;

impl Handler for TestHandler {
    fn serve_route(
        &mut self,
        client_meta: SocketAddr,
        s_type: Box<dyn StructureType>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, Vec<u8>> {
        let base_s_type = s_type.as_any().downcast_ref::<TestStructureType>().unwrap();
        if data.is_empty() {
            println!("{}", base_s_type.clone() as u8);
            let test_error = TestError {
                s_type: TestStructureType::TestError,
                error: "no meta!".to_string(),
            };
            return Err(s_type::to_vec(&test_error).unwrap());
        }

        match base_s_type {
            TestStructureType::InitialRequest => {
                let request = s_type::from_slice::<InitialRequest>(data.as_slice()).unwrap();
                let response = InitialResponse {
                    s_type: TestStructureType::InitialResponse,
                    response: request.request * 5,
                };
                return Ok(s_type::to_vec(&response).unwrap());
            }
            TestStructureType::PayloadRequest => {
                println!("{}", data.len());
                let request = s_type::from_slice::<PayloadRequest>(data.as_slice()).unwrap();
                let mut response = PayloadResponse {
                    s_type: TestStructureType::PayloadResponse,
                    medium_payload: request.medium_payload.clone(),
                    request: request.request,
                };
                response.medium_payload.sort();
                return Ok(s_type::to_vec(&response).unwrap());
            }
            TestStructureType::HighPayloadRequest => {
                let request = s_type::from_slice::<HighPayloadRequest>(data.as_slice()).unwrap();
                let test_error = TestError {
                    s_type: TestStructureType::TestError,
                    error: "TestError".to_string(),
                };
                return Err(s_type::to_vec(&test_error).unwrap());
            }
            _ => {
                let test_error = TestError {
                    s_type: TestStructureType::TestError,
                    error: "TestError".to_string(),
                };
                return Err(s_type::to_vec(&test_error).unwrap());
            }
        }
    }

    fn request_to_move_stream(&self) -> Option<Vec<SocketAddr>> {
        None
    }

    fn accept_stream(&mut self, stream: Vec<Arc<Mutex<WebSocket<TcpStream>>>>) {
        todo!()
    }
}


pub fn main() {
    let mut router = TcpServerRouter::new(Box::new(TestStructureType::HighPayloadRequest));
    router.add_route(
        Arc::new(Mutex::new(TestHandler {})),
        "TestHandler".to_string(),
        vec![
            Box::new(TestStructureType::InitialRequest),
            Box::new(TestStructureType::PayloadRequest),
            Box::new(TestStructureType::HighPayloadRequest),
        ],
    );
    router.commit_routes();
    let router = Arc::new(router);


    let server = Arc::new(Mutex::new(TcpServer::new(
        "127.0.0.1:3333".to_string(),
        router,
        ThreadPool::new(28),
    )));

    TcpServer::start(server.clone());
    let mut client = init_client();
    client.start();

    sleep(Duration::from_millis(100000));
    server.lock().unwrap().send_stop();
    println!("sended stop waiting before exit");
    sleep(Duration::from_millis(500));
    println!("now the process will need to shutdown, if not this is trouble");
}

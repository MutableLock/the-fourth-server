/*
use crate::client::{ClientConnection, Receiver};
use crate::structures::s_type;
use crate::structures::s_type::StructureType;
use crate::testing::test_s_type::{
    InitialRequest, InitialResponse, PayloadRequest, PayloadResponse, TestStructureType,
};
use crate::util::rand_utils::generate_random_u8_vec;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestRecv {
    counter: Arc<Mutex<AtomicU64>>,
    response_send: Arc<Mutex<AtomicU64>>,
    id: u64,
}

impl Receiver for TestRecv {
    fn get_handler_name(&self) -> String {
        String::from("TestHandler")
    }


    fn get_request(&mut self) -> Option<(Vec<u8>, Box<dyn StructureType>)> {
        let res: (Vec<u8>, Box<dyn StructureType>) =
            if self.counter.lock().unwrap().load(Relaxed) % 2 == 0 {
                let pre_val = InitialRequest {
                    s_type: TestStructureType::InitialRequest,
                    request: 500,
                };
                (
                    s_type::to_vec(&pre_val).unwrap(),
                    Box::new(TestStructureType::InitialRequest),
                )
            } else {
                let pre_val = PayloadRequest {
                    s_type: TestStructureType::PayloadRequest,
                    request: InitialRequest {
                        s_type: TestStructureType::InitialRequest,
                        request: 500,
                    },
                    medium_payload: generate_random_u8_vec(500),
                };
                (
                    s_type::to_vec(&pre_val).unwrap(),
                    Box::new(TestStructureType::PayloadRequest),
                )
            };

        //  let pre_val = PayloadRequest{s_type: TestStructureType::PayloadRequest, request: InitialRequest {s_type: TestStructureType::InitialRequest, request: 500 }, medium_payload: generate_random_u8_vec(500)};
        //  let res: (Vec<u8>, Box<dyn StructureType>) = (s_type::to_vec(&pre_val).unwrap(), Box::new(TestStructureType::PayloadRequest));
        let val = self.counter.lock().unwrap().fetch_add(1, Relaxed);
        self.counter.lock().unwrap().store(val + 1, Relaxed);
        self.response_send.lock().unwrap().store(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_micros() as u64,
            Relaxed,
        );
        Some(res)
    }

    fn receive_response(&mut self, response: Vec<u8>) {
        let received = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_micros() as u64;

        println!(
            "delay {} microseconds",
            received - self.response_send.lock().unwrap().load(Relaxed)
        );
        let resp: Result<InitialResponse, String> = s_type::from_slice(&response);
        if resp.is_err() {
            let resp: Result<PayloadResponse, String> = s_type::from_slice(&response);
            if resp.is_err() {
                println!("Erro!");
                return;
            }
            println!("success");
        } else {
            let resp = resp.unwrap();
            println!("success {}", resp.response);
        }
    }
}

pub fn init_client() -> ClientConnection {
    let connection = ClientConnection::new(
        "ws://127.0.0.1:3333".to_string(),
        vec![Arc::new(Mutex::new(TestRecv {
            counter: Arc::new(Mutex::new(AtomicU64::new(0))),
            response_send: Arc::new(Mutex::new(AtomicU64::new(0))),
            id: 0,
        }))],
    );
    return connection;
}


 */
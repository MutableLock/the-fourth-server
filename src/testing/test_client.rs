#![allow(dead_code)]

use crate::client::{ClientConnect, DataConsumer};
use crate::structures::s_type;
use crate::structures::s_type::StructureType;
use crate::structures::traffic_proc::TrafficProcessorHolder;
use crate::testing::test_proc::TestProcessor;
use crate::testing::test_s_type::{
    InitialRequest, InitialResponse, PayloadRequest, PayloadResponse, TestStructureType,
};
use crate::util::rand_utils::generate_random_u8_vec;
use async_trait::async_trait;
use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

pub struct TestRecv {
    counter: Arc<Mutex<AtomicU64>>,
    response_send: Arc<Mutex<AtomicU64>>,
    id: u64,
}
#[async_trait]
impl DataConsumer for TestRecv {

    async fn response_received(&mut self, handler_id: u64, payload_id: u64 ,response: BytesMut) {
        let received = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_micros() as u64;

        println!(
            "delay {} microseconds",
            received - self.response_send.lock().await.load(Relaxed)
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
impl TestRecv {
    pub async fn get_request(&mut self) -> Option<(Vec<u8>, Box<dyn StructureType>)> {
        let res: (Vec<u8>, Box<dyn StructureType>) =
            if self.counter.lock().await.load(Relaxed) % 2 == 0 {
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
        let val = self.counter.lock().await.fetch_add(1, Relaxed);
        self.counter.lock().await.store(val + 1, Relaxed);
        self.response_send.lock().await.store(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_micros() as u64,
            Relaxed,
        );
        Some(res)
    }

}

pub async fn init_client<C>(c: C) -> ((ClientConnect), Arc<Mutex<TestRecv>>)
where
    C: Encoder<Bytes, Error = io::Error>
        + Decoder<Item = BytesMut, Error = io::Error>
        + Clone
        + Send
        + 'static
        + std::marker::Sync,
{
    let mut processor_holder: TrafficProcessorHolder<C> = TrafficProcessorHolder::new();
    processor_holder.register_processor(Box::new(TestProcessor::new(c.clone())));
    let test_recv = Arc::new(tokio::sync::Mutex::new(TestRecv {
        counter: Arc::new(Mutex::new(AtomicU64::new(0))),
        response_send: Arc::new(Mutex::new(AtomicU64::new(0))),
        id: 0,
    }));
    let connection = ClientConnect::new(
        "127.0.0.1".to_string(),
        "127.0.0.1:3333".to_string(),
        Some(processor_holder),
        c,
        None,
        15
    )
    .await;
    return (connection.unwrap(), test_recv);
}

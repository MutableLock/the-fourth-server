use std::time::{Duration, Instant};
use rand::{random, Rng};
use tfserver::client::{ClientConnect, ClientRequest, DataRequest, HandlerInfo};
use tfserver::codec::length_delimited::LengthDelimitedCodec;
use tfserver::structures::s_type;
use tfserver::tokio;
use tfserver::tokio::time::sleep;
use crate::s_type_example::{ExampleSType, ExpensiveMsg, ExpensiveResponse, TestMsg};

mod s_type_example;

fn make_test_request() -> DataRequest {
    let request = TestMsg{
        s_type: ExampleSType::TestMessage,
        id: 12,
        data: vec![32, 32, 23, 42],
        message: "hello from client".to_string(),
    };
    let data_req = DataRequest{
        handler_info: HandlerInfo::new_named("TEST_HANDLER".to_string()),
        data: s_type::to_vec(&request).unwrap(),
        s_type: Box::new(ExampleSType::TestMessage),
    };
    data_req
}

fn make_big_payload_request() -> DataRequest {
    let mut rng = rand::rng();
    let size = 8096; // 250 MB
    let request = ExpensiveMsg{
        s_type: ExampleSType::ExpensiveMessage,
        id: 12,
        data: (0..size).map(|_| rng.r#gen()).collect(),
    };
    let data_req = DataRequest{
        handler_info: HandlerInfo::new_named("BIG_PAYLOAD".to_string()),
        data: s_type::to_vec(&request).unwrap(),
        s_type: Box::new(ExampleSType::ExpensiveMessage),
    };
    data_req
}

fn make_very_big_payload_request() -> DataRequest {
    let mut rng = rand::rng();
    let size = 250 * 1024 * 1024; // 250 MB
    let request = ExpensiveResponse{
        s_type: ExampleSType::ExpensiveResponse,
        id: 12,
        data: (0..size).map(|_| rng.r#gen()).collect(),
    };
    let data_req = DataRequest{
        handler_info: HandlerInfo::new_named("BIG_PAYLOAD".to_string()),
        data: s_type::to_vec(&request).unwrap(),
        s_type: Box::new(ExampleSType::ExpensiveResponse),
    };
    data_req
}

#[tokio::main]
async fn main() {
    let mut client_connect = ClientConnect::new(
        "localhost".to_string(),
        "127.0.0.1:9973".to_string(),
        None,
        LengthDelimitedCodec::new(1024 * 1024 * 1024),
        None,
        2500,
    )
    .await
    .expect("Connecting to server failed");
    loop {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let test_req = make_test_request();
        let client_req = ClientRequest{ req: test_req, consumer: tx};
        let start = Instant::now();
        client_connect.dispatch_request(client_req).await.expect("Sending request failed");
        if let Ok(resp) = rx.await {
            println!("Delay {} microseconds", start.elapsed().as_micros());
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        let test_req = make_big_payload_request();
        let client_req = ClientRequest{ req: test_req, consumer: tx};
        let start = Instant::now();
        client_connect.dispatch_request(client_req).await.expect("Sending request failed");
        if let Ok(mut resp) = rx.await {
            println!("Delay on high payload request: {} microseconds", start.elapsed().as_micros());
            let resp: ExpensiveMsg = s_type::from_slice(resp.as_mut()).unwrap();
         //   println!("Received response: {:?}", resp);
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        let test_req = make_very_big_payload_request();
        let client_req = ClientRequest{ req: test_req, consumer: tx};
        let start = Instant::now();
        client_connect.dispatch_request(client_req).await.expect("Sending request failed");
        if let Ok(mut resp) = rx.await {
            println!("Delay on high payload request: {} microseconds", start.elapsed().as_micros());
            let resp: ExpensiveResponse = s_type::from_slice(resp.as_mut()).unwrap();
            //   println!("Received response: {:?}", resp);
        }
    }
}

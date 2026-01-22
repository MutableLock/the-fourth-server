pub mod client;
pub mod server;
pub mod structures;
pub mod util;
pub mod testing;
pub mod codec;

use std::time::Duration;
use tokio::time::sleep;
use crate::client::{ClientRequest, DataRequest, HandlerInfo};
use crate::codec::length_delimited::LengthDelimitedCodec;
use crate::testing::test_client::init_client;


#[tokio::main]
pub async fn main() {
    let client = init_client(LengthDelimitedCodec::new()).await;
    loop{
        sleep(Duration::from_secs(1)).await;
        let req = client.1.lock().await.get_request().await;
        let one_shot = tokio::sync::oneshot::channel();
        if let Some(req) = req{
            let cli_request = ClientRequest{req: DataRequest{
                handler_info: HandlerInfo::new_named("TestHandler".parse().unwrap()),
                data: req.0,
                s_type: req.1,
            },
                consumer: one_shot.0,
                payload_id: 25,
            };
            client.0.dispatch_request(cli_request).await.unwrap();
            client.1.lock().await.response_received(one_shot.1.await.unwrap()).await;
        }
    }
   // println!("sended stop waiting before exit");
   // sleep(Duration::from_millis(1500)).await;
}
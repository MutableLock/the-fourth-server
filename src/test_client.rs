pub mod client;
pub mod server;
pub mod structures;
pub mod util;
pub mod testing;

use std::time::Duration;
use futures_util::SinkExt;
use tokio::time::sleep;
use tokio_util::codec::LengthDelimitedCodec;
use crate::client::{ClientRequest, DataRequest, HandlerInfo};
use crate::testing::test_client::init_client;


#[tokio::main]
pub async fn main() {
    let mut client = init_client(LengthDelimitedCodec::new()).await;
    loop{
        sleep(Duration::from_secs(1)).await;
        let req = client.1.lock().await.get_request().await;
        if let Some(req) = req{
            let cli_request = ClientRequest{req: DataRequest{
                handler_info: HandlerInfo::new_named("TestHandler".parse().unwrap()),
                data: req.0,
                s_type: req.1,
            },
                consumer: client.1.clone(),
            };
            client.0.dispatch_request(cli_request).await;
        }
    }
    println!("sended stop waiting before exit");
    sleep(Duration::from_millis(1500)).await;
}
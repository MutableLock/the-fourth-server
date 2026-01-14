pub mod client;
pub mod server;
pub mod structures;
pub mod util;
pub mod testing;

use std::time::Duration;
use futures_util::SinkExt;
use tokio::time::sleep;
use tokio_util::codec::LengthDelimitedCodec;
use crate::client::Receiver;
use crate::testing::test_client::init_client;


#[tokio::main]
pub async fn main() {
    let mut client = init_client(LengthDelimitedCodec::new()).await;
    client.0.0.start()
    loop{
        sleep(Duration::from_secs(1)).await;
        let req = client.1.lock().await.get_request().await;
        if let Some(req) = req{
            client.0.push_request()
        }
    }

    client.0.stop();
    println!("sended stop waiting before exit");
    sleep(Duration::from_millis(1500)).await;
}
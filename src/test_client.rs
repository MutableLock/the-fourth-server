pub mod client;
pub mod server;
pub mod structures;
pub mod util;
pub mod testing;

use std::time::Duration;
use tokio::time::sleep;
use crate::testing::test_client::init_client;


#[tokio::main]
pub async fn main() {
    let mut client = init_client().await;
    client.start().await;
    sleep(Duration::from_millis(600000)).await;

    client.stop();
    println!("sended stop waiting before exit");
    sleep(Duration::from_millis(1500)).await;
}
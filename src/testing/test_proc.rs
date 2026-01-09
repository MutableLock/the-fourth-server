#![allow(dead_code)]
use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use crate::structures::traffic_proc::TrafficProcess;

pub struct TestProcessor{

}

impl TestProcessor {
    pub fn new() -> TestProcessor {
        TestProcessor {}
    }
}
#[async_trait]
impl TrafficProcess for TestProcessor {
    async fn initial_connect(&mut self, _: &mut TcpStream) -> bool {
        return true;
    }

    async fn initial_framed_connect(&mut self, _: &mut Framed<TcpStream, LengthDelimitedCodec>) -> bool {
        return true;
    }

    async fn post_process_traffic(&mut self, mut data: Vec<u8>) -> Vec<u8> {
        for x in data.iter_mut() {
            if *x < 254{
                *x+=1
            }
        }
        data
    }

    async fn pre_process_traffic(&mut self, mut data: BytesMut) -> BytesMut {
        for x in data.iter_mut() {
            if *x<255{
                *x-=1
            }
        }
        data
    }

    fn clone(&self) -> Box<dyn TrafficProcess> {
       Box::new(Self{})
    }
}
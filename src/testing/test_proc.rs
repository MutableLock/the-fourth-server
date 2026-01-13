#![allow(dead_code)]

use std::io;
use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder, Framed, LengthDelimitedCodec};
use crate::structures::traffic_proc::TrafficProcess;

pub struct TestProcessor<C> where C: Encoder<Bytes> + Decoder<Item = BytesMut, Error = io::Error> + Clone + Send +Sync + 'static{
    c: C
}

impl<C> TestProcessor<C> where C: Encoder<Bytes> + Decoder<Item = BytesMut, Error = io::Error> + Clone + Send +Sync + 'static {
    pub fn new(c: C) -> TestProcessor<C> {
        TestProcessor {c}
    }
}
#[async_trait]
impl<C> TrafficProcess for TestProcessor<C> where C: Encoder<Bytes> + Decoder<Item = BytesMut, Error = io::Error> + Clone + Send + Sync + 'static{
    type Codec = C;

    async fn initial_connect(&mut self, _: &mut TcpStream) -> bool {
        return true;
    }

    async fn initial_framed_connect(&mut self, _: &mut Framed<TcpStream, C>) -> bool {
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

    fn clone(&self) -> Box<dyn TrafficProcess<Codec=C>> {
       Box::new(Self{c: self.c.clone()})
    }
}
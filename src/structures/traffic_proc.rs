use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
#[async_trait]
pub trait TrafficProcess: Send + Sync {
    async fn initial_connect(&mut self, source: &mut TcpStream) -> bool;
    async fn initial_framed_connect(&mut self, source: &mut Framed<TcpStream, LengthDelimitedCodec>) -> bool;
    async fn post_process_traffic(&mut self, data: Vec<u8>) -> Vec<u8>;
    async fn pre_process_traffic(&mut self, data: BytesMut) -> BytesMut;

    fn clone(&self) -> Box<dyn TrafficProcess>;
}

pub struct TrafficProcessorHolder {
    processors: Vec<Box<dyn TrafficProcess>>,
}

impl Clone for TrafficProcessorHolder {
    fn clone(&self) -> Self {
        let mut processors = Vec::new();
        self.processors
            .iter()
            .for_each(|p| processors.push(p.as_ref().clone()));

        Self { processors }
    }
}

impl TrafficProcessorHolder {
    pub fn new() -> Self {
        TrafficProcessorHolder { processors: vec![] }
    }
    pub fn register_processor(&mut self, processor: Box<dyn TrafficProcess>) {
        self.processors.push(processor);
    }

    pub async fn initial_connect(&mut self, source: &mut TcpStream) -> bool{
        for processor in self.processors.iter_mut() {
            if !processor.as_mut().initial_connect(source).await{
                return false;
            }
        }
        true
    }

    pub async fn initial_framed_connect(&mut self, source: &mut Framed<TcpStream, LengthDelimitedCodec>) -> bool {
        for processor in self.processors.iter_mut() {
            if !processor.as_mut().initial_framed_connect(source).await{
                return false;
            }
        }
        true
    }

    pub async fn post_process_traffic(&mut self, mut data: Vec<u8>) -> Vec<u8> {
        for proc in self.processors.iter_mut() {
            data = proc.post_process_traffic(data).await;
        }
        data
    }

    pub async fn pre_process_traffic(&mut self, mut data: BytesMut) -> BytesMut {
        for proc in self.processors.iter_mut() {
            data = proc.pre_process_traffic(data).await;
        }
        data
    }
}

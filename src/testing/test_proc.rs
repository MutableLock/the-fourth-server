use tokio::net::TcpStream;
use tokio_util::bytes::BytesMut;
use crate::structures::traffic_proc::TrafficProcess;

pub struct TestProcessor{

}

impl TestProcessor {
    pub fn new() -> TestProcessor {
        TestProcessor {}
    }
}

impl TrafficProcess for TestProcessor {
    fn initial_connect(&mut self, source: &mut TcpStream) -> bool {
        return true;
    }

    fn post_process_traffic(&mut self, mut data: Vec<u8>) -> Vec<u8> {
        for x in data.iter_mut() {
            if *x < 254{
                *x+=1
            }
        }
        data
    }

    fn pre_process_traffic(&mut self, mut data: BytesMut) -> BytesMut {
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
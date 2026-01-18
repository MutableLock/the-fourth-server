use std::io;
use async_trait::async_trait;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use crate::codec::codec_trait::TfCodec;
use crate::structures::transport::Transport;
#[derive(Clone)]
pub struct LengthDelimitedCodec {
    codec: tokio_util::codec::LengthDelimitedCodec
}

impl LengthDelimitedCodec {
    pub fn new() -> Self {
        Self{
            codec: tokio_util::codec::LengthDelimitedCodec::new(),
        }
    }
}

impl Decoder for LengthDelimitedCodec {
    type Item = BytesMut;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.codec.decode(src)
    }
}

impl Encoder<Bytes> for LengthDelimitedCodec {
    type Error = io::Error;
    fn encode(&mut self, item: Bytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.codec.encode(item, dst)
    }
}
#[async_trait]
impl TfCodec for LengthDelimitedCodec {
    async fn initial_setup(&mut self, transport: &mut Transport) -> bool {
        true
    }
}
use async_trait::async_trait;
use crate::structures::transport::Transport;

#[async_trait]
pub trait TfCodec

{
    async fn initial_setup(&mut self, transport: &mut Transport) -> bool;
}

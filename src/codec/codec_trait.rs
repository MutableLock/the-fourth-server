use async_trait::async_trait;
use crate::structures::transport::Transport;

#[async_trait]
///The additional trait that gaves ability to setup codec per connection.
pub trait TfCodec
{
    async fn initial_setup(&mut self, transport: &mut Transport) -> bool;
}

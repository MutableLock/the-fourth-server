use crate::client::wait_for_data;
use crate::structures::s_type;
use crate::structures::s_type::{HandlerMetaAns, HandlerMetaReq, SystemSType};
use crate::structures::traffic_proc::TrafficProcessorHolder;
use crate::structures::transport::Transport;
use futures_util::SinkExt;
use std::collections::HashMap;
use std::io;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder, Framed};

pub struct TargetRouter {
    known_routes: HashMap<String, u64>,
}

impl TargetRouter {
    pub fn new() -> Self {
        Self {
            known_routes: HashMap::new(),
        }
    }

    pub fn lookup_route(&self, route: &str) -> Option<u64> {
        self.known_routes.get(route).cloned()
    }

    pub async fn request_route<
        C: Encoder<Bytes, Error = io::Error>
            + Decoder<Item = BytesMut, Error = io::Error>
            + Clone
            + Send
            + Sync
            + 'static,
    >(
        &mut self,
        route: &str,
        stream: &mut Framed<Transport, C>,
        processor: &mut TrafficProcessorHolder<C>,
    ) -> Option<u64> {
        let pre_res = self.lookup_route(route);
        if pre_res.is_some() {
            return pre_res;
        }
        let id = Self::request_route_from_server(route, stream, processor).await;
        self.known_routes.insert(route.to_string(), id);
        Some(id)
    }
    async fn request_route_from_server<
        C: Encoder<Bytes, Error = io::Error>
            + Decoder<Item = BytesMut, Error = io::Error>
            + Clone
            + Send
            + Sync
            + 'static,
    >(
        name: &str,
        stream: &mut Framed<Transport, C>,
        processor: &mut TrafficProcessorHolder<C>,
    ) -> u64 {
        let meta_req = HandlerMetaReq {
            s_type: SystemSType::HandlerMetaReq,
            handler_name: name.to_string(),
        };
        let data = s_type::to_vec(&meta_req).unwrap();
        let mut data = processor.post_process_traffic(data).await;
        stream.send(data.into()).await.unwrap();
        let mut ans = processor
            .pre_process_traffic(wait_for_data(stream).await)
            .await;
        let meta_ans = s_type::from_slice::<HandlerMetaAns>(ans.as_mut()).unwrap();
        meta_ans.id
    }
}

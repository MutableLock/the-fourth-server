use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr};
use std::ops::Deref;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc};
use tokio::io;
use tokio::sync::{Mutex};
use tokio::sync::oneshot::Sender;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use crate::codec::codec_trait::TfCodec;
use crate::server::handler::Handler;
use crate::structures::s_type;
use crate::structures::s_type::{HandlerMetaAns, HandlerMetaReq, PacketMeta, ServerError, ServerErrorEn, StructureType, SystemSType, TypeContainer, TypeTupple};
use crate::structures::s_type::ServerErrorEn::InternalError;

pub struct TcpServerRouter<C>
where
    C:  Encoder<Bytes, Error = io::Error> + Decoder<Item = BytesMut, Error = io::Error> + Clone + Send  + Sync+ 'static +TfCodec {
    routes: Arc<HashMap<TypeTupple, Arc<Mutex<dyn Handler<Codec = C>>>>>,
    routes_text_names: Arc<HashMap<String, u64>>,
    routes_to_add: Vec<(TypeTupple, (Arc<Mutex<dyn Handler<Codec = C>>>, String))>,
    router_incremental: u64,
    routes_commited: bool,
    user_s_type: Box<dyn StructureType>,
}

impl<C> TcpServerRouter<C>
where
    C: Encoder<Bytes, Error = io::Error> + Decoder<Item = BytesMut, Error = io::Error> + Clone + Send  + Sync + 'static +TfCodec {
    pub fn new(user_s_type: Box<dyn StructureType>) -> Self {
        Self {
            routes: Arc::new(HashMap::new()),
            routes_text_names: Arc::new(HashMap::new()),
            routes_to_add: Vec::new(),
            router_incremental: 0,
            routes_commited: false,
            user_s_type,
        }
    }

    pub fn add_route(
        &mut self,
        handler: Arc<Mutex<dyn Handler<Codec = C>>>,
        handler_name: String,
        mut s_types: Vec<Box<dyn StructureType>>,
    ) {
        if self.routes_commited {
            return;
        }
        let mut s_typess: HashSet<TypeContainer> = HashSet::new();
        while !s_types.is_empty(){
            s_typess.insert(TypeContainer::new(s_types.pop().unwrap()));
        }
        let types_tupple = TypeTupple {
            s_types: s_typess,
            handler_id: self.router_incremental,
        };

        self.routes_to_add.push((types_tupple, (handler, handler_name)));
        self.router_incremental += 1;
    }

    pub fn commit_routes(&mut self) {
        if self.routes_commited || self.routes_to_add.is_empty() {
            return;
        }

        let mut routes = HashMap::new();
        let mut names = HashMap::new();

        for (types, (handler, name)) in self.routes_to_add.drain(..) {
            routes.insert(types.clone(), handler);
            names.insert(name, types.handler_id);
        }

        self.routes = Arc::new(routes);
        self.routes_text_names = Arc::new(names);
        self.routes_commited = true;
    }


    pub fn get_routes(&self) -> Arc<HashMap<TypeTupple, Arc<Mutex<dyn Handler<Codec = C>>>>> {
        self.routes.clone()
    }

    pub async fn serve_packet(
        &self,
        meta: BytesMut,
        payload: BytesMut,
        client_meta: (SocketAddr,  &mut Option<Sender<Arc<Mutex<dyn Handler<Codec = C>>>>>),
    ) -> Result<Vec<u8>, ServerError> {
        // Try to deserialize normal PacketMeta
        if let Ok(meta_pack) = s_type::from_slice::<PacketMeta>(&meta) {
            let s_type = self.user_s_type.get_deserialize_function().deref()(meta_pack.s_type_req);
            let key = TypeTupple {
                s_types: HashSet::from([TypeContainer::new(s_type.clone_unique())]),
                handler_id: meta_pack.handler_id,
            };

            let handler = self.routes.get(&key).ok_or(ServerError::new(ServerErrorEn::NoSuchHandler(None)))?;
            let mut handler_lock = handler.lock().await;
            let res = catch_unwind(AssertUnwindSafe(async || {
                handler_lock.serve_route(client_meta, s_type, payload).await
            }));

            return match res {
                Ok(data) => match data.await{
                    Ok(data) => Ok(data),
                    Err(err) => {Err(ServerError::new(ServerErrorEn::InternalError(Some(err.to_vec()))))}
                },
                Err(_) => Err(ServerError::new(InternalError(Some("handler died :(".as_bytes().to_vec())))),
            };
        }

        // Try to handle as HandlerMetaReq
        if let Ok(meta_req) = s_type::from_slice::<HandlerMetaReq>(&meta) {
            if let Some(route_id) = self.routes_text_names.get(&meta_req.handler_name) {
                let meta_ans = HandlerMetaAns {
                    s_type: SystemSType::HandlerMetaAns,
                    id: *route_id,
                };
                return Ok(s_type::to_vec(&meta_ans).unwrap());
            } else {
                return Err(ServerError::new(ServerErrorEn::NoSuchHandler(None)));
            }
        }

        Err(ServerError::new(ServerErrorEn::MalformedMetaInfo(None)))
    }
}
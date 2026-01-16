use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use bincode::config::{Configuration, Fixint, LittleEndian};
use num_enum::TryFromPrimitive;
use serde::de::DeserializeOwned;
use crate::util::data_cipher::{DataCipher, EncryptionType};

pub static BINCODE_CFG: Configuration<LittleEndian, Fixint> =
    bincode::config::standard()
        .with_little_endian()
        .with_fixed_int_encoding()
        .with_no_limit();

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerErrorEn {
    MalformedMetaInfo(Option<String>),
    NoSuchHandler(Option<String>),
    InternalError(Option<Vec<u8>>),
    PayloadLost
}
#[derive(Serialize, Deserialize)]
pub struct ServerError {
    s_type: SystemSType,
    pub en: ServerErrorEn,
}

impl ServerError {
    pub fn new(en: ServerErrorEn) -> Self {
        Self{s_type: SystemSType::ServerError, en}
    }
}


pub trait StructureType: Any + Send + Sync{
     fn get_type_id(&self) -> TypeId;
    fn equals(&self, other: &dyn StructureType) -> bool;
    fn as_any(&self) -> &dyn Any;

    fn hash(&self) -> u64;

    fn clone_unique(&self) -> Box<dyn StructureType>;

    fn get_deserialize_function(&self) -> Box<dyn Fn(u64) -> Box<dyn StructureType>>;

    fn get_serialize_function(&self) -> Box<dyn Fn(Box<dyn StructureType>) -> u64>;
}
#[repr(u8)]
#[derive(Serialize, Deserialize, PartialEq, Clone, Hash, Eq, TryFromPrimitive, Copy)]
pub enum SystemSType {
    PacketMeta,
    HandlerMetaReq,
    HandlerMetaAns,
    ServerError,
    EncryptedContainer,
}
#[derive(Serialize, Deserialize)]
pub struct EncryptedContainer{
    s_type: SystemSType,
    data: Vec<u8>,
}

impl StrongType for EncryptedContainer{
    fn get_s_type(&self) -> &(dyn StructureType + 'static) {
        &self.s_type
    }
}

impl StrongType for ServerError {
    fn get_s_type(&self) -> &(dyn StructureType + 'static) {
        &self.s_type
    }
}

impl SystemSType{
    pub fn deserialize(val: u64) -> Box<dyn StructureType> {
        Box::new(SystemSType::try_from(val as u8).unwrap())
    }

    pub fn serialize(refer: Box<dyn StructureType>) -> u64{
        refer.as_any().downcast_ref::<SystemSType>().unwrap().clone() as u8 as u64
    }
}

impl StructureType for SystemSType {
    fn get_type_id(&self) -> TypeId {
        return match self {
            Self::PacketMeta => {
                TypeId::of::<PacketMeta>()
            }
            Self::HandlerMetaAns => {
                TypeId::of::<HandlerMetaAns>()
            }
            Self::HandlerMetaReq => {
                TypeId::of::<HandlerMetaReq>()
            }
            Self::ServerError => {
                TypeId::of::<ServerError>()
            }

            Self::EncryptedContainer => {
                TypeId::of::<EncryptedContainer>()
            }
        }
    }

    fn equals(&self, other: &dyn StructureType) -> bool {
        let downcast = other.as_any().downcast_ref::<Self>();
        if downcast.is_none(){
            return false;
        }
        let downcast = downcast.unwrap();
        return downcast.eq(self);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn hash(&self) -> u64{
        let mut hasher  = DefaultHasher::default();
        TypeId::of::<Self>().hash(&mut hasher);
        ((*self).clone() as u8).hash(&mut hasher);
        return hasher.finish();
    }

    fn clone_unique(&self) -> Box<dyn StructureType> {
        Box::new(self.clone())
    }

    fn get_deserialize_function(&self) -> Box<dyn Fn(u64) -> Box<dyn StructureType>> {
        Box::new(SystemSType::deserialize)
    }

    fn get_serialize_function(&self) -> Box<dyn Fn(Box<dyn StructureType>) -> u64> {
        Box::new(SystemSType::serialize)
    }
}

pub trait StrongType: Any{
    fn get_s_type(&self) -> &dyn StructureType;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PacketMeta {
    pub s_type: SystemSType,
    pub s_type_req: u64,
    pub handler_id: u64,
    pub has_payload: bool,
}

impl StrongType for PacketMeta {
    fn get_s_type(&self) -> &dyn StructureType{
        &self.s_type
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HandlerMetaReq {
    pub s_type: SystemSType,
    pub handler_name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HandlerMetaAns {
    pub s_type: SystemSType,
    pub id: u64,
}

impl StrongType for HandlerMetaReq {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

impl StrongType for HandlerMetaAns {
    fn get_s_type(&self) -> &(dyn StructureType + 'static) {
        &self.s_type
    }
}


pub struct TypeContainer{
    s_type: Box<dyn StructureType>,
}

impl TypeContainer {
    pub fn new(s_type: Box<dyn StructureType>) -> Self {
        Self{s_type}
    }
}

impl PartialEq<Self> for TypeContainer {
    fn eq(&self, other: &Self) -> bool {
        self.s_type.equals(other.s_type.as_ref())
    }
}

impl Eq for TypeContainer {

}

impl Hash for TypeContainer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.s_type.hash().hash(state);
    }
}

impl Clone for TypeContainer {
    fn clone(&self) -> Self {
        Self{s_type: self.s_type.clone_unique()}
    }
}


#[derive(Eq, Clone)]
pub struct TypeTupple {
    pub s_types: HashSet<TypeContainer>,
    pub handler_id: u64,
}

pub fn validate_s_type(target: &dyn StrongType) -> bool {
    let s_type = target.get_s_type();
    return s_type.get_type_id() == target.type_id();
}

pub fn to_vec<T: Serialize + StrongType>(arg: &T) -> Option<Vec<u8>> {
    if !validate_s_type(arg) {
        eprintln!("stype validation failed");
        return None;
    }
    let res = bincode::serde::encode_to_vec(arg, BINCODE_CFG.clone());
    if res.is_err() {
        eprintln!("bincode serialization failed");
        return None;
    }
    Some(res.unwrap())
}

pub fn to_vec_encrypted<T: Serialize + StrongType>(arg: &T, encryption_type: EncryptionType, key: String, iv: &[u8]) -> Option<Vec<u8>> {
    let data = to_vec(arg);
    if data.is_none() {
        return None;
    }
    let data = data.unwrap();
    let cipher = DataCipher::new_init(encryption_type, key);
    let len = cipher.required_buffer_size(data.len());
    let mut res = Vec::with_capacity(len);
    unsafe {
        res.set_len(len);
    }
    let len_res = cipher.encrypt_block(data.as_slice(), res.as_mut_slice(), iv);
    if len_res.is_err() {
        return None;
    }
    res.shrink_to(len_res.unwrap());
    let container = EncryptedContainer{s_type: SystemSType::EncryptedContainer, data: res};
    to_vec(&container)
}

pub fn from_encrypted_slice<T: for<'a> Deserialize<'a> + StrongType>(arg: &[u8], encryption_type: EncryptionType, key: String, iv: &[u8]) -> Result<T, String>{
    let container: Result<EncryptedContainer, String> = from_slice(arg);
    if container.is_err() {
        return Err(container.err().unwrap());
    }
    let container = container.unwrap();
    let cipher = DataCipher::new_init(encryption_type, key);
    let mut pre_res: Vec<u8> = Vec::with_capacity(container.data.len());
    unsafe{
        pre_res.set_len(container.data.len());
    }
    let res = cipher.decrypt_block(container.data.as_slice(), pre_res.as_mut_slice(), iv);
    if res.is_err() {
        return Err("Failed to decrypt block".to_string());
    }
    let res = res.unwrap();
    pre_res.shrink_to(res);
    from_slice(&pre_res)
}

pub fn from_slice<T: for<'a> Deserialize<'a> + StrongType>(arg: &[u8]) -> Result<T, String> {
    let res = bincode::serde::decode_from_slice::<T, Configuration<LittleEndian, Fixint>>(arg, BINCODE_CFG.clone());
    if res.is_err() {
        let error_server = bincode::serde::decode_from_slice::<ServerError, Configuration<LittleEndian, Fixint>>(arg, BINCODE_CFG.clone());
        if error_server.is_err() {
            return Err("Unknown packet type".to_string());
        }
        return Err(error_server.unwrap().0.en.to_string());
    }
    let res = res.unwrap().0;
    if !validate_s_type(&res) {
        let error_server = bincode::serde::decode_from_slice::<ServerError, Configuration<LittleEndian, Fixint>>(&arg, BINCODE_CFG.clone());
        if error_server.is_err() {
            return Err("Unknown packet type".to_string());
        }
        return Err(error_server.unwrap().0.en.to_string());
    }
    Ok(res)
}

impl PartialEq<Self> for TypeTupple {
    fn eq(&self, other: &Self) -> bool {
        let iterator_list = if self.s_types.len()<other.s_types.len(){
            self.s_types.iter()
        } else {
            other.s_types.iter()
        };

        for s_type in iterator_list {
            if !self.s_types.contains(&s_type) {
                return false;
            }
        }
        self.handler_id == other.handler_id
    }
}

impl Hash for TypeTupple {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.handler_id.hash(state);
    }
}

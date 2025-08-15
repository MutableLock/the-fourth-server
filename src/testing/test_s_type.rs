use crate::structures::s_type::{StrongType, StructureType};
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::hash::{DefaultHasher, Hash, Hasher};

#[repr(u8)]
#[derive(Serialize, Deserialize, PartialEq, Clone, Hash, Eq, TryFromPrimitive, Copy)]
pub enum TestStructureType {
    InitialRequest,
    InitialResponse,
    PayloadRequest,
    PayloadResponse,
    HighPayloadRequest,
    HighPayloadResponse,
    TestError,
}

impl TestStructureType {
    pub fn deserialize(val: u64) -> Box<dyn StructureType> {
        Box::new(TestStructureType::try_from(val as u8).unwrap())
    }

    pub fn serialize(refer: Box<dyn StructureType>) -> u64 {
        let res = refer
            .as_any()
            .downcast_ref::<TestStructureType>()
            .unwrap()
            .clone() as u8 as u64;
        res
    }
}

impl StructureType for TestStructureType {
    fn get_type_id(&self) -> TypeId {
        match self {
            TestStructureType::InitialRequest => TypeId::of::<InitialRequest>(),
            TestStructureType::InitialResponse => TypeId::of::<InitialResponse>(),
            TestStructureType::PayloadRequest => TypeId::of::<PayloadRequest>(),
            TestStructureType::PayloadResponse => TypeId::of::<PayloadResponse>(),
            TestStructureType::HighPayloadRequest => TypeId::of::<HighPayloadRequest>(),
            TestStructureType::HighPayloadResponse => TypeId::of::<HighPayloadResponse>(),
            TestStructureType::TestError => TypeId::of::<TestError>(),
        }
    }

    fn equals(&self, other: &dyn StructureType) -> bool {
        let downcast = other.as_any().downcast_ref::<Self>();
        if downcast.is_none() {
            return false;
        }
        let downcast = downcast.unwrap();
        downcast.clone() as u8 == self.clone() as u8
    }
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::default();
        TypeId::of::<Self>().hash(&mut hasher);
        ((*self).clone() as u8 as u64).hash(&mut hasher);
        return hasher.finish();
    }

    fn clone_unique(&self) -> Box<dyn StructureType> {
        Box::new(self.clone())
    }

    fn get_deserialize_function(&self) -> Box<dyn Fn(u64) -> Box<dyn StructureType>> {
        Box::new(TestStructureType::deserialize)
    }

    fn get_serialize_function(&self) -> Box<dyn Fn(Box<dyn StructureType>) -> u64> {
        Box::new(TestStructureType::serialize)
    }
}

#[derive(Serialize, Deserialize)]
pub struct InitialRequest {
    pub(crate) s_type: TestStructureType,
    pub request: u64,
}
#[derive(Serialize, Deserialize)]
pub struct InitialResponse {
    pub s_type: TestStructureType,
    pub response: u64,
}
#[derive(Serialize, Deserialize)]
pub struct PayloadRequest {
    pub(crate) s_type: TestStructureType,
    pub request: InitialRequest,
    pub medium_payload: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct PayloadResponse {
    pub s_type: TestStructureType,
    pub request: InitialRequest,
    pub medium_payload: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct HighPayloadRequest {
    s_type: TestStructureType,
    request: InitialRequest,
    payload: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct HighPayloadResponse {
    s_type: TestStructureType,
    request: InitialRequest,
    medium_payload: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct TestError {
    pub s_type: TestStructureType,
    pub error: String,
}

impl StrongType for TestError {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

impl StrongType for HighPayloadResponse {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

impl StrongType for HighPayloadRequest {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

impl StrongType for PayloadResponse {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

impl StrongType for PayloadRequest {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

impl StrongType for InitialRequest {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

impl StrongType for InitialResponse {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

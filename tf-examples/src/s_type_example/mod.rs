pub mod usage_example;

use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
///In this example we will create structure type for our other examples
use std::any::{Any, TypeId};
use std::hash::{DefaultHasher, Hash, Hasher};
use tfserver::structures::s_type::{StrongType, StructureType};

///the repr statement, must be used, can be any unsigned integer including up to u64
#[repr(u8)]
///Keep this derives as default, they will help us in implementing StructureType trait for this enum, the debug is optional for sure
#[derive(Serialize, Deserialize, PartialEq, Clone, Hash, Eq, TryFromPrimitive, Copy, Debug)]
pub enum ExampleSType {
    TestMessage,
    TestResponse,
    ExpensiveMessage,
    ExpensiveResponse,
    ManualHandlerRequest,
}

impl StructureType for ExampleSType {
    ///Local use only function, used for only determining in what type will be deserialized data and verify it
    fn get_type_id(&self) -> TypeId {
        match self {
            ExampleSType::TestMessage => TypeId::of::<TestMsg>(),
            ExampleSType::TestResponse => TypeId::of::<TestResponse>(),
            ExampleSType::ExpensiveMessage => TypeId::of::<ExpensiveMsg>(),
            ExampleSType::ExpensiveResponse => TypeId::of::<ExpensiveResponse>(),
            ExampleSType::ManualHandlerRequest => TypeId::of::<ManualHandlerRequest>(),
        }
    }

    ///We can't use equals from default trait set due to dyn compatibility, so i strongly recommend this approach
    fn equals(&self, other: &dyn StructureType) -> bool {
        let downcast = other.as_any().downcast_ref::<Self>();
        if downcast.is_none() {
            return false;
        }
        let downcast = downcast.unwrap();
        return downcast.eq(self);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    ///This hash function only for local use again. This function required for s_type, storing and determining optimisation inside the server/client code.
    fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::default();
        TypeId::of::<Self>().hash(&mut hasher);
        ((*self).clone() as u8).hash(&mut hasher);
        return hasher.finish();
    }
    ///Same thing as equals. We can't use default clone due to dyn compatibility.
    fn clone_unique(&self) -> Box<dyn StructureType> {
        Box::new(self.clone())
    }
    ///Here is the interesting part, the server need this two function, due to server does not know exactly what type it serialized and deserializes
    fn get_deserialize_function(&self) -> Box<dyn Fn(u64) -> Box<dyn StructureType>> {
        Box::new(ExampleSType::deserialize)
    }

    ///Here is the interesting part, the server need this two function, due to server does not know exactly what type it serialized and deserializes
    fn get_serialize_function(&self) -> Box<dyn Fn(Box<dyn StructureType>) -> u64> {
        Box::new(ExampleSType::serialize)
    }
}

impl ExampleSType {
    ///And here is why we need representation of unsigned integer. Each enum value will be serialized as it representation.
    /// There will be no collision, due to one project - one structure type approach
    pub fn deserialize(val: u64) -> Box<dyn StructureType> {
        Box::new(Self::try_from(val as u8).unwrap())
    }
    ///And here is why we need representation of unsigned integer. Each enum value will be serialized as it representation.
    /// There will be no collision, due to one project - one structure type approach
    pub fn serialize(refer: Box<dyn StructureType>) -> u64 {
        refer.as_any().downcast_ref::<Self>().unwrap().clone() as u8 as u64
    }
}

///In this part we define our structures, each structure must have it's own s_type field
#[derive(Serialize, Deserialize, Debug)]
pub struct TestMsg {
    pub s_type: ExampleSType,
    pub id: u64,
    pub data: Vec<u8>,
    pub message: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct TestResponse {
    pub s_type: ExampleSType,
    pub id: u64,
    pub data: Vec<u8>,
    pub another_message: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ExpensiveMsg {
    pub s_type: ExampleSType,
    pub id: u64,
    pub data: Vec<u8>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ExpensiveResponse {
    pub s_type: ExampleSType,
    pub id: u64,
    pub data: Vec<u8>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ManualHandlerRequest {
    s_type: ExampleSType,
}

///We need this boilerplate functions for type safety. The system will check if it deserialized properly into specified type
impl StrongType for TestMsg {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

impl StrongType for TestResponse {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

impl StrongType for ExpensiveMsg {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

impl StrongType for ExpensiveResponse {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

impl StrongType for ManualHandlerRequest {
    fn get_s_type(&self) -> &dyn StructureType {
        &self.s_type
    }
}

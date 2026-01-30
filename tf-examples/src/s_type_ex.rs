use crate::s_type_example::usage_example::{try_deserialize_structures, try_serialize_structures};

mod s_type_example;

fn main() {
    let structures = try_serialize_structures();
    try_deserialize_structures(structures);
}
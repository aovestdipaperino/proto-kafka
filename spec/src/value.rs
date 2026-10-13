/// Generic value tree. Only fields present in the decoded version appear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    I8(i8),
    I16(i16),
    U16(u16),
    I32(i32),
    I64(i64),
    /// IEEE-754 bits; compared bitwise.
    F64(u64),
    Uuid([u8; 16]),
    Str(Option<Vec<u8>>),
    Bytes(Option<Vec<u8>>),
    Records(Option<Vec<u8>>),
    Array(Option<Vec<Value>>),
    Struct(Option<Box<StructValue>>),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StructValue {
    /// (schema field name, value) in schema declaration order.
    pub fields: Vec<(&'static str, Value)>,
    /// Unknown tagged fields in wire order.
    pub unknown_tags: Vec<(u32, Vec<u8>)>,
}

impl StructValue {
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields.iter().find(|(n, _)| *n == name).map(|(_, v)| v)
    }
}

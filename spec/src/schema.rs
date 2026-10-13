//! Data-only description of Kafka message schemas. Filled by
//! `generated_schemas.rs` (emitted by protocol_codegen) and by tests.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Versions {
    pub min: i16,
    pub max: i16,
}

impl Versions {
    pub const NONE: Versions = Versions { min: 0, max: -1 };
    pub const fn range(min: i16, max: i16) -> Self {
        Versions { min, max }
    }
    pub const fn since(min: i16) -> Self {
        Versions { min, max: i16::MAX }
    }
    #[allow(clippy::manual_range_contains)] // RangeInclusive::contains is not const
    pub const fn contains(self, v: i16) -> bool {
        v >= self.min && v <= self.max
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prim {
    Bool,
    Int8,
    Int16,
    Uint16,
    Int32,
    Int64,
    Float64,
    String,
    Bytes,
    Records,
    Uuid,
}

/// Kafka has no arrays of arrays, so array element types are flattened.
#[derive(Debug, Clone, Copy)]
pub enum FieldType {
    Prim(Prim),
    Struct(&'static StructSchema),
    PrimArray(Prim),
    StructArray(&'static StructSchema),
}

/// Schema `default`. `Type` = the type's own default (0, false, "", empty, zero UUID,
/// default struct; null for records), per Java FieldSpec.fieldDefault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultVal {
    Type,
    Null,
    Int(i64),
    Bool(bool),
    Str(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub struct FieldDesc {
    pub name: &'static str,
    pub ty: FieldType,
    pub versions: Versions,
    pub nullable: Versions,
    pub tag: Option<u32>,
    pub tagged: Versions,
    /// Field-level flexibleVersions override (strings/bytes only, e.g. RequestHeader.ClientId).
    pub flexible: Option<Versions>,
    pub default: DefaultVal,
    pub ignorable: bool,
}

impl FieldDesc {
    pub const DEFAULT: FieldDesc = FieldDesc {
        name: "",
        ty: FieldType::Prim(Prim::Bool),
        versions: Versions::NONE,
        nullable: Versions::NONE,
        tag: None,
        tagged: Versions::NONE,
        flexible: None,
        default: DefaultVal::Type,
        ignorable: false,
    };
}

#[derive(Debug)]
pub struct StructSchema {
    pub name: &'static str,
    pub fields: &'static [FieldDesc],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Request,
    Response,
    Header,
    Data,
}

#[derive(Debug)]
pub struct MessageSchema {
    pub name: &'static str,
    pub api_key: Option<i16>,
    pub kind: Kind,
    pub valid: Versions,
    pub flexible: Versions,
    pub root: &'static StructSchema,
}

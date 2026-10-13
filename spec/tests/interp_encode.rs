use proto_kafka_spec::error::SpecError;
use proto_kafka_spec::header::{request_header_version, response_header_version};
use proto_kafka_spec::interp::{decode, encode};
use proto_kafka_spec::schema::*;
use proto_kafka_spec::value::{StructValue, Value};

static INNER: StructSchema = StructSchema {
    name: "Inner",
    fields: &[FieldDesc { name: "X", ty: FieldType::Prim(Prim::Int8), versions: Versions::since(0), ..FieldDesc::DEFAULT }],
};

static ROOT: StructSchema = StructSchema {
    name: "EncRequest",
    fields: &[
        FieldDesc { name: "Name", ty: FieldType::Prim(Prim::String), versions: Versions::since(0), ..FieldDesc::DEFAULT },
        FieldDesc { name: "Maybe", ty: FieldType::Struct(&INNER), versions: Versions::since(0), nullable: Versions::since(0), default: DefaultVal::Null, ..FieldDesc::DEFAULT },
        FieldDesc { name: "Ids", ty: FieldType::PrimArray(Prim::Int32), versions: Versions::since(0), nullable: Versions::since(1), ..FieldDesc::DEFAULT },
        FieldDesc { name: "Hint", ty: FieldType::Prim(Prim::Int64), versions: Versions::since(1), tag: Some(0), tagged: Versions::since(1), default: DefaultVal::Int(-1), ..FieldDesc::DEFAULT },
    ],
};

static ENC: MessageSchema = MessageSchema {
    name: "EncRequest",
    api_key: Some(1001),
    kind: Kind::Request,
    valid: Versions::range(0, 1),
    flexible: Versions::since(1),
    root: &ROOT,
};

fn sv(fields: Vec<(&'static str, Value)>, unknown_tags: Vec<(u32, Vec<u8>)>) -> StructValue {
    StructValue { fields, unknown_tags }
}

fn v1(maybe: Value, ids: Value, hint: i64, unknown: Vec<(u32, Vec<u8>)>) -> StructValue {
    sv(
        vec![("Name", Value::Str(Some(b"n".to_vec()))), ("Maybe", maybe), ("Ids", ids), ("Hint", Value::I64(hint))],
        unknown,
    )
}

#[test]
fn nullable_struct_marker() {
    let null = encode(&ENC, 1, &v1(Value::Struct(None), Value::Array(Some(vec![])), -1, vec![])).unwrap();
    assert_eq!(null, vec![0x02, b'n', 0xff, 0x01, 0x00]);
    let inner = Value::Struct(Some(Box::new(sv(vec![("X", Value::I8(3))], vec![]))));
    let present = encode(&ENC, 1, &v1(inner.clone(), Value::Array(Some(vec![])), -1, vec![])).unwrap();
    assert_eq!(present, vec![0x02, b'n', 0x01, 0x03, 0x00, 0x01, 0x00]);
    // decode: any negative marker is null
    let (d, _) = decode(&ENC, 1, &[0x02, b'n', 0x80, 0x01, 0x00]).unwrap();
    assert_eq!(d.get("Maybe"), Some(&Value::Struct(None)));
    let (d, _) = decode(&ENC, 1, &present).unwrap();
    assert_eq!(d.get("Maybe"), Some(&inner));
}

#[test]
fn tagged_default_is_omitted_non_default_written() {
    let omitted = encode(&ENC, 1, &v1(Value::Struct(None), Value::Array(None), -1, vec![])).unwrap();
    assert_eq!(omitted, vec![0x02, b'n', 0xff, 0x00, 0x00]);
    let written = encode(&ENC, 1, &v1(Value::Struct(None), Value::Array(None), 5, vec![])).unwrap();
    assert_eq!(written, vec![0x02, b'n', 0xff, 0x00, 0x01, 0x00, 0x08, 0, 0, 0, 0, 0, 0, 0, 5]);
}

#[test]
fn unknown_tags_merge_in_tag_order() {
    let out = encode(&ENC, 1, &v1(Value::Struct(None), Value::Array(None), 5, vec![(7, vec![0xaa])])).unwrap();
    assert_eq!(&out[4..], &[0x02, 0x00, 0x08, 0, 0, 0, 0, 0, 0, 0, 5, 0x07, 0x01, 0xaa]);
    let dup = v1(Value::Struct(None), Value::Array(None), 5, vec![(0, vec![])]);
    assert!(matches!(encode(&ENC, 1, &dup), Err(SpecError::Shape(_))));
}

#[test]
fn shape_errors() {
    // null in non-nullable array at v0
    let v0 = sv(vec![("Name", Value::Str(Some(vec![]))), ("Maybe", Value::Struct(None)), ("Ids", Value::Array(None))], vec![]);
    assert!(matches!(encode(&ENC, 0, &v0), Err(SpecError::Shape(_))));
    // field list does not match version (Hint absent in v0)
    let wrong = v1(Value::Struct(None), Value::Array(None), -1, vec![]);
    assert!(matches!(encode(&ENC, 0, &wrong), Err(SpecError::Shape(_))));
    // unknown tags in a non-flexible version
    let v0_tags = sv(vec![("Name", Value::Str(Some(vec![]))), ("Maybe", Value::Struct(None)), ("Ids", Value::Array(Some(vec![])))], vec![(9, vec![])]);
    assert!(matches!(encode(&ENC, 0, &v0_tags), Err(SpecError::Shape(_))));
}

#[test]
fn encode_decode_roundtrip_v0() {
    let v0 = sv(
        vec![("Name", Value::Str(Some(b"xy".to_vec()))), ("Maybe", Value::Struct(None)), ("Ids", Value::Array(Some(vec![Value::I32(4)])))],
        vec![],
    );
    let bytes = encode(&ENC, 0, &v0).unwrap();
    assert_eq!(bytes, vec![0, 2, b'x', b'y', 0xff, 0, 0, 0, 1, 0, 0, 0, 4]);
    assert_eq!(decode(&ENC, 0, &bytes).unwrap(), (v0, bytes.len()));
}

static API_VERSIONS_RESPONSE: MessageSchema = MessageSchema {
    name: "ApiVersionsResponse",
    api_key: Some(18),
    kind: Kind::Response,
    valid: Versions::range(0, 4),
    flexible: Versions::since(3),
    root: &ROOT,
};

#[test]
fn header_versions() {
    assert_eq!(request_header_version(&ENC, 0), 1);
    assert_eq!(request_header_version(&ENC, 1), 2);
    assert_eq!(response_header_version(&ENC, 0), 0);
    assert_eq!(response_header_version(&ENC, 1), 1);
    // KIP-511: ApiVersions responses always use header v0
    assert_eq!(response_header_version(&API_VERSIONS_RESPONSE, 3), 0);
}

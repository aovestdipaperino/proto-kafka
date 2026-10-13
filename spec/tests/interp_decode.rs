use proto_kafka_spec::error::{SpecError, Strict};
use proto_kafka_spec::interp::{decode, decode_with, Mode};
use proto_kafka_spec::schema::*;
use proto_kafka_spec::value::{StructValue, Value};

static TOPIC: StructSchema = StructSchema {
    name: "Topic",
    fields: &[
        FieldDesc {
            name: "Name",
            ty: FieldType::Prim(Prim::String),
            versions: Versions::since(0),
            ..FieldDesc::DEFAULT
        },
        FieldDesc {
            name: "Partitions",
            ty: FieldType::PrimArray(Prim::Int32),
            versions: Versions::since(0),
            ..FieldDesc::DEFAULT
        },
    ],
};

static LEADER: StructSchema = StructSchema {
    name: "Leader",
    fields: &[FieldDesc {
        name: "Id",
        ty: FieldType::Prim(Prim::Int32),
        versions: Versions::since(0),
        ..FieldDesc::DEFAULT
    }],
};

static ROOT: StructSchema = StructSchema {
    name: "TestRequest",
    fields: &[
        FieldDesc {
            name: "Id",
            ty: FieldType::Prim(Prim::Int32),
            versions: Versions::since(0),
            ..FieldDesc::DEFAULT
        },
        FieldDesc {
            name: "Note",
            ty: FieldType::Prim(Prim::String),
            versions: Versions::since(1),
            nullable: Versions::since(1),
            ..FieldDesc::DEFAULT
        },
        FieldDesc {
            name: "Topics",
            ty: FieldType::StructArray(&TOPIC),
            versions: Versions::since(0),
            ..FieldDesc::DEFAULT
        },
        FieldDesc {
            name: "Hint",
            ty: FieldType::Prim(Prim::Int64),
            versions: Versions::since(2),
            tag: Some(0),
            tagged: Versions::since(2),
            default: DefaultVal::Int(-1),
            ..FieldDesc::DEFAULT
        },
        FieldDesc {
            name: "CurrentLeader",
            ty: FieldType::Struct(&LEADER),
            versions: Versions::since(2),
            tag: Some(2),
            tagged: Versions::since(2),
            ..FieldDesc::DEFAULT
        },
        FieldDesc {
            name: "Later",
            ty: FieldType::Prim(Prim::Int8),
            versions: Versions::since(3),
            tag: Some(1),
            tagged: Versions::since(3),
            ..FieldDesc::DEFAULT
        },
    ],
};

static TEST: MessageSchema = MessageSchema {
    name: "TestRequest",
    api_key: Some(1000),
    kind: Kind::Request,
    valid: Versions::range(0, 3),
    flexible: Versions::since(2),
    root: &ROOT,
};

fn s(v: &[u8]) -> Value {
    Value::Str(Some(v.to_vec()))
}
fn st(fields: Vec<(&'static str, Value)>) -> Value {
    Value::Struct(Some(Box::new(StructValue {
        fields,
        unknown_tags: vec![],
    })))
}
fn topic_a12() -> Value {
    Value::Array(Some(vec![st(vec![
        ("Name", s(b"a")),
        (
            "Partitions",
            Value::Array(Some(vec![Value::I32(1), Value::I32(2)])),
        ),
    ])]))
}
fn default_leader() -> Value {
    st(vec![("Id", Value::I32(0))])
}

const V0: &[u8] = &[
    0, 0, 0, 7, // Id
    0, 0, 0, 1, // Topics len
    0, 1, b'a', // Name
    0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 2, // Partitions
];

#[test]
fn v0_classic_encoding() {
    let (v, n) = decode(&TEST, 0, V0).unwrap();
    assert_eq!(n, V0.len());
    assert_eq!(
        v.fields,
        vec![("Id", Value::I32(7)), ("Topics", topic_a12())]
    );
}

#[test]
fn v1_nullable_string_null() {
    let bytes = [&[0, 0, 0, 7, 0xff, 0xff][..], &V0[4..]].concat();
    let (v, _) = decode(&TEST, 1, &bytes).unwrap();
    assert_eq!(v.get("Note"), Some(&Value::Str(None)));
}

#[test]
fn v1_negative_length_below_minus_one_is_strict() {
    let bytes = [&[0, 0, 0, 7, 0xff, 0xfe][..], &V0[4..]].concat();
    assert_eq!(
        decode(&TEST, 1, &bytes),
        Err(SpecError::Strict(Strict::S1NegativeLength))
    );
}

#[test]
fn non_nullable_null_is_rejected() {
    let mut bytes = V0.to_vec();
    bytes[8] = 0xff; // Name length -1
    bytes[9] = 0xff;
    assert_eq!(
        decode(&TEST, 0, &bytes),
        Err(SpecError::NullNotAllowed("Name"))
    );
}

#[test]
fn invalid_utf8_is_strict() {
    let mut bytes = V0.to_vec();
    bytes[10] = 0xff;
    assert_eq!(
        decode(&TEST, 0, &bytes),
        Err(SpecError::Strict(Strict::S2InvalidUtf8))
    );
}

/// v2 body up to (not including) the root tagged section.
const V2_BODY: &[u8] = &[
    0, 0, 0, 7,    // Id
    0x00, // Note: compact null
    0x02, // Topics: compact len 1
    0x02, b'a', // Name compact
    0x03, 0, 0, 0, 1, 0, 0, 0, 2,    // Partitions compact len 2
    0x00, // Topic tagged section: empty
];

#[test]
fn v2_flexible_with_tagged_field() {
    let bytes = [V2_BODY, &[0x01, 0x00, 0x08, 0, 0, 0, 0, 0, 0, 0, 5]].concat();
    let (v, n) = decode(&TEST, 2, &bytes).unwrap();
    assert_eq!(n, bytes.len());
    assert_eq!(
        v.fields,
        vec![
            ("Id", Value::I32(7)),
            ("Note", Value::Str(None)),
            ("Topics", topic_a12()),
            ("Hint", Value::I64(5)),
            ("CurrentLeader", default_leader()),
        ]
    );
}

#[test]
fn v2_absent_tagged_field_takes_default() {
    let bytes = [V2_BODY, &[0x00]].concat();
    let (v, _) = decode(&TEST, 2, &bytes).unwrap();
    assert_eq!(v.get("Hint"), Some(&Value::I64(-1)));
}

#[test]
fn tagged_struct_field_decodes_in_window() {
    // tag 2, size 6: Leader.Id = 9, then Leader's own empty tagged section, then 1 pad byte => S4
    let bad = [V2_BODY, &[0x01, 0x02, 0x06, 0, 0, 0, 9, 0x00, 0xaa]].concat();
    assert_eq!(
        decode(&TEST, 2, &bad),
        Err(SpecError::Strict(Strict::S4TagSize))
    );
    let good = [V2_BODY, &[0x01, 0x02, 0x05, 0, 0, 0, 9, 0x00]].concat();
    let (v, _) = decode(&TEST, 2, &good).unwrap();
    assert_eq!(
        v.get("CurrentLeader"),
        Some(&st(vec![("Id", Value::I32(9))]))
    );
}

#[test]
fn unknown_tags_are_preserved_in_order() {
    let bytes = [V2_BODY, &[0x02, 0x05, 0x01, 0xaa, 0x07, 0x00]].concat();
    let (v, _) = decode(&TEST, 2, &bytes).unwrap();
    assert_eq!(v.unknown_tags, vec![(5, vec![0xaa]), (7, vec![])]);
}

#[test]
fn tags_out_of_order_or_duplicated_are_strict() {
    let dup = [V2_BODY, &[0x02, 0x05, 0x00, 0x05, 0x00]].concat();
    assert_eq!(
        decode(&TEST, 2, &dup),
        Err(SpecError::Strict(Strict::S3TagOrder))
    );
    let desc = [
        V2_BODY,
        &[0x02, 0x05, 0x00, 0x00, 0x08, 0, 0, 0, 0, 0, 0, 0, 5],
    ]
    .concat();
    assert_eq!(
        decode(&TEST, 2, &desc),
        Err(SpecError::Strict(Strict::S3TagOrder))
    );
}

#[test]
fn known_tag_short_size_is_strict() {
    let bytes = [V2_BODY, &[0x01, 0x00, 0x04, 0, 0, 0, 5]].concat();
    assert_eq!(
        decode(&TEST, 2, &bytes),
        Err(SpecError::Strict(Strict::S4TagSize))
    );
}

#[test]
fn known_tag_invalid_for_version_is_rejected() {
    let bytes = [V2_BODY, &[0x01, 0x01, 0x01, 0x00]].concat();
    assert_eq!(
        decode(&TEST, 2, &bytes),
        Err(SpecError::TagNotValidForVersion(1))
    );
}

#[test]
fn array_longer_than_remaining_is_rejected() {
    let bytes = [&[0, 0, 0, 7, 0x00, 0x7f][..]].concat(); // Topics compact len 126
    assert_eq!(
        decode(&TEST, 2, &bytes),
        Err(SpecError::ArrayTooLong("Topics"))
    );
}

#[test]
fn compact_string_over_0x7fff_is_rejected() {
    // Note: compact length 0x8000 + 1 => uvarint 0x8001 = [0x81, 0x80, 0x02]
    let bytes = [&[0, 0, 0, 7, 0x81, 0x80, 0x02][..], &[0u8; 0x8000]].concat();
    assert_eq!(
        decode(&TEST, 2, &bytes),
        Err(SpecError::StringTooLong("Note"))
    );
}

#[test]
fn unsupported_version() {
    assert_eq!(decode(&TEST, 4, V0), Err(SpecError::UnsupportedVersion(4)));
}

#[test]
fn truncated_input_is_eof() {
    assert_eq!(decode(&TEST, 0, &V0[..V0.len() - 1]), Err(SpecError::Eof));
}

fn lenient(version: i16, b: &[u8]) -> (StructValue, usize) {
    decode_with(&TEST, version, b, Mode::JavaLenient).unwrap()
}

#[test]
fn java_lenient_negative_length_is_null() {
    let bytes = [&[0, 0, 0, 7, 0xff, 0xfe][..], &V0[4..]].concat();
    assert_eq!(lenient(1, &bytes).0.get("Note"), Some(&Value::Str(None)));
}

/// Java: `new String(arr, UTF_8)` accepts invalid UTF-8; JavaLenient keeps the raw bytes.
#[test]
fn java_lenient_keeps_invalid_utf8_bytes() {
    let mut bytes = V0.to_vec();
    bytes[10] = 0xff;
    let (v, n) = lenient(0, &bytes);
    assert_eq!(n, bytes.len());
    let Some(Value::Array(Some(topics))) = v.get("Topics") else {
        panic!()
    };
    let Value::Struct(Some(t)) = &topics[0] else {
        panic!()
    };
    assert_eq!(t.get("Name"), Some(&s(&[0xff])));
}

/// Java's tag loop has no order check; a repeated known tag overwrites (last write wins).
#[test]
fn java_lenient_duplicate_known_tag_last_write_wins() {
    let bytes = [
        V2_BODY,
        &[0x02, 0x00, 0x08, 0, 0, 0, 0, 0, 0, 0, 5],
        &[0x00, 0x08, 0, 0, 0, 0, 0, 0, 0, 6],
    ]
    .concat();
    assert_eq!(
        decode(&TEST, 2, &bytes),
        Err(SpecError::Strict(Strict::S3TagOrder))
    );
    let (v, n) = lenient(2, &bytes);
    assert_eq!((v.get("Hint"), n), (Some(&Value::I64(6)), bytes.len()));
}

/// Java reads a known tag's value from the main stream and ignores `_size`.
#[test]
fn java_lenient_known_tag_ignores_size() {
    let bytes = [V2_BODY, &[0x01, 0x00, 0x01, 0, 0, 0, 0, 0, 0, 0, 5]].concat();
    assert_eq!(
        decode(&TEST, 2, &bytes),
        Err(SpecError::Strict(Strict::S4TagSize))
    );
    let (v, n) = lenient(2, &bytes);
    assert_eq!((v.get("Hint"), n), (Some(&Value::I64(5)), bytes.len()));
}

/// Java: `int _numTaggedFields = readUnsignedVarint()`; 0x80000000 is negative, so the
/// loop runs zero times. Strict: S7.
#[test]
fn tag_count_at_or_above_2_pow_31() {
    let bytes = [V2_BODY, &[0x80, 0x80, 0x80, 0x80, 0x08]].concat();
    assert_eq!(
        decode(&TEST, 2, &bytes),
        Err(SpecError::Strict(Strict::S7TagCountOverflow))
    );
    let (v, n) = lenient(2, &bytes);
    assert_eq!((v.get("Hint"), n), (Some(&Value::I64(-1)), bytes.len()));
    // 2^31 - 1 is a valid (non-negative) Java count: the loop runs and hits Eof.
    let max = [V2_BODY, &[0xff, 0xff, 0xff, 0xff, 0x07]].concat();
    assert_eq!(decode(&TEST, 2, &max), Err(SpecError::Eof));
}

static TAGGED_ARRAY_ROOT: StructSchema = StructSchema {
    name: "TaggedArray",
    fields: &[FieldDesc {
        name: "Ids",
        ty: FieldType::PrimArray(Prim::Int32),
        versions: Versions::since(0),
        tag: Some(0),
        tagged: Versions::since(0),
        ..FieldDesc::DEFAULT
    }],
};

static TAGGED_ARRAY: MessageSchema = MessageSchema {
    name: "TaggedArray",
    api_key: None,
    kind: Kind::Data,
    valid: Versions::range(0, 0),
    flexible: Versions::since(0),
    root: &TAGGED_ARRAY_ROOT,
};

/// Java checks an array length against the whole buffer's remaining bytes, not the tag's
/// declared size. An array that overruns the window but fits the buffer is accepted by
/// Java (and JavaLenient), so the strict rejection must be S4, not ArrayTooLong.
#[test]
fn array_too_long_inside_known_tag_window_is_s4() {
    let bytes = [0x01, 0x00, 0x02, 0x03, 0, 0, 0, 1, 0, 0, 0, 2];
    assert_eq!(
        decode(&TAGGED_ARRAY, 0, &bytes),
        Err(SpecError::Strict(Strict::S4TagSize))
    );
    let (v, n) = decode_with(&TAGGED_ARRAY, 0, &bytes, Mode::JavaLenient).unwrap();
    assert_eq!(n, bytes.len());
    assert_eq!(
        v.get("Ids"),
        Some(&Value::Array(Some(vec![Value::I32(1), Value::I32(2)])))
    );
}

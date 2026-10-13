#![allow(dead_code)]

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use proto_kafka_spec::interp::{default_value, is_tagged};
use proto_kafka_spec::schema::{
    find, find_by_name, FieldType, Kind, MessageSchema, Prim, StructSchema,
};
use proto_kafka_spec::value::{StructValue, Value};
use serde_json::Value as J;
use std::io::BufRead;
use std::path::PathBuf;

pub fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../verification/golden/vectors")
}

pub fn lines(name: &str) -> Vec<J> {
    let f = std::fs::File::open(vectors_dir().join(name))
        .unwrap_or_else(|e| panic!("{name}: {e} (run Task 6 generator)"));
    std::io::BufReader::new(f)
        .lines()
        .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
        .collect()
}

pub fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

pub fn schema_for(line: &J) -> &'static MessageSchema {
    let key = line["api_key"].as_i64().unwrap() as i16;
    match line["kind"].as_str().unwrap() {
        "request" => find(key, Kind::Request).unwrap(),
        "response" => find(key, Kind::Response).unwrap(),
        "request_header" => find_by_name("RequestHeader").unwrap(),
        "response_header" => find_by_name("ResponseHeader").unwrap(),
        k => panic!("unknown kind {k}"),
    }
}

fn json_key(name: &str) -> String {
    let mut c = name.chars();
    let first = c.next().unwrap().to_ascii_lowercase();
    std::iter::once(first).chain(c).collect()
}

/// Java JsonConverter JSON -> Value. Absent *tagged* fields take their default;
/// an absent *untagged* field is a generator/spec bug and panics.
pub fn json_struct(s: &StructSchema, version: i16, j: &J) -> StructValue {
    let obj = j
        .as_object()
        .unwrap_or_else(|| panic!("{}: expected object, got {j}", s.name));
    let mut fields = Vec::new();
    for f in s.fields.iter().filter(|f| f.versions.contains(version)) {
        let v = match obj.get(&json_key(f.name)) {
            Some(node) => json_field(f.ty, version, node),
            None if is_tagged(f, version) => default_value(f, version),
            None => panic!(
                "{}.{}: untagged field missing from Java JSON {j}",
                s.name, f.name
            ),
        };
        fields.push((f.name, v));
    }
    let unknown_tags = obj
        .get("_unknownTaggedFields")
        .map(|a| {
            a.as_array()
                .unwrap()
                .iter()
                .map(|t| {
                    (
                        t["tag"].as_u64().unwrap() as u32,
                        hex(t["data"].as_str().unwrap()),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    StructValue {
        fields,
        unknown_tags,
    }
}

fn json_field(ty: FieldType, version: i16, n: &J) -> Value {
    match ty {
        FieldType::Prim(p) => json_prim(p, n),
        FieldType::Struct(s) => {
            Value::Struct((!n.is_null()).then(|| Box::new(json_struct(s, version, n))))
        }
        FieldType::PrimArray(p) => Value::Array(
            n.as_array()
                .map(|a| a.iter().map(|e| json_prim(p, e)).collect()),
        ),
        FieldType::StructArray(s) => Value::Array(n.as_array().map(|a| {
            a.iter()
                .map(|e| Value::Struct(Some(Box::new(json_struct(s, version, e)))))
                .collect()
        })),
    }
}

fn json_prim(p: Prim, n: &J) -> Value {
    match p {
        Prim::Bool => Value::Bool(n.as_bool().unwrap()),
        Prim::Int8 => Value::I8(n.as_i64().unwrap() as i8),
        Prim::Int16 => Value::I16(n.as_i64().unwrap() as i16),
        Prim::Uint16 => Value::U16(n.as_i64().unwrap() as u16),
        Prim::Int32 => Value::I32(n.as_i64().unwrap() as i32),
        Prim::Int64 => Value::I64(n.as_i64().unwrap()),
        Prim::Float64 => Value::F64(
            u64::from_str_radix(n.as_str().unwrap().strip_prefix("f64:").unwrap(), 16).unwrap(),
        ),
        Prim::Uuid => Value::Uuid(
            URL_SAFE_NO_PAD
                .decode(n.as_str().unwrap())
                .unwrap()
                .try_into()
                .unwrap(),
        ),
        Prim::String => Value::Str(n.as_str().map(|s| s.as_bytes().to_vec())),
        Prim::Bytes => Value::Bytes(n.as_str().map(|s| STANDARD.decode(s).unwrap())),
        Prim::Records => Value::Records(n.as_str().map(hex)),
    }
}

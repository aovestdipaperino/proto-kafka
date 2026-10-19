//! Bounded framing proofs: String/CompactString/Bytes/CompactBytes/Array/CompactArray codecs
//! against the spec interpreter. Each one-field schema below is non-flexible at the message
//! level (no tagged section); the field-level `flexible` override selects the compact
//! encoding, exactly as RequestHeader.ClientId does in reverse.
//!
//! Decode harnesses (exploration code) compare against `interp::decode_field`, the Value-free
//! field decoder that `interp::decode` applies to every field (a whole-message `interp::decode`
//! builds a recursive `Value` tree that CBMC cannot symbolically execute). They are kept here
//! for reference, but disabled due to CBMC capacity limits. See docs/FINDINGS.md "Task 6 blocker".

#![allow(dead_code, unused_imports)]

use crate::protocol::types;
use crate::protocol::{Decoder, Encoder};
use proto_kafka_spec::interp::{self, decode_field, Field, Mode, PrimValue};
use proto_kafka_spec::schema::*;
use proto_kafka_spec::value::{StructValue, Value};
use proto_kafka_spec::wire::LenKind;

macro_rules! one_field {
    ($name:ident, $ty:expr, $nullable:expr, $compact:expr) => {
        static $name: MessageSchema = MessageSchema {
            name: stringify!($name),
            api_key: None,
            kind: Kind::Data,
            valid: Versions::range(0, 0),
            flexible: Versions::NONE,
            root: &StructSchema {
                name: stringify!($name),
                fields: &[FieldDesc {
                    name: "V",
                    ty: $ty,
                    versions: Versions::since(0),
                    nullable: $nullable,
                    flexible: $compact,
                    ..FieldDesc::DEFAULT
                }],
            },
        };
    };
}

const NULL: Versions = Versions::since(0);
const NOT_NULL: Versions = Versions::NONE;
const COMPACT: Option<Versions> = Some(Versions::since(0));
const CLASSIC: Option<Versions> = Some(Versions::NONE);

one_field!(STR_N, FieldType::Prim(Prim::String), NULL, CLASSIC);
one_field!(STR, FieldType::Prim(Prim::String), NOT_NULL, CLASSIC);
one_field!(CSTR_N, FieldType::Prim(Prim::String), NULL, COMPACT);
one_field!(BYTES_N, FieldType::Prim(Prim::Bytes), NULL, CLASSIC);
one_field!(CBYTES_N, FieldType::Prim(Prim::Bytes), NULL, COMPACT);
one_field!(ARR_N, FieldType::PrimArray(Prim::Int32), NULL, CLASSIC);
one_field!(CARR_N, FieldType::PrimArray(Prim::Int32), NULL, COMPACT);
/// the spec's.
#[kani::proof]
#[kani::unwind(8)]
fn compact_bytes_encode() {
    let data: [u8; 3] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 3);
    let is_null: bool = kani::any();
    let v: Option<Vec<u8>> = if is_null {
        None
    } else {
        Some(data[..len].to_vec())
    };
    let mut buf: Vec<u8> = Vec::new();
    types::CompactBytes.encode(&mut buf, &v).unwrap();
    let sv = StructValue {
        fields: vec![("V", Value::Bytes(v))],
        unknown_tags: vec![],
    };
    assert_eq!(buf, interp::encode(&CBYTES_N, 0, &sv).unwrap());
}

/// Encode side: every ASCII string of exactly `N` bytes encodes as the spec says. One
/// harness per `N` in 0..=3 (a symbolic length makes the UTF-8 validation in
/// `String::from_utf8` intractable; the four together cover every ASCII string of up to 3 bytes).
fn string_encode_ascii_n<const N: usize>() {
    let data: [u8; N] = kani::any();
    kani::assume(data.iter().all(|b| *b < 0x80));
    let s = String::from_utf8(data.to_vec()).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    types::String.encode(&mut buf, &s).unwrap();
    let sv = StructValue {
        fields: vec![("V", Value::Str(Some(s.into_bytes())))],
        unknown_tags: vec![],
    };
    assert_eq!(buf, interp::encode(&STR, 0, &sv).unwrap());
}

macro_rules! string_encode_harness {
    ($($name:ident = $n:literal),* $(,)?) => {$(
        #[kani::proof]
        #[kani::unwind(8)]
        fn $name() {
            string_encode_ascii_n::<$n>();
        }
    )*};
}

string_encode_harness!(
    string_encode_ascii_len0 = 0,
    string_encode_ascii_len1 = 1,
    string_encode_ascii_len2 = 2,
    string_encode_ascii_len3 = 3,
);

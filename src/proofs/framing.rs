//! Bounded framing proofs: String/CompactString/Bytes/CompactBytes/Array/CompactArray codecs
//! against the spec interpreter. Each one-field schema below is non-flexible at the message
//! level (no tagged section); the field-level `flexible` override selects the compact
//! encoding, exactly as RequestHeader.ClientId does in reverse.
//!
//! Status: the encode harnesses are proofs. The decode harnesses are BLOCKED (written, not
//! registered): `interp::decode` is out of reach for Kani 0.68 / CBMC 6.11; see the note above
//! `string_nullable_decode`.

use crate::protocol::types;
use crate::protocol::{Decoder, Encoder};
use proto_kafka_spec::interp;
use proto_kafka_spec::schema::*;
use proto_kafka_spec::value::{StructValue, Value};

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

/// Spec decode of the single field: (value, bytes consumed).
fn spec(schema: &'static MessageSchema, input: &[u8]) -> Option<(Value, usize)> {
    interp::decode(schema, 0, input).ok().map(|(sv, n)| (sv.fields[0].1.clone(), n))
}

fn check<T>(got: crate::error::Result<T>, consumed: usize, want: Option<(Value, usize)>, to_value: impl Fn(T) -> Value) {
    match (got, want) {
        (Ok(v), Some((w, n))) => {
            assert_eq!(n, consumed);
            assert!(to_value(v) == w);
        }
        (Err(_), None) => {}
        _ => panic!("crate and spec disagree on acceptance"),
    }
}

// BLOCKED: decode harnesses against `interp::decode`.
//
// The seven decode harnesses below are written as specified but are NOT registered as Kani
// proofs, because none of them terminates under Kani 0.68.0 / CBMC 6.11.0. The cause is the
// spec interpreter, not the crate and not the input bound: `interp::decode` builds and drops
// a recursive `Value` tree (`Array(Vec<Value>)`, `Struct(Box<StructValue>)`) through heap
// memory (`read_struct`'s `vec![None; n]` slots, the zip `IntoIter`, `?` error paths). CBMC
// loses the enum discriminant through the heap and unrolls `drop_glue::<Value>` to the unwind
// depth on ~130-150 sites, so symbolic execution never finishes. Evidence: a fully concrete
// 3-byte input does not finish in 240 s at unwind(10), nor does symbolic N = 3 at unwind(4);
// the brief's versions ran 1300 s (timeout). The encode harnesses (where the harness builds
// the `StructValue` concretely) are fine. Unblocking needs a Value-free spec entry point for a
// single primitive/array field, or Kani stubbing; see docs/FINDINGS.md ("Phase 2 proofs").

// BLOCKED (not a #[kani::proof]): intended unwind(10); see the note above.
#[allow(dead_code)]
fn string_nullable_decode() {
    let bytes: [u8; 6] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 6);
    let mut s: &[u8] = &bytes[..len];
    let got: crate::error::Result<Option<String>> = types::String.decode(&mut s);
    check(got, len - s.len(), spec(&STR_N, &bytes[..len]), |v| Value::Str(v.map(String::into_bytes)));
}

// BLOCKED (not a #[kani::proof]): intended unwind(10); see the note above.
#[allow(dead_code)]
fn string_decode() {
    let bytes: [u8; 6] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 6);
    let mut s: &[u8] = &bytes[..len];
    let got: crate::error::Result<String> = types::String.decode(&mut s);
    check(got, len - s.len(), spec(&STR, &bytes[..len]), |v| Value::Str(Some(v.into_bytes())));
}

// BLOCKED (not a #[kani::proof]): intended unwind(10); see the note above.
#[allow(dead_code)]
fn compact_string_nullable_decode() {
    let bytes: [u8; 6] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 6);
    let mut s: &[u8] = &bytes[..len];
    let got: crate::error::Result<Option<String>> = types::CompactString.decode(&mut s);
    check(got, len - s.len(), spec(&CSTR_N, &bytes[..len]), |v| Value::Str(v.map(String::into_bytes)));
}

// BLOCKED (not a #[kani::proof]): intended unwind(10); see the note above.
#[allow(dead_code)]
fn bytes_nullable_decode() {
    let bytes: [u8; 7] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 7);
    let mut s: &[u8] = &bytes[..len];
    let got: crate::error::Result<Option<Vec<u8>>> = types::Bytes.decode(&mut s);
    check(got, len - s.len(), spec(&BYTES_N, &bytes[..len]), Value::Bytes);
}

// BLOCKED (not a #[kani::proof]): intended unwind(10); see the note above.
#[allow(dead_code)]
fn compact_bytes_nullable_decode() {
    let bytes: [u8; 6] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 6);
    let mut s: &[u8] = &bytes[..len];
    let got: crate::error::Result<Option<Vec<u8>>> = types::CompactBytes.decode(&mut s);
    check(got, len - s.len(), spec(&CBYTES_N, &bytes[..len]), Value::Bytes);
}

// BLOCKED (not a #[kani::proof]): intended unwind(14); see the note above.
#[allow(dead_code)]
fn array_int32_nullable_decode() {
    let bytes: [u8; 12] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 12);
    let mut s: &[u8] = &bytes[..len];
    let got: crate::error::Result<Option<Vec<i32>>> = types::Array(types::Int32).decode(&mut s);
    check(got, len - s.len(), spec(&ARR_N, &bytes[..len]), |v| {
        Value::Array(v.map(|xs| xs.into_iter().map(Value::I32).collect()))
    });
}

// BLOCKED (not a #[kani::proof]): intended unwind(14); see the note above.
#[allow(dead_code)]
fn compact_array_int32_nullable_decode() {
    let bytes: [u8; 10] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 10);
    let mut s: &[u8] = &bytes[..len];
    let got: crate::error::Result<Option<Vec<i32>>> = types::CompactArray(types::Int32).decode(&mut s);
    check(got, len - s.len(), spec(&CARR_N, &bytes[..len]), |v| {
        Value::Array(v.map(|xs| xs.into_iter().map(Value::I32).collect()))
    });
}

/// Encode side: for every nullable byte string of up to 3 bytes, the crate's encoding equals
/// the spec's.
#[kani::proof]
#[kani::unwind(8)]
fn compact_bytes_encode() {
    let data: [u8; 3] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 3);
    let is_null: bool = kani::any();
    let v: Option<Vec<u8>> = if is_null { None } else { Some(data[..len].to_vec()) };
    let mut buf: Vec<u8> = Vec::new();
    types::CompactBytes.encode(&mut buf, &v).unwrap();
    let sv = StructValue { fields: vec![("V", Value::Bytes(v))], unknown_tags: vec![] };
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
    let sv = StructValue { fields: vec![("V", Value::Str(Some(s.into_bytes())))], unknown_tags: vec![] };
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


//! Complete proofs: a varint decoder reads at most 5 (10) bytes, so symbolic inputs of up to
//! 6 (11) bytes cover every prefix; the encoders take every u32/u64. Error kinds are not
//! compared (design spec), only acceptance, value and bytes consumed.

use crate::protocol::primitives as p;
use proto_kafka_spec::wire::{LenKind, Reader, Writer};

fn spec_uvarint(input: &[u8]) -> Option<(u32, usize)> {
    let mut r = Reader::new(input);
    r.uvarint().ok().map(|v| (v, r.pos()))
}

fn spec_uvarlong(input: &[u8]) -> Option<(u64, usize)> {
    let mut r = Reader::new(input);
    r.uvarlong().ok().map(|v| (v, r.pos()))
}

#[kani::proof]
#[kani::unwind(8)]
fn uvarint_decode_conforms() {
    let bytes: [u8; 6] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 6);
    let input = &bytes[..len];
    assert_eq!(p::read_uvarint(input).ok(), spec_uvarint(input));
}

#[kani::proof]
#[kani::unwind(13)]
fn uvarlong_decode_conforms() {
    let bytes: [u8; 11] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 11);
    let input = &bytes[..len];
    assert_eq!(p::read_uvarlong(input).ok(), spec_uvarlong(input));
}

#[kani::proof]
#[kani::unwind(8)]
fn varint_signed_decode_conforms() {
    let bytes: [u8; 6] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 6);
    let input = &bytes[..len];
    let mut r = Reader::new(input);
    let spec = r.varint().ok().map(|v| (v, r.pos()));
    let ours = p::read_uvarint(input)
        .ok()
        .map(|(u, n)| (p::zigzag_decode32(u), n));
    assert_eq!(ours, spec);
}

#[kani::proof]
#[kani::unwind(13)]
fn varlong_signed_decode_conforms() {
    let bytes: [u8; 11] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 11);
    let input = &bytes[..len];
    let mut r = Reader::new(input);
    let spec = r.varlong().ok().map(|v| (v, r.pos()));
    let ours = p::read_uvarlong(input)
        .ok()
        .map(|(u, n)| (p::zigzag_decode64(u), n));
    assert_eq!(ours, spec);
}

#[kani::proof]
#[kani::unwind(7)]
fn uvarint_encode_conforms() {
    let v: u32 = kani::any();
    let mut out = [0u8; 5];
    let n = p::write_uvarint(v, &mut out);
    let mut w = Writer::new();
    w.uvarint(v);
    assert_eq!(&out[..n], w.into_bytes().as_slice());
    assert_eq!(p::uvarint_size(v), n);
    assert_eq!(p::read_uvarint(&out[..n]), Ok((v, n)));
}

#[kani::proof]
#[kani::unwind(12)]
fn uvarlong_encode_conforms() {
    let v: u64 = kani::any();
    let mut out = [0u8; 10];
    let n = p::write_uvarlong(v, &mut out);
    let mut w = Writer::new();
    w.uvarlong(v);
    assert_eq!(&out[..n], w.into_bytes().as_slice());
    assert_eq!(p::uvarlong_size(v), n);
    assert_eq!(p::read_uvarlong(&out[..n]), Ok((v, n)));
}

#[kani::proof]
#[kani::unwind(7)]
fn varint_signed_encode_conforms() {
    let v: i32 = kani::any();
    let mut out = [0u8; 5];
    let n = p::write_uvarint(p::zigzag_encode32(v), &mut out);
    let mut w = Writer::new();
    w.varint(v);
    assert_eq!(&out[..n], w.into_bytes().as_slice());
}

#[kani::proof]
#[kani::unwind(12)]
fn varlong_signed_encode_conforms() {
    let v: i64 = kani::any();
    let mut out = [0u8; 10];
    let n = p::write_uvarlong(p::zigzag_encode64(v), &mut out);
    let mut w = Writer::new();
    w.varlong(v);
    assert_eq!(&out[..n], w.into_bytes().as_slice());
}

/// The wrapper rule: `None` (null) is an error for non-nullable fields.
fn ours_len(raw: i32, nullable: bool) -> Option<Option<usize>> {
    match p::interpret_len(raw) {
        Ok(None) if !nullable => None,
        Ok(x) => Some(x),
        Err(_) => None,
    }
}

fn spec_len(bytes: &[u8], kind: LenKind, nullable: bool) -> Option<Option<usize>> {
    Reader::new(bytes).len_prefix(kind, nullable, "f").ok()
}

#[kani::proof]
fn int16_length_conforms() {
    let raw: i16 = kani::any();
    let nullable: bool = kani::any();
    assert_eq!(
        ours_len(i32::from(raw), nullable),
        spec_len(&raw.to_be_bytes(), LenKind::Int16, nullable)
    );
}

#[kani::proof]
fn int32_length_conforms() {
    let raw: i32 = kani::any();
    let nullable: bool = kani::any();
    assert_eq!(
        ours_len(raw, nullable),
        spec_len(&raw.to_be_bytes(), LenKind::Int32, nullable)
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn compact_length_conforms() {
    let raw: u32 = kani::any();
    let nullable: bool = kani::any();
    let mut w = Writer::new();
    w.uvarint(raw);
    let b = w.into_bytes();
    assert_eq!(
        ours_len(p::compact_len(raw), nullable),
        spec_len(&b, LenKind::Compact, nullable)
    );
}

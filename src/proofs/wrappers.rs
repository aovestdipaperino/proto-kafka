//! The `Buf`/`BufMut` adapters in `protocol::types` equal the pure codecs (complete).

use crate::protocol::primitives as p;
use crate::protocol::types;
use crate::protocol::{Decoder, Encoder};

fn wrap_u32(input: &[u8]) -> Option<(u32, usize)> {
    let mut s: &[u8] = input;
    let r: crate::error::Result<u32> = types::UnsignedVarInt.decode(&mut s);
    r.ok().map(|v| (v, input.len() - s.len()))
}

fn wrap_u64(input: &[u8]) -> Option<(u64, usize)> {
    let mut s: &[u8] = input;
    let r: crate::error::Result<u64> = types::UnsignedVarLong.decode(&mut s);
    r.ok().map(|v| (v, input.len() - s.len()))
}

#[kani::proof]
#[kani::unwind(8)]
fn unsigned_varint_wrapper_decode() {
    let bytes: [u8; 6] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 6);
    assert_eq!(wrap_u32(&bytes[..len]), p::read_uvarint(&bytes[..len]).ok());
}

#[kani::proof]
#[kani::unwind(13)]
fn unsigned_varlong_wrapper_decode() {
    let bytes: [u8; 11] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 11);
    assert_eq!(wrap_u64(&bytes[..len]), p::read_uvarlong(&bytes[..len]).ok());
}

#[kani::proof]
#[kani::unwind(7)]
fn unsigned_varint_wrapper_encode() {
    let v: u32 = kani::any();
    let mut buf: Vec<u8> = Vec::new();
    types::UnsignedVarInt.encode(&mut buf, v).unwrap();
    let mut out = [0u8; 5];
    let n = p::write_uvarint(v, &mut out);
    assert_eq!(buf.as_slice(), &out[..n]);
    assert_eq!(types::UnsignedVarInt.compute_size(v).unwrap(), n);
}

#[kani::proof]
#[kani::unwind(12)]
fn unsigned_varlong_wrapper_encode() {
    let v: u64 = kani::any();
    let mut buf: Vec<u8> = Vec::new();
    types::UnsignedVarLong.encode(&mut buf, v).unwrap();
    let mut out = [0u8; 10];
    let n = p::write_uvarlong(v, &mut out);
    assert_eq!(buf.as_slice(), &out[..n]);
    assert_eq!(types::UnsignedVarLong.compute_size(v).unwrap(), n);
}

#[kani::proof]
#[kani::unwind(8)]
fn varint_wrapper_decode() {
    let bytes: [u8; 6] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 6);
    let mut s: &[u8] = &bytes[..len];
    let r: crate::error::Result<i32> = types::VarInt.decode(&mut s);
    let want = p::read_uvarint(&bytes[..len]).ok().map(|(u, n)| (p::zigzag_decode32(u), n));
    assert_eq!(r.ok().map(|v| (v, len - s.len())), want);
}

#[kani::proof]
#[kani::unwind(13)]
fn varlong_wrapper_decode() {
    let bytes: [u8; 11] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 11);
    let mut s: &[u8] = &bytes[..len];
    let r: crate::error::Result<i64> = types::VarLong.decode(&mut s);
    let want = p::read_uvarlong(&bytes[..len]).ok().map(|(u, n)| (p::zigzag_decode64(u), n));
    assert_eq!(r.ok().map(|v| (v, len - s.len())), want);
}

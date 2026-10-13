//! Pure primitive codecs for the Kafka wire protocol.
//!
//! This module is the single source of truth for primitive wire semantics in
//! proto-kafka: [`super::types`] wraps it for `bytes::Buf`/`BufMut`. It is written in the
//! subset Charon/Aeneas can extract to Lean (Phase 3): no generics, traits, closures,
//! iterators or heap allocation; only `while` loops, `match`, `Result` and fixed arrays.
//! Kani proves it equal to the reference spec (`proto-kafka-spec`, strict mode).
//! Semantics and the cases where it is stricter than Java: `docs/FINDINGS.md`.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless
)]

/// A primitive decoding failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimError {
    /// Input ended before the value was complete.
    Eof,
    /// Continuation bit set on the last permitted varint byte (Java rejects this too).
    VarintTooLong,
    /// Last varint byte carries bits beyond the target width (Java drops them; spec S5).
    VarintOverflow,
    /// Length prefix below -1 (Java reads it as null; spec S1).
    NegativeLength(i32),
    /// String length above 0x7fff (Java rejects).
    StringTooLong(usize),
    /// Array length (first) above the bytes remaining (second) (Java rejects).
    ArrayTooLong(usize, usize),
}

/// Maximum encoded size of an unsigned 32-bit varint.
pub const UVARINT_MAX_BYTES: u32 = 5;
/// Maximum encoded size of an unsigned 64-bit varint.
pub const UVARLONG_MAX_BYTES: u32 = 10;

/// Decoder state between varint bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarintState {
    /// Bits accumulated so far.
    pub value: u64,
    /// Number of bytes consumed so far.
    pub index: u32,
}

/// State before the first byte.
pub const VARINT_START: VarintState = VarintState { value: 0, index: 0 };

/// Result of feeding one byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarintStep {
    /// More bytes are needed.
    More(VarintState),
    /// The varint is complete.
    Done(u64),
}

/// Feeds one byte of an unsigned varint of at most `max_bytes` bytes
/// ([`UVARINT_MAX_BYTES`] or [`UVARLONG_MAX_BYTES`]).
pub fn varint_step(st: VarintState, b: u8, max_bytes: u32) -> Result<VarintStep, PrimError> {
    if st.index + 1 == max_bytes {
        if b & 0x80 != 0 {
            return Err(PrimError::VarintTooLong);
        }
        // 32-bit: the 5th byte holds bits 28..31 (mask 0x0f).
        // 64-bit: the 10th byte holds bit 63 (mask 0x01).
        let allowed: u8 = if max_bytes == UVARINT_MAX_BYTES { 0x0f } else { 0x01 };
        if b & !allowed != 0 {
            return Err(PrimError::VarintOverflow);
        }
    }
    let value = st.value | (u64::from(b & 0x7f) << (7 * st.index));
    if b & 0x80 == 0 {
        Ok(VarintStep::Done(value))
    } else {
        Ok(VarintStep::More(VarintState { value, index: st.index + 1 }))
    }
}

fn read_varint(input: &[u8], max_bytes: u32) -> Result<(u64, usize), PrimError> {
    let mut st = VARINT_START;
    let mut i: usize = 0;
    while i < input.len() {
        match varint_step(st, input[i], max_bytes) {
            Ok(VarintStep::Done(v)) => return Ok((v, i + 1)),
            Ok(VarintStep::More(next)) => st = next,
            Err(e) => return Err(e),
        }
        i += 1;
    }
    Err(PrimError::Eof)
}

/// Decodes an unsigned 32-bit varint from the front of `input`; returns (value, bytes consumed).
pub fn read_uvarint(input: &[u8]) -> Result<(u32, usize), PrimError> {
    match read_varint(input, UVARINT_MAX_BYTES) {
        Ok((v, n)) => Ok((v as u32, n)),
        Err(e) => Err(e),
    }
}

/// Decodes an unsigned 64-bit varint from the front of `input`; returns (value, bytes consumed).
pub fn read_uvarlong(input: &[u8]) -> Result<(u64, usize), PrimError> {
    read_varint(input, UVARLONG_MAX_BYTES)
}

/// Encodes `v` as an unsigned varint into `out`; returns the number of bytes written.
pub fn write_uvarint(v: u32, out: &mut [u8; 5]) -> usize {
    let mut v = v;
    let mut i: usize = 0;
    while v >= 0x80 {
        out[i] = (v as u8) | 0x80;
        v >>= 7;
        i += 1;
    }
    out[i] = v as u8;
    i + 1
}

/// Encodes `v` as an unsigned varint into `out`; returns the number of bytes written.
pub fn write_uvarlong(v: u64, out: &mut [u8; 10]) -> usize {
    let mut v = v;
    let mut i: usize = 0;
    while v >= 0x80 {
        out[i] = (v as u8) | 0x80;
        v >>= 7;
        i += 1;
    }
    out[i] = v as u8;
    i + 1
}

/// Encoded size of `v` as an unsigned varint.
pub fn uvarint_size(v: u32) -> usize {
    uvarlong_size(u64::from(v))
}

/// Encoded size of `v` as an unsigned varint.
pub fn uvarlong_size(v: u64) -> usize {
    let mut v = v;
    let mut n: usize = 1;
    while v >= 0x80 {
        v >>= 7;
        n += 1;
    }
    n
}

/// Zigzag-encodes a signed 32-bit value.
pub fn zigzag_encode32(v: i32) -> u32 {
    ((v << 1) ^ (v >> 31)) as u32
}

/// Zigzag-decodes to a signed 32-bit value.
pub fn zigzag_decode32(u: u32) -> i32 {
    ((u >> 1) as i32) ^ -((u & 1) as i32)
}

/// Zigzag-encodes a signed 64-bit value.
pub fn zigzag_encode64(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

/// Zigzag-decodes to a signed 64-bit value.
pub fn zigzag_decode64(u: u64) -> i64 {
    ((u >> 1) as i64) ^ -((u & 1) as i64)
}

/// COMPACT_* lengths: unsigned varint N encodes length N - 1, in Java `int` arithmetic.
pub fn compact_len(raw: u32) -> i32 {
    (raw as i32).wrapping_sub(1)
}

/// Interprets a length prefix: -1 is null, below -1 is rejected (spec S1), otherwise the length.
/// Whether null is allowed is the caller's decision (nullable vs non-nullable field).
pub fn interpret_len(len: i32) -> Result<Option<usize>, PrimError> {
    if len == -1 {
        Ok(None)
    } else if len < 0 {
        Err(PrimError::NegativeLength(len))
    } else {
        Ok(Some(len as usize))
    }
}

/// Kafka strings are at most 0x7fff bytes (Java rejects longer compact strings).
pub fn check_string_len(len: usize) -> Result<(), PrimError> {
    if len > 0x7fff {
        Err(PrimError::StringTooLong(len))
    } else {
        Ok(())
    }
}

/// An array may not declare more elements than there are bytes left (Java's guard against
/// hostile lengths; it also bounds pre-allocation).
pub fn check_array_len(len: usize, remaining: usize) -> Result<(), PrimError> {
    if len > remaining {
        Err(PrimError::ArrayTooLong(len, remaining))
    } else {
        Ok(())
    }
}

/// Nullable struct presence marker (int8): negative means null, anything else present
/// (Java: `readByte() < 0`).
pub fn struct_present(marker: i8) -> bool {
    marker >= 0
}

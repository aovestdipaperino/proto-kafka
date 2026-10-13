//! Primitive codecs from https://kafka.apache.org/protocol#protocol_types,
//! with edge behaviour taken from Java's ByteUtils / MessageDataGenerator
//! (see docs/FINDINGS.md).

use crate::error::{Result, SpecError, Strict};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LenKind {
    /// int16 length (STRING / NULLABLE_STRING).
    Int16,
    /// int32 length (BYTES, ARRAY, RECORDS).
    Int32,
    /// unsigned varint of length + 1 (COMPACT_*).
    Compact,
}

pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.remaining() < n {
            return Err(SpecError::Eof);
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut a = [0u8; N];
        a.copy_from_slice(self.take(N)?);
        Ok(a)
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }
    pub fn i8(&mut self) -> Result<i8> {
        Ok(i8::from_be_bytes(self.array()?))
    }
    pub fn i16(&mut self) -> Result<i16> {
        Ok(i16::from_be_bytes(self.array()?))
    }
    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    pub fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_be_bytes(self.array()?))
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    pub fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(self.array()?))
    }
    pub fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    pub fn uuid(&mut self) -> Result<[u8; 16]> {
        self.array()
    }

    /// At most 5 bytes. 5th byte: continuation => VarintTooLong (Java),
    /// bits 4..6 set => Strict S5 (Java drops them).
    pub fn uvarint(&mut self) -> Result<u32> {
        let mut result: u32 = 0;
        for i in 0..5 {
            let b = self.u8()?;
            if i == 4 {
                if b & 0x80 != 0 {
                    return Err(SpecError::VarintTooLong);
                }
                if b & 0x70 != 0 {
                    return Err(SpecError::Strict(Strict::S5VarintHighBits));
                }
            }
            result |= u32::from(b & 0x7f) << (7 * i);
            if b & 0x80 == 0 {
                return Ok(result);
            }
        }
        unreachable!("the 5th byte always returns")
    }

    /// At most 10 bytes. 10th byte: continuation => VarintTooLong, bits 1..6 set => Strict S5.
    pub fn uvarlong(&mut self) -> Result<u64> {
        let mut result: u64 = 0;
        for i in 0..10 {
            let b = self.u8()?;
            if i == 9 {
                if b & 0x80 != 0 {
                    return Err(SpecError::VarintTooLong);
                }
                if b & 0x7e != 0 {
                    return Err(SpecError::Strict(Strict::S5VarintHighBits));
                }
            }
            result |= u64::from(b & 0x7f) << (7 * i);
            if b & 0x80 == 0 {
                return Ok(result);
            }
        }
        unreachable!("the 10th byte always returns")
    }

    pub fn varint(&mut self) -> Result<i32> {
        let u = self.uvarint()?;
        Ok(((u >> 1) as i32) ^ -((u & 1) as i32))
    }

    pub fn varlong(&mut self) -> Result<i64> {
        let u = self.uvarlong()?;
        Ok(((u >> 1) as i64) ^ -((u & 1) as i64))
    }

    /// Reads a length prefix. `None` is null. Java treats every negative length as
    /// null; the spec accepts only -1 (anything else is Strict S1).
    pub fn len_prefix(
        &mut self,
        kind: LenKind,
        nullable: bool,
        field: &'static str,
    ) -> Result<Option<usize>> {
        let len: i32 = match kind {
            LenKind::Int16 => i32::from(self.i16()?),
            LenKind::Int32 => self.i32()?,
            // Java: readUnsignedVarint() - 1 in int arithmetic.
            LenKind::Compact => (self.uvarint()? as i32).wrapping_sub(1),
        };
        if len < 0 {
            if !nullable {
                return Err(SpecError::NullNotAllowed(field));
            }
            if len != -1 {
                return Err(SpecError::Strict(Strict::S1NegativeLength));
            }
            return Ok(None);
        }
        Ok(Some(len as usize))
    }
}

#[derive(Debug, Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Writer::default()
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
    pub fn bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }
    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    pub fn i8(&mut self, v: i8) {
        self.bytes(&v.to_be_bytes());
    }
    pub fn i16(&mut self, v: i16) {
        self.bytes(&v.to_be_bytes());
    }
    pub fn u16(&mut self, v: u16) {
        self.bytes(&v.to_be_bytes());
    }
    pub fn i32(&mut self, v: i32) {
        self.bytes(&v.to_be_bytes());
    }
    pub fn u32(&mut self, v: u32) {
        self.bytes(&v.to_be_bytes());
    }
    pub fn i64(&mut self, v: i64) {
        self.bytes(&v.to_be_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.bytes(&v.to_be_bytes());
    }
    pub fn uvarint(&mut self, mut v: u32) {
        while v >= 0x80 {
            self.buf.push((v as u8) | 0x80);
            v >>= 7;
        }
        self.buf.push(v as u8);
    }
    pub fn uvarlong(&mut self, mut v: u64) {
        while v >= 0x80 {
            self.buf.push((v as u8) | 0x80);
            v >>= 7;
        }
        self.buf.push(v as u8);
    }
    pub fn varint(&mut self, v: i32) {
        self.uvarint(((v << 1) ^ (v >> 31)) as u32);
    }
    pub fn varlong(&mut self, v: i64) {
        self.uvarlong(((v << 1) ^ (v >> 63)) as u64);
    }
    pub fn len_prefix(&mut self, kind: LenKind, len: Option<usize>) {
        match (kind, len) {
            (LenKind::Int16, None) => self.i16(-1),
            (LenKind::Int16, Some(n)) => self.i16(n as i16),
            (LenKind::Int32, None) => self.i32(-1),
            (LenKind::Int32, Some(n)) => self.i32(n as i32),
            (LenKind::Compact, None) => self.uvarint(0),
            (LenKind::Compact, Some(n)) => self.uvarint(n as u32 + 1),
        }
    }
}

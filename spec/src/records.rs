//! Reference spec of Kafka record sets: v2 RecordBatch (KIP-98) and legacy
//! v0/v1 message sets (KIP-32). Framing follows Java's ByteBufferLogInputStream,
//! DefaultRecordBatch, DefaultRecord and LegacyRecord. Compressed payloads are opaque.

use crate::error::{Result, SpecError, Strict};
use crate::wire::Reader;

const LOG_OVERHEAD: usize = 12; // offset (8) + size (4)
const MAGIC_OFFSET: usize = 16;
const V2_BATCH_OVERHEAD: usize = 61; // Java DefaultRecordBatch.RECORD_BATCH_OVERHEAD
const V2_CRC_START: usize = 21; // CRC32C covers attributes .. end
const LEGACY_MIN_SIZE: i32 = 14; // Java LegacyRecord.RECORD_OVERHEAD_V0
const LEGACY_CRC_START: usize = 16; // CRC32 covers magic .. end

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordSet {
    pub entries: Vec<Entry>,
    /// Bytes of an incomplete final batch, ignored as Java does.
    pub truncated_tail: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    V2(Batch),
    Legacy(LegacyMessage),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub base_offset: i64,
    pub partition_leader_epoch: i32,
    pub magic: i8,
    pub crc: u32,
    pub attributes: i16,
    pub last_offset_delta: i32,
    pub base_timestamp: i64,
    pub max_timestamp: i64,
    pub producer_id: i64,
    pub producer_epoch: i16,
    pub base_sequence: i32,
    pub payload: Payload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    Records(Vec<Record>),
    Compressed(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub attributes: i8,
    pub timestamp_delta: i64,
    pub offset_delta: i32,
    pub key: Option<Vec<u8>>,
    pub value: Option<Vec<u8>>,
    pub headers: Vec<RecordHeader>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordHeader {
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyMessage {
    pub offset: i64,
    pub crc: u32,
    pub magic: i8,
    pub attributes: i8,
    pub timestamp: Option<i64>,
    pub key: Option<Vec<u8>>,
    pub value: Option<Vec<u8>>,
}

fn err(msg: impl Into<String>) -> SpecError {
    SpecError::Records(msg.into())
}

pub fn decode(input: &[u8]) -> Result<RecordSet> {
    let mut entries = Vec::new();
    let mut pos = 0;
    loop {
        let rest = &input[pos..];
        if rest.len() < LOG_OVERHEAD {
            return Ok(RecordSet {
                entries,
                truncated_tail: rest.len(),
            });
        }
        let size = i32::from_be_bytes(rest[8..12].try_into().expect("4 bytes"));
        if size < LEGACY_MIN_SIZE {
            return Err(err(format!("record size {size} below minimum")));
        }
        let total = LOG_OVERHEAD + size as usize;
        if rest.len() < total {
            return Ok(RecordSet {
                entries,
                truncated_tail: rest.len(),
            });
        }
        let frame = &rest[..total];
        entries.push(match frame[MAGIC_OFFSET] as i8 {
            2 => Entry::V2(decode_v2(frame)?),
            0 | 1 => Entry::Legacy(decode_legacy(frame)?),
            _ => return Err(SpecError::Strict(Strict::S6UnknownMagic)),
        });
        pos += total;
    }
}

fn decode_v2(frame: &[u8]) -> Result<Batch> {
    if frame.len() < V2_BATCH_OVERHEAD {
        return Err(err("v2 batch smaller than batch overhead"));
    }
    let mut r = Reader::new(frame);
    let base_offset = r.i64()?;
    let _batch_length = r.i32()?;
    let partition_leader_epoch = r.i32()?;
    let magic = r.i8()?;
    let crc = r.u32()?;
    if crc32c::crc32c(&frame[V2_CRC_START..]) != crc {
        return Err(err("v2 batch crc mismatch"));
    }
    let attributes = r.i16()?;
    let last_offset_delta = r.i32()?;
    let base_timestamp = r.i64()?;
    let max_timestamp = r.i64()?;
    let producer_id = r.i64()?;
    let producer_epoch = r.i16()?;
    let base_sequence = r.i32()?;
    let count = r.i32()?;
    if count < 0 {
        return Err(err("negative record count"));
    }
    let payload = if attributes & 0x07 != 0 {
        Payload::Compressed(r.take(r.remaining())?.to_vec())
    } else {
        let mut records = Vec::new();
        for _ in 0..count {
            records.push(decode_record(&mut r)?);
        }
        if r.remaining() != 0 {
            return Err(err("records still remaining after declared count"));
        }
        Payload::Records(records)
    };
    Ok(Batch {
        base_offset,
        partition_leader_epoch,
        magic,
        crc,
        attributes,
        last_offset_delta,
        base_timestamp,
        max_timestamp,
        producer_id,
        producer_epoch,
        base_sequence,
        payload,
    })
}

/// varint length; -1 = null; < -1 = Strict S1 (Java: null).
fn opt_bytes(r: &mut Reader<'_>) -> Result<Option<Vec<u8>>> {
    match r.varint()? {
        -1 => Ok(None),
        n if n < 0 => Err(SpecError::Strict(Strict::S1NegativeLength)),
        n => Ok(Some(r.take(n as usize)?.to_vec())),
    }
}

fn decode_record(r: &mut Reader<'_>) -> Result<Record> {
    let size = r.varint()?;
    if size < 0 {
        return Err(err("negative record size"));
    }
    let body = r
        .take(size as usize)
        .map_err(|_| err("record size exceeds remaining bytes"))?;
    let mut b = Reader::new(body);
    let attributes = b.i8()?;
    let timestamp_delta = b.varlong()?;
    let offset_delta = b.varint()?;
    let key = opt_bytes(&mut b)?;
    let value = opt_bytes(&mut b)?;
    let header_count = b.varint()?;
    if header_count < 0 {
        return Err(err("negative header count"));
    }
    let mut headers = Vec::new();
    for _ in 0..header_count {
        let klen = b.varint()?;
        if klen < 0 {
            return Err(err("null header key"));
        }
        let key = b.take(klen as usize)?;
        // Java decodes header keys as lossy UTF-8.
        if core::str::from_utf8(key).is_err() {
            return Err(SpecError::Strict(Strict::S2InvalidUtf8));
        }
        headers.push(RecordHeader {
            key: key.to_vec(),
            value: opt_bytes(&mut b)?,
        });
    }
    if b.remaining() != 0 {
        return Err(err("record body size mismatch"));
    }
    Ok(Record {
        attributes,
        timestamp_delta,
        offset_delta,
        key,
        value,
        headers,
    })
}

/// int32 length; -1 = null; < -1 = Strict S1 (Java: null).
fn legacy_bytes(r: &mut Reader<'_>) -> Result<Option<Vec<u8>>> {
    match r.i32()? {
        -1 => Ok(None),
        n if n < 0 => Err(SpecError::Strict(Strict::S1NegativeLength)),
        n => Ok(Some(r.take(n as usize)?.to_vec())),
    }
}

fn decode_legacy(frame: &[u8]) -> Result<LegacyMessage> {
    let mut r = Reader::new(frame);
    let offset = r.i64()?;
    let _size = r.i32()?;
    let crc = r.u32()?;
    if crc32fast::hash(&frame[LEGACY_CRC_START..]) != crc {
        return Err(err("legacy message crc mismatch"));
    }
    let magic = r.i8()?;
    let attributes = r.i8()?;
    let timestamp = if magic == 1 { Some(r.i64()?) } else { None };
    let key = legacy_bytes(&mut r)?;
    let value = legacy_bytes(&mut r)?;
    // Java LegacyRecord does not reject trailing bytes inside the frame; neither do we.
    Ok(LegacyMessage {
        offset,
        crc,
        magic,
        attributes,
        timestamp,
        key,
        value,
    })
}

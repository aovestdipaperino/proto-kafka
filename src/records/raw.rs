//! Raw byte-level Kafka V2 RecordBatch manipulation.
//!
//! This module provides zero-copy utilities for working directly with
//! on-wire Kafka V2 record batch bytes — extracting individual records,
//! rewriting offset deltas, building batches from raw components, and
//! repacking batches with records removed.
//!
//! These complement the higher-level [`RecordBatchEncoder`] /
//! [`RecordBatchDecoder`] API (which works with [`Record`] structs) by
//! operating on raw `Bytes` without full deserialization.
//!
//! [`RecordBatchEncoder`]: super::RecordBatchEncoder
//! [`RecordBatchDecoder`]: super::RecordBatchDecoder
//! [`Record`]: super::Record

use bytes::{BufMut, Bytes, BytesMut};
use crate::error::{ProtoError, Result};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Size of the Kafka V2 RecordBatch header in bytes (bytes 0..60 inclusive).
pub const RECORD_BATCH_HEADER_SIZE: usize = 61;

/// Byte offset of the CRC32C field within the batch header.
const CRC_OFFSET: usize = 17;

/// CRC32C is computed over bytes 21..end of the batch.
const CRC_COMPUTE_START: usize = 21;

/// Maximum number of records to iterate over in a single batch scan,
/// guarding against malformed data causing unbounded loops.
const MAX_RECORDS: usize = 100_000;

// Batch header field offsets (all big-endian).
/// Byte offset of the base offset field (i64, bytes 0..7).
const BASE_OFFSET_OFFSET: usize = 0;
/// Byte offset of the batch length field (i32, bytes 8..11).
const BATCH_LENGTH_OFFSET: usize = 8;
/// Byte offset of the partition leader epoch field (i32, bytes 12..15).
const PARTITION_LEADER_EPOCH_OFFSET: usize = 12;
/// Byte offset of the magic byte (i8, byte 16).
const MAGIC_OFFSET: usize = 16;
/// Byte offset of the last offset delta field (i32, bytes 23..26).
const LAST_OFFSET_DELTA_OFFSET: usize = 23;
/// Byte offset of the attributes field (i16, bytes 21..22).
const ATTRIBUTES_OFFSET: usize = 21;
/// Byte offset of the base timestamp field (i64, bytes 27..34).
const BASE_TIMESTAMP_OFFSET: usize = 27;
/// Byte offset of the max timestamp field (i64, bytes 35..42).
const MAX_TIMESTAMP_OFFSET: usize = 35;
/// Byte offset of the producer ID field (i64, bytes 43..50).
const PRODUCER_ID_OFFSET: usize = 43;
/// Byte offset of the producer epoch field (i16, bytes 51..52).
const PRODUCER_EPOCH_OFFSET: usize = 51;
/// Byte offset of the base sequence field (i32, bytes 53..56).
const BASE_SEQUENCE_OFFSET: usize = 53;
/// Byte offset of the record count field (i32, bytes 57..60).
const RECORD_COUNT_OFFSET: usize = 57;

// ---------------------------------------------------------------------------
// Varint helpers
// ---------------------------------------------------------------------------

/// Decode an unsigned protobuf-style varint from `data`.
///
/// Returns `Some((value, bytes_consumed))`, or `None` if the data is
/// truncated or the varint exceeds 10 continuation bytes.
#[must_use]
pub fn decode_varint(data: &[u8]) -> Option<(i64, usize)> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    for (i, &byte) in data.iter().enumerate() {
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some((result as i64, i + 1));
        }
        shift += 7;
        if shift >= 70 {
            return None;
        }
    }
    None
}

/// Encode an unsigned varint into a freshly-allocated `Vec<u8>`.
#[must_use]
pub fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(10);
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
    buf
}

/// ZigZag-decode a signed value from its unsigned wire representation.
#[must_use]
pub fn zigzag_decode(n: i64) -> i64 {
    let n = n as u64;
    ((n >> 1) as i64) ^ (-((n & 1) as i64))
}

/// ZigZag-encode a signed value to its unsigned wire representation.
#[must_use]
pub fn zigzag_encode(n: i64) -> i64 {
    (n << 1) ^ (n >> 63)
}

// ---------------------------------------------------------------------------
// Header field readers
// ---------------------------------------------------------------------------

/// Read a big-endian `i64` from `data` at the given byte offset.
///
/// # Panics
///
/// Panics if `data.len() < offset + 8` (caller contract violation).
fn read_i64(data: &[u8], offset: usize) -> i64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&data[offset..offset + 8]);
    i64::from_be_bytes(buf)
}

/// Read a big-endian `i32` from `data` at the given byte offset.
///
/// # Panics
///
/// Panics if `data.len() < offset + 4` (caller contract violation).
fn read_i32(data: &[u8], offset: usize) -> i32 {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&data[offset..offset + 4]);
    i32::from_be_bytes(buf)
}

/// Read a big-endian `i16` from `data` at the given byte offset.
///
/// # Panics
///
/// Panics if `data.len() < offset + 2` (caller contract violation).
fn read_i16(data: &[u8], offset: usize) -> i16 {
    let mut buf = [0u8; 2];
    buf.copy_from_slice(&data[offset..offset + 2]);
    i16::from_be_bytes(buf)
}

// ---------------------------------------------------------------------------
// Batch header accessors
// ---------------------------------------------------------------------------

/// Read the base offset (bytes 0..7) from a raw batch.
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_base_offset(batch: &[u8]) -> i64 {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    read_i64(batch, BASE_OFFSET_OFFSET)
}

/// Read the attributes field (bytes 21..22) from a raw batch.
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_attributes(batch: &[u8]) -> i16 {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    read_i16(batch, ATTRIBUTES_OFFSET)
}

/// Read the base timestamp (bytes 27..34) from a raw batch.
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_base_timestamp(batch: &[u8]) -> i64 {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    read_i64(batch, BASE_TIMESTAMP_OFFSET)
}

/// Read the max timestamp (bytes 35..42) from a raw batch.
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_max_timestamp(batch: &[u8]) -> i64 {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    read_i64(batch, MAX_TIMESTAMP_OFFSET)
}

/// Read the producer ID (bytes 43..50) from a raw batch.
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_producer_id(batch: &[u8]) -> i64 {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    read_i64(batch, PRODUCER_ID_OFFSET)
}

/// Read the producer epoch (bytes 51..52) from a raw batch.
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_producer_epoch(batch: &[u8]) -> i16 {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    read_i16(batch, PRODUCER_EPOCH_OFFSET)
}

/// Read the base sequence (bytes 53..56) from a raw batch.
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_base_sequence(batch: &[u8]) -> i32 {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    read_i32(batch, BASE_SEQUENCE_OFFSET)
}

/// Read the record count (bytes 57..60) from a raw batch.
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_record_count(batch: &[u8]) -> i32 {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    read_i32(batch, RECORD_COUNT_OFFSET)
}

/// Read the batch length field (bytes 8..11).
///
/// This is the total size of the batch minus the leading 12 bytes
/// (base_offset + batch_length itself).
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_length(batch: &[u8]) -> i32 {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    read_i32(batch, BATCH_LENGTH_OFFSET)
}

/// Read the partition leader epoch (bytes 12..15) from a raw batch.
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_partition_leader_epoch(batch: &[u8]) -> i32 {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    read_i32(batch, PARTITION_LEADER_EPOCH_OFFSET)
}

/// Read the magic byte (byte 16) from a raw batch.
///
/// For Kafka V2 record batches this must be `2`.
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_magic(batch: &[u8]) -> i8 {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    batch[MAGIC_OFFSET] as i8
}

/// Read the last offset delta (bytes 23..26) from a raw batch.
///
/// This should equal `record_count - 1` for a well-formed batch.
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_last_offset_delta(batch: &[u8]) -> i32 {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    read_i32(batch, LAST_OFFSET_DELTA_OFFSET)
}

/// Verify CRC32C integrity of a raw batch.
///
/// Returns `true` if the CRC stored at bytes 17..20 matches the
/// CRC computed over bytes 21..end.
///
/// # Panics
///
/// Panics if `batch.len() < RECORD_BATCH_HEADER_SIZE`.
#[must_use]
pub fn batch_crc_valid(batch: &[u8]) -> bool {
    debug_assert!(batch.len() >= RECORD_BATCH_HEADER_SIZE);
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&batch[CRC_OFFSET..CRC_OFFSET + 4]);
    let stored = u32::from_be_bytes(buf);
    let computed = crc32c::crc32c(&batch[CRC_COMPUTE_START..]);
    stored == computed
}

// ---------------------------------------------------------------------------
// RawRecord
// ---------------------------------------------------------------------------

/// A single record extracted from a Kafka V2 RecordBatch via zero-copy
/// slicing, carrying enough information to be re-inserted into a
/// reassembled batch or processed independently.
#[derive(Debug, Clone)]
pub struct RawRecord {
    /// Record key (`None` when the wire key-length is -1).
    pub key: Option<Bytes>,
    /// Record value (`None` when the wire value-length is -1, i.e. tombstone).
    pub value: Option<Bytes>,
    /// The full on-wire bytes for this record, starting from the
    /// record-length varint through the end of the last header.
    pub record_bytes: Bytes,
    /// Absolute timestamp (base_timestamp + timestamp_delta).
    pub timestamp: i64,
}

impl RawRecord {
    /// A tombstone is a record whose value is `None`.
    #[must_use]
    pub fn is_tombstone(&self) -> bool {
        self.value.is_none()
    }
}

// ---------------------------------------------------------------------------
// Record extraction
// ---------------------------------------------------------------------------

/// Skip over record headers, advancing `offset` past each key-value pair.
///
/// Returns `Some(new_offset)` on success, or `None` if truncated.
fn skip_record_headers(data: &[u8], mut offset: usize, header_count: usize) -> Option<usize> {
    for _ in 0..header_count {
        // header key length (signed zigzag)
        let (raw_key_len, key_len_bytes) = decode_varint(data.get(offset..)?)?;
        let key_len = zigzag_decode(raw_key_len);
        offset += key_len_bytes;
        if key_len > 0 {
            offset += key_len as usize;
        }
        // header value length (signed zigzag)
        let (raw_val_len, val_len_bytes) = decode_varint(data.get(offset..)?)?;
        let val_len = zigzag_decode(raw_val_len);
        offset += val_len_bytes;
        if val_len > 0 {
            offset += val_len as usize;
        }
    }
    Some(offset)
}

/// Decode a nullable field (key or value) from the record body.
///
/// Returns `(field_value, new_offset)`, or `None` if truncated.
fn decode_nullable_field(
    batch_bytes: &Bytes,
    data: &[u8],
    offset: usize,
) -> Option<(Option<Bytes>, usize)> {
    let (raw_len, len_bytes) = decode_varint(data.get(offset..)?)?;
    let field_length = zigzag_decode(raw_len);
    let mut new_offset = offset + len_bytes;

    let field = if field_length < 0 {
        None
    } else {
        let fl = field_length as usize;
        let abs_start = RECORD_BATCH_HEADER_SIZE + new_offset;
        let f = batch_bytes.slice(abs_start..abs_start + fl);
        new_offset += fl;
        Some(f)
    };
    Some((field, new_offset))
}

/// Parse a single record starting at `offset` within the records section.
///
/// Returns `Some((record, next_offset))` on success, or `None` if the data
/// is truncated or the record is malformed.
fn parse_one_record(
    batch_bytes: &Bytes,
    data: &[u8],
    offset: usize,
    base_timestamp: i64,
) -> Option<(RawRecord, usize)> {
    let record_start = offset;

    // record length (signed zigzag varint)
    let (raw_len, len_bytes) = decode_varint(data.get(offset..)?)?;
    let record_length = zigzag_decode(raw_len) as usize;
    let mut pos = offset + len_bytes;
    let record_end = pos + record_length;

    // attributes (i8) — skip
    pos += 1;

    // timestamp delta (signed zigzag)
    let (raw_ts, ts_bytes) = decode_varint(data.get(pos..)?)?;
    let timestamp_delta = zigzag_decode(raw_ts);
    pos += ts_bytes;

    // offset delta (signed zigzag) — skip value
    let (_raw_od, od_bytes) = decode_varint(data.get(pos..)?)?;
    pos += od_bytes;

    // key
    let (key, new_pos) = decode_nullable_field(batch_bytes, data, pos)?;
    pos = new_pos;

    // value
    let (value, new_pos) = decode_nullable_field(batch_bytes, data, pos)?;
    pos = new_pos;

    // headers
    let (raw_hdr_count, hc_bytes) = decode_varint(data.get(pos..)?)?;
    let header_count = zigzag_decode(raw_hdr_count) as usize;
    pos += hc_bytes;
    pos = skip_record_headers(data, pos, header_count)?;

    // Validate: parsed length must match declared record_length.
    if pos != record_end {
        return None;
    }

    let record_bytes = batch_bytes.slice(
        RECORD_BATCH_HEADER_SIZE + record_start..RECORD_BATCH_HEADER_SIZE + record_end,
    );

    Some((
        RawRecord {
            key,
            value,
            record_bytes,
            timestamp: base_timestamp + timestamp_delta,
        },
        record_end,
    ))
}

/// Parse the records section of a Kafka V2 RecordBatch.
///
/// `batch_bytes` must contain the **entire** batch (header + records).
/// `base_timestamp` is the batch-level base timestamp used to resolve deltas.
///
/// Returns one [`RawRecord`] per record found. Malformed trailing records
/// are silently skipped.
#[must_use]
pub fn extract_records(batch_bytes: &Bytes, base_timestamp: i64) -> Vec<RawRecord> {
    debug_assert!(!batch_bytes.is_empty(), "batch_bytes must not be empty");

    if batch_bytes.len() <= RECORD_BATCH_HEADER_SIZE {
        return Vec::new();
    }

    let data = &batch_bytes[RECORD_BATCH_HEADER_SIZE..];
    let mut offset = 0;
    let mut records = Vec::new();

    while offset < data.len() && records.len() < MAX_RECORDS {
        match parse_one_record(batch_bytes, data, offset, base_timestamp) {
            Some((record, next_offset)) => {
                records.push(record);
                offset = next_offset;
            }
            None => break,
        }
    }

    debug_assert!(records.len() <= MAX_RECORDS);
    records
}

// ---------------------------------------------------------------------------
// Record offset_delta rewriting
// ---------------------------------------------------------------------------

/// Rewrite a Kafka V2 record's `offset_delta` varint to `new_offset_delta`.
///
/// Record wire format (all varints are zigzag-encoded):
/// ```text
///   varint  record_length
///   i8      attributes
///   varint  timestamp_delta
///   varint  offset_delta       <-- rewritten
///   ...rest (key, value, headers)
/// ```
///
/// After rewriting `offset_delta` the `record_length` varint is recalculated
/// because the replacement varint may differ in encoded size.
/// # Errors
///
/// Returns [`ProtoError::MalformedRecord`] if any varint in the record is
/// truncated or otherwise invalid.
pub fn rewrite_offset_delta(record_bytes: &Bytes, new_offset_delta: i64) -> Result<Bytes> {
    let data = &record_bytes[..];

    // 1. Decode record_length varint
    let (_raw_len, len_varint_bytes) = decode_varint(data)
        .ok_or(ProtoError::MalformedRecord { detail: "truncated record_length varint" })?;
    let body_start = len_varint_bytes;

    // 2. Skip attributes (1 byte)
    let mut pos = body_start + 1;

    // 3. Skip timestamp_delta varint
    let (_ts_raw, ts_bytes) = decode_varint(data.get(pos..).unwrap_or_default())
        .ok_or(ProtoError::MalformedRecord { detail: "truncated timestamp_delta varint" })?;
    pos += ts_bytes;

    // 4. Decode existing offset_delta varint (so we know how many bytes it occupies)
    let (_od_raw, od_bytes) = decode_varint(data.get(pos..).unwrap_or_default())
        .ok_or(ProtoError::MalformedRecord { detail: "truncated offset_delta varint" })?;
    let od_start = pos;
    let od_end = pos + od_bytes;

    // 5. Encode the new offset_delta
    let new_od_encoded = encode_varint(zigzag_encode(new_offset_delta) as u64);

    // 6. Build new body: [attributes + ts_delta] + [new_od] + [rest]
    let prefix = &data[body_start..od_start]; // attributes + timestamp_delta
    let suffix = &data[od_end..]; // key, value, headers

    let new_body_len = prefix.len() + new_od_encoded.len() + suffix.len();

    // 7. Re-encode record_length
    let new_len_encoded = encode_varint(zigzag_encode(new_body_len as i64) as u64);

    // 8. Assemble
    let mut buf = BytesMut::with_capacity(new_len_encoded.len() + new_body_len);
    buf.extend_from_slice(&new_len_encoded);
    buf.extend_from_slice(prefix);
    buf.extend_from_slice(&new_od_encoded);
    buf.extend_from_slice(suffix);
    Ok(buf.freeze())
}

// ---------------------------------------------------------------------------
// Batch builder
// ---------------------------------------------------------------------------

/// Build a complete Kafka V2 RecordBatch from header fields and raw record
/// bytes.
///
/// Writes all header fields in big-endian order, appends `record_bytes`, and
/// computes the CRC32C.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn build_batch(
    base_offset: i64,
    base_timestamp: i64,
    max_timestamp: i64,
    producer_id: i64,
    producer_epoch: i16,
    base_sequence: i32,
    attributes: i16,
    record_count: i32,
    record_bytes: &[u8],
) -> Bytes {
    let total_size = RECORD_BATCH_HEADER_SIZE + record_bytes.len();
    let batch_length = (total_size - 12) as i32; // batchLength excludes first 12 bytes

    let mut buf = BytesMut::with_capacity(total_size);

    // bytes 0-7: baseOffset
    buf.put_i64(base_offset);
    // bytes 8-11: batchLength
    buf.put_i32(batch_length);
    // bytes 12-15: partitionLeaderEpoch
    buf.put_i32(0);
    // byte 16: magic
    buf.put_i8(2);
    // bytes 17-20: CRC32C placeholder (filled below)
    buf.put_i32(0);
    // bytes 21-22: attributes
    buf.put_i16(attributes);
    // bytes 23-26: lastOffsetDelta
    buf.put_i32(record_count - 1);
    // bytes 27-34: baseTimestamp
    buf.put_i64(base_timestamp);
    // bytes 35-42: maxTimestamp
    buf.put_i64(max_timestamp);
    // bytes 43-50: producerId
    buf.put_i64(producer_id);
    // bytes 51-52: producerEpoch
    buf.put_i16(producer_epoch);
    // bytes 53-56: baseSequence
    buf.put_i32(base_sequence);
    // bytes 57-60: recordCount
    buf.put_i32(record_count);

    // Append record data
    buf.extend_from_slice(record_bytes);

    // Compute CRC32C over bytes 21..end and patch it in at offset 17
    let crc = crc32c::crc32c(&buf[CRC_COMPUTE_START..]);
    let crc_bytes = (crc as i32).to_be_bytes();
    buf[CRC_OFFSET] = crc_bytes[0];
    buf[CRC_OFFSET + 1] = crc_bytes[1];
    buf[CRC_OFFSET + 2] = crc_bytes[2];
    buf[CRC_OFFSET + 3] = crc_bytes[3];

    buf.freeze()
}

// ---------------------------------------------------------------------------
// Batch repacking
// ---------------------------------------------------------------------------

/// Remove records from a batch based on a predicate and return the repacked
/// batch.
///
/// The `keep` predicate receives `(record_index, &RawRecord)` and should
/// return `true` for records that should be retained.
///
/// Returns `None` when no records are kept (the batch should be discarded).
///
/// The returned batch preserves the original producer metadata and timestamps
/// so that idempotent / transactional guarantees are not broken for the
/// remaining records.
/// # Errors
///
/// Returns [`ProtoError::MalformedRecord`] if a kept record's on-wire bytes
/// contain a truncated varint.
pub fn repack_batch(
    batch_bytes: &Bytes,
    base_offset: i64,
    keep: impl Fn(usize, &RawRecord) -> bool,
) -> Result<Option<Bytes>> {
    if batch_bytes.len() < RECORD_BATCH_HEADER_SIZE {
        return Ok(None);
    }

    // Read original header fields
    let base_timestamp = read_i64(batch_bytes, BASE_TIMESTAMP_OFFSET);
    let max_timestamp = read_i64(batch_bytes, MAX_TIMESTAMP_OFFSET);
    let producer_id = read_i64(batch_bytes, PRODUCER_ID_OFFSET);
    let producer_epoch = read_i16(batch_bytes, PRODUCER_EPOCH_OFFSET);
    let base_sequence = read_i32(batch_bytes, BASE_SEQUENCE_OFFSET);
    let attributes = read_i16(batch_bytes, ATTRIBUTES_OFFSET);

    // Extract all records
    let records = extract_records(batch_bytes, base_timestamp);

    // Keep only matching records, rewriting offset deltas to be contiguous
    let mut record_buf = BytesMut::new();
    let mut kept_count: i32 = 0;

    for (i, record) in records.iter().enumerate() {
        if !keep(i, record) {
            continue;
        }
        let rewritten = rewrite_offset_delta(&record.record_bytes, kept_count as i64)?;
        record_buf.extend_from_slice(&rewritten);
        kept_count += 1;
    }

    if kept_count == 0 {
        return Ok(None);
    }

    Ok(Some(build_batch(
        base_offset,
        base_timestamp,
        max_timestamp,
        producer_id,
        producer_epoch,
        base_sequence,
        attributes,
        kept_count,
        &record_buf,
    )))
}

/// Assemble a new RecordBatch from individual records.
///
/// Each entry in `records` is `(record_bytes, timestamp)` where
/// `record_bytes` is the full on-wire record (length varint + body) and
/// `timestamp` is the absolute timestamp.
///
/// The produced batch has no idempotent producer metadata (`producerId = -1`,
/// `producerEpoch = -1`, `baseSequence = -1`).
/// # Errors
///
/// Returns [`ProtoError::MalformedRecord`] if any record's on-wire bytes
/// contain a truncated varint.
pub fn reassemble_batch(records: &[(Bytes, i64)], base_offset: i64) -> Result<Bytes> {
    let record_count = records.len() as i32;

    // Determine base and max timestamps
    let base_timestamp = records.iter().map(|(_, ts)| *ts).min().unwrap_or(0);
    let max_timestamp = records.iter().map(|(_, ts)| *ts).max().unwrap_or(0);

    // Rewrite offset deltas to be contiguous and concatenate
    let mut record_buf = BytesMut::new();
    for (i, (rec_bytes, _ts)) in records.iter().enumerate() {
        let rewritten = rewrite_offset_delta(rec_bytes, i as i64)?;
        record_buf.extend_from_slice(&rewritten);
    }

    Ok(build_batch(
        base_offset,
        base_timestamp,
        max_timestamp,
        -1, // producerId — no idempotency
        -1, // producerEpoch
        -1, // baseSequence
        0,  // attributes — no compression, no txn
        record_count,
        &record_buf,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Varint roundtrip tests ---------------------------------------------

    #[test]
    fn varint_roundtrip() {
        for &val in &[0u64, 1, 127, 128, 300, 16384, u64::MAX >> 1] {
            let encoded = encode_varint(val);
            let (decoded, consumed) = decode_varint(&encoded).expect("valid varint");
            assert_eq!(decoded as u64, val, "roundtrip failed for {val}");
            assert_eq!(consumed, encoded.len());
        }
    }

    #[test]
    fn varint_single_byte() {
        let encoded = encode_varint(0);
        assert_eq!(encoded, vec![0x00]);

        let encoded = encode_varint(1);
        assert_eq!(encoded, vec![0x01]);

        let encoded = encode_varint(127);
        assert_eq!(encoded, vec![0x7F]);
    }

    #[test]
    fn varint_multi_byte() {
        let encoded = encode_varint(128);
        assert_eq!(encoded, vec![0x80, 0x01]);

        let encoded = encode_varint(300);
        assert_eq!(encoded, vec![0xAC, 0x02]);
    }

    // -- Zigzag roundtrip tests ---------------------------------------------

    #[test]
    fn zigzag_roundtrip() {
        for &val in &[0i64, -1, 1, -2, 2, i64::MIN, i64::MAX] {
            let encoded = zigzag_encode(val);
            let decoded = zigzag_decode(encoded);
            assert_eq!(decoded, val, "zigzag roundtrip failed for {val}");
        }
    }

    #[test]
    fn zigzag_known_values() {
        // Standard zigzag mapping: 0->0, -1->1, 1->2, -2->3, 2->4
        assert_eq!(zigzag_encode(0), 0);
        assert_eq!(zigzag_encode(-1), 1);
        assert_eq!(zigzag_encode(1), 2);
        assert_eq!(zigzag_encode(-2), 3);
        assert_eq!(zigzag_encode(2), 4);

        assert_eq!(zigzag_decode(0), 0);
        assert_eq!(zigzag_decode(1), -1);
        assert_eq!(zigzag_decode(2), 1);
        assert_eq!(zigzag_decode(3), -2);
        assert_eq!(zigzag_decode(4), 2);
    }

    // -- Helper: build a minimal test record --------------------------------

    /// Build a single Kafka V2 record (length varint + body) with the given
    /// key, value, timestamp_delta, and offset_delta.
    fn make_record(
        key: Option<&[u8]>,
        value: Option<&[u8]>,
        timestamp_delta: i64,
        offset_delta: i64,
    ) -> Bytes {
        let mut body = BytesMut::new();

        // attributes
        body.put_i8(0);

        // timestamp delta (zigzag varint)
        body.extend_from_slice(&encode_varint(zigzag_encode(timestamp_delta) as u64));

        // offset delta (zigzag varint)
        body.extend_from_slice(&encode_varint(zigzag_encode(offset_delta) as u64));

        // key
        match key {
            Some(k) => {
                body.extend_from_slice(&encode_varint(zigzag_encode(k.len() as i64) as u64));
                body.extend_from_slice(k);
            }
            None => {
                body.extend_from_slice(&encode_varint(zigzag_encode(-1) as u64));
            }
        }

        // value
        match value {
            Some(v) => {
                body.extend_from_slice(&encode_varint(zigzag_encode(v.len() as i64) as u64));
                body.extend_from_slice(v);
            }
            None => {
                body.extend_from_slice(&encode_varint(zigzag_encode(-1) as u64));
            }
        }

        // header count = 0
        body.extend_from_slice(&encode_varint(zigzag_encode(0) as u64));

        // Prepend record_length
        let body_len = body.len();
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&encode_varint(zigzag_encode(body_len as i64) as u64));
        buf.extend_from_slice(&body);
        buf.freeze()
    }

    /// Build a minimal Kafka V2 RecordBatch from the given records.
    /// Each record entry is (key, value, timestamp_delta).
    #[allow(clippy::type_complexity)]
    fn build_test_batch(
        _base_timestamp: i64,
        records: &[(Option<&[u8]>, Option<&[u8]>, i64)],
    ) -> Bytes {
        let mut buf = BytesMut::new();

        // 61-byte header (filled with zeros; base_timestamp passed separately).
        buf.put_bytes(0u8, RECORD_BATCH_HEADER_SIZE);

        for (offset_delta, (key, value, ts_delta)) in records.iter().enumerate() {
            let mut body = BytesMut::new();

            // attributes
            body.put_i8(0);

            // timestamp delta (zigzag + varint)
            body.extend_from_slice(&encode_varint(zigzag_encode(*ts_delta) as u64));

            // offset delta (zigzag + varint)
            body.extend_from_slice(&encode_varint(zigzag_encode(offset_delta as i64) as u64));

            // key
            match key {
                Some(k) => {
                    body.extend_from_slice(&encode_varint(zigzag_encode(k.len() as i64) as u64));
                    body.extend_from_slice(k);
                }
                None => {
                    body.extend_from_slice(&encode_varint(zigzag_encode(-1) as u64));
                }
            }

            // value
            match value {
                Some(v) => {
                    body.extend_from_slice(&encode_varint(zigzag_encode(v.len() as i64) as u64));
                    body.extend_from_slice(v);
                }
                None => {
                    body.extend_from_slice(&encode_varint(zigzag_encode(-1) as u64));
                }
            }

            // header count = 0
            body.extend_from_slice(&encode_varint(zigzag_encode(0) as u64));

            // Prepend record_length (zigzag-encoded body length)
            let body_len = body.len();
            buf.extend_from_slice(&encode_varint(zigzag_encode(body_len as i64) as u64));
            buf.extend_from_slice(&body);
        }

        buf.freeze()
    }

    // -- RawRecord ----------------------------------------------------------

    #[test]
    fn tombstone_detection() {
        let tombstone = RawRecord {
            key: Some(Bytes::from_static(b"key")),
            value: None,
            record_bytes: Bytes::new(),
            timestamp: 0,
        };
        assert!(tombstone.is_tombstone());

        let normal = RawRecord {
            key: Some(Bytes::from_static(b"key")),
            value: Some(Bytes::from_static(b"value")),
            record_bytes: Bytes::new(),
            timestamp: 0,
        };
        assert!(!normal.is_tombstone());
    }

    // -- extract_records ----------------------------------------------------

    #[test]
    fn extract_single_record() {
        let batch = build_test_batch(1000, &[(Some(b"k1"), Some(b"v1"), 10)]);
        let records = extract_records(&batch, 1000);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].key.as_deref(), Some(b"k1".as_slice()));
        assert_eq!(records[0].value.as_deref(), Some(b"v1".as_slice()));
        assert_eq!(records[0].timestamp, 1010);
        assert!(!records[0].is_tombstone());
    }

    #[test]
    fn extract_multiple_records() {
        let batch = build_test_batch(
            500,
            &[
                (Some(b"a"), Some(b"1"), 0),
                (Some(b"b"), Some(b"2"), 5),
                (None, Some(b"3"), 10),
            ],
        );
        let records = extract_records(&batch, 500);
        assert_eq!(records.len(), 3);

        assert_eq!(records[0].timestamp, 500);
        assert_eq!(records[1].timestamp, 505);
        assert_eq!(records[2].timestamp, 510);

        assert!(records[2].key.is_none());
    }

    #[test]
    fn extract_tombstone_record() {
        let batch = build_test_batch(0, &[(Some(b"key"), None, 0)]);
        let records = extract_records(&batch, 0);
        assert_eq!(records.len(), 1);
        assert!(records[0].is_tombstone());
        assert_eq!(records[0].key.as_deref(), Some(b"key".as_slice()));
    }

    #[test]
    fn record_bytes_span_is_correct() {
        let batch = build_test_batch(0, &[(Some(b"k"), Some(b"v"), 0)]);
        let records = extract_records(&batch, 0);
        assert_eq!(records.len(), 1);

        // Re-parse the record_bytes to make sure they're self-consistent.
        let rb = &records[0].record_bytes;
        let (raw_len, len_bytes) = decode_varint(rb).expect("valid varint");
        let body_len = zigzag_decode(raw_len) as usize;
        assert_eq!(rb.len(), len_bytes + body_len);
    }

    #[test]
    fn extract_empty_batch() {
        // A batch with only a header and no records
        let mut buf = BytesMut::new();
        buf.put_bytes(0u8, RECORD_BATCH_HEADER_SIZE);
        let batch = buf.freeze();
        let records = extract_records(&batch, 0);
        assert!(records.is_empty());
    }

    #[test]
    fn extract_undersized_batch() {
        let batch = Bytes::from_static(&[0u8; 10]);
        let records = extract_records(&batch, 0);
        assert!(records.is_empty());
    }

    // -- rewrite_offset_delta -----------------------------------------------

    #[test]
    fn rewrite_offset_delta_preserves_content() {
        let original = make_record(Some(b"key"), Some(b"val"), 10, 5);
        let rewritten = rewrite_offset_delta(&original, 0).unwrap();

        // Parse the rewritten record and verify offset_delta is now 0
        let (raw_len, len_bytes) = decode_varint(&rewritten).expect("valid varint");
        let body_len = zigzag_decode(raw_len) as usize;
        assert_eq!(
            rewritten.len(),
            len_bytes + body_len,
            "record_length varint should match actual body"
        );

        let body = &rewritten[len_bytes..];
        // skip attributes (1 byte)
        let (raw_ts, ts_bytes) = decode_varint(&body[1..]).expect("valid varint");
        let ts_delta = zigzag_decode(raw_ts);
        assert_eq!(ts_delta, 10, "timestamp_delta should be preserved");

        let (raw_od, _od_bytes) = decode_varint(&body[1 + ts_bytes..]).expect("valid varint");
        let od = zigzag_decode(raw_od);
        assert_eq!(od, 0, "offset_delta should be rewritten to 0");
    }

    // -- build_batch CRC verification ---------------------------------------

    #[test]
    fn build_batch_crc_matches() {
        let r0 = make_record(Some(b"hello"), Some(b"world"), 0, 0);

        let batch = build_batch(
            100,  // base_offset
            5000, // base_timestamp
            5000, // max_timestamp
            -1,   // producer_id
            -1,   // producer_epoch
            -1,   // base_sequence
            0,    // attributes
            1,    // record_count
            &r0,
        );

        let stored_crc =
            u32::from_be_bytes(batch[CRC_OFFSET..CRC_OFFSET + 4].try_into().unwrap());
        let computed_crc = crc32c::crc32c(&batch[CRC_COMPUTE_START..]);
        assert_eq!(stored_crc, computed_crc, "CRC32C mismatch");
    }

    // -- reassemble_batch ---------------------------------------------------

    #[test]
    fn reassemble_batch_produces_valid_header() {
        let r0 = make_record(Some(b"k0"), Some(b"v0"), 0, 0);
        let r1 = make_record(Some(b"k1"), Some(b"v1"), 5, 1);
        let r2 = make_record(Some(b"k2"), Some(b"v2"), 10, 2);

        let records = vec![(r0, 1000i64), (r1, 1005i64), (r2, 1010i64)];

        let batch = reassemble_batch(&records, 42).unwrap();

        assert_eq!(batch_base_offset(&batch), 42);
        assert_eq!(batch[16], 2, "magic byte should be 2");
        assert_eq!(batch_record_count(&batch), 3);
        assert_eq!(read_i32(&batch, 23), 2, "lastOffsetDelta");
        assert_eq!(batch_producer_id(&batch), -1);
        assert_eq!(batch_producer_epoch(&batch), -1);
        assert_eq!(batch_base_sequence(&batch), -1);
        assert_eq!(batch_base_timestamp(&batch), 1000);
        assert_eq!(batch_max_timestamp(&batch), 1010);
    }

    // -- repack_batch -------------------------------------------------------

    #[test]
    fn repack_removes_filtered_records() {
        let r0 = make_record(Some(b"k0"), Some(b"v0"), 0, 0);
        let r1 = make_record(Some(b"k1"), Some(b"v1"), 5, 1);
        let r2 = make_record(Some(b"k2"), Some(b"v2"), 10, 2);

        let mut record_buf = BytesMut::new();
        record_buf.extend_from_slice(&r0);
        record_buf.extend_from_slice(&r1);
        record_buf.extend_from_slice(&r2);

        let batch = build_batch(
            0,    // base_offset
            1000, // base_timestamp
            1010, // max_timestamp
            42,   // producer_id
            1,    // producer_epoch
            0,    // base_sequence
            0,    // attributes
            3,    // record_count
            &record_buf,
        );

        // Remove record 1 (middle record)
        let repacked = repack_batch(&batch, 0, |i, _rec| i != 1)
            .unwrap()
            .expect("should return Some when some kept");

        assert_eq!(batch_record_count(&repacked), 2);
        assert_eq!(read_i32(&repacked, 23), 1, "lastOffsetDelta after repack");

        // Verify CRC
        let stored_crc =
            u32::from_be_bytes(repacked[CRC_OFFSET..CRC_OFFSET + 4].try_into().unwrap());
        let computed_crc = crc32c::crc32c(&repacked[CRC_COMPUTE_START..]);
        assert_eq!(stored_crc, computed_crc, "CRC should be valid after repack");

        // Producer metadata should be preserved
        assert_eq!(batch_producer_id(&repacked), 42);
        assert_eq!(batch_producer_epoch(&repacked), 1);
    }

    #[test]
    fn repack_returns_none_when_all_removed() {
        let r0 = make_record(Some(b"k"), Some(b"v"), 0, 0);

        let batch = build_batch(0, 1000, 1000, -1, -1, -1, 0, 1, &r0);

        assert!(
            repack_batch(&batch, 0, |_, _| false)
                .unwrap()
                .is_none(),
            "should return None when all records removed"
        );
    }

    // -- batch header accessors ---------------------------------------------

    #[test]
    fn batch_header_accessors() {
        let r0 = make_record(Some(b"k"), Some(b"v"), 0, 0);
        let batch = build_batch(99, 5000, 5000, 123, 7, 42, 0, 1, &r0);

        assert_eq!(batch_base_offset(&batch), 99);
        assert_eq!(batch_attributes(&batch), 0);
        assert_eq!(batch_base_timestamp(&batch), 5000);
        assert_eq!(batch_max_timestamp(&batch), 5000);
        assert_eq!(batch_producer_id(&batch), 123);
        assert_eq!(batch_producer_epoch(&batch), 7);
        assert_eq!(batch_base_sequence(&batch), 42);
        assert_eq!(batch_record_count(&batch), 1);
    }

    #[test]
    fn batch_header_accessors_extended() {
        let r0 = make_record(Some(b"k"), Some(b"v"), 0, 0);
        let r1 = make_record(Some(b"k2"), Some(b"v2"), 5, 1);
        let batch = build_batch(10, 5000, 5005, 123, 7, 42, 0, 2, &{
            let mut buf = BytesMut::new();
            buf.extend_from_slice(&r0);
            buf.extend_from_slice(&r1);
            buf.freeze()
        });

        assert_eq!(batch_magic(&batch), 2, "V2 magic byte");
        assert_eq!(batch_last_offset_delta(&batch), 1, "record_count-1");
        assert_eq!(batch_partition_leader_epoch(&batch), 0);
        assert!(batch_crc_valid(&batch), "CRC must be valid after build");
        assert!(batch_length(&batch) > 0, "batch_length must be positive");
    }

    // -- Nuance: zero-copy non-mutation (KAFKA-PROTOCOL-NUANCES 6b) ----------

    #[test]
    fn rewrite_offset_delta_does_not_mutate_original() {
        // Section 6b: mutating stored records corrupts metadata for
        // other consumers sharing the same Bytes via refcounting.
        let original = make_record(Some(b"key"), Some(b"val"), 10, 5);
        let original_copy = original.clone();

        let _rewritten = rewrite_offset_delta(&original, 99).unwrap();

        assert_eq!(
            original, original_copy,
            "original Bytes must not be mutated by rewrite"
        );
    }

    #[test]
    fn repack_does_not_mutate_source_batch() {
        // Section 6b: repack creates a new batch; source must be untouched.
        let r0 = make_record(Some(b"k0"), Some(b"v0"), 0, 0);
        let r1 = make_record(Some(b"k1"), Some(b"v1"), 5, 1);
        let mut record_buf = BytesMut::new();
        record_buf.extend_from_slice(&r0);
        record_buf.extend_from_slice(&r1);
        let batch = build_batch(0, 1000, 1005, 42, 1, 0, 0, 2, &record_buf);
        let batch_copy = batch.clone();

        let _repacked = repack_batch(&batch, 0, |i, _| i == 0).unwrap();

        assert_eq!(batch, batch_copy, "source batch must not be mutated");
    }

    // -- Nuance: producer metadata preservation (KAFKA-PROTOCOL-NUANCES 5a) --

    #[test]
    fn repack_preserves_idempotent_producer_metadata() {
        // Section 5a: duplicate batch detection relies on producer_id, epoch,
        // and base_sequence being preserved through repack operations.
        let r0 = make_record(Some(b"k0"), Some(b"v0"), 0, 0);
        let r1 = make_record(Some(b"k1"), Some(b"v1"), 5, 1);
        let mut record_buf = BytesMut::new();
        record_buf.extend_from_slice(&r0);
        record_buf.extend_from_slice(&r1);

        let producer_id = 12345_i64;
        let producer_epoch = 3_i16;
        let base_sequence = 100_i32;

        let batch = build_batch(
            0, 1000, 1005,
            producer_id, producer_epoch, base_sequence,
            0, 2, &record_buf,
        );

        // Remove one record — metadata must survive
        let repacked = repack_batch(&batch, 0, |i, _| i == 0)
            .unwrap()
            .expect("at least one record kept");

        assert_eq!(batch_producer_id(&repacked), producer_id);
        assert_eq!(batch_producer_epoch(&repacked), producer_epoch);
        assert_eq!(batch_base_sequence(&repacked), base_sequence);
        assert!(batch_crc_valid(&repacked), "CRC must be valid after repack");
    }

    // -- Nuance: V2 batch invariants (KAFKA-PROTOCOL-NUANCES 5c, 6a) --------

    #[test]
    fn build_batch_produces_valid_v2_invariants() {
        // Section 5c: clients send V2 batches with baseOffset=0.
        // Section 6a: one batch should hold multiple records with correct
        // base_offset, last_offset_delta, and record_count.
        let r0 = make_record(Some(b"a"), Some(b"1"), 0, 0);
        let r1 = make_record(Some(b"b"), Some(b"2"), 5, 1);
        let r2 = make_record(Some(b"c"), Some(b"3"), 10, 2);
        let mut record_buf = BytesMut::new();
        record_buf.extend_from_slice(&r0);
        record_buf.extend_from_slice(&r1);
        record_buf.extend_from_slice(&r2);

        let batch = build_batch(0, 1000, 1010, -1, -1, -1, 0, 3, &record_buf);

        // V2 invariants
        assert_eq!(batch_magic(&batch), 2, "must be V2");
        assert_eq!(batch_base_offset(&batch), 0, "clients send baseOffset=0");
        assert_eq!(batch_record_count(&batch), 3);
        assert_eq!(
            batch_last_offset_delta(&batch), 2,
            "last_offset_delta must equal record_count - 1"
        );
        assert!(batch_crc_valid(&batch));

        // All 3 records should be extractable
        let records = extract_records(&batch, batch_base_timestamp(&batch));
        assert_eq!(records.len(), 3, "all records must be recoverable");
    }

    // -- Nuance: empty batch handling (KAFKA-PROTOCOL-NUANCES 1a) ------------

    #[test]
    fn extract_records_from_zero_record_batch() {
        // Section 1a: an empty Bytes is a valid zero-record batch.
        // A batch header with record_count=0 and no record payload is valid.
        let batch = build_batch(0, 0, 0, -1, -1, -1, 0, 0, &[]);

        assert_eq!(batch_record_count(&batch), 0);
        assert!(batch_crc_valid(&batch));

        let records = extract_records(&batch, 0);
        assert!(records.is_empty(), "zero-record batch must yield no records");
    }

    // -- Nuance: CRC corruption detection ------------------------------------

    #[test]
    fn batch_crc_detects_bit_flip() {
        let r0 = make_record(Some(b"k"), Some(b"v"), 0, 0);
        let batch = build_batch(0, 1000, 1000, -1, -1, -1, 0, 1, &r0);
        assert!(batch_crc_valid(&batch));

        // Flip a bit in the records section
        let mut corrupted = BytesMut::from(&batch[..]);
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0x01;
        let corrupted = corrupted.freeze();

        assert!(!batch_crc_valid(&corrupted), "CRC must detect corruption");
    }

    // -- Nuance: offset delta accounting after repack (KAFKA-PROTOCOL-NUANCES 6a)

    #[test]
    fn repack_reindexes_offset_deltas_contiguously() {
        // After removing records, offset deltas must be contiguous 0..N-1.
        // Section 6a: correct last_offset_delta is critical for consumers.
        let r0 = make_record(Some(b"k0"), Some(b"v0"), 0, 0);
        let r1 = make_record(Some(b"k1"), Some(b"v1"), 5, 1);
        let r2 = make_record(Some(b"k2"), Some(b"v2"), 10, 2);
        let r3 = make_record(Some(b"k3"), Some(b"v3"), 15, 3);
        let mut record_buf = BytesMut::new();
        record_buf.extend_from_slice(&r0);
        record_buf.extend_from_slice(&r1);
        record_buf.extend_from_slice(&r2);
        record_buf.extend_from_slice(&r3);

        let batch = build_batch(0, 1000, 1015, -1, -1, -1, 0, 4, &record_buf);

        // Keep records 0 and 3, remove 1 and 2
        let repacked = repack_batch(&batch, 100, |i, _| i == 0 || i == 3)
            .unwrap()
            .expect("some records kept");

        assert_eq!(batch_record_count(&repacked), 2);
        assert_eq!(
            batch_last_offset_delta(&repacked), 1,
            "last_offset_delta must be record_count-1 after reindex"
        );
        assert_eq!(batch_base_offset(&repacked), 100, "base_offset from caller");
        assert!(batch_crc_valid(&repacked));

        // Verify both records are extractable
        let records = extract_records(&repacked, batch_base_timestamp(&repacked));
        assert_eq!(records.len(), 2);
    }

    // -- Nuance: malformed varint returns error, not panic -------------------

    #[test]
    fn rewrite_offset_delta_on_empty_bytes_returns_error() {
        let empty = Bytes::new();
        assert!(
            rewrite_offset_delta(&empty, 0).is_err(),
            "empty record bytes must return Err, not panic"
        );
    }

    #[test]
    fn rewrite_offset_delta_on_truncated_record_returns_error() {
        // A single byte is a valid start of a varint but not a valid record
        let truncated = Bytes::from_static(&[0x05]);
        assert!(
            rewrite_offset_delta(&truncated, 0).is_err(),
            "truncated record must return Err"
        );
    }
}

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

// ---------------------------------------------------------------------------
// Varint helpers
// ---------------------------------------------------------------------------

/// Decode an unsigned protobuf-style varint from `data`.
///
/// Returns `Some((value, bytes_consumed))`, or `None` if the data is
/// truncated or the varint exceeds 10 continuation bytes.
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
pub fn zigzag_decode(n: i64) -> i64 {
    let n = n as u64;
    ((n >> 1) as i64) ^ (-((n & 1) as i64))
}

/// ZigZag-encode a signed value to its unsigned wire representation.
pub fn zigzag_encode(n: i64) -> i64 {
    ((n << 1) ^ (n >> 63)) as i64
}

// ---------------------------------------------------------------------------
// Header field readers
// ---------------------------------------------------------------------------

/// Read a big-endian `i64` from `data` at the given byte offset.
fn read_i64(data: &[u8], offset: usize) -> i64 {
    i64::from_be_bytes(data[offset..offset + 8].try_into().unwrap())
}

/// Read a big-endian `i32` from `data` at the given byte offset.
fn read_i32(data: &[u8], offset: usize) -> i32 {
    i32::from_be_bytes(data[offset..offset + 4].try_into().unwrap())
}

/// Read a big-endian `i16` from `data` at the given byte offset.
fn read_i16(data: &[u8], offset: usize) -> i16 {
    i16::from_be_bytes(data[offset..offset + 2].try_into().unwrap())
}

// ---------------------------------------------------------------------------
// Batch header accessors
// ---------------------------------------------------------------------------

/// Read the base offset (bytes 0..7) from a raw batch.
pub fn batch_base_offset(batch: &[u8]) -> i64 {
    read_i64(batch, 0)
}

/// Read the attributes field (bytes 21..22) from a raw batch.
pub fn batch_attributes(batch: &[u8]) -> i16 {
    read_i16(batch, 21)
}

/// Read the base timestamp (bytes 27..34) from a raw batch.
pub fn batch_base_timestamp(batch: &[u8]) -> i64 {
    read_i64(batch, 27)
}

/// Read the max timestamp (bytes 35..42) from a raw batch.
pub fn batch_max_timestamp(batch: &[u8]) -> i64 {
    read_i64(batch, 35)
}

/// Read the producer ID (bytes 43..50) from a raw batch.
pub fn batch_producer_id(batch: &[u8]) -> i64 {
    read_i64(batch, 43)
}

/// Read the producer epoch (bytes 51..52) from a raw batch.
pub fn batch_producer_epoch(batch: &[u8]) -> i16 {
    read_i16(batch, 51)
}

/// Read the base sequence (bytes 53..56) from a raw batch.
pub fn batch_base_sequence(batch: &[u8]) -> i32 {
    read_i32(batch, 53)
}

/// Read the record count (bytes 57..60) from a raw batch.
pub fn batch_record_count(batch: &[u8]) -> i32 {
    read_i32(batch, 57)
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
    pub fn is_tombstone(&self) -> bool {
        self.value.is_none()
    }
}

// ---------------------------------------------------------------------------
// Record extraction
// ---------------------------------------------------------------------------

/// Parse the records section of a Kafka V2 RecordBatch.
///
/// `batch_bytes` must contain the **entire** batch (header + records).
/// `base_timestamp` is the batch-level base timestamp used to resolve deltas.
///
/// Returns one [`RawRecord`] per record found.
pub fn extract_records(batch_bytes: &Bytes, base_timestamp: i64) -> Vec<RawRecord> {
    if batch_bytes.len() <= RECORD_BATCH_HEADER_SIZE {
        return Vec::new();
    }

    let data = &batch_bytes[RECORD_BATCH_HEADER_SIZE..];
    let mut offset = 0;
    let mut records = Vec::new();

    while offset < data.len() {
        if records.len() >= MAX_RECORDS {
            break;
        }

        let record_start = offset;

        // --- record length (signed zigzag varint) ---
        let (raw_len, len_bytes) = match decode_varint(&data[offset..]) {
            Some(v) => v,
            None => break,
        };
        let record_length = zigzag_decode(raw_len) as usize;
        offset += len_bytes;

        // The total on-wire size for this record is the length-varint + body.
        let record_end = offset + record_length;

        // --- attributes (i8) ---
        offset += 1;

        // --- timestamp delta (signed zigzag) ---
        let (raw_ts, ts_bytes) = match decode_varint(&data[offset..]) {
            Some(v) => v,
            None => break,
        };
        let timestamp_delta = zigzag_decode(raw_ts);
        offset += ts_bytes;

        // --- offset delta (signed zigzag) ---
        let (raw_od, od_bytes) = match decode_varint(&data[offset..]) {
            Some(v) => v,
            None => break,
        };
        let _offset_delta = zigzag_decode(raw_od);
        offset += od_bytes;

        // --- key ---
        let (raw_key_len, kl_bytes) = match decode_varint(&data[offset..]) {
            Some(v) => v,
            None => break,
        };
        let key_length = zigzag_decode(raw_key_len);
        offset += kl_bytes;

        let key = if key_length < 0 {
            None
        } else {
            let kl = key_length as usize;
            let k = batch_bytes
                .slice(RECORD_BATCH_HEADER_SIZE + offset..RECORD_BATCH_HEADER_SIZE + offset + kl);
            offset += kl;
            Some(k)
        };

        // --- value ---
        let (raw_val_len, vl_bytes) = match decode_varint(&data[offset..]) {
            Some(v) => v,
            None => break,
        };
        let value_length = zigzag_decode(raw_val_len);
        offset += vl_bytes;

        let value = if value_length < 0 {
            None
        } else {
            let vl = value_length as usize;
            let v = batch_bytes
                .slice(RECORD_BATCH_HEADER_SIZE + offset..RECORD_BATCH_HEADER_SIZE + offset + vl);
            offset += vl;
            Some(v)
        };

        // --- headers ---
        let (raw_hdr_count, hc_bytes) = match decode_varint(&data[offset..]) {
            Some(v) => v,
            None => break,
        };
        let header_count = zigzag_decode(raw_hdr_count) as usize;
        offset += hc_bytes;

        let mut header_ok = true;
        for _ in 0..header_count {
            // header key length (signed zigzag)
            let (raw_hkl, hkl_bytes) = match decode_varint(&data[offset..]) {
                Some(v) => v,
                None => {
                    header_ok = false;
                    break;
                }
            };
            let hkl = zigzag_decode(raw_hkl);
            offset += hkl_bytes;
            if hkl > 0 {
                offset += hkl as usize;
            }
            // header value length (signed zigzag)
            let (raw_hvl, hvl_bytes) = match decode_varint(&data[offset..]) {
                Some(v) => v,
                None => {
                    header_ok = false;
                    break;
                }
            };
            let hvl = zigzag_decode(raw_hvl);
            offset += hvl_bytes;
            if hvl > 0 {
                offset += hvl as usize;
            }
        }
        if !header_ok {
            break;
        }

        // Skip malformed record instead of panicking on external data.
        if offset != record_end {
            offset = record_end;
            continue;
        }

        let record_bytes = batch_bytes.slice(
            RECORD_BATCH_HEADER_SIZE + record_start..RECORD_BATCH_HEADER_SIZE + record_end,
        );

        records.push(RawRecord {
            key,
            value,
            record_bytes,
            timestamp: base_timestamp + timestamp_delta,
        });
    }

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
pub fn rewrite_offset_delta(record_bytes: &Bytes, new_offset_delta: i64) -> Bytes {
    let data = &record_bytes[..];

    // 1. Decode record_length varint
    let (_raw_len, len_varint_bytes) = decode_varint(data).expect("valid record_length varint");
    let body_start = len_varint_bytes;

    // 2. Skip attributes (1 byte)
    let mut pos = body_start + 1;

    // 3. Skip timestamp_delta varint
    let (_ts_raw, ts_bytes) = decode_varint(&data[pos..]).expect("valid timestamp_delta varint");
    pos += ts_bytes;

    // 4. Decode existing offset_delta varint (so we know how many bytes it occupies)
    let (_od_raw, od_bytes) = decode_varint(&data[pos..]).expect("valid offset_delta varint");
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
    buf.freeze()
}

// ---------------------------------------------------------------------------
// Batch builder
// ---------------------------------------------------------------------------

/// Build a complete Kafka V2 RecordBatch from header fields and raw record
/// bytes.
///
/// Writes all header fields in big-endian order, appends `record_bytes`, and
/// computes the CRC32C.
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
pub fn repack_batch(
    batch_bytes: &Bytes,
    base_offset: i64,
    keep: impl Fn(usize, &RawRecord) -> bool,
) -> Option<Bytes> {
    if batch_bytes.len() < RECORD_BATCH_HEADER_SIZE {
        return None;
    }

    // Read original header fields
    let base_timestamp = read_i64(batch_bytes, 27);
    let max_timestamp = read_i64(batch_bytes, 35);
    let producer_id = read_i64(batch_bytes, 43);
    let producer_epoch = read_i16(batch_bytes, 51);
    let base_sequence = read_i32(batch_bytes, 53);
    let attributes = read_i16(batch_bytes, 21);

    // Extract all records
    let records = extract_records(batch_bytes, base_timestamp);

    // Keep only matching records, rewriting offset deltas to be contiguous
    let mut record_buf = BytesMut::new();
    let mut kept_count: i32 = 0;

    for (i, record) in records.iter().enumerate() {
        if !keep(i, record) {
            continue;
        }
        let rewritten = rewrite_offset_delta(&record.record_bytes, kept_count as i64);
        record_buf.extend_from_slice(&rewritten);
        kept_count += 1;
    }

    if kept_count == 0 {
        return None;
    }

    Some(build_batch(
        base_offset,
        base_timestamp,
        max_timestamp,
        producer_id,
        producer_epoch,
        base_sequence,
        attributes,
        kept_count,
        &record_buf,
    ))
}

/// Assemble a new RecordBatch from individual records.
///
/// Each entry in `records` is `(record_bytes, timestamp)` where
/// `record_bytes` is the full on-wire record (length varint + body) and
/// `timestamp` is the absolute timestamp.
///
/// The produced batch has no idempotent producer metadata (`producerId = -1`,
/// `producerEpoch = -1`, `baseSequence = -1`).
pub fn reassemble_batch(records: &[(Bytes, i64)], base_offset: i64) -> Bytes {
    let record_count = records.len() as i32;

    // Determine base and max timestamps
    let base_timestamp = records.iter().map(|(_, ts)| *ts).min().unwrap_or(0);
    let max_timestamp = records.iter().map(|(_, ts)| *ts).max().unwrap_or(0);

    // Rewrite offset deltas to be contiguous and concatenate
    let mut record_buf = BytesMut::new();
    for (i, (rec_bytes, _ts)) in records.iter().enumerate() {
        let rewritten = rewrite_offset_delta(rec_bytes, i as i64);
        record_buf.extend_from_slice(&rewritten);
    }

    build_batch(
        base_offset,
        base_timestamp,
        max_timestamp,
        -1, // producerId — no idempotency
        -1, // producerEpoch
        -1, // baseSequence
        0,  // attributes — no compression, no txn
        record_count,
        &record_buf,
    )
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
        let rewritten = rewrite_offset_delta(&original, 0);

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

        let batch = reassemble_batch(&records, 42);

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
        let repacked =
            repack_batch(&batch, 0, |i, _rec| i != 1).expect("should return Some when some kept");

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
            repack_batch(&batch, 0, |_, _| false).is_none(),
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
}

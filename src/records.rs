//! Provides utilities for working with records (Kafka messages).
// Casts in this module are guarded by explicit range checks at each cast site. The pedantic cast
// lints would require changing the protocol encoding API; allow them at module level.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::option_if_let_else)]
//!
//! [`FetchResponse`](crate::messages::fetch_response::FetchResponse) and associated APIs for interacting with reading and writing
//! contain records in a raw format, allowing the user to implement their own logic for interacting
//! with those values.
//!
//! # Example
//!
//! Decoding a set of records from a [`FetchResponse`](crate::messages::fetch_response::FetchResponse):
//! ```rust
//! use kafka_protocol::messages::FetchResponse;
//! use kafka_protocol::protocol::Decodable;
//! use kafka_protocol::records::RecordBatchDecoder;
//! use bytes::Bytes;
//!
//! # const HEADER: [u8; 45] = [ 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,];
//! # const RECORD: [u8; 79] = [ 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x43, 0x0, 0x0, 0x0, 0x0, 0x2, 0x73, 0x6d, 0x29, 0x7b, 0x0, 0b00000000, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x22, 0x1, 0xd0, 0xf, 0x2, 0xa, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0xa, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x0,];
//! # let mut res = vec![];
//! # res.extend_from_slice(&HEADER[..]);
//! # res.extend_from_slice(&[0x00, 0x00, 0x00, 0x4f]);
//! # res.extend_from_slice(&RECORD[..]);
//! # let mut buf = Bytes::from(res);
//!
//! let res = FetchResponse::decode(&mut buf, 4).unwrap();
//!
//! for topic in res.responses {
//!     for partition in topic.partitions {
//!          let mut records = partition.records.unwrap();
//!          let records = RecordBatchDecoder::decode(&mut records).unwrap();
//!     }
//! }
//! ```
use bytes::Bytes;
use indexmap::IndexMap;
use crc32c::crc32c;
use log::error;
use crc::{CRC_32_CKSUM, Crc};
use string::TryFrom;

use crate::protocol::{
    buf::{gap, ByteBuf, ByteBufMut},
    types, DecodeError, DecodeErrorKind, Decoder, EncodeError, EncodeErrorKind, Encoder, StrBytes,
};

use super::compression::{self as cmpr, Compressor, Decompressor};
use std::cmp::Ordering;

/// IEEE (checksum) cyclic redundancy check.
pub const IEEE: Crc<u32> = Crc::<u32>::new(&CRC_32_CKSUM);

/// The different types of compression supported by Kafka.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Compression {
    /// No compression.
    None = 0,
    /// gzip compression library.
    Gzip = 1,
    /// Google's Snappy compression library.
    Snappy = 2,
    /// The LZ4 compression library.
    Lz4 = 3,
    /// Facebook's `ZStandard` compression library.
    Zstd = 4,
}

/// Indicates the meaning of the timestamp field on a record.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TimestampType {
    /// The timestamp represents when the record was created by the client.
    Creation = 0,
    /// The timestamp represents when the record was appended to the log.
    LogAppend = 1,
}

/// Options for encoding and compressing a batch of records. Note, not all compression algorithms
/// are currently implemented by this library.
#[derive(Debug, Clone)]
pub struct RecordEncodeOptions {
    /// Record version, 0, 1, or 2.
    pub version: i8,

    /// The compression algorithm to use.
    pub compression: Compression,
}

/// Value to indicate missing producer id.
pub const NO_PRODUCER_ID: i64 = -1;
/// Value to indicate missing producer epoch.
pub const NO_PRODUCER_EPOCH: i16 = -1;
/// Value to indicated missing leader epoch.
pub const NO_PARTITION_LEADER_EPOCH: i32 = -1;
/// Value to indicate missing sequence id.
pub const NO_SEQUENCE: i32 = -1;
/// Value to indicate missing timestamp.
pub const NO_TIMESTAMP: i64 = -1;
/// Maximum nesting depth for compressed legacy record wrappers.
const MAX_LEGACY_NESTING: usize = 4;

#[derive(Debug, Clone)]
/// Batch encoder for Kafka records.
pub struct RecordBatchEncoder;

#[derive(Debug, Clone)]
/// Batch decoder for Kafka records.
pub struct RecordBatchDecoder;

struct BatchProperties {
    num_records: usize,
    min_offset: i64,
    max_offset: i64,
    min_timestamp: i64,
    max_timestamp: i64,
    base_sequence: i32,
}

struct BatchDecodeInfo {
    record_count: usize,
    timestamp_type: TimestampType,
    min_offset: i64,
    min_timestamp: i64,
    base_sequence: i32,
    transactional: bool,
    control: bool,
    partition_leader_epoch: i32,
    producer_id: i64,
    producer_epoch: i16,
    compression: Compression,
}

/// A Kafka message containing key, payload value, and all associated metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    // Batch properties
    /// Whether this record is transactional.
    pub transactional: bool,
    /// Whether this record is a control message, which should not be exposed to the client.
    pub control: bool,
    /// Epoch of the leader for this record 's partition.
    pub partition_leader_epoch: i32,
    /// The identifier of the producer.
    pub producer_id: i64,
    /// Producer metadata used to implement transactional writes.
    pub producer_epoch: i16,

    // Record properties
    /// Indicates whether timestamp represents record creation or appending to the log.
    pub timestamp_type: TimestampType,
    /// Message offset within a partition.
    pub offset: i64,
    /// Sequence identifier used for idempotent delivery.
    pub sequence: i32,
    /// Timestamp the record. See also `timestamp_type`.
    pub timestamp: i64,
    /// The key of the record.
    pub key: Option<Bytes>,
    /// The payload of the record.
    pub value: Option<Bytes>,
    /// Headers associated with the record's payload.
    pub headers: IndexMap<StrBytes, Option<Bytes>>,
}

const MAGIC_BYTE_OFFSET: usize = 16;

impl RecordBatchEncoder {
    /// Encode records into given buffer, using provided encoding options that select the encoding
    /// strategy based on version.
    pub fn encode<'a, B, I>(
        buf: &mut B,
        records: I,
        options: &RecordEncodeOptions,
    ) -> Result<(), EncodeError>
    where
        B: ByteBufMut,
        I: Iterator<Item = &'a Record> + Clone,
    {
        match options.version {
            0..=1 => Self::encode_legacy(buf, records, options),
            2 => Self::encode_new(buf, records, options),
            _ => {
                error!("Unknown record batch version ({})", options.version);
                Err(EncodeError { kind: EncodeErrorKind::InvalidData })
            }
        }
    }
    fn encode_legacy_records<'a, B, I>(
        buf: &mut B,
        records: I,
        options: &RecordEncodeOptions,
    ) -> Result<(), EncodeError>
    where
        B: ByteBufMut,
        I: Iterator<Item = &'a Record> + Clone,
    {
        for record in records {
            record.encode_legacy(buf, options)?;
        }
        Ok(())
    }
    fn encode_legacy<'a, B, I>(
        buf: &mut B,
        records: I,
        options: &RecordEncodeOptions,
    ) -> Result<(), EncodeError>
    where
        B: ByteBufMut,
        I: Iterator<Item = &'a Record> + Clone,
    {
        if options.compression == Compression::None {
            // No wrapper needed
            Self::encode_legacy_records(buf, records, options)?;
        } else {
            // Need a "wrapper" message
            let inner_opts = RecordEncodeOptions {
                compression: Compression::None,
                version: options.version,
            };

            Record::encode_legacy_static(buf, options, |buf| {
                // Timestamp
                if options.version > 0 {
                    let min_timestamp = records
                        .clone()
                        .map(|r| r.timestamp)
                        .min()
                        .unwrap_or_default();
                    types::Int64.encode(buf, min_timestamp)?;
                }

                // Key
                buf.put_i32(-1);

                // Value (Compressed MessageSet)
                let size_gap = buf.put_typed_gap(gap::I32);
                let value_start = buf.offset();
                match options.compression {
                    Compression::Snappy => cmpr::Snappy::compress(buf, |buf| {
                        Self::encode_legacy_records(buf, records, &inner_opts)
                    })?,
                    Compression::Gzip => cmpr::Gzip::compress(buf, |buf| {
                        Self::encode_legacy_records(buf, records, &inner_opts)
                    })?,
                    _ => {
                        error!(
                            "Compression algorithm '{:?}' is not implemented for legacy encoding",
                            options.compression
                        );
                        return Err(EncodeError { kind: EncodeErrorKind::InvalidData });
                    }
                }

                let value_end = buf.offset();
                let value_size = value_end - value_start;
                if value_size > i32::MAX as usize {
                    error!(
                        "Record batch was too large to encode ({value_size} bytes)"
                    );
                    return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
                }
                buf.fill_typed_gap(size_gap, value_size as i32);

                Ok(())
            })?;
        }
        Ok(())
    }

    fn encode_new_records<'a, B, I>(
        buf: &mut B,
        records: I,
        min_offset: i64,
        min_timestamp: i64,
        options: &RecordEncodeOptions,
    ) -> Result<(), EncodeError>
    where
        B: ByteBufMut,
        I: Iterator<Item = &'a Record>,
    {
        for record in records {
            record.encode_new(buf, min_offset, min_timestamp, options)?;
        }
        Ok(())
    }

    fn compute_batch_properties<'a, I>(
        records: &I,
        first_record: &Record,
        num_records: usize,
    ) -> Result<BatchProperties, EncodeError>
    where
        I: Iterator<Item = &'a Record> + Clone,
    {
        let min_offset = records.clone().take(num_records).map(|r| r.offset).min()
            .ok_or(EncodeError { kind: EncodeErrorKind::InvalidData })?;
        let max_offset = records.clone().take(num_records).map(|r| r.offset).max()
            .ok_or(EncodeError { kind: EncodeErrorKind::InvalidData })?;
        let min_timestamp = records.clone().take(num_records).map(|r| r.timestamp).min()
            .ok_or(EncodeError { kind: EncodeErrorKind::InvalidData })?;
        let max_timestamp = records.clone().take(num_records).map(|r| r.timestamp).max()
            .ok_or(EncodeError { kind: EncodeErrorKind::InvalidData })?;
        let base_sequence = first_record
            .sequence
            .wrapping_sub((first_record.offset - min_offset) as i32);
        Ok(BatchProperties { num_records, min_offset, max_offset, min_timestamp, max_timestamp, base_sequence })
    }

    fn write_batch_header<B: ByteBufMut>(
        buf: &mut B,
        first_record: &Record,
        props: &BatchProperties,
        options: &RecordEncodeOptions,
    ) -> Result<(), EncodeError> {
        let mut attributes = options.compression as i16;
        if first_record.transactional {
            attributes |= 1 << 4;
        }
        if first_record.control {
            attributes |= 1 << 5;
        }
        types::Int16.encode(buf, attributes)?;
        types::Int32.encode(buf, (props.max_offset - props.min_offset) as i32)?;
        types::Int64.encode(buf, props.min_timestamp)?;
        types::Int64.encode(buf, props.max_timestamp)?;
        types::Int64.encode(buf, first_record.producer_id)?;
        types::Int16.encode(buf, first_record.producer_epoch)?;
        types::Int32.encode(buf, props.base_sequence)?;
        if props.num_records > i32::MAX as usize {
            error!("Too many records to encode in one batch ({} records)", props.num_records);
            return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
        }
        types::Int32.encode(buf, props.num_records as i32)?;
        Ok(())
    }

    fn compress_and_write_records<'a, B, I>(
        buf: &mut B,
        records: I,
        props: &BatchProperties,
        options: &RecordEncodeOptions,
    ) -> Result<(), EncodeError>
    where
        B: ByteBufMut,
        I: Iterator<Item = &'a Record>,
    {
        let min_offset = props.min_offset;
        let min_timestamp = props.min_timestamp;
        match options.compression {
            Compression::None => cmpr::None::compress(buf, |buf| {
                Self::encode_new_records(buf, records, min_offset, min_timestamp, options)
            })?,
            Compression::Snappy => cmpr::Snappy::compress(buf, |buf| {
                Self::encode_new_records(buf, records, min_offset, min_timestamp, options)
            })?,
            Compression::Gzip => cmpr::Gzip::compress(buf, |buf| {
                Self::encode_new_records(buf, records, min_offset, min_timestamp, options)
            })?,
            _ => {
                error!(
                    "Compression algorithm '{:?}' is not implemented for new batch encoding",
                    options.compression
                );
                return Err(EncodeError { kind: EncodeErrorKind::InvalidData });
            }
        }
        Ok(())
    }

    fn encode_new_batch<'a, B, I>(
        buf: &mut B,
        records: &mut I,
        options: &RecordEncodeOptions,
    ) -> Result<bool, EncodeError>
    where
        B: ByteBufMut,
        I: Iterator<Item = &'a Record> + Clone,
    {
        let mut record_peeker = records.clone();
        let Some(first_record) = record_peeker.next() else { return Ok(false) };

        let num_records = record_peeker
            .take_while(|record| {
                record.transactional == first_record.transactional
                    && record.control == first_record.control
                    && record.partition_leader_epoch == first_record.partition_leader_epoch
                    && record.producer_id == first_record.producer_id
                    && record.producer_epoch == first_record.producer_epoch
                    && (record.offset as i32).wrapping_sub(record.sequence)
                        == (first_record.offset as i32).wrapping_sub(first_record.sequence)
            })
            .count()
            + 1;

        let props = Self::compute_batch_properties(records, first_record, num_records)?;

        // Base offset
        types::Int64.encode(buf, props.min_offset)?;

        // Batch length gap
        let size_gap = buf.put_typed_gap(gap::I32);
        let batch_start = buf.offset();

        types::Int32.encode(buf, first_record.partition_leader_epoch)?;
        types::Int8.encode(buf, options.version)?;

        let crc_gap = buf.put_typed_gap(gap::U32);
        let content_start = buf.offset();

        Self::write_batch_header(buf, first_record, &props, options)?;
        Self::compress_and_write_records(buf, records.take(num_records), &props, options)?;

        let batch_end = buf.offset();
        let batch_size = batch_end - batch_start;
        if batch_size > i32::MAX as usize {
            error!("Record batch was too large to encode ({batch_size} bytes)");
            return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
        }
        buf.fill_typed_gap(size_gap, batch_size as i32);

        let crc = crc32c(buf.range(content_start..batch_end));
        buf.fill_typed_gap(crc_gap, crc);

        Ok(true)
    }

    fn encode_new<'a, B, I>(
        buf: &mut B,
        mut records: I,
        options: &RecordEncodeOptions,
    ) -> Result<(), EncodeError>
    where
        B: ByteBufMut,
        I: Iterator<Item = &'a Record> + Clone,
    {
        while Self::encode_new_batch(buf, &mut records, options)? {}
        Ok(())
    }
}

impl RecordBatchDecoder {
    /// Decode the provided buffer into a vec of records.
    pub fn decode<B: ByteBuf>(buf: &mut B) -> Result<Vec<Record>, DecodeError> {
        let mut records = Vec::new();
        while buf.has_remaining() {
            Self::decode_batch(buf, &mut records)?;
        }
        Ok(records)
    }
    fn decode_batch<B: ByteBuf>(buf: &mut B, records: &mut Vec<Record>) -> Result<(), DecodeError> {
        let version = buf.try_peek_bytes(MAGIC_BYTE_OFFSET..(MAGIC_BYTE_OFFSET + 1))?[0] as i8;
        debug!("Decoding record batch (version: {version})");
        match version {
            0..=1 => Record::decode_legacy(buf, version, records),
            2 => Self::decode_new_batch(buf, version, records),
            _ => {
                error!("Unknown record batch version ({version})");
                Err(DecodeError { kind: DecodeErrorKind::InvalidData })
            }
        }
    }
    fn decode_new_records<B: ByteBuf>(
        buf: &mut B,
        batch_decode_info: &BatchDecodeInfo,
        version: i8,
        records: &mut Vec<Record>,
    ) -> Result<(), DecodeError> {
        records.reserve(batch_decode_info.record_count);
        for _ in 0..batch_decode_info.record_count {
            records.push(Record::decode_new(buf, batch_decode_info, version)?);
        }
        Ok(())
    }
    fn decode_batch_attributes<B: ByteBuf>(
        buf: &mut B,
        min_offset: i64,
        partition_leader_epoch: i32,
    ) -> Result<BatchDecodeInfo, DecodeError> {
        let attributes: i16 = types::Int16.decode(buf)?;
        let transactional = (attributes & (1 << 4)) != 0;
        let control = (attributes & (1 << 5)) != 0;
        let compression = match attributes & 0x7 {
            0 => Compression::None,
            1 => Compression::Gzip,
            2 => Compression::Snappy,
            3 => Compression::Lz4,
            4 => Compression::Zstd,
            other => {
                error!("Unknown compression algorithm used: {other}");
                return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
            }
        };
        let timestamp_type = if (attributes & (1 << 3)) != 0 {
            TimestampType::LogAppend
        } else {
            TimestampType::Creation
        };

        let _max_offset_delta: i32 = types::Int32.decode(buf)?;
        let min_timestamp = types::Int64.decode(buf)?;
        let _max_timestamp: i64 = types::Int64.decode(buf)?;
        let producer_id = types::Int64.decode(buf)?;
        let producer_epoch = types::Int16.decode(buf)?;
        let base_sequence = types::Int32.decode(buf)?;

        let record_count: i32 = types::Int32.decode(buf)?;
        if record_count < 0 {
            error!("Unexpected negative record count ({record_count})");
            return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
        }

        Ok(BatchDecodeInfo {
            record_count: record_count as usize,
            timestamp_type,
            min_offset,
            min_timestamp,
            base_sequence,
            transactional,
            control,
            partition_leader_epoch,
            producer_id,
            producer_epoch,
            compression,
        })
    }

    fn decompress_and_decode_records<B: ByteBuf>(
        buf: &mut B,
        info: &BatchDecodeInfo,
        version: i8,
        records: &mut Vec<Record>,
    ) -> Result<(), DecodeError> {
        match info.compression {
            Compression::None => cmpr::None::decompress(buf, |buf| {
                Self::decode_new_records(buf, info, version, records)
            })?,
            Compression::Snappy => cmpr::Snappy::decompress(buf, |buf| {
                Self::decode_new_records(buf, info, version, records)
            })?,
            Compression::Gzip => cmpr::Gzip::decompress(buf, |buf| {
                Self::decode_new_records(buf, info, version, records)
            })?,
            _ => {
                error!(
                    "Compression algorithm '{:?}' is not implemented for decoding",
                    info.compression
                );
                return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
            }
        }
        Ok(())
    }

    fn decode_new_batch<B: ByteBuf>(
        buf: &mut B,
        version: i8,
        records: &mut Vec<Record>,
    ) -> Result<(), DecodeError> {
        let min_offset = types::Int64.decode(buf)?;

        let batch_length: i32 = types::Int32.decode(buf)?;
        if batch_length < 0 {
            error!("Unexpected negative batch size: {batch_length}");
            return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
        }

        let buf = &mut buf.try_get_bytes(batch_length as usize)?;
        let partition_leader_epoch = types::Int32.decode(buf)?;

        let magic: i8 = types::Int8.decode(buf)?;
        if magic != version {
            error!("Version mismatch ({magic} != {version})");
            return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
        }

        let supplied_crc: u32 = types::UInt32.decode(buf)?;
        let actual_crc = crc32c(buf);
        if supplied_crc != actual_crc {
            error!("Cyclic redundancy check failed ({supplied_crc} != {actual_crc})");
            return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
        }

        let info = Self::decode_batch_attributes(buf, min_offset, partition_leader_epoch)?;
        Self::decompress_and_decode_records(buf, &info, version, records)
    }
}

impl Record {
    fn encode_legacy_static<B, F>(
        buf: &mut B,
        options: &RecordEncodeOptions,
        content_writer: F,
    ) -> Result<(), EncodeError>
    where
        B: ByteBufMut,
        F: FnOnce(&mut B) -> Result<(), EncodeError>,
    {
        types::Int64.encode(buf, 0)?;
        let size_gap = buf.put_typed_gap(gap::I32);
        let message_start = buf.offset();
        let crc_gap = buf.put_typed_gap(gap::U32);
        let content_start = buf.offset();

        types::Int8.encode(buf, options.version)?;

        let compression = options.compression as i8;
        if compression > 2 + options.version {
            error!(
                "Compression algorithm '{:?}' is unsupported for record version '{}'",
                options.compression, options.version
            );
            return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
        }
        types::Int8.encode(buf, compression)?;

        // Write content
        content_writer(buf)?;

        let message_end = buf.offset();

        let message_size = message_end - message_start;
        if message_start > i32::MAX as usize {
            error!("Record was too large to encode ({message_size} bytes)");
            return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
        }
        buf.fill_typed_gap(size_gap, message_size as i32);

        let crc = IEEE.checksum(buf.range(content_start..message_end));
        buf.fill_typed_gap(crc_gap, crc);

        Ok(())
    }
    fn encode_legacy<B: ByteBufMut>(
        &self,
        buf: &mut B,
        options: &RecordEncodeOptions,
    ) -> Result<(), EncodeError> {
        if self.transactional || self.control {
            error!("Transactional and control records are not supported in this version of the protocol!");
            return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
        }

        if !self.headers.is_empty() {
            error!("Record headers are not supported in this version of the protocol!");
            return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
        }

        Self::encode_legacy_static(buf, options, |buf| {
            if options.version > 0 {
                types::Int64.encode(buf, self.timestamp)?;
            }
            types::Bytes.encode(buf, &self.key)?;
            types::Bytes.encode(buf, &self.value)?;

            Ok(())
        })
    }
    fn checked_delta(base: i64, actual: i64, label: &str) -> Result<i32, EncodeError> {
        let delta = actual - base;
        if delta > i32::MAX as i64 || delta < i32::MIN as i64 {
            error!("{label} within batch are too far apart ({base}, {actual})");
            return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
        }
        Ok(delta as i32)
    }

    fn encode_optional_bytes<B: ByteBufMut>(
        buf: &mut B,
        data: Option<&Bytes>,
        label: &str,
    ) -> Result<(), EncodeError> {
        if let Some(d) = data {
            if d.len() > i32::MAX as usize {
                error!("Record {} was too large to encode ({} bytes)", label, d.len());
                return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
            }
            types::VarInt.encode(buf, d.len() as i32)?;
            buf.put_slice(d);
        } else {
            types::VarInt.encode(buf, -1)?;
        }
        Ok(())
    }

    fn encode_record_headers<B: ByteBufMut>(
        buf: &mut B,
        headers: &IndexMap<StrBytes, Option<Bytes>>,
    ) -> Result<(), EncodeError> {
        if headers.len() > i32::MAX as usize {
            error!("Too many record headers encode ({})", headers.len());
            return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
        }
        types::VarInt.encode(buf, headers.len() as i32)?;
        for (k, v) in headers {
            if k.len() > i32::MAX as usize {
                error!("Record header key was too large to encode ({} bytes)", k.len());
                return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
            }
            types::VarInt.encode(buf, k.len() as i32)?;
            buf.put_slice(k.as_ref());
            Self::encode_optional_bytes(buf, v.as_ref(), "header value")?;
        }
        Ok(())
    }

    fn encode_new<B: ByteBufMut>(
        &self,
        buf: &mut B,
        min_offset: i64,
        min_timestamp: i64,
        options: &RecordEncodeOptions,
    ) -> Result<(), EncodeError> {
        let size = self.compute_size_new(min_offset, min_timestamp, options)?;
        if size > i32::MAX as usize {
            error!("Record was too large to encode ({size} bytes)");
            return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
        }
        types::VarInt.encode(buf, size as i32)?;
        types::Int8.encode(buf, 0)?;

        let timestamp_delta = Self::checked_delta(min_timestamp, self.timestamp, "Timestamps")?;
        types::VarInt.encode(buf, timestamp_delta)?;

        let offset_delta = Self::checked_delta(min_offset, self.offset, "Offsets")?;
        types::VarInt.encode(buf, offset_delta)?;

        Self::encode_optional_bytes(buf, self.key.as_ref(), "key")?;
        Self::encode_optional_bytes(buf, self.value.as_ref(), "value")?;
        Self::encode_record_headers(buf, &self.headers)?;
        Ok(())
    }

    fn compute_optional_bytes_size(data: Option<&Bytes>, label: &str) -> Result<usize, EncodeError> {
        if let Some(d) = data {
            if d.len() > i32::MAX as usize {
                error!("Record {} was too large to encode ({} bytes)", label, d.len());
                return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
            }
            Ok(types::VarInt.compute_size(d.len() as i32)? + d.len())
        } else {
            Ok(types::VarInt.compute_size(-1)?)
        }
    }

    fn compute_headers_size(
        headers: &IndexMap<StrBytes, Option<Bytes>>,
    ) -> Result<usize, EncodeError> {
        if headers.len() > i32::MAX as usize {
            error!("Too many record headers encode ({})", headers.len());
            return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
        }
        let mut size = types::VarInt.compute_size(headers.len() as i32)?;
        for (k, v) in headers {
            if k.len() > i32::MAX as usize {
                error!("Record header key was too large to encode ({} bytes)", k.len());
                return Err(EncodeError { kind: EncodeErrorKind::TooLarge });
            }
            size += types::VarInt.compute_size(k.len() as i32)?;
            size += k.len();
            size += Self::compute_optional_bytes_size(v.as_ref(), "header value")?;
        }
        Ok(size)
    }

    fn compute_size_new(
        &self,
        min_offset: i64,
        min_timestamp: i64,
        _options: &RecordEncodeOptions,
    ) -> Result<usize, EncodeError> {
        let mut total_size = types::Int8.compute_size(0)?;

        let timestamp_delta = Self::checked_delta(min_timestamp, self.timestamp, "Timestamps")?;
        total_size += types::VarInt.compute_size(timestamp_delta)?;

        let offset_delta = Self::checked_delta(min_offset, self.offset, "Offsets")?;
        total_size += types::VarInt.compute_size(offset_delta)?;

        total_size += Self::compute_optional_bytes_size(self.key.as_ref(), "key")?;
        total_size += Self::compute_optional_bytes_size(self.value.as_ref(), "value")?;
        total_size += Self::compute_headers_size(&self.headers)?;

        Ok(total_size)
    }
    fn decode_legacy<B: ByteBuf>(
        buf: &mut B,
        version: i8,
        records: &mut Vec<Record>,
    ) -> Result<(), DecodeError> {
        Self::decode_legacy_inner(buf, version, records, 0)
    }

    fn decode_legacy_attributes(
        attributes: i8,
        version: i8,
    ) -> Result<(Compression, TimestampType), DecodeError> {
        let compression = match attributes & 0x7 {
            0 => Compression::None,
            1 => Compression::Gzip,
            2 => Compression::Snappy,
            3 if version > 0 => Compression::Lz4,
            other => {
                error!("Unknown compression algorithm used: {other}");
                return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
            }
        };
        let timestamp_type = if (attributes & (1 << 3)) != 0 {
            TimestampType::LogAppend
        } else {
            TimestampType::Creation
        };
        Ok((compression, timestamp_type))
    }

    fn build_legacy_record(
        offset: i64,
        timestamp: i64,
        timestamp_type: TimestampType,
        key: Option<Bytes>,
        value: Option<Bytes>,
    ) -> Record {
        Record {
            transactional: false,
            control: false,
            partition_leader_epoch: NO_PARTITION_LEADER_EPOCH,
            producer_id: NO_PRODUCER_ID,
            producer_epoch: NO_PRODUCER_EPOCH,
            sequence: NO_SEQUENCE,
            timestamp_type,
            offset,
            timestamp,
            key,
            value,
            headers: IndexMap::default(),
        }
    }

    fn decode_legacy_inner<B: ByteBuf>(
        buf: &mut B,
        version: i8,
        records: &mut Vec<Record>,
        depth: usize,
    ) -> Result<(), DecodeError> {
        if depth > MAX_LEGACY_NESTING {
            error!("Legacy record nesting depth exceeded maximum ({MAX_LEGACY_NESTING})");
            return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
        }

        let offset = types::Int64.decode(buf)?;
        let size: i32 = types::Int32.decode(buf)?;
        if size < 0 {
            error!("Unexpected negative record size: {size}");
            return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
        }

        let buf = &mut buf.try_get_bytes(size as usize)?;

        let supplied_crc: u32 = types::UInt32.decode(buf)?;
        let actual_crc = IEEE.checksum(buf);
        if supplied_crc != actual_crc {
            error!("Cyclic redundancy check failed ({supplied_crc} != {actual_crc})");
            return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
        }

        let magic: i8 = types::Int8.decode(buf)?;
        if magic != version {
            error!("Version mismatch ({magic} != {version})");
            return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
        }

        let attributes: i8 = types::Int8.decode(buf)?;
        let (compression, timestamp_type) = Self::decode_legacy_attributes(attributes, version)?;

        let timestamp = if version > 0 { types::Int64.decode(buf)? } else { NO_TIMESTAMP };
        let key = types::Bytes.decode(buf)?;
        let value = types::Bytes.decode(buf)?;

        if compression == Compression::None {
            records.push(Self::build_legacy_record(offset, timestamp, timestamp_type, key, value));
        } else {
            let mut value = value.ok_or_else(|| {
                error!("Received compressed legacy record without a value");
                DecodeError { kind: DecodeErrorKind::InvalidData }
            })?;
            while !value.is_empty() {
                Self::decode_legacy_inner(&mut value, version, records, depth + 1)?;
            }
        }

        Ok(())
    }
    fn decode_optional_bytes<B: ByteBuf>(
        buf: &mut B,
        label: &str,
    ) -> Result<Option<Bytes>, DecodeError> {
        let len: i32 = types::VarInt.decode(buf)?;
        match len.cmp(&-1) {
            Ordering::Less => {
                error!("Unexpected negative record {label} length ({len} bytes)");
                Err(DecodeError { kind: DecodeErrorKind::InvalidData })
            }
            Ordering::Equal => Ok(None),
            Ordering::Greater => Ok(Some(buf.try_get_bytes(len as usize)?)),
        }
    }

    fn decode_record_headers<B: ByteBuf>(
        buf: &mut B,
    ) -> Result<IndexMap<StrBytes, Option<Bytes>>, DecodeError> {
        let num_headers: i32 = types::VarInt.decode(buf)?;
        if num_headers < 0 {
            error!("Unexpected negative record header count: {num_headers}");
            return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
        }
        let num_headers = num_headers as usize;
        let mut headers = IndexMap::with_capacity(num_headers);
        for _ in 0..num_headers {
            let key_len: i32 = types::VarInt.decode(buf)?;
            if key_len < 0 {
                error!("Unexpected negative record header key length ({key_len} bytes)");
                return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
            }
            let key = StrBytes::try_from(buf.try_get_bytes(key_len as usize)?)?;
            let value_len: i32 = types::VarInt.decode(buf)?;
            let value = match value_len.cmp(&-2) {
                Ordering::Less => {
                    error!("Unexpected negative record header value length ({value_len} bytes)");
                    return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
                }
                Ordering::Equal => None,
                Ordering::Greater => Some(buf.try_get_bytes(value_len as usize)?),
            };
            headers.insert(key, value);
        }
        Ok(headers)
    }

    fn decode_new<B: ByteBuf>(
        buf: &mut B,
        batch_decode_info: &BatchDecodeInfo,
        _version: i8,
    ) -> Result<Self, DecodeError> {
        let size: i32 = types::VarInt.decode(buf)?;
        if size < 0 {
            error!("Unexpected negative record size: {size}");
            return Err(DecodeError { kind: DecodeErrorKind::InvalidData });
        }

        let buf = &mut buf.try_get_bytes(size as usize)?;
        let _attributes: i8 = types::Int8.decode(buf)?;

        let timestamp_delta: i32 = types::VarInt.decode(buf)?;
        let timestamp = batch_decode_info.min_timestamp + timestamp_delta as i64;

        let offset_delta: i32 = types::VarInt.decode(buf)?;
        let offset = batch_decode_info.min_offset + offset_delta as i64;
        let sequence = batch_decode_info.base_sequence.wrapping_add(offset_delta);

        let key = Self::decode_optional_bytes(buf, "key")?;
        let value = Self::decode_optional_bytes(buf, "value")?;
        let headers = Self::decode_record_headers(buf)?;

        Ok(Self {
            transactional: batch_decode_info.transactional,
            control: batch_decode_info.control,
            timestamp_type: batch_decode_info.timestamp_type,
            partition_leader_epoch: batch_decode_info.partition_leader_epoch,
            producer_id: batch_decode_info.producer_id,
            producer_epoch: batch_decode_info.producer_epoch,
            sequence,
            offset,
            timestamp,
            key,
            value,
            headers,
        })
    }
}

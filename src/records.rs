//! Provides utilities for working with records (Kafka messages).
//!
//! [`FetchResponse`](crate::messages::fetch_response::FetchResponse) and associated APIs for interacting with reading and writing
//! contain records in a raw format, allowing the user to implement their own logic for interacting
//! with those values.
//!
//! # Example
//!
//! Decoding a set of records from a [`FetchResponse`](crate::messages::fetch_response::FetchResponse):
//! ```rust
//! use proto_kafka::messages::FetchResponse;
//! use proto_kafka::protocol::Decodable;
//! use proto_kafka::records::RecordBatchDecoder;
//! use bytes::Bytes;
//! use proto_kafka::records::Compression;
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
//!          let records = RecordBatchDecoder::decode_with_custom_compression(&mut records, Some(decompress_record_batch_data)).unwrap();
//!     }
//! }
//!
//! fn decompress_record_batch_data(compressed_buffer: &mut bytes::Bytes, compression: Compression) -> proto_kafka::error::Result<Bytes> {
//!         match compression {
//!             Compression::None => Ok(compressed_buffer.to_vec().into()),
//!             _ => { panic!("Compression not implemented") }
//!         }
//!  }
//! ```
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::missing_errors_doc,
    clippy::doc_markdown,
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::semicolon_if_nothing_returned,
    clippy::unnecessary_wraps,
    clippy::struct_field_names,
    clippy::redundant_else,
    clippy::if_not_else,
    clippy::range_plus_one,
    clippy::inconsistent_struct_constructor
)]

use crate::error::{ProtoError, Result};
use bytes::{Bytes, BytesMut};
use crc::{Crc, CRC_32_ISO_HDLC};
use crc32c::crc32c;
use indexmap::IndexMap;

use crate::protocol::{
    buf::{gap, ByteBuf, ByteBufMut, TypedGap},
    types, Decoder, Encoder, StrBytes,
};

use super::compression::{self as cmpr, Compressor, Decompressor};
use std::cmp::Ordering;
use std::convert::TryFrom;

/// IEEE (checksum) cyclic redundancy check.
pub const IEEE: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

/// The different types of compression supported by Kafka.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Compression {
    /// No compression.
    None = 0,
    /// gzip compression library.
    Gzip = 1,
    /// Google's Snappy compression library.
    Snappy = 2,
    /// The LZ4 compression library.
    Lz4 = 3,
    /// Facebook's ZStandard compression library.
    Zstd = 4,
}

/// Indicates the meaning of the timestamp field on a record.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TimestampType {
    /// The timestamp represents when the record was created by the client.
    Creation = 0,
    /// The timestamp represents when the record was appended to the log.
    LogAppend = 1,
}

/// Options for encoding and compressing a batch of records. Note, not all compression algorithms
/// are currently implemented by this library.
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
// In this range of versions of the Kafka protocol
// the only valid record batch request version is 2.
// 0-1 are officially deprecated with Kafka 4.0
const CURRENT_VALID_RECORD_BATCH_VERSION: i8 = 2;

#[derive(Debug, Clone)]
/// Batch encoder for Kafka records.
pub struct RecordBatchEncoder;

#[derive(Debug, Clone)]
/// Batch decoder for Kafka records.
pub struct RecordBatchDecoder;

struct BatchBounds {
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

/// Decoded records plus information about compression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordSet {
    /// Compression used for this set of records
    pub compression: Compression,
    /// Version used to encode the set of records
    pub version: i8,
    /// Records decoded in this set
    pub records: Vec<Record>,
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
    pub fn encode<'a, B, I>(buf: &mut B, records: I, options: &RecordEncodeOptions) -> Result<()>
    where
        B: ByteBufMut,
        I: IntoIterator<Item = &'a Record>,
        I::IntoIter: Clone,
    {
        Self::encode_with_custom_compression(
            buf,
            records,
            options,
            None::<fn(&mut BytesMut, &mut B, Compression) -> Result<()>>,
        )
    }

    /// Encode records into given buffer, using provided encoding options that select the encoding
    /// strategy based on version.
    /// # Arguments
    /// * `compressor` - A function that compresses the given batch of records.
    ///
    /// If `None`, the right compression algorithm will automatically be selected and applied.
    pub fn encode_with_custom_compression<'a, B, I, CF>(
        buf: &mut B,
        records: I,
        options: &RecordEncodeOptions,
        compressor: Option<CF>,
    ) -> Result<()>
    where
        B: ByteBufMut,
        I: IntoIterator<Item = &'a Record>,
        I::IntoIter: Clone,
        CF: Fn(&mut BytesMut, &mut B, Compression) -> Result<()>,
    {
        debug_assert!(
            (0..=2).contains(&options.version),
            "record batch version must be 0-2, got {}",
            options.version
        );
        debug_assert!(
            matches!(
                options.compression,
                Compression::None
                    | Compression::Gzip
                    | Compression::Snappy
                    | Compression::Lz4
                    | Compression::Zstd
            ),
            "compression must be a known variant"
        );
        let records = records.into_iter();
        match options.version {
            0..=1 => Err(ProtoError::UnsupportedMessageSetVersion {
                version: options.version,
            }),
            2 => Self::encode_new(buf, records, options, compressor),
            _ => Err(ProtoError::UnknownRecordBatchVersion {
                version: options.version,
            }),
        }
    }

    fn encode_new_records<'a, B, I>(
        buf: &mut B,
        records: I,
        min_offset: i64,
        min_timestamp: i64,
        options: &RecordEncodeOptions,
    ) -> Result<()>
    where
        B: ByteBufMut,
        I: Iterator<Item = &'a Record>,
    {
        for record in records {
            record.encode_new(buf, min_offset, min_timestamp, options)?;
        }
        Ok(())
    }

    /// Compute the number of batch-compatible records and their aggregate
    /// offset/timestamp bounds. Returns `None` when the iterator is empty.
    fn compute_batch_bounds<'a, I>(records: &I) -> Option<(&'a Record, BatchBounds)>
    where
        I: Iterator<Item = &'a Record> + Clone,
    {
        let mut peeker = records.clone();
        let first = peeker.next()?;

        let num_records = peeker
            .take_while(|record| {
                record.transactional == first.transactional
                    && record.control == first.control
                    && record.partition_leader_epoch == first.partition_leader_epoch
                    && record.producer_id == first.producer_id
                    && record.producer_epoch == first.producer_epoch
                    && (record.offset as i32).wrapping_sub(record.sequence)
                        == (first.offset as i32).wrapping_sub(first.sequence)
            })
            .count()
            + 1;

        let (min_offset, max_offset, min_timestamp, max_timestamp) =
            records.clone().take(num_records).fold(
                (i64::MAX, i64::MIN, i64::MAX, i64::MIN),
                |(min_off, max_off, min_ts, max_ts), r| {
                    (
                        min_off.min(r.offset),
                        max_off.max(r.offset),
                        min_ts.min(r.timestamp),
                        max_ts.max(r.timestamp),
                    )
                },
            );

        let base_sequence = first
            .sequence
            .wrapping_sub((first.offset - min_offset) as i32);

        debug_assert!(num_records > 0, "batch must contain at least one record");
        debug_assert!(
            min_offset <= max_offset,
            "min_offset ({min_offset}) must be <= max_offset ({max_offset})"
        );

        Some((
            first,
            BatchBounds {
                num_records,
                min_offset,
                max_offset,
                min_timestamp,
                max_timestamp,
                base_sequence,
            },
        ))
    }

    /// Write the batch header fields (base offset through record count).
    /// Returns `(size_gap, crc_gap, batch_start, content_start)` so the
    /// caller can back-fill size and CRC after writing records.
    fn write_batch_header<B: ByteBufMut>(
        buf: &mut B,
        first_record: &Record,
        bounds: &BatchBounds,
        options: &RecordEncodeOptions,
    ) -> Result<(TypedGap<gap::I32>, TypedGap<gap::U32>, usize, usize)> {
        debug_assert!(
            options.version == 2,
            "only v2 batch headers are supported, got v{}",
            options.version
        );
        let header_start = buf.offset();
        types::Int64.encode(buf, bounds.min_offset)?;

        let size_gap = buf.put_typed_gap(gap::I32);
        let batch_start = buf.offset();

        types::Int32.encode(buf, first_record.partition_leader_epoch)?;
        types::Int8.encode(buf, options.version)?;

        let crc_gap = buf.put_typed_gap(gap::U32);
        let content_start = buf.offset();

        let mut attributes = options.compression as i16;
        if first_record.transactional {
            attributes |= 1 << 4;
        }
        if first_record.control {
            attributes |= 1 << 5;
        }
        types::Int16.encode(buf, attributes)?;

        types::Int32.encode(buf, (bounds.max_offset - bounds.min_offset) as i32)?;
        types::Int64.encode(buf, bounds.min_timestamp)?;
        types::Int64.encode(buf, bounds.max_timestamp)?;
        types::Int64.encode(buf, first_record.producer_id)?;
        types::Int16.encode(buf, first_record.producer_epoch)?;
        types::Int32.encode(buf, bounds.base_sequence)?;

        if bounds.num_records > i32::MAX as usize {
            return Err(ProtoError::TooManyRecords {
                count: bounds.num_records,
            });
        }
        types::Int32.encode(buf, bounds.num_records as i32)?;

        debug_assert!(buf.offset() > header_start, "header must write some bytes");
        Ok((size_gap, crc_gap, batch_start, content_start))
    }

    fn encode_new_batch<'a, B, I, CF>(
        buf: &mut B,
        records: &mut I,
        options: &RecordEncodeOptions,
        compressor: Option<&CF>,
    ) -> Result<bool>
    where
        B: ByteBufMut,
        I: Iterator<Item = &'a Record> + Clone,
        CF: Fn(&mut BytesMut, &mut B, Compression) -> Result<()>,
    {
        debug_assert!(
            options.version == 2,
            "only v2 batches are supported, got v{}",
            options.version
        );
        let Some((first_record, bounds)) = Self::compute_batch_bounds(records) else {
            return Ok(false);
        };

        let (size_gap, crc_gap, batch_start, content_start) =
            Self::write_batch_header(buf, first_record, &bounds, options)?;

        // Compress and write records
        let records = records.take(bounds.num_records);
        let min_offset = bounds.min_offset;
        let min_timestamp = bounds.min_timestamp;

        if let Some(compressor) = compressor {
            let mut record_buf = BytesMut::new();
            Self::encode_new_records(&mut record_buf, records, min_offset, min_timestamp, options)?;
            compressor(&mut record_buf, buf, options.compression)?;
        } else {
            match options.compression {
                Compression::None => cmpr::None::compress(buf, |buf| {
                    Self::encode_new_records(buf, records, min_offset, min_timestamp, options)
                })?,
                #[cfg(feature = "snappy")]
                Compression::Snappy => cmpr::Snappy::compress(buf, |buf| {
                    Self::encode_new_records(buf, records, min_offset, min_timestamp, options)
                })?,
                #[cfg(feature = "gzip")]
                Compression::Gzip => cmpr::Gzip::compress(buf, |buf| {
                    Self::encode_new_records(buf, records, min_offset, min_timestamp, options)
                })?,
                #[cfg(feature = "lz4")]
                Compression::Lz4 => cmpr::Lz4::compress(buf, |buf| {
                    Self::encode_new_records(buf, records, min_offset, min_timestamp, options)
                })?,
                #[cfg(feature = "zstd")]
                Compression::Zstd => cmpr::Zstd::compress(buf, |buf| {
                    Self::encode_new_records(buf, records, min_offset, min_timestamp, options)
                })?,
                #[allow(unreachable_patterns)]
                c => {
                    let name = match c {
                        Compression::None => "none",
                        Compression::Gzip => "gzip",
                        Compression::Snappy => "snappy",
                        Compression::Lz4 => "lz4",
                        Compression::Zstd => "zstd",
                    };
                    return Err(ProtoError::CompressionNotEnabled { algorithm: name });
                }
            }
        }

        // Back-fill size and CRC gaps
        let batch_end = buf.offset();
        debug_assert!(
            batch_end > batch_start,
            "batch must contain data (end={batch_end}, start={batch_start})"
        );
        let batch_size = batch_end - batch_start;
        if batch_size > i32::MAX as usize {
            return Err(ProtoError::BatchTooLarge { size: batch_size });
        }
        buf.fill_typed_gap(size_gap, batch_size as i32);

        let crc = crc32c(buf.range(content_start..batch_end));
        buf.fill_typed_gap(crc_gap, crc);

        Ok(true)
    }

    fn encode_new<'a, B, I, CF>(
        buf: &mut B,
        mut records: I,
        options: &RecordEncodeOptions,
        compressor: Option<CF>,
    ) -> Result<()>
    where
        B: ByteBufMut,
        I: Iterator<Item = &'a Record> + Clone,
        CF: Fn(&mut BytesMut, &mut B, Compression) -> Result<()>,
    {
        while Self::encode_new_batch(buf, &mut records, options, compressor.as_ref())? {}
        Ok(())
    }
}

/// An iterator over records in one or more Kafka record batches.
///
/// This iterator provides a way to sequentially access individual records
/// across all batches in the buffer. When records in the current batch are
/// exhausted, it automatically advances to the next batch if more data is
/// available, mirroring the behavior of [`RecordBatchDecoder::decode_all`].
pub struct RecordIterator {
    buf: Bytes,
    source: Bytes,
    batch_decode_info: BatchDecodeInfo,
    current: i64,
}

impl Iterator for RecordIterator {
    type Item = Result<Record>;

    fn next(&mut self) -> Option<Result<Record>> {
        if self.current >= self.batch_decode_info.record_count as i64 {
            // Current batch exhausted — try loading the next batch
            if !self.source.is_empty() {
                match self.try_load_next_batch() {
                    Ok(()) => {}
                    Err(e) => return Some(Err(e)),
                }
            } else {
                return None;
            }
        }
        self.current += 1;
        Some(Record::decode_new(
            &mut self.buf,
            &self.batch_decode_info,
            CURRENT_VALID_RECORD_BATCH_VERSION,
        ))
    }
}

impl RecordIterator {
    fn new(buf: &mut Bytes) -> Result<Self> {
        debug_assert!(
            !buf.is_empty(),
            "buffer must have remaining bytes to create a RecordIterator"
        );
        // try_peek_bytes validates the range; indexing is guaranteed safe.
        let version = buf.try_peek_bytes(MAGIC_BYTE_OFFSET..(MAGIC_BYTE_OFFSET + 1))?[0] as i8;
        let (batch_decode_info, record_buf) = RecordBatchDecoder::decode_batch_info(buf, version)?;

        debug_assert!(
            batch_decode_info.record_count > 0 || record_buf.is_empty(),
            "zero-record batch should have empty record buffer"
        );
        Ok(RecordIterator {
            buf: record_buf,
            source: buf.clone(),
            batch_decode_info,
            current: 0,
        })
    }

    fn try_load_next_batch(&mut self) -> Result<()> {
        debug_assert!(
            !self.source.is_empty(),
            "source must have remaining bytes to load next batch"
        );
        // try_peek_bytes validates the range; indexing is guaranteed safe.
        let version =
            self.source
                .try_peek_bytes(MAGIC_BYTE_OFFSET..(MAGIC_BYTE_OFFSET + 1))?[0] as i8;
        let (batch_decode_info, record_buf) =
            RecordBatchDecoder::decode_batch_info(&mut self.source, version)?;
        self.buf = record_buf;
        self.batch_decode_info = batch_decode_info;
        self.current = 0;
        debug_assert!(
            self.current == 0,
            "current must be reset to 0 after loading a new batch"
        );
        Ok(())
    }
}

impl RecordBatchDecoder {
    /// Decode the provided buffer into an iterator over records.
    ///
    /// The returned iterator yields `Result<Record>` items and automatically
    /// advances across multiple batches if the buffer contains more than one.
    ///
    /// # Arguments
    /// * `buf` - The buffer to decode.
    pub fn records(buf: &mut Bytes) -> Result<RecordIterator> {
        RecordIterator::new(buf)
    }

    /// Decode the provided buffer into a vec of records.
    /// # Arguments
    /// * `decompressor` - A function that decompresses the given batch of records.
    ///
    /// If `None`, the right decompression algorithm will automatically be selected and applied.
    pub fn decode_with_custom_compression<B: ByteBuf, F>(
        buf: &mut B,
        decompressor: Option<F>,
    ) -> Result<RecordSet>
    where
        F: Fn(&mut bytes::Bytes, Compression) -> Result<B>,
    {
        let mut records = Vec::new();
        let (version, compression) =
            Self::decode_into_vec(buf, &mut records, decompressor.as_ref())?;
        Ok(RecordSet {
            version,
            compression,
            records,
        })
    }

    /// Decode the entire buffer into a vec of RecordSets.
    pub fn decode_all<B: ByteBuf>(buf: &mut B) -> Result<Vec<RecordSet>> {
        let mut batches = Vec::new();
        while buf.has_remaining() {
            batches.push(Self::decode(buf)?);
        }
        Ok(batches)
    }

    /// Decode one RecordSet from the provided buffer.
    pub fn decode<B: ByteBuf>(buf: &mut B) -> Result<RecordSet> {
        Self::decode_with_custom_compression(
            buf,
            None::<fn(&mut bytes::Bytes, Compression) -> Result<B>>.as_ref(),
        )
    }

    fn decode_into_vec<B: ByteBuf, F>(
        buf: &mut B,
        records: &mut Vec<Record>,
        decompress_func: Option<&F>,
    ) -> Result<(i8, Compression)>
    where
        F: Fn(&mut bytes::Bytes, Compression) -> Result<B>,
    {
        // try_peek_bytes validates the range; indexing is guaranteed safe.
        let version = buf.try_peek_bytes(MAGIC_BYTE_OFFSET..(MAGIC_BYTE_OFFSET + 1))?[0] as i8;
        let compression = match version {
            0..=1 => Err(ProtoError::UnsupportedMessageSetVersion { version }),
            2 => Self::decode_new_batch(buf, version, records, decompress_func),
            _ => Err(ProtoError::UnknownRecordBatchVersion { version }),
        }?;
        Ok((version, compression))
    }

    fn decode_batch_info<B: ByteBuf>(buf: &mut B, version: i8) -> Result<(BatchDecodeInfo, Bytes)> {
        debug_assert!(
            version == 2,
            "only v2 batch decoding is supported, got v{version}"
        );
        // Base offset
        let min_offset = types::Int64.decode(buf)?;

        // Batch length
        let batch_length: i32 = types::Int32.decode(buf)?;
        if batch_length < 0 {
            return Err(ProtoError::NegativeLength {
                field: "batch size",
                value: batch_length as i64,
            });
        }

        // Convert buf to bytes
        let buf = &mut buf.try_get_bytes(batch_length as usize)?;

        // Partition leader epoch
        let partition_leader_epoch = types::Int32.decode(buf)?;

        // Magic byte
        let magic: i8 = types::Int8.decode(buf)?;
        if magic != version {
            return Err(ProtoError::VersionMismatch {
                expected: version,
                actual: magic,
            });
        }

        // CRC
        let supplied_crc: u32 = types::UInt32.decode(buf)?;
        let actual_crc = crc32c(buf);

        if supplied_crc != actual_crc {
            return Err(ProtoError::CrcMismatch {
                expected: actual_crc,
                actual: supplied_crc,
            });
        }

        // Attributes
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
                return Err(ProtoError::UnknownCompression { algorithm: other });
            }
        };
        let timestamp_type = if (attributes & (1 << 3)) != 0 {
            TimestampType::LogAppend
        } else {
            TimestampType::Creation
        };

        // Last offset delta
        let _max_offset_delta: i32 = types::Int32.decode(buf)?;

        // First timestamp
        let min_timestamp = types::Int64.decode(buf)?;

        // Last timestamp
        let _max_timestamp: i64 = types::Int64.decode(buf)?;

        // Producer ID
        let producer_id = types::Int64.decode(buf)?;

        // Producer epoch
        let producer_epoch = types::Int16.decode(buf)?;

        // Base sequence
        let base_sequence = types::Int32.decode(buf)?;

        // Record count
        let record_count: i32 = types::Int32.decode(buf)?;
        if record_count < 0 {
            return Err(ProtoError::NegativeLength {
                field: "record count",
                value: record_count as i64,
            });
        }
        let record_count = record_count as usize;
        debug_assert!(
            i32::try_from(record_count).is_ok(),
            "record_count ({record_count}) must fit in i32"
        );

        Ok((
            BatchDecodeInfo {
                record_count,
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
            },
            buf.to_owned(),
        ))
    }

    fn decode_new_records<B: ByteBuf>(
        buf: &mut B,
        batch_decode_info: &BatchDecodeInfo,
        version: i8,
        records: &mut Vec<Record>,
    ) -> Result<()> {
        records.reserve(batch_decode_info.record_count);
        for _ in 0..batch_decode_info.record_count {
            records.push(Record::decode_new(buf, batch_decode_info, version)?);
        }
        Ok(())
    }

    fn decode_new_batch<B: ByteBuf, F>(
        buf: &mut B,
        version: i8,
        records: &mut Vec<Record>,
        decompress_func: Option<&F>,
    ) -> Result<Compression>
    where
        F: Fn(&mut bytes::Bytes, Compression) -> Result<B>,
    {
        debug_assert!(
            version == 2,
            "only v2 batch decoding is supported, got v{version}"
        );
        let records_before = records.len();
        let (batch_decode_info, mut buf) = Self::decode_batch_info(buf, version)?;
        let compression = batch_decode_info.compression;

        if let Some(decompress_func) = decompress_func {
            let mut decompressed_buf = decompress_func(&mut buf, compression)?;

            Self::decode_new_records(&mut decompressed_buf, &batch_decode_info, version, records)?;
        } else {
            match compression {
                Compression::None => cmpr::None::decompress(&mut buf, |buf| {
                    Self::decode_new_records(buf, &batch_decode_info, version, records)
                })?,
                #[cfg(feature = "snappy")]
                Compression::Snappy => cmpr::Snappy::decompress(&mut buf, |buf| {
                    Self::decode_new_records(buf, &batch_decode_info, version, records)
                })?,
                #[cfg(feature = "gzip")]
                Compression::Gzip => cmpr::Gzip::decompress(&mut buf, |buf| {
                    Self::decode_new_records(buf, &batch_decode_info, version, records)
                })?,
                #[cfg(feature = "zstd")]
                Compression::Zstd => cmpr::Zstd::decompress(&mut buf, |buf| {
                    Self::decode_new_records(buf, &batch_decode_info, version, records)
                })?,
                #[cfg(feature = "lz4")]
                Compression::Lz4 => cmpr::Lz4::decompress(&mut buf, |buf| {
                    Self::decode_new_records(buf, &batch_decode_info, version, records)
                })?,
                #[allow(unreachable_patterns)]
                c => {
                    let name = match c {
                        Compression::None => "none",
                        Compression::Gzip => "gzip",
                        Compression::Snappy => "snappy",
                        Compression::Lz4 => "lz4",
                        Compression::Zstd => "zstd",
                    };
                    return Err(ProtoError::CompressionNotEnabled { algorithm: name });
                }
            }
        }

        debug_assert!(
            records.len() >= records_before,
            "decoding must not shrink the records vec"
        );
        Ok(compression)
    }
}

/// Encode an optional bytes field with a VarInt length prefix.
/// Writes the length (or -1 for `None`) followed by the raw bytes.
fn encode_optional_bytes<B: ByteBufMut>(
    buf: &mut B,
    data: Option<&[u8]>,
    field_name: &'static str,
) -> Result<()> {
    debug_assert!(!field_name.is_empty(), "field_name must not be empty");
    debug_assert!(
        data.as_ref().is_none_or(|d| i32::try_from(d.len()).is_ok()),
        "data length must fit in i32 for field '{field_name}'"
    );
    if let Some(d) = data {
        if d.len() > i32::MAX as usize {
            return Err(ProtoError::FieldTooLarge {
                field: field_name,
                size: d.len(),
            });
        }
        types::VarInt.encode(buf, d.len() as i32)?;
        buf.put_slice(d);
    } else {
        types::VarInt.encode(buf, -1)?;
    }
    Ok(())
}

/// Encode the headers map: count followed by key-value pairs.
fn encode_headers<B: ByteBufMut>(
    buf: &mut B,
    headers: &IndexMap<StrBytes, Option<Bytes>>,
) -> Result<()> {
    debug_assert!(
        i32::try_from(headers.len()).is_ok(),
        "headers count ({}) must fit in i32",
        headers.len()
    );
    debug_assert!(
        headers.keys().all(|k| !k.is_empty()),
        "all header keys should be non-empty"
    );
    if headers.len() > i32::MAX as usize {
        return Err(ProtoError::FieldTooLarge {
            field: "record headers count",
            size: headers.len(),
        });
    }
    types::VarInt.encode(buf, headers.len() as i32)?;
    for (k, v) in headers {
        encode_optional_bytes(buf, Some(k.as_ref()), "record header key")?;
        encode_optional_bytes(buf, v.as_deref(), "record header value")?;
    }
    Ok(())
}

/// Decode an optional bytes field with a VarInt length prefix.
fn decode_optional_bytes<B: ByteBuf>(
    buf: &mut B,
    field_name: &'static str,
) -> Result<Option<Bytes>> {
    debug_assert!(!field_name.is_empty(), "field_name must not be empty");
    debug_assert!(
        buf.has_remaining(),
        "buffer must have remaining bytes for field '{field_name}'"
    );
    let len: i32 = types::VarInt.decode(buf)?;
    match len.cmp(&-1) {
        Ordering::Less => Err(ProtoError::NegativeLength {
            field: field_name,
            value: len as i64,
        }),
        Ordering::Equal => Ok(None),
        Ordering::Greater => Ok(Some(buf.try_get_bytes(len as usize)?)),
    }
}

/// Decode the headers map from a buffer.
fn decode_headers<B: ByteBuf>(buf: &mut B) -> Result<IndexMap<StrBytes, Option<Bytes>>> {
    debug_assert!(
        buf.has_remaining(),
        "buffer must have remaining bytes to decode headers"
    );
    let num_headers: i32 = types::VarInt.decode(buf)?;
    if num_headers < 0 {
        return Err(ProtoError::NegativeLength {
            field: "record header count",
            value: num_headers as i64,
        });
    }
    let num_headers = num_headers as usize;
    let mut headers = IndexMap::with_capacity(num_headers);
    for _ in 0..num_headers {
        let key_len: i32 = types::VarInt.decode(buf)?;
        if key_len < 0 {
            return Err(ProtoError::NegativeLength {
                field: "record header key length",
                value: key_len as i64,
            });
        }
        let key = StrBytes::try_from(buf.try_get_bytes(key_len as usize)?)?;
        let value = decode_optional_bytes(buf, "record header value length")?;
        headers.insert(key, value);
    }
    debug_assert!(
        headers.len() == num_headers,
        "decoded header count ({}) must match declared count ({num_headers})",
        headers.len()
    );
    Ok(headers)
}

/// Compute the encoded size of an optional bytes field.
fn compute_optional_bytes_size(data: Option<&[u8]>, field_name: &'static str) -> Result<usize> {
    debug_assert!(!field_name.is_empty(), "field_name must not be empty");
    debug_assert!(
        data.as_ref().is_none_or(|d| i32::try_from(d.len()).is_ok()),
        "data length must fit in i32 for field '{field_name}'"
    );
    if let Some(d) = data {
        if d.len() > i32::MAX as usize {
            return Err(ProtoError::FieldTooLarge {
                field: field_name,
                size: d.len(),
            });
        }
        Ok(types::VarInt.compute_size(d.len() as i32)? + d.len())
    } else {
        types::VarInt.compute_size(-1)
    }
}

/// Compute the encoded size of the headers map.
fn compute_headers_size(headers: &IndexMap<StrBytes, Option<Bytes>>) -> Result<usize> {
    debug_assert!(
        i32::try_from(headers.len()).is_ok(),
        "headers count ({}) must fit in i32",
        headers.len()
    );
    debug_assert!(
        headers.keys().all(|k| !k.is_empty()),
        "all header keys should be non-empty"
    );
    if headers.len() > i32::MAX as usize {
        return Err(ProtoError::FieldTooLarge {
            field: "record headers count",
            size: headers.len(),
        });
    }
    let mut size = types::VarInt.compute_size(headers.len() as i32)?;
    for (k, v) in headers {
        size += compute_optional_bytes_size(Some(k.as_ref()), "record header key")?;
        size += compute_optional_bytes_size(v.as_deref(), "record header value")?;
    }
    Ok(size)
}

impl Record {
    fn encode_new<B: ByteBufMut>(
        &self,
        buf: &mut B,
        min_offset: i64,
        min_timestamp: i64,
        options: &RecordEncodeOptions,
    ) -> Result<()> {
        debug_assert!(
            options.version == 2,
            "only v2 record encoding is supported, got v{}",
            options.version
        );
        debug_assert!(
            self.offset >= min_offset,
            "record offset ({}) must be >= min_offset ({})",
            self.offset,
            min_offset
        );
        let size = self.compute_size_new(min_offset, min_timestamp, options)?;
        if size > i32::MAX as usize {
            return Err(ProtoError::RecordTooLarge { size });
        }
        types::VarInt.encode(buf, size as i32)?;

        // Attributes
        types::Int8.encode(buf, 0)?;

        // Timestamp delta
        let timestamp_delta = self.timestamp - min_timestamp;
        if timestamp_delta > i32::MAX as i64 || timestamp_delta < i32::MIN as i64 {
            return Err(ProtoError::TimestampDeltaOverflow {
                min: min_timestamp,
                max: self.timestamp,
            });
        }
        types::VarInt.encode(buf, timestamp_delta as i32)?;

        // Offset delta
        let offset_delta = self.offset - min_offset;
        if offset_delta > i32::MAX as i64 || offset_delta < i32::MIN as i64 {
            return Err(ProtoError::OffsetDeltaOverflow {
                min: min_offset,
                max: self.offset,
            });
        }
        types::VarInt.encode(buf, offset_delta as i32)?;

        encode_optional_bytes(buf, self.key.as_deref(), "record key")?;
        encode_optional_bytes(buf, self.value.as_deref(), "record value")?;
        encode_headers(buf, &self.headers)?;

        Ok(())
    }
    fn compute_size_new(
        &self,
        min_offset: i64,
        min_timestamp: i64,
        _options: &RecordEncodeOptions,
    ) -> Result<usize> {
        let mut total_size = 0;

        // Attributes
        total_size += types::Int8.compute_size(0)?;

        // Timestamp delta
        let timestamp_delta = self.timestamp - min_timestamp;
        if timestamp_delta > i32::MAX as i64 || timestamp_delta < i32::MIN as i64 {
            return Err(ProtoError::TimestampDeltaOverflow {
                min: min_timestamp,
                max: self.timestamp,
            });
        }
        total_size += types::VarInt.compute_size(timestamp_delta as i32)?;

        // Offset delta
        let offset_delta = self.offset - min_offset;
        if offset_delta > i32::MAX as i64 || offset_delta < i32::MIN as i64 {
            return Err(ProtoError::OffsetDeltaOverflow {
                min: min_offset,
                max: self.offset,
            });
        }
        total_size += types::VarInt.compute_size(offset_delta as i32)?;

        total_size += compute_optional_bytes_size(self.key.as_deref(), "record key")?;
        total_size += compute_optional_bytes_size(self.value.as_deref(), "record value")?;
        total_size += compute_headers_size(&self.headers)?;

        Ok(total_size)
    }
    fn decode_new<B: ByteBuf>(
        buf: &mut B,
        batch_decode_info: &BatchDecodeInfo,
        _version: i8,
    ) -> Result<Self> {
        let size: i32 = types::VarInt.decode(buf)?;
        if size < 0 {
            return Err(ProtoError::NegativeLength {
                field: "record size",
                value: size as i64,
            });
        }

        let buf = &mut buf.try_get_bytes(size as usize)?;

        let _attributes: i8 = types::Int8.decode(buf)?;

        let timestamp_delta: i32 = types::VarInt.decode(buf)?;
        let timestamp = batch_decode_info.min_timestamp + timestamp_delta as i64;

        let offset_delta: i32 = types::VarInt.decode(buf)?;
        let offset = batch_decode_info.min_offset + offset_delta as i64;
        let sequence = batch_decode_info.base_sequence.wrapping_add(offset_delta);

        let key = decode_optional_bytes(buf, "record key length")?;
        let value = decode_optional_bytes(buf, "record value length")?;
        let headers = decode_headers(buf)?;

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

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;

    #[test]
    fn lookup_header_via_u8_slice() {
        let record = Record {
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: 0,
            producer_epoch: 0,
            sequence: 0,
            timestamp_type: TimestampType::Creation,
            offset: Default::default(),
            timestamp: Default::default(),
            key: Option::default(),
            value: Option::default(),
            headers: [
                ("some-key".into(), Some("some-value".into())),
                ("other-header".into(), None),
            ]
            .into(),
        };
        assert_eq!(
            Bytes::from("some-value"),
            record
                .headers
                // This relies on `impl Borrow<[u8]> for StrBytes`
                .get("some-key".as_bytes())
                .expect("key exists in headers")
                .as_ref()
                .expect("value is present")
        );
    }

    #[test]
    fn decode_record_header_no_value() {
        let record = Record {
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: 0,
            producer_epoch: 0,
            sequence: 0,
            timestamp_type: TimestampType::Creation,
            offset: Default::default(),
            timestamp: Default::default(),
            key: Option::default(),
            value: Option::default(),
            headers: [("other-header".into(), None)].into(),
        };
        let mut buf = &mut bytes::BytesMut::new();
        record
            .encode_new(
                buf,
                0,
                0,
                &RecordEncodeOptions {
                    version: 2,
                    compression: super::Compression::None,
                },
            )
            .expect("encode works");

        Record::decode_new(
            &mut buf,
            &BatchDecodeInfo {
                record_count: 1,
                timestamp_type: TimestampType::Creation,
                min_offset: 0,
                min_timestamp: 0,
                base_sequence: 0,
                transactional: false,
                control: false,
                partition_leader_epoch: 0,
                producer_id: 0,
                producer_epoch: 0,
                compression: Compression::None,
            },
            2,
        )
        .expect("decode works");
    }

    #[test]
    fn test_record_iterator() {
        let records_to_encode = vec![
            Record {
                transactional: false,
                control: false,
                partition_leader_epoch: 0,
                producer_id: 0,
                producer_epoch: 0,
                sequence: 0,
                timestamp_type: TimestampType::Creation,
                offset: 0,
                timestamp: 0,
                key: Some(Bytes::from("key1")),
                value: Some(Bytes::from("value1")),
                headers: IndexMap::new(),
            },
            Record {
                transactional: false,
                control: false,
                partition_leader_epoch: 0,
                producer_id: 0,
                producer_epoch: 0,
                sequence: 1,
                timestamp_type: TimestampType::Creation,
                offset: 1,
                timestamp: 1,
                key: Some(Bytes::from("key2")),
                value: Some(Bytes::from("value2")),
                headers: IndexMap::new(),
            },
        ];

        let mut buf = BytesMut::new();
        RecordBatchEncoder::encode(
            &mut buf,
            &records_to_encode,
            &RecordEncodeOptions {
                version: 2,
                compression: Compression::None,
            },
        )
        .unwrap();

        let mut iterator = RecordBatchDecoder::records(&mut buf.freeze()).unwrap();
        let decoded_records: Vec<Record> = iterator.by_ref().map(|r| r.unwrap()).collect();

        assert_eq!(records_to_encode, decoded_records);
    }

    #[test]
    fn test_record_iterator_invalid_record() {
        // Create a minimal batch with valid header but invalid record data
        // We'll create the BatchDecodeInfo manually and test Record::decode_new directly
        let batch_decode_info = BatchDecodeInfo {
            record_count: 1,
            timestamp_type: TimestampType::Creation,
            min_offset: 0,
            min_timestamp: 0,
            base_sequence: 0,
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: 0,
            producer_epoch: 0,
            compression: Compression::None,
        };

        // Create invalid record data - this will be missing required fields
        // causing Record::decode_new to fail when it tries to read beyond the buffer
        let invalid_record_data = Bytes::from(vec![0x01, 0x02]); // Too short to be a valid record

        // Create RecordIterator directly with invalid data
        let mut iterator = RecordIterator {
            buf: invalid_record_data,
            source: Bytes::new(),
            batch_decode_info,
            current: 0,
        };

        // The first call to next() should return Some(Err(...)) due to invalid record data
        match iterator.next() {
            Some(Err(_)) => {
                // This is what we expect - an error when trying to decode invalid record data
            }
            Some(Ok(_)) => panic!("Expected error when decoding invalid record, but got success"),
            None => panic!("Expected error when decoding invalid record, but got None"),
        }

        // After the first record fails, there are no more records to decode
        assert!(iterator.next().is_none());
    }

    #[test]
    fn test_record_iterator_error_handling_usage() {
        // This test demonstrates how to properly handle errors when using the iterator
        // in practical scenarios where you want to collect both successful and failed records

        let batch_decode_info = BatchDecodeInfo {
            record_count: 2, // Tell it there are 2 records
            timestamp_type: TimestampType::Creation,
            min_offset: 0,
            min_timestamp: 0,
            base_sequence: 0,
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: 0,
            producer_epoch: 0,
            compression: Compression::None,
        };

        // Create invalid record data that will cause parsing errors
        let invalid_data = Bytes::from(vec![0x01, 0x02, 0x03]); // Too short to be valid records

        let mut iterator = RecordIterator {
            buf: invalid_data,
            source: Bytes::new(),
            batch_decode_info,
            current: 0,
        };

        // Demonstrate proper error handling patterns
        let mut successful_records = Vec::new();
        let mut error_count = 0;

        // Pattern 1: Handle each result individually
        for result in &mut iterator {
            match result {
                Ok(record) => successful_records.push(record),
                Err(_err) => {
                    error_count += 1;
                    // In practice, you might log the error or handle it specifically
                    // println!("Failed to parse record: {}", _err);
                }
            }
        }

        // Verify we got the expected error handling behavior
        assert_eq!(successful_records.len(), 0); // No valid records in this test
        assert!(error_count > 0); // At least one error occurred

        // Pattern 2: Collect only successful records (filter out errors)
        let batch_decode_info2 = BatchDecodeInfo {
            record_count: 1,
            timestamp_type: TimestampType::Creation,
            min_offset: 0,
            min_timestamp: 0,
            base_sequence: 0,
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: 0,
            producer_epoch: 0,
            compression: Compression::None,
        };

        let iterator2 = RecordIterator {
            buf: Bytes::from(vec![0xFF]),
            source: Bytes::new(),
            batch_decode_info: batch_decode_info2,
            current: 0,
        };

        let only_successful: Vec<Record> = iterator2.filter_map(Result::ok).collect();

        assert_eq!(only_successful.len(), 0); // No successful records with invalid data
    }

    #[test]
    fn test_record_iterator_multiple_batches() {
        let batch1_records = vec![
            Record {
                transactional: false,
                control: false,
                partition_leader_epoch: 0,
                producer_id: 0,
                producer_epoch: 0,
                sequence: 0,
                timestamp_type: TimestampType::Creation,
                offset: 0,
                timestamp: 0,
                key: Some(Bytes::from("batch1_key1")),
                value: Some(Bytes::from("batch1_value1")),
                headers: IndexMap::new(),
            },
            Record {
                transactional: false,
                control: false,
                partition_leader_epoch: 0,
                producer_id: 0,
                producer_epoch: 0,
                sequence: 1,
                timestamp_type: TimestampType::Creation,
                offset: 1,
                timestamp: 1,
                key: Some(Bytes::from("batch1_key2")),
                value: Some(Bytes::from("batch1_value2")),
                headers: IndexMap::new(),
            },
        ];

        let batch2_records = vec![Record {
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: 0,
            producer_epoch: 0,
            sequence: 0,
            timestamp_type: TimestampType::Creation,
            offset: 0,
            timestamp: 0,
            key: Some(Bytes::from("batch2_key1")),
            value: Some(Bytes::from("batch2_value1")),
            headers: IndexMap::new(),
        }];

        let opts = RecordEncodeOptions {
            version: 2,
            compression: Compression::None,
        };

        // Encode two batches into a single buffer
        let mut buf = BytesMut::new();
        RecordBatchEncoder::encode(&mut buf, &batch1_records, &opts).unwrap();
        RecordBatchEncoder::encode(&mut buf, &batch2_records, &opts).unwrap();

        let iterator = RecordBatchDecoder::records(&mut buf.freeze()).unwrap();
        let all_records: Vec<Record> = iterator.map(|r| r.unwrap()).collect();

        assert_eq!(all_records.len(), 3);
        assert_eq!(all_records[0].key, Some(Bytes::from("batch1_key1")));
        assert_eq!(all_records[1].key, Some(Bytes::from("batch1_key2")));
        assert_eq!(all_records[2].key, Some(Bytes::from("batch2_key1")));
    }
}

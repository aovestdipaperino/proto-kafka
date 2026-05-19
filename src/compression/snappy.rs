use crate::error::{ProtoError, Result};
use bytes::{Buf as _, BufMut as _, Bytes, BytesMut};
use snap::raw::{decompress_len, max_compress_len, Decoder, Encoder};

use crate::protocol::buf::{ByteBuf, ByteBufMut};

use super::{Compressor, Decompressor};

/// Kafka variant of the snappy compression algorithm. See
/// <https://github.com/xerial/snappy-java?tab=readme-ov-file#compatibility-notes> for notes about
/// the difference from standard snappy.
/// See [Kafka's broker configuration](https://kafka.apache.org/documentation/#brokerconfigs_compression.type)
/// for more information about compression.
pub struct Snappy;

// See https://github.com/xerial/snappy-java/blob/98e66a1d02d022d333c20ffa6574a72e9fcfb165/src/main/java/org/xerial/snappy/SnappyOutputStream.java#L64
const DEFAULT_BLOCK_SIZE: usize = 32 * 1024;

const LENGTH_FIELD_LENGTH: usize = std::mem::size_of::<u32>();

impl<B: ByteBufMut> Compressor<B> for Snappy {
    type BufMut = BytesMut;
    fn compress<R, F>(compressed: &mut B, f: F) -> Result<R>
    where
        F: FnOnce(&mut Self::BufMut) -> Result<R>,
    {
        let offset_before = compressed.offset();
        let mut uncompressed = BytesMut::new();
        let res = f(&mut uncompressed)?;
        // See https://github.com/xerial/snappy-java?tab=readme-ov-file#compatibility-notes
        compressed.put_slice(MAGIC_HEADER);

        while uncompressed.has_remaining() {
            let uncompressed_block_size =
                std::cmp::min(uncompressed.remaining(), DEFAULT_BLOCK_SIZE);
            let estimated_compressed_block_size = max_compress_len(uncompressed_block_size);

            let length_gap = compressed.put_gap(LENGTH_FIELD_LENGTH);

            let block_offset = compressed.offset();
            let compressed_block = compressed.put_gap(estimated_compressed_block_size);

            let bytes_written = Encoder::new()
                .compress(
                    &uncompressed.split_to(uncompressed_block_size),
                    compressed.gap_buf(compressed_block),
                )
                .map_err(|e| ProtoError::Compression {
                    operation: "compress",
                    codec: "snappy",
                    source: Box::new(e),
                })?;
            // Truncate down to the final compressed size
            compressed.seek(block_offset + bytes_written);

            let mut num_written_buf = compressed.gap_buf(length_gap);
            #[allow(clippy::cast_possible_truncation)]
            num_written_buf.put_u32(bytes_written as u32);
        }
        debug_assert!(
            compressed.offset() >= offset_before + MAGIC_HEADER.len(),
            "compressed output must contain at least the magic header"
        );
        debug_assert!(
            compressed.offset() > offset_before,
            "compress must write some output bytes"
        );
        Ok(res)
    }
}

const MAGIC_HEADER: &[u8; 16] = b"\x82SNAPPY\x00\x00\x00\x00\x01\x00\x00\x00\x01";

fn snappy_err<E>(e: E) -> ProtoError
where
    E: std::error::Error + Send + Sync + 'static,
{
    ProtoError::Compression {
        operation: "decompress",
        codec: "snappy",
        source: Box::new(e),
    }
}

fn try_strip_kafka_magic_header<B: ByteBuf>(compressed: &mut B) -> bool {
    // Peek without consuming. If the magic doesn't match, the raw fallback
    // path needs the original buffer intact (regression test:
    // `decompression_fallback_large_payload`).
    let is_kafka_magic = compressed
        .try_peek_bytes(0..MAGIC_HEADER.len())
        .ok()
        .is_some_and(|magic| *magic == MAGIC_HEADER[..]);
    if is_kafka_magic {
        let _ = compressed.try_get_bytes(MAGIC_HEADER.len());
    }
    is_kafka_magic
}

fn decompress_raw_fallback<B, R, F>(compressed: &mut B, f: F) -> Result<R>
where
    B: ByteBuf,
    F: FnOnce(&mut Bytes) -> Result<R>,
{
    let compressed = compressed.copy_to_bytes(compressed.remaining());
    let actual_len = decompress_len(&compressed).map_err(snappy_err)?;
    let mut tmp = BytesMut::zeroed(actual_len);
    Decoder::new()
        .decompress(&compressed, &mut tmp)
        .map_err(snappy_err)?;
    f(&mut tmp.into())
}

fn decompress_kafka_chunk<B: ByteBuf>(
    compressed: &mut B,
    uncompressed: &mut BytesMut,
) -> Result<()> {
    let compressed_block_size = compressed.try_get_u32().map_err(snappy_err)? as usize;
    let compressed_block = compressed
        .try_get_bytes(compressed_block_size)
        .map_err(snappy_err)?;
    let uncompressed_block_length = decompress_len(&compressed_block).map_err(snappy_err)?;
    let start = uncompressed.len();
    uncompressed.resize(start.saturating_add(uncompressed_block_length), 0);
    Decoder::new()
        .decompress(&compressed_block, &mut uncompressed[start..])
        .map_err(snappy_err)?;
    Ok(())
}

impl<B: ByteBuf> Decompressor<B> for Snappy {
    type Buf = Bytes;
    fn decompress<R, F>(compressed: &mut B, f: F) -> Result<R>
    where
        F: FnOnce(&mut Self::Buf) -> Result<R>,
    {
        debug_assert!(
            compressed.has_remaining(),
            "compressed input must not be empty"
        );
        if !compressed.has_remaining() {
            return Err(snappy_err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "expected some bytes in snappy stream",
            )));
        }

        // Fall back to non-Kafka "raw" snappy if the magic header is missing, mirroring the
        // Java implementation: https://github.com/xerial/snappy-java/blob/48b31663c8b9d01d758368a26416ef6194045c5f/src/main/java/org/xerial/snappy/SnappyInputStream.java#L114
        if !try_strip_kafka_magic_header(compressed) {
            return decompress_raw_fallback(compressed, f);
        }

        let mut uncompressed = BytesMut::new();
        while compressed.has_remaining() {
            decompress_kafka_chunk(compressed, &mut uncompressed)?;
        }

        debug_assert!(
            !uncompressed.is_empty(),
            "decompressed output must not be empty for non-empty input"
        );
        f(&mut uncompressed.into())
    }
}

#[cfg(test)]
mod tests {
    use bytes::{Buf as _, Bytes, BytesMut};
    use indexmap::IndexMap;

    use crate::records::{
        Compression, Record, RecordBatchEncoder, RecordEncodeOptions, TimestampType,
    };

    use super::super::{Compressor as _, Decompressor as _};
    use super::Snappy;

    #[test]
    fn compression() {
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
            key: None,
            value: Some(Bytes::from_static(b"sdfdsf")),
            headers: IndexMap::default(),
        };

        // The module doesn't expose record encode/decode directly so we have to put everything
        // into a batch to compare the bytes
        let mut raw_bytes = BytesMut::new();
        RecordBatchEncoder::encode(
            &mut raw_bytes,
            vec![&record],
            &RecordEncodeOptions {
                version: 2,
                compression: Compression::None,
            },
        )
        .expect("should encode");

        // Chop off all the record batch bytes before the records
        raw_bytes.advance(61);

        let mut compressed = BytesMut::new();
        Snappy::compress(&mut compressed, |uncompressed| {
            std::mem::swap(uncompressed, &mut raw_bytes);
            Ok(())
        })
        .expect("should compress");

        // Some upstream snappy-compressed record batch bytes
        let expected_bytes = Bytes::from_static(
            b"\x82\x53\x4e\x41\x50\x50\x59\x00\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\x00\x0f\x0d\x30\x18\x00\x00\x00\x01\x0csdfdsf\x00",
        );

        assert_eq!(expected_bytes, compressed);
    }

    #[test]
    fn decompression() {
        // Some upstream kafka snappy-compressed record batch bytes
        let mut raw_bytes = Bytes::from_static(
            b"\x82\x53\x4e\x41\x50\x50\x59\x00\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\x00\x0f\x0d\x30\x18\x00\x00\x00\x01\x0csdfdsf\x00",
        );
        let decompressed = Snappy::decompress(&mut raw_bytes, |buf| {
            let mut out = Bytes::new();
            std::mem::swap(buf, &mut out);
            Ok(out)
        })
        .expect("valid snappy");

        // The module doesn't expose record encode/decode directly so we have to put everything
        // into a batch to compare the bytes
        let mut expected_bytes = BytesMut::new();
        let expected_record = Record {
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: 0,
            producer_epoch: 0,
            sequence: 0,
            timestamp_type: TimestampType::Creation,
            offset: Default::default(),
            timestamp: Default::default(),
            key: None,
            value: Some(Bytes::from_static(b"sdfdsf")),
            headers: IndexMap::default(),
        };
        RecordBatchEncoder::encode(
            &mut expected_bytes,
            vec![&expected_record],
            &RecordEncodeOptions {
                version: 2,
                compression: Compression::None,
            },
        )
        .expect("should encode");

        let mut expected_bytes = expected_bytes.freeze();
        // Chop off all the record batch bytes before the records
        expected_bytes.advance(61);
        assert_eq!(expected_bytes, decompressed);
    }

    #[test]
    fn decompression_fallback() {
        // Some pure snappy-compressed record batch bytes
        let mut raw_bytes = Bytes::from_static(b"\r0\x18\0\0\0\x01\x0csdfdsf\0");
        let decompressed = Snappy::decompress(&mut raw_bytes, |buf| {
            let mut out = Bytes::new();
            std::mem::swap(buf, &mut out);
            Ok(out)
        })
        .expect("valid snappy");

        // The module doesn't expose record encode/decode directly so we have to put everything
        // into a batch to compare the bytes
        let mut expected_bytes = BytesMut::new();
        let expected_record = Record {
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: 0,
            producer_epoch: 0,
            sequence: 0,
            timestamp_type: TimestampType::Creation,
            offset: Default::default(),
            timestamp: Default::default(),
            key: None,
            value: Some(Bytes::from_static(b"sdfdsf")),
            headers: IndexMap::default(),
        };
        RecordBatchEncoder::encode(
            &mut expected_bytes,
            vec![&expected_record],
            &RecordEncodeOptions {
                version: 2,
                compression: Compression::None,
            },
        )
        .expect("should encode");

        let mut expected_bytes = expected_bytes.freeze();
        // Chop off all the record batch bytes before the records
        expected_bytes.advance(61);
        assert_eq!(expected_bytes, decompressed);
    }

    /// Regression test: raw snappy data ≥16 bytes.
    ///
    /// Before the fix, `try_get_bytes(16)` consumed the first 16 bytes
    /// when checking for the Xerial magic header. If the magic didn't
    /// match (raw snappy), the fallback got truncated data and failed
    /// with "failed to decompress raw snappy bytes".
    ///
    /// The `decompression_fallback` test above uses only 13 bytes of
    /// compressed data, so `try_get_bytes(16)` failed (not enough bytes)
    /// and the bug was never triggered.
    #[test]
    fn decompression_fallback_large_payload() {
        // Create a payload large enough that raw-snappy-compressed output is ≥16 bytes.
        let original = b"Hello world! This is a test payload that is long enough to \
            produce more than sixteen bytes of snappy-compressed output, which triggers \
            the regression where try_get_bytes consumed the magic header probe bytes.";

        // Compress with raw snappy (no Xerial framing)
        let compressed_vec = snap::raw::Encoder::new()
            .compress_vec(original)
            .expect("raw snappy compress");
        assert!(
            compressed_vec.len() >= 16,
            "compressed payload must be ≥16 bytes to trigger the regression (got {})",
            compressed_vec.len()
        );

        let mut compressed = Bytes::from(compressed_vec);
        let decompressed = Snappy::decompress(&mut compressed, |buf| {
            let mut out = Bytes::new();
            std::mem::swap(buf, &mut out);
            Ok(out)
        })
        .expect("raw snappy decompression of ≥16 byte payload should succeed");

        assert_eq!(&decompressed[..], &original[..]);
    }
}

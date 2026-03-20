use std::io::Write;

use crate::error::{ProtoError, Result};
use bytes::buf::BufMut;
use bytes::{Bytes, BytesMut};
use flate2::write::{GzDecoder, GzEncoder};
use flate2::Compression;

use crate::protocol::buf::{ByteBuf, ByteBufMut};

use super::{Compressor, Decompressor};

/// Gzip compression algorithm. See [Kafka's broker configuration](https://kafka.apache.org/documentation/#brokerconfigs_compression.type)
/// for more information.
pub struct Gzip;

impl<B: ByteBufMut> Compressor<B> for Gzip {
    type BufMut = BytesMut;
    fn compress<R, F>(buf: &mut B, f: F) -> Result<R>
    where
        F: FnOnce(&mut Self::BufMut) -> Result<R>,
    {
        // Write uncompressed bytes into a temporary buffer
        let mut tmp = BytesMut::new();
        let res = f(&mut tmp)?;

        // Compress directly into the target buffer
        let mut e = GzEncoder::new(buf.writer(), Compression::default());
        e.write_all(&tmp).map_err(|e| ProtoError::Compression {
            operation: "compress",
            codec: "gzip",
            source: Box::new(e),
        })?;
        e.finish().map_err(|e| ProtoError::Compression {
            operation: "compress",
            codec: "gzip",
            source: Box::new(e),
        })?;

        Ok(res)
    }
}

impl<B: ByteBuf> Decompressor<B> for Gzip {
    type Buf = Bytes;
    fn decompress<R, F>(buf: &mut B, f: F) -> Result<R>
    where
        F: FnOnce(&mut Self::Buf) -> Result<R>,
    {
        let mut tmp = BytesMut::new();

        // Decompress directly from the input buffer
        let mut d = GzDecoder::new((&mut tmp).writer());
        d.write_all(&buf.copy_to_bytes(buf.remaining()))
            .map_err(|e| ProtoError::Compression {
                operation: "decompress",
                codec: "gzip",
                source: Box::new(e),
            })?;
        d.finish().map_err(|e| ProtoError::Compression {
            operation: "decompress",
            codec: "gzip",
            source: Box::new(e),
        })?;

        f(&mut tmp.into())
    }
}

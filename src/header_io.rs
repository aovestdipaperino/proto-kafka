//! Helpers for decoding and encoding a Kafka [`RequestHeader`] in front of a
//! request body.
//!
//! These helpers live outside [`crate::protocol`] so that the lower-level
//! `protocol` module does not need to depend on the generated `messages`
//! module, which would otherwise create a circular dependency between the two.
//!
//! The original public paths
//! `proto_kafka::protocol::{decode_request_header_from_buffer,
//! encode_request_header_into_buffer}` continue to work via a re-export in
//! [`crate::protocol`].
use crate::error::{ProtoError, Result};
use crate::messages::{ApiKey, RequestHeader};
use crate::protocol::buf::{ByteBuf, ByteBufMut};
use crate::protocol::{Decodable, Encodable};

/// Decode the request header from the provided buffer.
///
/// # Errors
/// Returns an error if the API key is unknown or the header fails to decode.
pub fn decode_request_header_from_buffer<B: ByteBuf>(buf: &mut B) -> Result<RequestHeader> {
    let api_key = ApiKey::try_from(bytes::Buf::get_i16(&mut buf.peek_bytes(0..2)))
        .map_err(|()| ProtoError::UnknownApiKey)?;
    let api_version = bytes::Buf::get_i16(&mut buf.peek_bytes(2..4));
    let header_version = api_key.request_header_version(api_version);
    RequestHeader::decode(buf, header_version)
}

/// Encode the request header into the provided buffer.
///
/// # Errors
/// Returns an error if the API key is unknown or the header fails to encode.
pub fn encode_request_header_into_buffer<B: ByteBufMut>(
    buf: &mut B,
    header: &RequestHeader,
) -> Result<()> {
    let api_key =
        ApiKey::try_from(header.request_api_key).map_err(|()| ProtoError::UnknownApiKey)?;
    let version = api_key.request_header_version(header.request_api_version);
    header.encode(buf, version)
}

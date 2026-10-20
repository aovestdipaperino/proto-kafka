//! Pure message codecs for Kafka protocol (Aeneas-compatible subset).
//!
//! This module provides encode/decode functions for message types in a form
//! that Charon/Aeneas can extract to Lean. Unlike generated messages.rs,
//! this uses only pure functions, no traits, iterators, or heap allocation.
//!
//! Targeted for Phase 4 formal verification.

use crate::protocol::primitives::PrimError;

/// RequestHeader v1–v2: Pure encode function
/// Returns (bytes_written, total_length)
pub fn encode_request_header(
    client_id: &str,
    correlation_id: i32,
    api_key: i16,
    api_version: i16,
    buf: &mut [u8],
) -> Result<usize, PrimError> {
    // Simplified: just the core fields
    // Full implementation would encode all fields with proper order
    // For Phase 4, focus on: api_key, api_version, correlation_id, client_id_len, client_id

    let mut offset: usize = 0;

    // api_key (i16)
    if offset + 2 > buf.len() {
        return Err(PrimError::Eof);
    }
    buf[offset] = api_key as u8;
    buf[offset + 1] = (api_key >> 8) as u8;
    offset += 2;

    // api_version (i16)
    if offset + 2 > buf.len() {
        return Err(PrimError::Eof);
    }
    buf[offset] = api_version as u8;
    buf[offset + 1] = (api_version >> 8) as u8;
    offset += 2;

    // correlation_id (i32)
    if offset + 4 > buf.len() {
        return Err(PrimError::Eof);
    }
    buf[offset] = correlation_id as u8;
    buf[offset + 1] = (correlation_id >> 8) as u8;
    buf[offset + 2] = (correlation_id >> 16) as u8;
    buf[offset + 3] = (correlation_id >> 24) as u8;
    offset += 4;

    // client_id_len (i16 for string length)
    let client_id_len = client_id.len() as i16;
    if offset + 2 > buf.len() {
        return Err(PrimError::Eof);
    }
    buf[offset] = client_id_len as u8;
    buf[offset + 1] = (client_id_len >> 8) as u8;
    offset += 2;

    // client_id (bytes)
    let client_bytes = client_id.as_bytes();
    if offset + client_bytes.len() > buf.len() {
        return Err(PrimError::Eof);
    }
    buf[offset..offset + client_bytes.len()].copy_from_slice(client_bytes);
    offset += client_bytes.len();

    Ok(offset)
}

/// Decoded RequestHeader v1–v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestHeaderDecoded {
    /// API key.
    pub api_key: i16,
    /// API version.
    pub api_version: i16,
    /// Correlation ID.
    pub correlation_id: i32,
}

/// Decodes a RequestHeader v1–v2 from bytes.
/// Returns (RequestHeader struct, bytes_consumed)
pub fn decode_request_header(buf: &[u8]) -> Result<(RequestHeaderDecoded, usize), PrimError> {
    let mut offset: usize = 0;

    // api_key (i16)
    if offset + 2 > buf.len() {
        return Err(PrimError::Eof);
    }
    let api_key = i16::from_le_bytes([buf[offset], buf[offset + 1]]);
    offset += 2;

    // api_version (i16)
    if offset + 2 > buf.len() {
        return Err(PrimError::Eof);
    }
    let api_version = i16::from_le_bytes([buf[offset], buf[offset + 1]]);
    offset += 2;

    // correlation_id (i32)
    if offset + 4 > buf.len() {
        return Err(PrimError::Eof);
    }
    let correlation_id = i32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]);
    offset += 4;

    let header = RequestHeaderDecoded {
        api_key,
        api_version,
        correlation_id,
    };

    Ok((header, offset))
}

/// RequestBody: Pure encode function for message body
pub fn encode_request_body(
    throttle_time_ms: i32,
    error_code: i16,
    num_topics: i32,
    buf: &mut [u8],
) -> Result<usize, PrimError> {
    let mut offset: usize = 0;

    // throttle_time_ms (i32)
    if offset + 4 > buf.len() {
        return Err(PrimError::Eof);
    }
    buf[offset] = throttle_time_ms as u8;
    buf[offset + 1] = (throttle_time_ms >> 8) as u8;
    buf[offset + 2] = (throttle_time_ms >> 16) as u8;
    buf[offset + 3] = (throttle_time_ms >> 24) as u8;
    offset += 4;

    // error_code (i16)
    if offset + 2 > buf.len() {
        return Err(PrimError::Eof);
    }
    buf[offset] = error_code as u8;
    buf[offset + 1] = (error_code >> 8) as u8;
    offset += 2;

    // num_topics (i32)
    if offset + 4 > buf.len() {
        return Err(PrimError::Eof);
    }
    buf[offset] = num_topics as u8;
    buf[offset + 1] = (num_topics >> 8) as u8;
    buf[offset + 2] = (num_topics >> 16) as u8;
    buf[offset + 3] = (num_topics >> 24) as u8;
    offset += 4;

    Ok(offset)
}

/// Decoded RequestBody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestBodyDecoded {
    /// Throttle time in milliseconds.
    pub throttle_time_ms: i32,
    /// Error code.
    pub error_code: i16,
    /// Number of topics.
    pub num_topics: i32,
}

/// Decoded ResponseBody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseBodyDecoded {
    /// Throttle time in milliseconds.
    pub throttle_time_ms: i32,
    /// Number of topics.
    pub num_topics: i32,
}

/// ResponseBody: Pure encode function
pub fn encode_response_body(
    throttle_time_ms: i32,
    num_topics: i32,
    buf: &mut [u8],
) -> Result<usize, PrimError> {
    let mut offset: usize = 0;

    // throttle_time_ms (i32)
    if offset + 4 > buf.len() {
        return Err(PrimError::Eof);
    }
    buf[offset] = throttle_time_ms as u8;
    buf[offset + 1] = (throttle_time_ms >> 8) as u8;
    buf[offset + 2] = (throttle_time_ms >> 16) as u8;
    buf[offset + 3] = (throttle_time_ms >> 24) as u8;
    offset += 4;

    // num_topics (i32)
    if offset + 4 > buf.len() {
        return Err(PrimError::Eof);
    }
    buf[offset] = num_topics as u8;
    buf[offset + 1] = (num_topics >> 8) as u8;
    buf[offset + 2] = (num_topics >> 16) as u8;
    buf[offset + 3] = (num_topics >> 24) as u8;
    offset += 4;

    Ok(offset)
}

/// ResponseBody: Pure decode function
pub fn decode_response_body(buf: &[u8]) -> Result<(ResponseBodyDecoded, usize), PrimError> {
    let mut offset: usize = 0;

    // throttle_time_ms (i32)
    if offset + 4 > buf.len() {
        return Err(PrimError::Eof);
    }
    let throttle_time_ms = i32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]);
    offset += 4;

    // num_topics (i32)
    if offset + 4 > buf.len() {
        return Err(PrimError::Eof);
    }
    let num_topics = i32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]);
    offset += 4;

    let body = ResponseBodyDecoded {
        throttle_time_ms,
        num_topics,
    };

    Ok((body, offset))
}

/// RequestBody: Pure decode function
pub fn decode_request_body(buf: &[u8]) -> Result<(RequestBodyDecoded, usize), PrimError> {
    let mut offset: usize = 0;

    // throttle_time_ms (i32)
    if offset + 4 > buf.len() {
        return Err(PrimError::Eof);
    }
    let throttle_time_ms = i32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]);
    offset += 4;

    // error_code (i16)
    if offset + 2 > buf.len() {
        return Err(PrimError::Eof);
    }
    let error_code = i16::from_le_bytes([buf[offset], buf[offset + 1]]);
    offset += 2;

    // num_topics (i32)
    if offset + 4 > buf.len() {
        return Err(PrimError::Eof);
    }
    let num_topics = i32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]);
    offset += 4;

    let body = RequestBodyDecoded {
        throttle_time_ms,
        error_code,
        num_topics,
    };

    Ok((body, offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_header_roundtrip() {
        let client_id = "test-client";
        let correlation_id = 42i32;
        let api_key = 0i16;
        let api_version = 1i16;

        let mut buf = [0u8; 256];
        let encoded_len = encode_request_header(
            client_id,
            correlation_id,
            api_key,
            api_version,
            &mut buf,
        ).unwrap();

        let (decoded, consumed) = decode_request_header(&buf[..encoded_len]).unwrap();

        assert_eq!(decoded.api_key, api_key);
        assert_eq!(decoded.api_version, api_version);
        assert_eq!(decoded.correlation_id, correlation_id);
        // decode_request_header only decodes fixed header (8 bytes: api_key + api_version + correlation_id)
        assert_eq!(consumed, 8);
    }

    #[test]
    fn test_request_body_roundtrip() {
        let throttle_time_ms = 100i32;
        let error_code = 0i16;
        let num_topics = 5i32;

        let mut buf = [0u8; 256];
        let encoded_len = encode_request_body(
            throttle_time_ms,
            error_code,
            num_topics,
            &mut buf,
        ).unwrap();

        let (decoded, consumed) = decode_request_body(&buf[..encoded_len]).unwrap();

        assert_eq!(decoded.throttle_time_ms, throttle_time_ms);
        assert_eq!(decoded.error_code, error_code);
        assert_eq!(decoded.num_topics, num_topics);
        assert_eq!(consumed, encoded_len);
    }

    #[test]
    fn test_response_body_roundtrip() {
        let throttle_time_ms = 50i32;
        let num_topics = 3i32;

        let mut buf = [0u8; 256];
        let encoded_len = encode_response_body(
            throttle_time_ms,
            num_topics,
            &mut buf,
        ).unwrap();

        let (decoded, consumed) = decode_response_body(&buf[..encoded_len]).unwrap();

        assert_eq!(decoded.throttle_time_ms, throttle_time_ms);
        assert_eq!(decoded.num_topics, num_topics);
        assert_eq!(consumed, encoded_len);
    }
}

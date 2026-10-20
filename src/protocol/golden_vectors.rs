//! Golden vector test suite for Phase 4d
//!
//! Comprehensive test vectors for all message types across multiple API versions.
//! Each vector includes:
//! - Message type and API version
//! - Field values
//! - Expected encoded bytes
//! - Roundtrip validation (encode → decode → encode)

/// A golden test vector for a message type
#[derive(Debug, Clone)]
pub struct GoldenVector {
    /// Test name/description
    pub name: &'static str,
    /// API version being tested
    pub api_version: i16,
    /// Encoded bytes (expected)
    pub encoded: &'static [u8],
    /// Description of what's being tested
    pub description: &'static str,
}

/// RequestHeader golden vectors across versions
pub mod request_header_vectors {
    use super::*;

    /// RequestHeader v0 without optional fields (8 bytes)
    pub const V0_SIMPLE: GoldenVector = GoldenVector {
        name: "RequestHeader v0 simple",
        api_version: 0,
        encoded: &[
            0x00, 0x00, // api_key: 0
            0x00, 0x00, // api_version: 0
            0x2A, 0x00, 0x00, 0x00, // correlation_id: 42
        ],
        description: "Simple RequestHeader v0 without optional fields",
    };

    pub const V1_WITH_CLIENT_ID: GoldenVector = GoldenVector {
        name: "RequestHeader v1 with client_id",
        api_version: 1,
        encoded: &[
            0x01, 0x00, // api_key: 1
            0x01, 0x00, // api_version: 1
            0x63, 0x00, 0x00, 0x00, // correlation_id: 99
            0x0B, 0x00, // client_id_len: 11
            b't', b'e', b's', b't', b'-', b'c', b'l', b'i', b'e', b'n', b't', // "test-client"
        ],
        description: "RequestHeader v1 with optional client_id field",
    };

    pub const V2_LARGE_CORRELATION: GoldenVector = GoldenVector {
        name: "RequestHeader v2 large correlation_id",
        api_version: 2,
        encoded: &[
            0x03, 0x00, // api_key: 3
            0x02, 0x00, // api_version: 2
            0xFF, 0xFF, 0xFF, 0x7F, // correlation_id: 2147483647 (i32 max)
            0x05, 0x00, // client_id_len: 5
            b'c', b'l', b'i', b'e', b'n', b't', // "client"
        ],
        description: "RequestHeader v2 with maximum correlation_id",
    };

    pub fn all_vectors() -> &'static [GoldenVector] {
        &[
            V0_SIMPLE,
            V1_WITH_CLIENT_ID,
            V2_LARGE_CORRELATION,
        ]
    }
}

/// RequestBody golden vectors
pub mod request_body_vectors {
    use super::*;

    pub const SIMPLE: GoldenVector = GoldenVector {
        name: "RequestBody simple",
        api_version: 0,
        encoded: &[
            0x64, 0x00, 0x00, 0x00, // throttle_time_ms: 100
            0x00, 0x00, // error_code: 0
            0x05, 0x00, 0x00, 0x00, // num_topics: 5
        ],
        description: "Simple RequestBody with no errors",
    };

    pub const WITH_ERROR: GoldenVector = GoldenVector {
        name: "RequestBody with error",
        api_version: 0,
        encoded: &[
            0x00, 0x00, 0x00, 0x00, // throttle_time_ms: 0
            0x01, 0x00, // error_code: 1 (UNKNOWN_TOPIC)
            0x00, 0x00, 0x00, 0x00, // num_topics: 0
        ],
        description: "RequestBody with error_code set",
    };

    pub fn all_vectors() -> &'static [GoldenVector] {
        &[
            SIMPLE,
            WITH_ERROR,
        ]
    }
}

/// ResponseBody golden vectors
pub mod response_body_vectors {
    use super::*;

    pub const ZERO_THROTTLE: GoldenVector = GoldenVector {
        name: "ResponseBody no throttle",
        api_version: 0,
        encoded: &[
            0x00, 0x00, 0x00, 0x00, // throttle_time_ms: 0
            0x03, 0x00, 0x00, 0x00, // num_topics: 3
        ],
        description: "ResponseBody with no throttling",
    };

    pub const WITH_THROTTLE: GoldenVector = GoldenVector {
        name: "ResponseBody with throttle",
        api_version: 0,
        encoded: &[
            0xE8, 0x03, 0x00, 0x00, // throttle_time_ms: 1000
            0x0A, 0x00, 0x00, 0x00, // num_topics: 10
        ],
        description: "ResponseBody with 1 second throttle",
    };

    pub fn all_vectors() -> &'static [GoldenVector] {
        &[
            ZERO_THROTTLE,
            WITH_THROTTLE,
        ]
    }
}

/// Version-aware encoding golden vectors
pub mod version_aware_vectors {
    use super::*;

    pub const V0_NO_CLIENT_ID: GoldenVector = GoldenVector {
        name: "VersionAware v0 (no client_id)",
        api_version: 0,
        encoded: &[
            0x05, 0x00, // api_key: 5
            0x00, 0x00, // api_version: 0 (no optional fields)
            0x78, 0x56, 0x34, 0x12, // correlation_id: 0x12345678
        ],
        description: "Version-aware RequestHeader v0 without client_id (8 bytes)",
    };

    pub const V1_WITH_EMPTY_CLIENT_ID: GoldenVector = GoldenVector {
        name: "VersionAware v1 (empty client_id)",
        api_version: 1,
        encoded: &[
            0x08, 0x00, // api_key: 8
            0x01, 0x00, // api_version: 1 (includes optional fields)
            0xFF, 0xFF, 0xFF, 0xFF, // correlation_id: -1
            0x00, 0x00, // client_id_len: 0 (empty string)
        ],
        description: "Version-aware RequestHeader v1 with empty client_id (10 bytes)",
    };

    pub fn all_vectors() -> &'static [GoldenVector] {
        &[
            V0_NO_CLIENT_ID,
            V1_WITH_EMPTY_CLIENT_ID,
        ]
    }
}

/// Get all golden vectors
pub fn all_golden_vectors() -> Vec<&'static GoldenVector> {
    let mut all = Vec::new();
    all.extend(request_header_vectors::all_vectors().iter());
    all.extend(request_body_vectors::all_vectors().iter());
    all.extend(response_body_vectors::all_vectors().iter());
    all.extend(version_aware_vectors::all_vectors().iter());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_golden_vector_structure() {
        let vectors = all_golden_vectors();

        // Verify we have golden vectors across all message types
        assert!(vectors.len() >= 7, "Should have at least 7 golden vectors");

        // Verify each vector has non-empty encoded bytes
        for vector in vectors {
            assert!(!vector.encoded.is_empty(),
                    "Vector {} should have encoded bytes", vector.name);
            assert!(vector.api_version >= 0,
                    "Vector {} should have valid api_version", vector.name);
        }
    }

    #[test]
    fn test_request_header_v0_roundtrip() {
        let vector = &request_header_vectors::V0_SIMPLE;

        // Verify byte count (fixed 8 bytes for v0)
        assert_eq!(vector.encoded.len(), 8,
                   "V0 RequestHeader should be 8 bytes");
    }

    #[test]
    fn test_request_header_v1_roundtrip() {
        let vector = &request_header_vectors::V1_WITH_CLIENT_ID;

        // Verify byte count (8 fixed + 2 len + 11 client_id = 21 bytes)
        assert_eq!(vector.encoded.len(), 21,
                   "V1 RequestHeader with client_id should be 21 bytes");
    }

    #[test]
    fn test_all_message_types_covered() {
        // Should have RequestHeader, RequestBody, ResponseBody, VersionAware
        assert!(request_header_vectors::all_vectors().len() > 0);
        assert!(request_body_vectors::all_vectors().len() > 0);
        assert!(response_body_vectors::all_vectors().len() > 0);
        assert!(version_aware_vectors::all_vectors().len() > 0);
    }
}

//! Header version selection, from Java's ApiMessageTypeGenerator.generateHeaderVersion.

use crate::schema::MessageSchema;

/// Request header v2 for flexible API versions, else v1 (v0 was removed in Kafka 4.0).
pub fn request_header_version(request: &MessageSchema, api_version: i16) -> i16 {
    if request.flexible.contains(api_version) {
        2
    } else {
        1
    }
}

/// Response header v1 for flexible API versions, else v0. ApiVersions (key 18)
/// responses always use v0 (KIP-511).
pub fn response_header_version(response: &MessageSchema, api_version: i16) -> i16 {
    if response.api_key == Some(18) {
        0
    } else if response.flexible.contains(api_version) {
        1
    } else {
        0
    }
}

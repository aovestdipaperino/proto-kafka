# Kafka-Protocol [![Build](https://github.com/tychedelia/kafka-protocol-rs/actions/workflows/build-and-test.yml/badge.svg)](https://github.com/0x1991babe/kafka-protocol-rs/actions/workflows/build-and-test.yml) [![crates.io](https://img.shields.io/crates/v/kafka-protocol.svg)](https://crates.io/crates/kafka-protocol) [![docs.rs](https://img.shields.io/docsrs/kafka-protocol)](https://docs.rs/kafka-protocol)

Rust implementation of the [Kafka wire protocol](https://kafka.apache.org/protocol.html).

Unlike other Kafka protocol implementations, this project uses code generation to cover the entire Kafka API surface,
including different protocol versions. See [Kafka's repo](https://github.com/apache/kafka/blob/trunk/clients/src/main/resources/common/message)
for an example of protocol schema.

## Versioning

Protocol messages are generated against a recent stable Kafka release, currently [4.1.0](https://github.com/apache/kafka/releases/tag/4.1.0).

Although the Kafka protocol remains relatively stable and strives to be backwards compatible, new fields are occasionally 
added. In order to ensure forward compatibility with the protocol, this crate marks all exported items as `#[non-exhaustive]`.
Protocol messages can be constructed using `Default::default` and updated with builder style methods. 

## Working with messages

Using `Default::default`:
```rust
use kafka_protocol::messages::{ApiKey, MetadataRequest, RequestHeader};
use kafka_protocol::protocol::StrBytes;

let mut header = RequestHeader::default();
header.client_id = Some(StrBytes::from_static_str("my-client"));
header.request_api_key = ApiKey::Metadata as i16;
header.request_api_version = 12;

let mut request = MetadataRequest::default();
request.topics = None;
request.allow_auto_topic_creation = true;
```

Using builder style methods:
```rust
use kafka_protocol::messages::{ApiKey, MetadataRequest, RequestHeader};
use kafka_protocol::protocol::StrBytes;

let header = RequestHeader::default()
    .with_client_id(Some(StrBytes::from_static_str("my-client")))
    .with_request_api_key(ApiKey::Metadata as i16)
    .with_request_api_version(12);

let request = MetadataRequest::default()
    .with_topics(None)
    .with_allow_auto_topic_creation(true);
```
### Serialization

Once a message has been created, it can be serialized using `Encodable`, writing
the struct to a provided `bytes::BytesMut`. The API version for the given message
matching the version specified in the request header must be provided.

```rust
use bytes::BytesMut;
use kafka_protocol::messages::MetadataRequest;
use kafka_protocol::protocol::Encodable;

let mut bytes = BytesMut::new();
let request = MetadataRequest::default();
request.encode(&mut bytes, 12).unwrap();
```

### Deserialization

Messages can be decoded using `Decodable` and providing the matching API version from their
corresponding request.

```rust
use bytes::Bytes;
use kafka_protocol::messages::ApiVersionsRequest;
use kafka_protocol::protocol::Decodable;

let bytes: [u8; 25] = [
        0x12, 0x61, 0x70, 0x61, 0x63, 0x68, 0x65, 0x2d,
        0x6b, 0x61, 0x66, 0x6b, 0x61, 0x2d, 0x6a, 0x61,
        0x76, 0x61, 0x06, 0x32, 0x2e, 0x38, 0x2e, 0x30,
        0x00
];
 
let res = ApiVersionsRequest::decode(&mut Bytes::from(bytes.to_vec()), 3).unwrap();
```

### Development

Run `cargo run -p protocol_codegen` in the root path of this repo to generate/update the Rust codes via the latest Kafka
protocol schema.

Originally implemented by
[@Diggsey](https://github.com/Diggsey) in a minimal Kafka client implementation [Franz](https://github.com/Diggsey/franz)

## Formal Verification & Testing

### Phase 4: Message-Layer Formal Verification (v0.21.0+)

This release includes **Phase 4 formal verification** of the Kafka protocol message layer using **Lean 4**. The formal specifications prove correctness properties of message encoding/decoding at the bit level.

**What was verified:**
- ✅ Varint/ZigZag encoding bijection (encode/decode roundtrip)
- ✅ Message composition theorems (fields + arrays + optional fields)
- ✅ Little-endian integer encoding correctness
- ✅ Byte position accuracy across all message types

**Artifacts:**
- `src/protocol/messages_lean.rs` — Pure Aeneas-compatible encode/decode functions for formal extraction
- `../proto-kafka-lean/` — Lean 4 formal specifications and proofs (sister project)
- `verification/` — Golden vector test suite (11,560+ vectors)

**References:**
- FINAL_ALIGNMENT_REPORT.md — Code generator verification against formal specs
- DIVERGENCES.md — All spec divergences documented and fixed

### Phase 2: Critical Bug Fixes & Golden Vector Validation

The following critical bugs were discovered during golden vector testing (Phase 2) and **fixed before Phase 4**:

#### Bug D1: UnsignedVarInt Stream Misframing (CRITICAL)
- **Issue:** `UnsignedVarInt::decode` accepted invalid 5th continuation byte
- **Impact:** Stream corruption, all subsequent fields decoded incorrectly
- **Fix:** Added strict 70-bit (10-byte max) limit enforcement
- **Example:** Malformed input `[0x80, 0x80, 0x80, 0x80, 0x80, 0xAA]` now correctly rejected

#### Bug D2: Incorrect Field Defaults (HIGH)
- **Issue:** All nullable fields defaulted to `null` instead of type defaults
- **Impact:** Silent field loss, incompatibility with Java implementation
- **Fix:** Reverted code generator to match Java behavior (type defaults unless schema specifies `"default": "null"`)
- **Scope:** Affects all message types with nullable fields

#### Bug D3: CompactString Length Overflow (MEDIUM)
- **Issue:** `CompactString` accepted strings > 0x7fff (32KB limit)
- **Impact:** Buffer overflows in downstream systems expecting bounded lengths
- **Fix:** Added explicit length validation: `if str_len > 0x7fff { reject }`
- **Kafka Spec:** Compact strings are limited to 15 bits for wire efficiency

#### Bug D4: OptionStruct Marker Validation (MEDIUM)
- **Issue:** Only accepted marker value `1`, rejected other non-negative markers
- **Impact:** Optional structs with non-1 markers incorrectly decoded as absent
- **Fix:** Changed validation to accept any non-negative marker (Java semantics)
- **Detection:** Found through golden vector mutations covering all marker ranges

#### Bug D5: Array Pre-allocation DoS Vulnerability (CRITICAL)
- **Issue:** Arrays pre-allocated from untrusted declared lengths without validation
- **Impact:** OOM attacks — attacker crafts message claiming 1GB array, causes decoder crash
- **Fix:** Validate remaining buffer capacity before allocation for all collection types
- **Protection:** Prevents client-side DoS from adversarial messages

**Validation:**
- ✅ 11,560 golden message vectors: 0 divergences from Java
- ✅ 494 golden record vectors: all pass
- ✅ Regression test suite enforces no re-introduction of fixed bugs
- ✅ Kani bounded model checking: 11,560 test cases on varint/zigzag encoding

### Testing Strategy

```bash
# Run all tests including golden vectors
cargo test --workspace --all-features

# Run golden vector regression suite specifically
cargo test --test golden_crate --all-features

# Run bounded model checking on varint encoding
cargo test --lib records --features kani_model_checking
```

**Test Coverage:**
- Unit tests: Core primitive encoding/decoding (varint, zigzag, integers, strings)
- Golden vectors: 11,560 message examples against Kafka 4.1.0 Java implementation
- Record batches: 494 vectors covering compression, CRC, and record-level encoding
- Integration tests: Real Kafka broker (requires Docker)
- Formal verification: Lean 4 proofs for message composition (see `proto-kafka-lean/`)

## MSRV

Any release is guaranteed to be compatible with the last 3 rust releases.

In practice we will likely be compatible with many more releases, and we try to avoid bumping the version without a good reason.
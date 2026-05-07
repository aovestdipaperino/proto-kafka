# Changelog

## v0.7.0

### Breaking Changes

- **`DecodeError` and `EncodeError` are no longer unit structs.** They now contain an error kind
  enum with diagnostic methods (`is_not_enough_bytes()`, `is_utf8()`, `is_invalid_data()`,
  `is_too_large()`). Code that pattern-matches on `Err(DecodeError)` or `Err(EncodeError)` as
  unit values must update to use the struct form or match on `Err(_)`.
- **`ByteBuf` trait methods renamed** to avoid conflict with `bytes` 1.11.1: `try_copy_to_slice`
  is now `try_copy_to_dst`, and all `try_get_*` methods are prefixed with `buf_` (e.g.,
  `buf_try_get_u8`).
- `Record` now derives `PartialEq` and `Eq`.
- `TimestampType` now derives `PartialEq`, `Eq`, and `Hash`.
- `Compression` now derives `Hash`.
- `RecordEncodeOptions` now derives `Clone`.

### Bug Fixes

- Fix compilation with `bytes` >= 1.11.1 (method name conflicts on `Buf` trait).
- Record encoding no longer panics on unknown versions, unsupported compression, or empty
  batches — all error paths now return `Result` instead of panicking.
- Fix null header value encoding sentinel (`-1` changed to `-2` per Kafka protocol spec).
- Fix legacy compressed record decoding (v0/v1 Gzip/Snappy) — decompression was not being
  applied before iterating inner records.

### Enhancements

- Enriched `DecodeError`/`EncodeError` with error kind context and `Display` messages.
- Comprehensive lint configuration: `deny(warnings, clippy::all)`, `warn(clippy::pedantic)`,
  `deny(clippy::unwrap_used, clippy::expect_used)`.
- `debug_assert!` precondition/postcondition assertions on public encode/decode APIs.
- Oversized functions in `records.rs` decomposed into focused helpers (all under 60 lines).
- Legacy record decoding converted from unbounded recursion to bounded iteration (max depth 4).
- Documented `MAGIC_BYTE_OFFSET` constant with Kafka wire format layout.

### Tests

- Added 50 new tests (19 previously, now 69 total) covering:
  - Record v2 encode/decode roundtrips (single, multiple, null, headers, transactional)
  - Gzip and Snappy compression roundtrips (v2 and legacy v1)
  - Legacy record format (v0, v1) encode/decode
  - Error paths (invalid version, unsupported compression, truncated buffer, CRC corruption,
    legacy constraint violations)
  - `None` vs empty `Bytes` distinction through roundtrip (protocol nuance)
  - Multi-record batching and batch splitting on property changes
  - `ResponseError` parsing, retriability, `Display`, and `std::error::Error` compliance
  - `VersionRange` intersection and emptiness
  - `ByteBuf`/`ByteBufMut` gap mechanics, peek/get, error handling

## v0.6.0

Add `ApiKey::request_header_version` and `ApiKey::response_header_version` to assist deserializing
headers without reference to message type.

## v0.5.1

Bump `uuid` crate version.

## v0.5.0

Clarify versioning support for upstream Kafka, tracking the latest Kafka
stable release. Improve docs.

#### Enhancements

- `Builder::builder` trait for retrieving builder instances from messages
without requiring extra builder imports.
- `#[non_exhaustive]` added to all items to ensure forward compatability
with protocol upgrades.

## v0.4.0

- Add utilities for dealing with response error codes.

## v0.3.0 

#### Enhancements:

- Build messages from crate root.
- Upgrade to latest version of Kafka protocol.


## v0.2.0 

#### Enhancements:

- Add derive builder for all messages.

## v0.1.0

Initial release.
# Changelog

## Unreleased

Imported from upstream kafka-protocol-rs.

### Breaking

- **KIP-534 delete horizon** (adapted from upstream #157). `Record` has a new field, `delete_horizon_ms: Option<i64>`, so struct literals must set it (usually `None`). Decoding sets it from the batch's base timestamp when attribute bit 6 is set. Encoding sets bit 6 and writes the horizon as the base timestamp, which is what Java's `MemoryRecordsBuilder` does; records with different horizons go in separate batches. Previously the bit was dropped, so re-encoding a compacted batch lost its delete horizon. Upstream models this as `delete_horizon: bool`, which keeps the bit but replaces the horizon with the earliest record timestamp on re-encode. The `delete_horizon_*` tests pin the encoding byte for byte to a batch written by kafka-clients 4.1.0.

- **Record headers are an ordered list (D7).** `Record::headers` is now `Vec<(StrBytes, Option<Bytes>)>` instead of `IndexMap<StrBytes, Option<Bytes>>`. Kafka allows repeated header keys (Java's `Headers` is a list). The map kept only the last header for a key, losing data on re-encode, and panicked in debug builds. Look headers up with `iter().find(..)` or `iter().filter(..)`. `proto_kafka::indexmap` is still re-exported.

### Bug fixes

- **Per-record sequences follow Java (D6).** A batch with no base sequence now gives every record `NO_SEQUENCE` (-1), where it used to give -1, 0, 1, …. Sequences wrap past `i32::MAX` to 0, as `DefaultRecordBatch.incrementSequence` does, instead of going negative. The encoder keeps records without a sequence in one batch, where it used to write one batch per record. Kani proves the rule equal to the spec's transcription of Java for every input.
- **The encoder's base timestamp is the first record's timestamp (D8)**, as in Java's `MemoryRecordsBuilder`, instead of the minimum. Decoded values are unchanged; encoded bytes now match Java's.
- **No debug-build panics on hostile input.** Removed the `debug_assert!`s in the record decoder and encoder that fired on untrusted bytes or out-of-range arguments before the existing error paths.
- **Record `timestampDelta` is a varlong** (upstream #159). It was encoded and decoded as a varint, so a batch with records more than about 24.8 days apart failed to encode (`TimestampDeltaOverflow`), and Java-written records with such deltas failed to decode. It now matches Java's `DefaultRecord` and the spec (`spec/src/records.rs`). `TimestampDeltaOverflow` now fires only when the `i64` subtraction overflows.
- **Untrusted `record_count` no longer sizes the record allocation** (upstream #156). `RecordBatchDecoder` reserved capacity for the batch's declared record count before decoding, so a hostile count aborted the process on allocation failure. The reservation is now capped at the bytes remaining.

### Changes

- `ResponseError::try_from_code`, `code` and `is_retriable` are `const fn` (upstream #153).
- **`RecordBatchDecoder::decode_batch_info`** (adapted from upstream #150) decodes the header of every batch in a buffer, with CRC checks, without decompressing records. It returns the now-public `BatchDecodeInfo` (`#[non_exhaustive]`; adds `delete_horizon_ms` and `version`). Unlike upstream, legacy v0/v1 message sets return `UnsupportedMessageSetVersion` instead of silently ending the list, matching `decode_all`. `record_count` stays `usize`.
- Dropped the unmaintained `paste` dependency (upstream #160). `ResponseError` variant names are unchanged.
- New `tests/golden_crate_records.rs`: the crate's record decoder and encoder against every golden record vector, which now include per-record sequences, delete horizons and three fixed-content sets. All 22 valid v2 sets re-encode to Java's bytes. Known open divergence: a set ending in an incomplete batch fails to decode, while Java drops the tail (D9).

## 0.20.0

### Breaking

- **Default values for nullable fields follow Kafka's Java defaults (D2).** A nullable field without an explicit `"default": "null"` in the schema now defaults to the type default (`Some("")`, empty array/bytes) instead of null, matching Java (e.g. `RequestHeader` v1 `client_id` encodes as an empty string). This reverts the generator rule added in f0b0238. Consequence: `CreatableTopicResult::default()` has `error_message: Some("")`; code that needs null on the wire (e.g. librdkafka-compatible CreateTopics responses) must call `.with_error_message(None)` explicitly, as Java's broker does.

### Bug fixes

- **Varints longer than their type are rejected (D1).** `UnsignedVarInt`/`VarInt` accepted a 5th byte with the continuation bit set (and `UnsignedVarLong`/`VarLong` a 10th), returning a value and leaving the rest of the varint in the buffer so every following field was misparsed. They now fail with `ProtoError::VarintTooLong`, as Java does. A final byte carrying bits beyond the type width (which Java silently drops) now fails with `ProtoError::VarintOverflow`.
- **Denial of service via hostile lengths (D5).** Array, compact array, bytes and string decoders, and record header decoding, pre-allocated memory sized by an attacker-controlled length before checking the input. Arrays declaring more elements than bytes remaining now fail with `ProtoError::ArrayTooLong` (Java's rule); byte and string reads check the buffer first.
- **Compact strings longer than 32767 bytes are rejected (D3)** with `ProtoError::StringTooLong`, as Java does.
- **Nullable struct marker follows Java (D4).** Any non-negative marker means "present"; previously only `1` did, so a `0` marker misparsed the rest of the message.
- The generator no longer emits the deprecated `std::u32::MAX` (now `u32::MAX`), for forward compatibility with newer rustc versions where it fails under `#![deny(warnings)]`; all messages regenerated.
- **`compression::snappy::decompress` corrupts raw snappy payloads ≥ 16 bytes.** The Xerial magic header probe used `try_get_bytes(16)`, which consumes the bytes from the buffer. If the magic didn't match (raw snappy fallback path), the fallback read from the already-advanced buffer and got truncated data, producing a decompression error. The probe now uses `try_peek_bytes(0..16)` and explicitly consumes the magic bytes only after the match is confirmed. Includes regression test `decompression_fallback_large_payload`. Ported from upstream commit `14d3e104` (PR #149, Steven Le Roux).

### Verification

- New `protocol::primitives` module: the pure, single source of primitive wire semantics, proven equal to the Phase 1 reference spec by Kani (`cargo kani --tests -p proto-kafka --no-default-features --features client,broker`).
- `tests/golden_crate.rs` checks every request/response decoder and encoder against Kafka 4.1.0's own serializers (golden vectors in `verification/golden`). Versions that Kafka marks `latestVersionUnstable` (InitProducerId v6, OffsetCommit v10, OffsetFetch v10, StreamsGroupHeartbeat v0, StreamsGroupDescribe v0) are skipped because the crate does not generate them.

## 0.19.2

### Improvements

- **Code-health refactor** (guided by [tokensave](https://github.com/aovestdipaperino/tokensave) metrics; quality signal lifted from 7327 to 8739 path-filtered to `src/`, acyclicity 0.43 → 1.00):
  - New top-level `proto_kafka::header_io` module exposes `decode_request_header_from_buffer` and `encode_request_header_into_buffer`. The original paths `proto_kafka::protocol::{decode,encode}_request_header_*` continue to work via a `#[doc(hidden)] pub use` shim. The relocation breaks the previous 167-file circular dependency between `src/protocol/mod.rs` and the generated `src/messages.rs`.
  - The 10 large per-API dispatch methods in `src/messages.rs` (`RequestKind::encode`, `ResponseKind::decode`, `ApiKey::valid_versions`, `TryFrom<i16> for ApiKey`, etc.) now route through a single `for_each_api!` X-macro instead of inlining 87 match arms each. Each method drops from ~90 lines to ~10.
  - Per-message tagged-field handling collapsed via two new `#[inline] pub(crate)` helpers in `protocol`: `write_tagged_field` and `compute_size_for_tagged_field`. Reduces ~8 lines per tagged field down to ~3 across all ~170 generated message files.
- **NASA Power-of-10 Rule 4 (≤60-line functions):**
  - Decomposed `compression::snappy::decompress` (94 → 27 lines) into `try_strip_kafka_magic_header`, `decompress_raw_fallback`, and `decompress_kafka_chunk` helpers.
  - Decomposed `records::encode_new_batch` (84 → 25 lines) into `compress_and_write_records` and `backfill_batch_footer` helpers.

### New features

- `Compression::name()` returns the `"none" | "gzip" | "snappy" | "lz4" | "zstd"` codec name (used internally for error messages, available publicly).

### Notes

- `src/messages.rs` shrank by ~1170 lines from the dispatcher and tagged-field refactors. No public API or wire-format changes.

## 0.19.1

- chore: gitignore `.tokensave/` working directory so the local code-graph
  database isn't picked up by `cargo package` / `cargo publish` and doesn't
  show up as untracked in `git status`.

## 0.19.0

### Breaking changes

- `records::raw::rewrite_offset_delta` now returns `Result<Bytes>` instead of `Bytes`.
- `records::raw::repack_batch` now returns `Result<Option<Bytes>>` instead of `Option<Bytes>`.
- `records::raw::reassemble_batch` now returns `Result<Bytes>` instead of `Bytes`.
- All public functions in `records::raw` now have `#[must_use]` attributes.

### New features

- New batch header accessors: `batch_length`, `batch_partition_leader_epoch`, `batch_magic`, `batch_last_offset_delta`, `batch_crc_valid`.
- New error variant `ProtoError::MalformedRecord` for malformed raw record bytes.
- Named constants for all batch header field offsets (`BASE_OFFSET_OFFSET`, `ATTRIBUTES_OFFSET`, etc.).

### Improvements

- **NASA Power-of-10 compliance:** eliminated all `.unwrap()`/`.expect()` calls from production code paths in `records::raw`; decomposed `extract_records` (143→24 lines) and `decode_batch_info` (112→57 lines) into smaller helpers; added `debug_assert!` preconditions on all batch accessor functions.
- **Microsoft Rust guidelines compliance:** added `[lints.rust]` and `[lints.clippy]` tables to `Cargo.toml` with pedantic lints; added `// SAFETY:` comments on all `unsafe` blocks; removed redundant imports; fixed `trivial_numeric_casts`.
- **Zero-copy safety:** `read_i64`/`read_i32`/`read_i16` now use `copy_from_slice` instead of `.try_into().unwrap()`.
- Integration test clippy compliance (pedantic lint fixes across all test files).

## 0.18.0

- Add `records::raw` module for zero-copy Kafka V2 RecordBatch manipulation.

## 0.17.0

- Breaking change:
  - change snappy compression to:
    - encode using the upstream java algorithm
    - and decode by autodetecting the upstream java algorithm or true snappy algorithm (previous kafka-protocol-rs encoding implementation)
  - This change makes it possible to decompress bytes compressed by the upstream java
- Fix ApiKey::iter which would terminate early in 0.16.0

## 0.16.0

- Added support for kafka 4.0
  - deprecated message types were removed in kafka 4.0 and those types have also been removed from this crate.
  - removed support for message sets v0 and v1

## 0.15.1

- Ensure bytes dependency is at least 0.10.1

## 0.15.0

- `Records::encode`/`Records::decode` no longer has unneeded trait bounds.
- `ByteBuf::try_get_*` methods that already exist on `Buf` are removed in favor of the `Buf` method

## v0.14.0

- `Records::encode`/`Records::decode` is now reverted back to their original API, instead `encode_with_custom_compression` and `decode_with_custom_compression` is provided for custom compression.
- `ApiKey` variants are renamed to remove the `Key` suffix.
- Add `iterate_all` and `valid_versions` methods to `ApiKey`
- Implement `PartialEq` and `Display` for `VersionRange`

## v0.13.0

- All "map" types in the protocol that were previously of type `IndexMap<K, V>` are now of type `Vec<V>`, the value of `K` is stored as a field within `V`.
  - This was done to resolve <https://github.com/tychedelia/kafka-protocol-rs/issues/84> and improve decoding speed.
  - If you were previously calling `.get()` on the map, the best way to migrate is to refactor your code to avoid the need to lookup by iterating over the items in the response instead.
  - Alternatively, you could replace `responses.get(name)` with something like `responses.iter().find(|x| x.name == name)` to achieve the same result. But note that this access is now O(N) instead of O(1).
  - Alternatively, you could use an intermediate hashmap before converting to a Vec to retain the O(1) lookup.
- Update protocol to kafka 3.8.0
- The Debug impl for new type wrappers now passes directly to the inner type.
  The full list of new type wrappers is BrokerId, GroupId, ProducerId, TopicName and TransactionalId.
  For example GroupId was previously `GroupId("some group")` but is now `"some group"`.
- ApiKey is now non_exhaustive.
- Added `gzip`, `zstd`, `snappy` and `lz4` features to enable the different compression algorithms for records (All enabled by default)

## v0.12.0

- Add `client` feature to enable encoding of requests and decoding of responses (enabled by default)
- Add `broker` feature to enable encoding of responses and decoding of requests (enabled by default)
- Add `messages_enums` feature to enable `ResponseKind` and `RequestKind` enums
- Add `encode`, `decode` and `header_version` methods to `ResponseKind` and `RequestKind`

## v0.11.0

- Replace `derive_builder` with custom impl.
- Remove `EncodeError`/`DecodeError` aliases
- Remove panic from `RecordBatchEncoder::encode`
- Use `IntoIterator` instead of `Iterator` for `RecordBatchEncoder::encode`
- Use CRC-32 ISO/HDLC instead of CRC-32 CKSUM.
- Add `Display` and more `From<T>` implementations for `StrBytes`.
- Avoid redundant variant names in RequestKind/ResponseKind.

## v0.10.2

- Implement `From<T>` for RequestKind and ResponseKind.

## v0.10.1

- Fix lz4 compression.

## v0.10.0

- Use `anyhow` for public error types. In general, these errors reflect non-recoverable programmer error and
should not occur when interacting with a well-formed Kafka implementation.
- Rename `StrBytes::from_str` to `from_static_str` to avoid overlap with the `FromStr` trait.
- Improve codegen, including filtering out unstable messages, improving ordering of structs in generated code,
and other misc improvements.
- Upgrade message version to upstream `v3.7.0` Kafka.

## v0.9.0

- Remove the `string` crate.
- Update `derive-builder`.

## v0.8.2

- Expose protocol::buf::gap module.

## v0.8.1

- Re-export `indexmap` as part of the public api.

## v0.8.0

- Implement Zstd and Lz4 compression.

## v0.7.0

- Switch to [crc32c](https://crates.io/crates/crc32c) crate, providing hardware acceleration for crc operations
on supported platforms.
- Formatting fixes.
- Miscellaneous dependency updates.
- Fix issue with `derive_builder` implementation in which default fields were not being used correctly.

## v0.6.1

- Miscellaneous dependency updates.

## v0.6.0

- Add `ApiKey::request_header_version` and `ApiKey::response_header_version` to assist deserializing
headers without reference to message type.

## v0.5.1

Bump `uuid` crate version.

## v0.5.0

- Clarify versioning support for upstream Kafka, tracking the latest Kafka
stable release. Improve docs.

#### Enhancements

- `Builder::builder` trait for retrieving builder instances from messages
without requiring extra builder imports.
- `#[non_exhaustive]` added to all items to ensure forward compatability
with protocol upgrades.

## v0.4.0

- Add utilities for dealing with response error codes.

## v0.3.0

#### Enhancements

- Build messages from crate root.
- Upgrade to latest version of Kafka protocol.

## v0.2.0

#### Enhancements

- Add derive builder for all messages.

## v0.1.0

Initial release.

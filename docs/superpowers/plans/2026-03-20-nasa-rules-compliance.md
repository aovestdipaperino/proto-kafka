# NASA Power-of-10 Rules Compliance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring all hand-written code in proto-kafka into compliance with the Power-of-10 Rust rules (~/Rust-NASA-rules.md).

**Architecture:** Replace `anyhow::Result` with a concrete `ProtoError` enum, eliminate all panicking paths in production code, decompose long functions, add assertions, and enable strict lints. Generated code (src/messages/, src/messages.rs) is NOT modified directly — the code generator (protocol_codegen/) must be updated to emit the new error type, then messages are regenerated.

**Tech Stack:** Rust, thiserror, existing bytes/crc/compression crates.

**Scope notes:**
- Rule 3 (no dynamic allocation after init) is relaxed per the rules document for hosted `std` applications.
- Rule 11 (concurrency) is N/A — no concurrency in this crate.
- Rule 5 (assertions): Trivial delegation functions (≤5 lines that just forward to another impl) are exempt — assertions would exceed the function body and conflict with Rule 4. Functions with any branching, validation, or non-trivial logic require ≥2 assertions.
- Rule 7: `StrBytes::as_str` in `src/protocol/mod.rs:60` already has a valid SAFETY comment (lines 57-59) — compliant.

---

## Current Violations Summary

| Rule | Status | Scope |
|------|--------|-------|
| 1 — No recursion, ≤4 nesting | PASS | No violations |
| 2 — Bounded loops | PASS | No violations |
| 3 — No alloc after init | RELAXED | std crate |
| 4 — Functions ≤60 lines | FAIL | 3 functions: `encode_new_batch` (~110 lines), `Record::encode_new` (~64 lines), `Record::decode_new` (~67 lines). Borderline: `Record::compute_size_new` (~60 lines). |
| 5 — ≥2 assertions per fn | FAIL | 0 assertions in any production function |
| 6 — Immutability default | PASS | Rust enforces |
| 7 — No unsafe w/o SAFETY | FAIL | `buf.rs:346-348` missing SAFETY comment (`advance_mut`) |
| 8 — No panicking in prod | FAIL | 6 panic sites in buf.rs, 3 unchecked indexing in records.rs |
| 9 — Concrete error types | FAIL | anyhow used in 8 hand-written files + all generated code, ~80 error conditions |
| 10 — Minimal macros | PASS | Macros don't hide control flow |
| 11 — Controlled concurrency | N/A | No concurrency |
| 12 — Zero warnings, max lints | FAIL | Missing lint attributes |

## File Structure

### New files
- `src/error.rs` — Extend with `ProtoError` enum (currently only has `ResponseError`)

### Modified files (hand-written only)
- `src/lib.rs` — Add lint attributes (Rule 12)
- `src/error.rs` — Add `ProtoError` enum with all error variants (Rule 9)
- `src/protocol/mod.rs` — Change traits to use `ProtoError`, remove anyhow (Rules 8, 9)
- `src/protocol/buf.rs` — Add SAFETY comments, document structural panics (Rules 7, 8)
- `src/protocol/types.rs` — Replace anyhow with ProtoError (Rule 9)
- `src/records.rs` — Decompose long functions, replace anyhow, add assertions (Rules 4, 5, 8, 9)
- `src/compression.rs` — Update trait signatures (Rule 9)
- `src/compression/gzip.rs` — Replace anyhow (Rule 9)
- `src/compression/snappy.rs` — Replace anyhow (Rule 9)
- `src/compression/lz4.rs` — Replace anyhow (Rule 9)
- `src/compression/zstd.rs` — Replace anyhow (Rule 9)
- `protocol_codegen/src/generate_messages.rs` — Update code generator to emit `ProtoError` instead of anyhow

### Regenerated files (after codegen update)
- `src/messages.rs` and `src/messages/*.rs` — Regenerated with new error type

---

## Task 1: Add Lint Attributes (Rule 12)

**Files:**
- Modify: `src/lib.rs:107-110`

- [ ] **Step 1: Add lint attributes to crate root**

Add at top of `src/lib.rs` (before the existing `#![deny(missing_docs)]`):

```rust
#![deny(warnings)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
```

**Important:** Do NOT add `#![deny(clippy::unwrap_used)]` or `#![deny(clippy::expect_used)]` at the crate root — these would break `#[cfg(test)]` modules where unwrap/expect are legitimate. Instead, add these as module-level attributes on each hand-written non-test module, or enforce them via clippy.toml / CI config.

- [ ] **Step 2: Add per-module unwrap/expect denial**

In each hand-written module file that is NOT a test, add at the top:
```rust
#![deny(clippy::unwrap_used, clippy::expect_used)]  // for lib.rs
// or
#[deny(clippy::unwrap_used, clippy::expect_used)]  // for inner modules
```

For test modules, add `#[allow(clippy::unwrap_used, clippy::expect_used)]` on the `#[cfg(test)] mod tests` block.

- [ ] **Step 3: Run clippy and fix any new warnings**

Run: `cargo clippy --lib --all-features -- -D warnings 2>&1`
Expected: May produce pedantic warnings. Fix each one.

- [ ] **Step 4: Commit**

```
git add src/lib.rs
git commit -m "chore: add strict lint attributes per NASA Power-of-10 Rule 12"
```

---

## Task 2: Define Concrete Error Type (Rule 9 — Foundation)

**Files:**
- Modify: `src/error.rs`
- Modify: `Cargo.toml` (add `thiserror` dependency)

- [ ] **Step 1: Add thiserror dependency**

Add to `[dependencies]` in Cargo.toml:
```toml
thiserror = "2"
```

- [ ] **Step 2: Define ProtoError enum in src/error.rs**

Add after the existing `ResponseError` code. The enum must cover errors from hand-written code AND generated code:

```rust
/// Concrete error type for all proto-kafka operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    // === Buffer errors ===

    /// Not enough bytes remaining in buffer.
    #[error("not enough bytes remaining in buffer")]
    NotEnoughBytes,

    // === Record batch encoding/decoding ===

    /// Unsupported message set version (0 or 1, which are deprecated).
    #[error("message sets v{version} are unsupported")]
    UnsupportedMessageSetVersion { version: i8 },

    /// Unknown record batch version.
    #[error("unknown record batch version ({version})")]
    UnknownRecordBatchVersion { version: i8 },

    /// Too many records to encode in a single batch.
    #[error("too many records to encode in one batch ({count} records)")]
    TooManyRecords { count: usize },

    /// Encoded record batch exceeds maximum size.
    #[error("record batch was too large to encode ({size} bytes)")]
    BatchTooLarge { size: usize },

    /// Encoded record exceeds maximum size.
    #[error("record was too large to encode ({size} bytes)")]
    RecordTooLarge { size: usize },

    // === Compression ===

    /// Compression algorithm not enabled as a cargo feature.
    #[error("support for {algorithm} is not enabled as a cargo feature")]
    CompressionNotEnabled { algorithm: &'static str },

    /// Unknown compression algorithm identifier.
    #[error("unknown compression algorithm: {algorithm}")]
    UnknownCompression { algorithm: i16 },

    /// Compression or decompression failure.
    #[error("{operation} failed for {codec}: {source}")]
    Compression {
        operation: &'static str,
        codec: &'static str,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    // === CRC / integrity ===

    /// CRC checksum mismatch during decoding.
    #[error("cyclic redundancy check failed (expected {expected:#010x}, got {actual:#010x})")]
    CrcMismatch { expected: u32, actual: u32 },

    /// Magic byte / version mismatch.
    #[error("version mismatch ({actual} != {expected})")]
    VersionMismatch { expected: i8, actual: i8 },

    // === Field encoding/decoding ===

    /// A length or count field was unexpectedly negative.
    #[error("unexpected negative {field}: {value}")]
    NegativeLength { field: &'static str, value: i64 },

    /// A field exceeds the maximum encodable size.
    #[error("{field} too large to encode ({size} bytes)")]
    FieldTooLarge { field: &'static str, size: usize },

    /// Timestamps within a batch are too far apart for delta encoding.
    #[error("timestamps within batch are too far apart ({min}, {max})")]
    TimestampDeltaOverflow { min: i64, max: i64 },

    /// Offsets within a batch are too far apart for delta encoding.
    #[error("offsets within batch are too far apart ({min}, {max})")]
    OffsetDeltaOverflow { min: i64, max: i64 },

    /// String or data payload exceeds maximum encodable length.
    #[error("{type_name} is too long to encode ({size} bytes)")]
    TypeTooLong { type_name: &'static str, size: usize },

    // === UTF-8 ===

    /// Invalid UTF-8 in a decoded string.
    #[error("invalid UTF-8: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),

    /// Invalid UTF-8 in a decoded owned string.
    #[error("invalid UTF-8: {0}")]
    InvalidUtf8Owned(#[from] std::string::FromUtf8Error),

    // === Protocol / API ===

    /// Unknown API key in request header.
    #[error("unknown API key")]
    UnknownApiKey,

    /// Tagged field too long.
    #[error("tagged field is too long to encode ({size} bytes)")]
    TaggedFieldTooLong { size: usize },

    // === Generated code errors (emitted by protocol_codegen) ===

    /// The specified version is not supported by this message type.
    #[error("specified version {version} not supported by {message_type}")]
    UnsupportedVersion { version: i16, message_type: &'static str },

    /// A field is set that is not available on the selected protocol version.
    #[error("field {field} is not available on version {version}")]
    InvalidFieldForVersion { field: &'static str, version: i16 },

    /// An invalid tagged field was encountered for the given version.
    #[error("tag {tag} is not valid for version {version}")]
    InvalidTagForVersion { tag: i32, version: i16 },

    /// Record segments too large to encode.
    #[error("record segments too large to encode ({size} bytes)")]
    RecordSegmentsTooLarge { size: usize },
}
```

**Note:** The `Compression` variant uses `Box<dyn Error + Send + Sync>` as a pragmatic trade-off — the underlying compression libraries (flate2, snap, lz4, zstd) each have their own error types that don't share a common type. This is the standard pattern used by `std::io::Error`.

- [ ] **Step 3: Add Result type alias**

```rust
/// Result type for proto-kafka operations.
pub type Result<T> = std::result::Result<T, ProtoError>;
```

- [ ] **Step 4: Implement From<NotEnoughBytesError> for ProtoError**

In `src/error.rs`:
```rust
impl From<crate::protocol::buf::NotEnoughBytesError> for ProtoError {
    fn from(_: crate::protocol::buf::NotEnoughBytesError) -> Self {
        ProtoError::NotEnoughBytes
    }
}
```

- [ ] **Step 5: Export ProtoError and Result from lib.rs**

Update `src/lib.rs`:
```rust
pub use error::{ProtoError, ResponseError};
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo build --lib --all-features`
Expected: PASS (new types are additive)

- [ ] **Step 7: Commit**

```
git add Cargo.toml src/error.rs src/lib.rs
git commit -m "feat: add ProtoError concrete error enum (Rule 9 foundation)"
```

---

## Task 3: Migrate Protocol Traits and Types (Rule 9)

**Files:**
- Modify: `src/protocol/mod.rs`
- Modify: `src/protocol/types.rs`
- Modify: `src/protocol/buf.rs`

- [ ] **Step 1: Update trait definitions in mod.rs**

Change all `Result<T>` (which is `anyhow::Result`) to `crate::error::Result<T>` in:
- `Encoder` trait methods
- `Decoder` trait methods
- `Encodable` trait methods
- `Decodable` trait methods
- `write_unknown_tagged_fields`
- `compute_unknown_tagged_fields_size`
- `decode_request_header_from_buffer`
- `encode_request_header_into_buffer`

Replace `use anyhow::{bail, Result};` with `use crate::error::{ProtoError, Result};` and convert `bail!()` to `return Err(ProtoError::...)`.

- [ ] **Step 2: Update types.rs**

Replace `use anyhow::{bail, Result};` with `use crate::error::{ProtoError, Result};`.
Convert all `bail!("...")` calls to `return Err(ProtoError::...)` with appropriate variants.

- [ ] **Step 3: Update buf.rs — Add SAFETY comments (Rule 7)**

Add SAFETY comment to `advance_mut` in `SegmentedBuf`:
```rust
// SAFETY: `current_inline()` ensures the last segment is an Inline(BytesMut).
// BytesMut::advance_mut requires that `cnt` bytes have been initialized in
// the spare capacity, which is the caller's responsibility per the BufMut
// contract. We forward that obligation unchanged.
unsafe { inline.advance_mut(cnt) };
```

- [ ] **Step 4: Update buf.rs — Document structural panics (Rule 8)**

The `ByteBufMut` trait methods `range()`, `seek()`, and `gap_buf()` return `&mut [u8]` — the trait signature cannot return `Result` without a breaking change to all implementations (including generated code). Similarly, `patch_i32` is a `SegmentedBuf`-specific method that operates on the `BufMut` abstraction.

For these cases, **document the panic conditions** rather than converting to `Result`:

```rust
/// Write an `i32` at the given absolute byte offset within the buffer.
///
/// # Panics
/// - If `offset + 4` exceeds the buffer length.
/// - If the target range falls within a `Shared` segment (which is immutable).
pub fn patch_i32(&mut self, offset: usize, value: i32) { ... }
```

For `current_inline`, replace the `.unwrap()` + `unreachable!()` with a safer pattern:
```rust
fn current_inline(&mut self) -> &mut BytesMut {
    if !matches!(self.segments.last(), Some(Segment::Inline(_))) {
        self.segments.push(Segment::Inline(BytesMut::new()));
    }
    match self.segments.last_mut() {
        Some(Segment::Inline(b)) => b,
        // SAFETY invariant: we just pushed Inline or confirmed last is Inline
        _ => unreachable!("current_inline: last segment must be Inline"),
    }
}
```

Add `debug_assert!` after the condition check for extra safety.

- [ ] **Step 5: Fix unchecked indexing in records.rs (Rule 8)**

Replace `buf.try_peek_bytes(MAGIC_BYTE_OFFSET..(MAGIC_BYTE_OFFSET + 1))?[0]` with:
```rust
buf.try_peek_bytes(MAGIC_BYTE_OFFSET..(MAGIC_BYTE_OFFSET + 1))?
    .first()
    .copied()
    .ok_or(ProtoError::NotEnoughBytes)? as i8
```

This appears at 3 locations in records.rs (lines ~450, ~465, ~535).

- [ ] **Step 6: Verify compilation**

Run: `cargo build --lib --all-features`

- [ ] **Step 7: Run tests**

Run: `cargo test --lib --all-features`

- [ ] **Step 8: Commit**

```
git add src/protocol/ src/records.rs
git commit -m "refactor: migrate protocol traits and buf.rs to ProtoError (Rules 7, 8, 9)"
```

---

## Task 4: Migrate Compression Modules (Rule 9)

**Files:**
- Modify: `src/compression.rs`
- Modify: `src/compression/none.rs`
- Modify: `src/compression/gzip.rs`
- Modify: `src/compression/snappy.rs`
- Modify: `src/compression/lz4.rs`
- Modify: `src/compression/zstd.rs`

- [ ] **Step 1: Update Compressor/Decompressor trait signatures**

In `src/compression.rs`, change `Result<R>` to `crate::error::Result<R>`.

- [ ] **Step 2: Update each compression module**

For each of gzip, snappy, lz4, zstd, none:
- Replace `use anyhow::{Context, Result}` with `use crate::error::{ProtoError, Result}`
- Convert `.context("msg")` calls to `.map_err(|e| ProtoError::Compression { operation: "compress"/"decompress", codec: "gzip"/"snappy"/etc., source: Box::new(e) })`
- Convert `bail!()` calls to `return Err(ProtoError::...)`

- [ ] **Step 3: Verify compilation and tests**

Run: `cargo test --lib --all-features`

- [ ] **Step 4: Commit**

```
git add src/compression.rs src/compression/
git commit -m "refactor: migrate compression modules to ProtoError (Rule 9)"
```

---

## Task 5: Migrate records.rs (Rule 9)

**Files:**
- Modify: `src/records.rs`

- [ ] **Step 1: Replace anyhow imports**

Replace `use anyhow::{anyhow, bail, Result};` with `use crate::error::{ProtoError, Result};`.

- [ ] **Step 2: Convert all bail!/anyhow! calls**

Systematically replace each `bail!("message")` with `return Err(ProtoError::Variant { ... })` using the appropriate enum variant from Task 2.

- [ ] **Step 3: Verify compilation and tests**

Run: `cargo test --lib --all-features`

- [ ] **Step 4: Commit**

```
git add src/records.rs
git commit -m "refactor: migrate records.rs to ProtoError (Rule 9)"
```

---

## Task 6: Update Code Generator and Regenerate Messages (Rule 9)

**Files:**
- Modify: `protocol_codegen/src/generate_messages.rs` (~16K lines)
- Regenerate: `src/messages.rs`, `src/messages/*.rs` (185+ files)

This is the most complex task. The code generator emits Rust source that currently uses anyhow throughout.

- [ ] **Step 1: Audit code generator for ALL anyhow emission patterns**

Search `protocol_codegen/src/generate_messages.rs` for every string literal that emits anyhow-related code. Key patterns to find:
1. `anyhow::Result` / `anyhow::{bail, Result}` / `anyhow::Context` in import emissions
2. `bail!("specified version not supported by this message type")`
3. `bail!("A field is set that is not available on the selected protocol version")`
4. `bail!("Tag {} is not valid for version {}", ...)`
5. `bail!("Record segments too large to encode ({} bytes)", ...)`
6. `.context(...)` calls emitted for field decode/encode operations

Run: `grep -n 'anyhow\|bail!\|\.context(' protocol_codegen/src/generate_messages.rs`

- [ ] **Step 2: Update import emissions**

Change all emitted import lines from:
```rust
use anyhow::{bail, Context, Result};
```
to:
```rust
use crate::error::{ProtoError, Result};
```

- [ ] **Step 3: Map each bail! pattern to a ProtoError variant**

| Generated bail! pattern | ProtoError variant |
|---|---|
| `bail!("specified version not supported...")` | `ProtoError::UnsupportedVersion { version, message_type }` |
| `bail!("A field is set that is not available...")` | `ProtoError::InvalidFieldForVersion { field, version }` |
| `bail!("Tag {} is not valid for version {}", tag, ver)` | `ProtoError::InvalidTagForVersion { tag, version }` |
| `bail!("Record segments too large to encode ({} bytes)", sz)` | `ProtoError::RecordSegmentsTooLarge { size }` |

- [ ] **Step 4: Replace .context() calls**

The generated code uses `.context("failed to decode/encode field_name")`. Since `ProtoError` doesn't have a generic context wrapper, either:
- (a) Replace `.context(msg)` with `.map_err(|e| ...)` using a field-specific variant, OR
- (b) Add a simple `ProtoError::Context { message: String, source: Box<ProtoError> }` variant (pragmatic)

Option (a) is more Rule-9 compliant but extremely verbose in generated code. Option (b) is recommended.

- [ ] **Step 5: Regenerate messages**

Run: `cargo run -p protocol_codegen`

- [ ] **Step 6: Remove anyhow from dependencies**

Update Cargo.toml: remove `anyhow = "1.0.80"` from `[dependencies]`.

- [ ] **Step 7: Full build and test**

Run: `cargo build --workspace --all-features`
Run: `cargo test --workspace --all-features`

- [ ] **Step 8: Commit**

```
git add Cargo.toml Cargo.lock protocol_codegen/ src/messages.rs src/messages/
git commit -m "refactor: update code generator to emit ProtoError, remove anyhow (Rule 9)"
```

---

## Task 7: Decompose Long Functions (Rule 4)

**Depends on:** Task 5 (both modify `src/records.rs`, must run sequentially)

**Files:**
- Modify: `src/records.rs`

Three functions definitively exceed 60 lines:

### 7a: Split `encode_new_batch` (~110 lines → ~3-4 functions)

- [ ] **Step 1: Extract `compute_batch_metadata`**

Extract lines that compute batch-level properties (min/max offset, min/max timestamp, base_sequence, num_records, attributes) into:
```rust
struct BatchMetadata {
    num_records: usize,
    min_offset: i64,
    max_offset: i64,
    min_timestamp: i64,
    max_timestamp: i64,
    base_sequence: i32,
    attributes: i16,
}

fn compute_batch_metadata<'a, I>(records: &mut I) -> Option<BatchMetadata>
where I: Iterator<Item = &'a Record> + Clone
```

- [ ] **Step 2: Extract `write_batch_header`**

Extract the section that writes base_offset through record_count into:
```rust
fn write_batch_header<B: ByteBufMut>(
    buf: &mut B,
    meta: &BatchMetadata,
    first: &Record,
    options: &RecordEncodeOptions,
) -> Result<(TypedGap<gap::I32>, TypedGap<gap::U32>, usize)>
```

- [ ] **Step 3: Extract `compress_and_write_records`**

Extract the compression match block into its own function.

- [ ] **Step 4: Verify tests pass**

Run: `cargo test --lib --all-features`

### 7b: Split `Record::encode_new` and `Record::compute_size_new` (~64 + ~60 lines)

- [ ] **Step 5: Extract `encode_optional_bytes` helper**

The key/value/header encoding follows the same pattern. Extract:
```rust
fn encode_optional_bytes<B: ByteBufMut>(buf: &mut B, data: Option<&[u8]>, field_name: &'static str) -> Result<()>
fn compute_optional_bytes_size(data: Option<&[u8]>, field_name: &'static str) -> Result<usize>
```

- [ ] **Step 6: Extract `encode_headers` / `compute_headers_size`**

```rust
fn encode_headers<B: ByteBufMut>(buf: &mut B, headers: &IndexMap<StrBytes, Option<Bytes>>) -> Result<()>
fn compute_headers_size(headers: &IndexMap<StrBytes, Option<Bytes>>) -> Result<usize>
```

- [ ] **Step 7: Verify tests pass**

### 7c: Split `Record::decode_new` (~67 lines)

- [ ] **Step 8: Extract `decode_optional_bytes`**

```rust
fn decode_optional_bytes<B: ByteBuf>(buf: &mut B, field_name: &'static str) -> Result<Option<Bytes>>
```

- [ ] **Step 9: Extract `decode_headers`**

```rust
fn decode_headers<B: ByteBuf>(buf: &mut B) -> Result<IndexMap<StrBytes, Option<Bytes>>>
```

- [ ] **Step 10: Verify all tests pass**

Run: `cargo test --lib --all-features`

- [ ] **Step 11: Commit**

```
git add src/records.rs
git commit -m "refactor: decompose long functions in records.rs (Rule 4, max 60 lines)"
```

---

## Task 8: Add Assertions (Rule 5)

**Depends on:** Task 7

**Files:**
- Modify: `src/records.rs`
- Modify: `src/protocol/buf.rs`
- Modify: `src/protocol/mod.rs`
- Modify: `src/compression/snappy.rs`

**Exemption criteria:** Functions that are ≤5 lines of code and consist entirely of delegation (calling another function and returning its result) are exempt from the 2-assertion requirement. This includes all the `Encoder`/`Decoder` forwarding impls in `types.rs` and simple trait method delegations. Rationale: adding 2 assertions to a 3-line function would triple its size, conflicting with Rule 4.

- [ ] **Step 1: Add assertions to record encoding/decoding functions**

Every non-trivial function in `records.rs` gets ≥2 assertions:

```rust
// encode_new_batch — precondition
debug_assert!(options.version == 2, "only record batch v2 is supported");
// encode_new_batch — postcondition
debug_assert!(batch_end > batch_start, "batch must contain data");

// decode_batch_info — precondition
debug_assert!(version == 2, "only record batch v2 is supported");
// decode_batch_info — postcondition
debug_assert!(batch_decode_info.record_count <= i32::MAX as usize);
```

- [ ] **Step 2: Add assertions to buffer functions**

```rust
// SegmentedBuf::put_shared_bytes — precondition (already checks is_empty)
debug_assert!(self.total_len == self.segments.iter().map(|s| s.len()).sum::<usize>());

// SegmentedBuf::chunk — postcondition
// (return value is a subslice of a segment, bounds checked by the loop)
```

- [ ] **Step 3: Add assertions to compression functions**

```rust
// Snappy::compress — postcondition
debug_assert!(compressed.offset() > MAGIC_HEADER.len(), "compressed output must contain header");

// Snappy::decompress — precondition
debug_assert!(compressed.has_remaining(), "input must not be empty");
```

- [ ] **Step 4: Verify all tests pass**

Run: `cargo test --lib --all-features`

- [ ] **Step 5: Commit**

```
git add src/
git commit -m "chore: add precondition/postcondition assertions (Rule 5)"
```

---

## Task 9: Final Clippy Compliance (Rule 12)

**Files:**
- Potentially any hand-written file

- [ ] **Step 1: Run full clippy with pedantic**

Run: `cargo clippy --lib --all-features -- -D warnings -W clippy::pedantic`

- [ ] **Step 2: Fix all warnings**

Address each clippy::pedantic finding. Common ones:
- Missing docs on public items
- Needless pass by value
- Trivially copy types taken by reference
- Must use `Self` instead of type name in impl blocks

- [ ] **Step 3: Run cargo hack clippy for feature powerset**

Run: `cargo hack --feature-powerset clippy --all-targets --locked -- -D warnings`

- [ ] **Step 4: Commit**

```
git add .
git commit -m "chore: fix all clippy pedantic warnings (Rule 12)"
```

---

## Execution Order and Dependencies

```
Task 1 (lints) ──────────────────────────────────────────┐
Task 2 (define ProtoError) ──┬── Task 3 (protocol/)  ────┤
                             ├── Task 4 (compression/) ───┤
                             └── Task 5 (records.rs) ─────┤
                                                    │     ├── Task 6 (codegen + regenerate)
                                Task 7 (decompose) ──┘     │
                                         │                 │
                                Task 8 (assertions) ───────┤
                                                           └── Task 9 (final clippy)
```

- Tasks 3 and 4 can run in parallel after Task 2.
- Task 5 must complete before Task 7 (both modify `records.rs`).
- Task 6 depends on Tasks 3, 4, and 5 being complete.
- Task 7 depends on Task 5.
- Task 8 depends on Task 7.
- Task 9 is last (after all other tasks).

# Divergences

Classified differences found by verification. Source of truth: Kafka 4.1.0 Java
(golden vectors in `verification/golden/vectors`). Details and evidence: `docs/FINDINGS.md`.

## Spec stricter than Java (deliberate)

| # | Input | Java | Spec | Strict variant |
|---|---|---|---|---|
| S1 | Length prefix < -1 on a nullable field / record key/value | null | reject | `S1NegativeLength` |
| S2 | Invalid UTF-8 in a string or record header key | lossy U+FFFD | reject | `S2InvalidUtf8` |
| S3 | Tagged fields duplicated / not strictly increasing | accepted | reject | `S3TagOrder` |
| S4 | Known tagged field value length != declared size | size ignored | reject | `S4TagSize` |
| S5 | Final varint/varlong byte with bits Java drops | dropped | reject | `S5VarintHighBits` |
| ~~S6~~ | Record batch magic not in {0,1,2} | **rejected** (`Invalid magic found in record`) | reject (`SpecError::Records`) | retracted, see below |
| S7 | Tagged-field count >= 2^31 | `int` count is negative, loop skipped: accepted as empty | reject | `S7TagCountOverflow` |

S4 includes an array inside a known tag that overruns the tag's `size` window (Java checks
array lengths against the whole buffer). `Mode::JavaLenient` (`interp::decode_with`,
`records::decode_with`) accepts every S1-S5/S7 input exactly as Java does; the golden tests
use it to check Java's value on every Java-accepted vector.

S6 was retracted in Task 7. Java's wire path (`MemoryRecords.batches()` ->
`ByteBufferLogInputStream.nextBatchSize`, `ByteBufferLogInputStream.java:79-84`) throws
`CorruptRecordException` for magic < 0 or > 2, so this is not a divergence. Only the
broker's on-disk reader (`FileLogInputStream.java:83-89`) treats magic >= 2 as v2. The
number stays reserved so S1-S5 keep their names.

Strict-rejection counts from the last golden run (asserted exactly by the golden tests):
`checked 11760 message vectors; strict rejections where Java accepts: {"S1NegativeLength": 45, "S2InvalidUtf8": 439, "S3TagOrder": 1, "S4TagSize": 6}`
and `checked 533 record vectors {"mutated": 246, "mutated_crc_fixed": 246, "valid": 41}; strict rejections where Java accepts: {"S1NegativeLength": 8, "S2InvalidUtf8": 2}; opaque compressed accepted where Java rejects: 1`

## Spec more lenient than Java (documented exception)

| # | Input | Java | Spec |
|---|---|---|---|
| C1 | Compressed payloads: v2 batch with codec 1-4 and count > 0; legacy wrapper with codec 1-3 | decompresses and parses inner records; rejects corrupt contents | opaque (`Payload::Compressed` / wrapper value kept as is); contents not validated |

C1 is a design non-goal (Phase 1 does not model compression). It is the only case where the
spec accepts what Java rejects. The golden records test counts these separately (1 vector:
codec 4 with a garbage payload, "Failed to decompress record stream").

## Spec bugs found by the golden vectors (fixed)

| # | Area | Java | Spec before fix | Fix |
|---|---|---|---|---|
| G1 | Record set: invalid magic in a truncated final batch | rejects once 17 bytes (`HEADER_SIZE_UP_TO_MAGIC`) are present, before checking the batch is complete | returned the bytes as `truncated_tail` | `records::decode` checks magic before the completeness check (5 golden vectors) |
| G2 | v2 batch with count 0 / codec 5-7 | count 0: empty, trailing bytes and codec ignored; count > 0 with codec 5-7: rejected (`CompressionType.forId`) | count 0 with trailing bytes rejected; codec 5-7 accepted as compressed | `records::decode_v2` follows `DefaultRecordBatch.iterator()` order |
| G3 | Legacy message codec bits | codec 5-7 rejected (`forId`), ZSTD wrapper rejected, compressed wrapper with null value rejected | codec bits ignored, all accepted | `records::decode_legacy` checks them |

## Crate (proto-kafka) vs. Java

| # | Area | Status |
|---|---|---|
| D1 | `UnsignedVarInt::decode` accepts a 5th byte with continuation, misframing the stream | fixed in Phase 2 |
| D2 | Generator rule from f0b0238 defaulted every nullable field to null; Java defaults to the type default (`""`, empty) unless `"default": "null"` | fixed in Phase 2 (generator rule reverted to Java's) |
| D3 | `CompactString` decoders accept strings longer than 0x7fff; Java rejects them | fixed in Phase 2 |
| D4 | `OptionStruct` treats only marker `1` as present; Java treats any non-negative marker as present | fixed in Phase 2 |
| D5 | Arrays, bytes, strings and record headers pre-allocate by an attacker-controlled length (DoS); Java bounds arrays by remaining bytes | fixed in Phase 2 |
| D6 | Per-record `sequence` was `base_sequence.wrapping_add(offset_delta)`. Java (`DefaultRecord.java:325-327`) gives `NO_SEQUENCE` when `baseSequence < 0`, and otherwise uses `incrementSequence`, which wraps past `i32::MAX` to 0. The encoder grouped records by `offset - sequence`, so Java-style records without a sequence went one per batch | fixed (`records::record_sequence`, Kani `record_sequence_conforms`) |
| D7 | Record headers were an `IndexMap`: a duplicate key dropped the earlier header (release) or panicked a `debug_assert!` (debug). Java's `Headers` is an ordered list. Hit by a `valid` golden vector | fixed (`Record::headers` is `Vec<(StrBytes, Option<Bytes>)>`; breaking) |
| D8 | Encoder wrote the minimum record timestamp as `baseTimestamp`. Java's `MemoryRecordsBuilder` (`:471-472`) uses the first record's timestamp. Both decode to the same records, but the bytes and CRC differ | fixed (6 valid vectors now re-encode identically) |
| D9 | A record set ending in an incomplete batch: Java (`ByteBufferLogInputStream.nextBatch`) silently drops the tail, and brokers send such tails in Fetch responses. The crate's `decode_all` / `decode_batch_info` fail on the whole set | **open**: counted as an allowed strictness (118 vectors) in `tests/golden_crate_records.rs` |
| D10 | Record `timestampDelta` decoded and encoded as varint; Java uses varlong | fixed (upstream #159, `f105d6c`) |
| D11 | Batch attribute bit 6 (KIP-534 delete horizon) was dropped on decode; re-encoding lost the horizon | fixed (`Record::delete_horizon_ms`, `2af1812`) |

`tests/golden_crate.rs` checks the crate's request/response/header decoders and encoders on
every golden message vector: 11,560 checked, 0 known divergences
(`verification/crate_known_divergences.txt` is empty). It also skips 200 vectors, listed
under Coverage notes.

`tests/golden_crate_records.rs` checks the crate's record batch decoder and encoder on every
golden record vector, with the outcome histogram asserted exactly:
`{"crate_rejects_spec_strict_rejects": 2, "crate_rejects_truncated_tail": 118, "java_accepts_crate_matches": 56, "java_rejects_crate_rejects": 233, "skipped_legacy": 124, "valid_reencoded_identically": 22}`.
Every Java-accepted v2 set decodes to Java's batch and record values, including each record's
`sequence` and the batch's `deleteHorizonMs`, unless the crate rejects with an allowed
strictness (S1/S2, or D9). All 22 valid v2 sets re-encode to Java's exact bytes. Legacy
v0/v1 sets are skipped because the crate rejects them by design. D6-D9 were found by this
test and by the Java vector built for D11.

## Coverage notes

- Golden messages never set records fields to non-empty values (Java's JsonConverter is
  lossy for records). Record contents are covered by `records.jsonl`.
- Golden records contain no valid compressed batches and no LOG_APPEND_TIME batches.
- `records.jsonl` records each record's `sequence` and each batch's `deleteHorizonMs`. Three
  fixed-content v2 sets are appended after the random ones, with their own `Random`, so the
  earlier 494 vectors are unchanged: an idempotent batch whose sequences wrap past `i32::MAX`,
  and delete-horizon batches without and with a producer sequence.
- Record mutations come in two kinds: `mutated` (CRC left broken, so Java mostly stops at
  `ensureValid()`) and `mutated_crc_fixed` (CRCs re-stamped, so Java reaches
  `DefaultRecord.readFrom` and legacy key/value parsing). Neither produced a
  "records still remaining" case; that path is covered by unit tests only.
- Random messages never set nullable fields to null. Null coverage comes from schema
  defaults (109 `"default": "null"` fields) and from mutations.
- Mutated vectors that Java accepts (2,869 messages, 242 record sets) are compared with Java
  through the `JavaLenient` decode (and the strict decode must equal it or be a `Strict`
  rejection), but not re-encoded, since a mutation need not be canonical.
- The 9 `Kind::Data` schemas (ConsumerProtocolAssignment, ConsumerProtocolSubscription,
  DefaultPrincipalData, EndTxnMarker, KRaftVersionRecord, LeaderChangeMessage,
  SnapshotFooterRecord, SnapshotHeaderRecord, VotersRecord) have no golden vectors; they are
  only exercised through the schema tables (`spec/tests/generated.rs`).
- The encode path for unknown tagged fields is not golden-checked: Java-generated valid
  vectors never contain unknown tags, so no vector with unknown tags is re-encoded.
- Tagged-field defaults are compared with `Value` equality, which compares float64 bitwise;
  Java's write rule compares a primitive `double` with `!=` (`FieldSpec.java:638-639`, so `-0.0 == 0.0` and `NaN != NaN`). Latent only:
  no 4.1.0 schema has a tagged float64 field (the two float64 fields are untagged).
- Unstable API versions are not crate-checked. Kafka 4.1.0 marks the last version of
  InitProducerIdRequest (v6), OffsetCommitRequest (v10), OffsetFetchRequest (v10),
  StreamsGroupHeartbeatRequest (v0) and StreamsGroupDescribeRequest (v0) as
  `"latestVersionUnstable": true`. The golden generator includes them
  (`GoldenGen.java:63,73`, `highestSupportedVersion(true)`), but the crate's generator
  drops them on purpose (`protocol_codegen/src/generate_messages/generate.rs:1403-1414`), so
  API keys 88 and 89 have no `ApiKey` variant. `tests/golden_crate.rs` skips these 200
  request/response vectors and counts them separately. This is by design, not a divergence:
  generating them would raise `VERSIONS.max`, and blink advertises
  `OffsetCommitRequest::VERSIONS.max` / `OffsetFetchRequest::VERSIONS.max` to clients.
  The spec still covers them through `spec/tests` golden runs.
- Tagged nullable struct fields are not supported (Java reads a varint presence marker); the
  schema-table emitter refuses them and none exist in 4.1.0.

## Tooling notes

- CI (`.github/workflows/build-and-test.yml`) runs `cargo test --workspace`, so the spec's
  tests run, but its clippy job (`cargo hack ... clippy` at the root package) and its fmt job
  (`protocol_codegen` only) do not cover `spec/`. Run
  `cargo clippy -p proto-kafka-spec --all-targets --locked -- -D warnings` and
  `cargo fmt -p proto-kafka-spec --check` locally. Adding them to CI is scheduled for Phase 5;
  the workflows are not changed in Phase 1.

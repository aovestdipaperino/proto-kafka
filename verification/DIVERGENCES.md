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

S6 was retracted in Task 7. Java's wire path (`MemoryRecords.batches()` ->
`ByteBufferLogInputStream.nextBatchSize`, `ByteBufferLogInputStream.java:79-84`) throws
`CorruptRecordException` for magic < 0 or > 2, so this is not a divergence. Only the
broker's on-disk reader (`FileLogInputStream.java:83-89`) treats magic >= 2 as v2. The
number stays reserved so S1-S5 keep their names.

Strict-rejection counts from the last golden run:
`checked 11760 message vectors; strict rejections where Java accepts: {"S1NegativeLength": 45, "S2InvalidUtf8": 439, "S3TagOrder": 1, "S4TagSize": 6}`
and `checked 266 record vectors; strict rejections where Java accepts: {}`

## Spec bugs found by the golden vectors (fixed)

| # | Area | Java | Spec before fix | Fix |
|---|---|---|---|---|
| G1 | Record set: invalid magic in a truncated final batch | rejects once 17 bytes (`HEADER_SIZE_UP_TO_MAGIC`) are present, before checking the batch is complete | returned the bytes as `truncated_tail` | `records::decode` checks magic before the completeness check (5 golden vectors) |
| G2 | v2 batch with count 0 / codec 5-7 | count 0: empty, trailing bytes and codec ignored; count > 0 with codec 5-7: rejected (`CompressionType.forId`) | count 0 with trailing bytes rejected; codec 5-7 accepted as compressed | `records::decode_v2` follows `DefaultRecordBatch.iterator()` order |
| G3 | Legacy message codec bits | codec 5-7 rejected (`forId`), ZSTD wrapper rejected, compressed wrapper with null value rejected | codec bits ignored, all accepted | `records::decode_legacy` checks them |

## Crate (proto-kafka) vs. Java

| # | Area | Status |
|---|---|---|
| D1 | `UnsignedVarInt::decode` accepts a 5th byte with continuation, misframing the stream | open, fixed in Phase 2 |
| D2 | Committed generated code in `src/` is stale: nullable fields defaulting to null still default to `Some("")` in 55 files | open, regenerate in Phase 2 |

## Coverage notes

- Golden messages never set records fields to non-empty values (Java's JsonConverter is
  lossy for records). Record contents are covered by `records.jsonl`.
- Golden records contain no compressed batches and no LOG_APPEND_TIME batches.
- Random messages never set nullable fields to null. Null coverage comes from schema
  defaults (109 `"default": "null"` fields) and from mutations.
- Mutated vectors that Java accepts (2,869 messages, 97 record sets) are compared on value
  and consumed length but not re-encoded, since a mutation need not be canonical.

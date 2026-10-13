use proto_kafka_spec::error::{SpecError, Strict};
use proto_kafka_spec::records::{decode, decode_with, Entry, Mode, Payload};
use proto_kafka_spec::wire::Writer;

fn opt(w: &mut Writer, b: Option<&[u8]>) {
    match b {
        None => w.varint(-1),
        Some(b) => {
            w.varint(b.len() as i32);
            w.bytes(b);
        }
    }
}

fn record(
    offset_delta: i32,
    key: Option<&[u8]>,
    value: Option<&[u8]>,
    headers: &[(&[u8], Option<&[u8]>)],
) -> Vec<u8> {
    let mut b = Writer::new();
    b.i8(0);
    b.varlong(5);
    b.varint(offset_delta);
    opt(&mut b, key);
    opt(&mut b, value);
    b.varint(headers.len() as i32);
    for (k, v) in headers {
        b.varint(k.len() as i32);
        b.bytes(k);
        opt(&mut b, *v);
    }
    let body = b.into_bytes();
    let mut w = Writer::new();
    w.varint(body.len() as i32);
    w.bytes(&body);
    w.into_bytes()
}

fn v2_batch_with_magic(magic: i8, records: &[Vec<u8>], attributes: i16) -> Vec<u8> {
    let mut body = Writer::new();
    body.i16(attributes);
    body.i32(records.len() as i32 - 1);
    body.i64(1000);
    body.i64(1005);
    body.i64(-1);
    body.i16(-1);
    body.i32(-1);
    body.i32(records.len() as i32);
    for r in records {
        body.bytes(r);
    }
    let body = body.into_bytes();
    let mut w = Writer::new();
    w.i64(42);
    w.i32((4 + 1 + 4 + body.len()) as i32);
    w.i32(7);
    w.i8(magic);
    w.u32(crc32c::crc32c(&body));
    w.bytes(&body);
    w.into_bytes()
}

fn v2_batch(records: &[Vec<u8>]) -> Vec<u8> {
    v2_batch_with_magic(2, records, 0)
}

fn legacy_v1(key: Option<&[u8]>, value: &[u8]) -> Vec<u8> {
    legacy(1, 0, key, Some(value))
}

fn legacy_len(w: &mut Writer, b: Option<&[u8]>) {
    match b {
        None => w.i32(-1),
        Some(b) => {
            w.i32(b.len() as i32);
            w.bytes(b);
        }
    }
}

fn legacy(magic: i8, attributes: i8, key: Option<&[u8]>, value: Option<&[u8]>) -> Vec<u8> {
    let mut body = Writer::new();
    body.i8(magic);
    body.i8(attributes);
    if magic == 1 {
        body.i64(1234);
    }
    legacy_len(&mut body, key);
    legacy_len(&mut body, value);
    legacy_frame(body.into_bytes())
}

/// Offset 9, size, CRC32 over `body` (magic .. end), then `body`.
fn legacy_frame(body: Vec<u8>) -> Vec<u8> {
    let mut w = Writer::new();
    w.i64(9);
    w.i32((4 + body.len()) as i32);
    w.u32(crc32fast::hash(&body));
    w.bytes(&body);
    w.into_bytes()
}

#[test]
fn v2_batch_fields_and_records() {
    let b = v2_batch(&[
        record(0, Some(b"k"), Some(b"v"), &[(b"h", None)]),
        record(1, None, Some(b"w"), &[]),
    ]);
    let set = decode(&b).unwrap();
    assert_eq!(set.truncated_tail, 0);
    let Entry::V2(batch) = &set.entries[0] else {
        panic!("expected v2")
    };
    assert_eq!(
        (
            batch.base_offset,
            batch.partition_leader_epoch,
            batch.last_offset_delta
        ),
        (42, 7, 1)
    );
    let Payload::Records(recs) = &batch.payload else {
        panic!("expected records")
    };
    assert_eq!(recs.len(), 2);
    assert_eq!(recs[0].key.as_deref(), Some(&b"k"[..]));
    assert_eq!(recs[0].headers[0].key, b"h");
    assert_eq!(recs[0].headers[0].value, None);
    assert_eq!(recs[1].key, None);
    assert_eq!(recs[1].offset_delta, 1);
}

#[test]
fn v2_crc_mismatch_is_rejected() {
    let mut b = v2_batch(&[record(0, None, Some(b"v"), &[])]);
    let last = b.len() - 1;
    b[last] ^= 0xff;
    assert!(matches!(decode(&b), Err(SpecError::Records(_))));
}

#[test]
fn partial_trailing_batch_is_a_tail_not_an_error() {
    let one = v2_batch(&[record(0, None, Some(b"v"), &[])]);
    let two = [one.clone(), one[..20].to_vec()].concat();
    let set = decode(&two).unwrap();
    assert_eq!(set.entries.len(), 1);
    assert_eq!(set.truncated_tail, 20);
}

/// Java's MemoryRecords path (ByteBufferLogInputStream.nextBatchSize) throws
/// "Invalid magic found in record" for magic outside 0..=2. Not a strictness divergence.
#[test]
fn unknown_magic_is_rejected_like_java() {
    for magic in [3i8, 44, -1] {
        let b = v2_batch_with_magic(magic, &[record(0, None, Some(b"v"), &[])], 0);
        assert!(
            matches!(decode(&b), Err(SpecError::Records(_))),
            "magic {magic}"
        );
    }
}

/// Golden regression (records.jsonl, mutated): Java checks the magic byte as soon as
/// 17 bytes (HEADER_SIZE_UP_TO_MAGIC) are available, before checking that the whole
/// batch is present, so an invalid magic in a truncated tail is an error, not a tail.
#[test]
fn invalid_magic_in_truncated_tail_is_rejected_like_java() {
    let golden =
        hex("00000000000003760000ff001d3005c004010000000174877b1b6d00000004cafd5cd200000003d73349");
    assert!(matches!(decode(&golden), Err(SpecError::Records(_))));

    let one = v2_batch(&[record(0, None, Some(b"v"), &[])]);
    let mut tail = one[..20].to_vec();
    tail[16] = 7;
    assert!(matches!(
        decode(&[one.clone(), tail].concat()),
        Err(SpecError::Records(_))
    ));
    // Fewer than 17 bytes: magic is not visible yet, so it is still a tail.
    let short = one[..16].to_vec();
    let set = decode(&[one, short].concat()).unwrap();
    assert_eq!((set.entries.len(), set.truncated_tail), (1, 16));
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn record_count_mismatch_is_rejected() {
    let mut b = v2_batch(&[record(0, None, Some(b"v"), &[])]);
    // records count field is the last 4 bytes before the records; bump count to 2 and fix crc
    let count_pos = 12 + 4 + 1 + 4 + 2 + 4 + 8 + 8 + 8 + 2 + 4;
    b[count_pos + 3] = 2;
    let crc = crc32c::crc32c(&b[21..]);
    b[17..21].copy_from_slice(&crc.to_be_bytes());
    assert!(matches!(
        decode(&b),
        Err(SpecError::Eof | SpecError::Records(_))
    ));
}

#[test]
fn compressed_payload_is_opaque() {
    let b = v2_batch_with_magic(2, &[vec![1, 2, 3]], 1); // gzip attribute, garbage payload
    let Entry::V2(batch) = &decode(&b).unwrap().entries[0] else {
        panic!()
    };
    assert_eq!(batch.payload, Payload::Compressed(vec![1, 2, 3]));
}

#[test]
fn legacy_v1_message() {
    let b = legacy_v1(Some(b"k"), b"v");
    let Entry::Legacy(m) = &decode(&b).unwrap().entries[0] else {
        panic!("expected legacy")
    };
    assert_eq!((m.offset, m.magic, m.timestamp), (9, 1, Some(1234)));
    assert_eq!(m.key.as_deref(), Some(&b"k"[..]));
    assert_eq!(m.value.as_deref(), Some(&b"v"[..]));
}

#[test]
fn record_size_below_minimum_is_rejected() {
    let mut b = vec![0u8; 12];
    b[11] = 13; // size 13 < 14
    assert!(matches!(decode(&b), Err(SpecError::Records(_))));
}

/// Body (attributes .. headers) wrapped in its varint size, for hand-made malformed records.
fn raw_record(body: &[u8]) -> Vec<u8> {
    let mut w = Writer::new();
    w.varint(body.len() as i32);
    w.bytes(body);
    w.into_bytes()
}

fn record_prefix() -> Writer {
    let mut b = Writer::new();
    b.i8(0);
    b.varlong(5);
    b.varint(0);
    b
}

fn only_record(b: &[u8]) -> proto_kafka_spec::records::Record {
    let set = decode(b).unwrap();
    let Entry::V2(batch) = &set.entries[0] else {
        panic!("expected v2")
    };
    let Payload::Records(recs) = &batch.payload else {
        panic!("expected records")
    };
    recs[0].clone()
}

/// Java DefaultRecordBatch.iterator(): `if (count() == 0) return emptyIterator` comes
/// before any codec lookup or size check, so trailing bytes and any codec are accepted.
#[test]
fn empty_batch_ignores_trailing_bytes_and_codec() {
    for attributes in [0i16, 1, 7] {
        let mut b = v2_batch_with_magic(2, &[record(0, None, Some(b"v"), &[])], attributes);
        let count_pos = 12 + 4 + 1 + 4 + 2 + 4 + 8 + 8 + 8 + 2 + 4;
        b[count_pos..count_pos + 4].copy_from_slice(&0i32.to_be_bytes());
        let crc = crc32c::crc32c(&b[21..]);
        b[17..21].copy_from_slice(&crc.to_be_bytes());
        let Entry::V2(batch) = &decode(&b).unwrap().entries[0] else {
            panic!("expected v2")
        };
        assert_eq!(
            batch.payload,
            Payload::Records(vec![]),
            "attributes {attributes}"
        );
    }
}

/// Java: isCompressed() -> CompressionType.forId(5..=7) throws IllegalArgumentException.
#[test]
fn unknown_codec_with_records_is_rejected_like_java() {
    for attributes in [5i16, 6, 7] {
        let b = v2_batch_with_magic(2, &[record(0, None, Some(b"v"), &[])], attributes);
        assert!(
            matches!(decode(&b), Err(SpecError::Records(_))),
            "attributes {attributes}"
        );
    }
}

#[test]
fn v2_batch_below_overhead_is_rejected() {
    // A 60-byte "v2 batch": size field 48 (12 + 48 = 60 < 61), correct magic.
    let mut b = vec![0u8; 60];
    b[8..12].copy_from_slice(&48i32.to_be_bytes());
    b[16] = 2;
    assert!(matches!(decode(&b), Err(SpecError::Records(_))));
}

#[test]
fn tail_shorter_than_log_overhead_is_ignored() {
    let one = v2_batch(&[record(0, None, Some(b"v"), &[])]);
    for n in [0usize, 1, 11] {
        let set = decode(&[one.clone(), vec![0xab; n]].concat()).unwrap();
        assert_eq!((set.entries.len(), set.truncated_tail), (1, n));
    }
}

#[test]
fn mixed_v2_and_legacy_entries() {
    let b = [
        v2_batch(&[record(0, None, Some(b"v"), &[])]),
        legacy(0, 0, None, Some(b"x")),
        legacy_v1(Some(b"k"), b"v"),
    ]
    .concat();
    let set = decode(&b).unwrap();
    assert!(matches!(
        set.entries.as_slice(),
        [Entry::V2(_), Entry::Legacy(a), Entry::Legacy(c)] if a.magic == 0 && c.magic == 1
    ));
}

#[test]
fn legacy_v0_message_has_no_timestamp() {
    let Entry::Legacy(m) = &decode(&legacy(0, 0, Some(b"k"), Some(b"v")))
        .unwrap()
        .entries[0]
    else {
        panic!("expected legacy")
    };
    assert_eq!((m.magic, m.timestamp), (0, None));
    assert_eq!(
        (m.key.as_deref(), m.value.as_deref()),
        (Some(&b"k"[..]), Some(&b"v"[..]))
    );
}

#[test]
fn legacy_null_key_and_value() {
    let Entry::Legacy(m) = &decode(&legacy(1, 0, None, None)).unwrap().entries[0] else {
        panic!("expected legacy")
    };
    assert_eq!((m.key.clone(), m.value.clone()), (None, None));
}

#[test]
fn legacy_crc_mismatch_is_rejected() {
    let mut b = legacy_v1(Some(b"k"), b"v");
    let last = b.len() - 1;
    b[last] ^= 0xff;
    assert!(matches!(decode(&b), Err(SpecError::Records(_))));
}

/// Java LegacyRecord: keySize()/valueSize() < 0 is null (Utils.sizeDelimited), so -2 is
/// accepted by Java as null. The spec is stricter (S1).
#[test]
fn legacy_negative_length_below_minus_one_is_strict_s1() {
    let mut body = Writer::new();
    body.i8(1);
    body.i8(0);
    body.i64(1234);
    body.i32(-2);
    body.i32(-1);
    let b = legacy_frame(body.into_bytes());
    assert_eq!(decode(&b), Err(SpecError::Strict(Strict::S1NegativeLength)));
}

/// Java AbstractLegacyRecordBatch.iterator(): CompressionType.forId(5..=7) throws;
/// DeepRecordsIterator rejects a ZSTD wrapper and a wrapper with a null value.
#[test]
fn legacy_codec_checks_follow_java() {
    for attributes in [4i8, 5, 6, 7] {
        let b = legacy(1, attributes, None, Some(b"zz"));
        assert!(
            matches!(decode(&b), Err(SpecError::Records(_))),
            "attributes {attributes}"
        );
    }
    assert!(matches!(
        decode(&legacy(1, 1, None, None)),
        Err(SpecError::Records(_))
    ));
    // gzip wrapper with a value: opaque, accepted (Java would decompress it).
    assert!(decode(&legacy(1, 1, None, Some(b"zz"))).is_ok());
}

/// Java DefaultRecord.readFrom reads key/value via Utils.readBytes, which returns null for
/// any negative size. The spec is stricter (S1).
#[test]
fn record_key_length_below_minus_one_is_strict_s1() {
    let mut body = record_prefix();
    body.varint(-2);
    body.varint(-1);
    body.varint(0);
    let b = v2_batch(&[raw_record(&body.into_bytes())]);
    assert_eq!(decode(&b), Err(SpecError::Strict(Strict::S1NegativeLength)));
}

/// Java decodes header keys with Utils.utf8 (lossy). The spec is stricter (S2).
#[test]
fn record_header_key_invalid_utf8_is_strict_s2() {
    let b = v2_batch(&[record(0, None, None, &[(&[0xff, 0xfe], None)])]);
    assert_eq!(decode(&b), Err(SpecError::Strict(Strict::S2InvalidUtf8)));
}

/// Java DefaultRecord.readFrom: `numHeaders < 0` throws InvalidRecordException.
#[test]
fn negative_header_count_is_rejected() {
    let mut body = record_prefix();
    body.varint(-1);
    body.varint(-1);
    body.varint(-1);
    let b = v2_batch(&[raw_record(&body.into_bytes())]);
    assert!(matches!(decode(&b), Err(SpecError::Records(_))));
}

/// Java DefaultRecord.readFrom: bytes read != sizeOfBodyInBytes throws.
#[test]
fn record_body_size_mismatch_is_rejected() {
    let mut body = record_prefix();
    body.varint(-1);
    body.varint(-1);
    body.varint(0);
    body.u8(0x00); // one byte the record body declares but does not use
    let b = v2_batch(&[raw_record(&body.into_bytes())]);
    assert!(matches!(decode(&b), Err(SpecError::Records(_))));
    // Sanity: without the extra byte it decodes.
    let mut ok = record_prefix();
    ok.varint(-1);
    ok.varint(-1);
    ok.varint(0);
    assert_eq!(
        only_record(&v2_batch(&[raw_record(&ok.into_bytes())])).key,
        None
    );
}

/// JavaLenient: negative record/legacy lengths are null, header keys are not UTF-8 checked.
#[test]
fn java_lenient_record_lengths_and_header_keys() {
    let mut body = record_prefix();
    body.varint(-2);
    body.varint(-7);
    body.varint(1);
    body.varint(2);
    body.bytes(&[0xff, 0xfe]);
    body.varint(-3);
    let b = v2_batch(&[raw_record(&body.into_bytes())]);
    let set = decode_with(&b, Mode::JavaLenient).unwrap();
    let Entry::V2(batch) = &set.entries[0] else {
        panic!()
    };
    let Payload::Records(recs) = &batch.payload else {
        panic!()
    };
    assert_eq!((recs[0].key.clone(), recs[0].value.clone()), (None, None));
    assert_eq!(recs[0].headers[0].key, vec![0xff, 0xfe]);
    assert_eq!(recs[0].headers[0].value, None);

    let mut legacy_body = Writer::new();
    legacy_body.i8(0);
    legacy_body.i8(0);
    legacy_body.i32(-2);
    legacy_body.i32(-9);
    let b = legacy_frame(legacy_body.into_bytes());
    let Entry::Legacy(m) = &decode_with(&b, Mode::JavaLenient).unwrap().entries[0] else {
        panic!()
    };
    assert_eq!((m.key.clone(), m.value.clone()), (None, None));
}

use bytes::{Bytes, BytesMut};
use indexmap::IndexMap;
use kafka_protocol::protocol::StrBytes;
use kafka_protocol::records::{
    Compression, Record, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions,
    TimestampType, NO_PARTITION_LEADER_EPOCH, NO_PRODUCER_EPOCH, NO_PRODUCER_ID, NO_SEQUENCE,
    NO_TIMESTAMP,
};
use string::TryFrom;

fn make_record(offset: i64, key: &str, value: &str) -> Record {
    Record {
        transactional: false,
        control: false,
        partition_leader_epoch: NO_PARTITION_LEADER_EPOCH,
        producer_id: NO_PRODUCER_ID,
        producer_epoch: NO_PRODUCER_EPOCH,
        timestamp_type: TimestampType::Creation,
        offset,
        sequence: NO_SEQUENCE,
        timestamp: 1000 + offset,
        key: Some(Bytes::from(key.to_owned())),
        value: Some(Bytes::from(value.to_owned())),
        headers: IndexMap::new(),
    }
}

#[test]
fn roundtrip_v2_uncompressed_single() {
    let record = make_record(0, "key1", "value1");
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, std::iter::once(&record), &options).unwrap();
    assert!(!buf.is_empty(), "encoded output must not be empty");

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].key, record.key);
    assert_eq!(decoded[0].value, record.value);
    assert_eq!(decoded[0].offset, 0);
}

#[test]
fn roundtrip_v2_uncompressed_multiple() {
    let records: Vec<Record> = (0..5)
        .map(|i| make_record(i, &format!("k{i}"), &format!("v{i}")))
        .collect();
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, records.iter(), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 5);
    for (i, rec) in decoded.iter().enumerate() {
        assert_eq!(
            rec.key.as_ref().unwrap().as_ref(),
            format!("k{i}").as_bytes()
        );
        assert_eq!(
            rec.value.as_ref().unwrap().as_ref(),
            format!("v{i}").as_bytes()
        );
    }
}

#[test]
fn roundtrip_v2_null_key_and_value() {
    let record = Record {
        transactional: false,
        control: false,
        partition_leader_epoch: NO_PARTITION_LEADER_EPOCH,
        producer_id: NO_PRODUCER_ID,
        producer_epoch: NO_PRODUCER_EPOCH,
        timestamp_type: TimestampType::Creation,
        offset: 0,
        sequence: NO_SEQUENCE,
        timestamp: 1000,
        key: None,
        value: None,
        headers: IndexMap::new(),
    };
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, std::iter::once(&record), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 1);
    assert!(decoded[0].key.is_none());
    assert!(decoded[0].value.is_none());
}

#[test]
fn roundtrip_v2_with_headers() {
    let mut headers = IndexMap::new();
    headers.insert(
        StrBytes::try_from(Bytes::from("hdr-key")).unwrap(),
        Some(Bytes::from("hdr-val")),
    );
    headers.insert(
        StrBytes::try_from(Bytes::from("hdr-null")).unwrap(),
        None,
    );

    let record = Record {
        transactional: false,
        control: false,
        partition_leader_epoch: NO_PARTITION_LEADER_EPOCH,
        producer_id: NO_PRODUCER_ID,
        producer_epoch: NO_PRODUCER_EPOCH,
        timestamp_type: TimestampType::Creation,
        offset: 0,
        sequence: NO_SEQUENCE,
        timestamp: 1000,
        key: Some(Bytes::from("k")),
        value: Some(Bytes::from("v")),
        headers,
    };
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, std::iter::once(&record), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].headers.len(), 2);
}

#[test]
fn roundtrip_v2_transactional() {
    let record = Record {
        transactional: true,
        control: false,
        partition_leader_epoch: 5,
        producer_id: 42,
        producer_epoch: 3,
        timestamp_type: TimestampType::Creation,
        offset: 10,
        sequence: 7,
        timestamp: 2000,
        key: Some(Bytes::from("tk")),
        value: Some(Bytes::from("tv")),
        headers: IndexMap::new(),
    };
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, std::iter::once(&record), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 1);
    assert!(decoded[0].transactional);
    assert!(!decoded[0].control);
    assert_eq!(decoded[0].producer_id, 42);
    assert_eq!(decoded[0].producer_epoch, 3);
}

#[test]
fn roundtrip_v2_gzip() {
    let records: Vec<Record> = (0..3)
        .map(|i| make_record(i, &format!("k{i}"), &format!("v{i}")))
        .collect();
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::Gzip,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, records.iter(), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 3);
    for (i, rec) in decoded.iter().enumerate() {
        assert_eq!(rec.key.as_ref().unwrap().as_ref(), format!("k{i}").as_bytes());
    }
}

#[test]
fn roundtrip_v2_snappy() {
    let records: Vec<Record> = (0..3)
        .map(|i| make_record(i, &format!("k{i}"), &format!("v{i}")))
        .collect();
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::Snappy,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, records.iter(), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 3);
    for (i, rec) in decoded.iter().enumerate() {
        assert_eq!(rec.key.as_ref().unwrap().as_ref(), format!("k{i}").as_bytes());
    }
}

#[test]
fn roundtrip_v1_uncompressed() {
    let record = make_record(0, "legacy-key", "legacy-val");
    let options = RecordEncodeOptions {
        version: 1,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, std::iter::once(&record), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].key.as_ref().unwrap().as_ref(), b"legacy-key");
    assert_eq!(decoded[0].value.as_ref().unwrap().as_ref(), b"legacy-val");
}

#[test]
fn roundtrip_v0_uncompressed() {
    let record = Record {
        transactional: false,
        control: false,
        partition_leader_epoch: NO_PARTITION_LEADER_EPOCH,
        producer_id: NO_PRODUCER_ID,
        producer_epoch: NO_PRODUCER_EPOCH,
        timestamp_type: TimestampType::Creation,
        offset: 0,
        sequence: NO_SEQUENCE,
        timestamp: NO_TIMESTAMP,
        key: Some(Bytes::from("v0key")),
        value: Some(Bytes::from("v0val")),
        headers: IndexMap::new(),
    };
    let options = RecordEncodeOptions {
        version: 0,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, std::iter::once(&record), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].key.as_ref().unwrap().as_ref(), b"v0key");
}

#[test]
fn roundtrip_v1_gzip() {
    let records: Vec<Record> = (0..2)
        .map(|i| make_record(i, &format!("lk{i}"), &format!("lv{i}")))
        .collect();
    let options = RecordEncodeOptions {
        version: 1,
        compression: Compression::Gzip,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, records.iter(), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 2);
}

#[test]
fn roundtrip_v1_snappy() {
    let records: Vec<Record> = (0..2)
        .map(|i| make_record(i, &format!("sk{i}"), &format!("sv{i}")))
        .collect();
    let options = RecordEncodeOptions {
        version: 1,
        compression: Compression::Snappy,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, records.iter(), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 2);
}

#[test]
fn encode_invalid_version_returns_error() {
    let record = make_record(0, "k", "v");
    let options = RecordEncodeOptions {
        version: 99,
        compression: Compression::None,
    };

    // In debug builds the `debug_assert!` inside encode() panics; in release it
    // returns an Err.  Either outcome proves the invalid version is rejected.
    let outcome = std::panic::catch_unwind(|| {
        let mut buf = BytesMut::new();
        RecordBatchEncoder::encode(&mut buf, std::iter::once(&record), &options)
    });
    match outcome {
        Err(_panic) => { /* debug_assert! fired — version correctly rejected */ }
        Ok(result) => {
            assert!(result.is_err());
            assert!(result.unwrap_err().is_invalid_data());
        }
    }
}

#[test]
fn encode_unsupported_compression_returns_error() {
    let record = make_record(0, "k", "v");
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::Lz4,
    };

    let mut buf = BytesMut::new();
    let result = RecordBatchEncoder::encode(&mut buf, std::iter::once(&record), &options);
    assert!(result.is_err());
}

#[test]
fn decode_truncated_buffer_returns_error() {
    let record = make_record(0, "k", "v");
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, std::iter::once(&record), &options).unwrap();

    let truncated = Bytes::from(buf[..10].to_vec());
    let result = RecordBatchDecoder::decode(&mut truncated.clone());
    assert!(result.is_err());
}

#[test]
fn decode_corrupted_crc_returns_error() {
    let record = make_record(0, "k", "v");
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, std::iter::once(&record), &options).unwrap();

    let mut corrupted = buf.to_vec();
    if corrupted.len() > 30 {
        corrupted[30] ^= 0xFF;
    }
    let result = RecordBatchDecoder::decode(&mut Bytes::from(corrupted));
    assert!(result.is_err());
}

#[test]
fn encode_legacy_with_headers_returns_error() {
    let mut headers = IndexMap::new();
    headers.insert(
        StrBytes::try_from(Bytes::from("h")).unwrap(),
        Some(Bytes::from("v")),
    );
    let record = Record {
        transactional: false,
        control: false,
        partition_leader_epoch: NO_PARTITION_LEADER_EPOCH,
        producer_id: NO_PRODUCER_ID,
        producer_epoch: NO_PRODUCER_EPOCH,
        timestamp_type: TimestampType::Creation,
        offset: 0,
        sequence: NO_SEQUENCE,
        timestamp: 1000,
        key: Some(Bytes::from("k")),
        value: Some(Bytes::from("v")),
        headers,
    };
    let options = RecordEncodeOptions {
        version: 1,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    let result = RecordBatchEncoder::encode(&mut buf, std::iter::once(&record), &options);
    assert!(result.is_err());
}

#[test]
fn encode_legacy_transactional_returns_error() {
    let record = Record {
        transactional: true,
        control: false,
        partition_leader_epoch: NO_PARTITION_LEADER_EPOCH,
        producer_id: NO_PRODUCER_ID,
        producer_epoch: NO_PRODUCER_EPOCH,
        timestamp_type: TimestampType::Creation,
        offset: 0,
        sequence: NO_SEQUENCE,
        timestamp: 1000,
        key: Some(Bytes::from("k")),
        value: Some(Bytes::from("v")),
        headers: IndexMap::new(),
    };
    let options = RecordEncodeOptions {
        version: 0,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    let result = RecordBatchEncoder::encode(&mut buf, std::iter::once(&record), &options);
    assert!(result.is_err());
}

// ============================================================
// Kafka Protocol Nuance Tests (from KAFKA-PROTOCOL-NUANCES.md)
// ============================================================

/// Nuance Summary #1: `None` vs empty bytes are distinct on the wire.
/// `None` key/value encodes as VarInt(-1), empty `Bytes` encodes as VarInt(0).
/// These must survive roundtrip as distinct values.
#[test]
fn none_vs_empty_bytes_are_distinct() {
    let none_record = Record {
        transactional: false,
        control: false,
        partition_leader_epoch: NO_PARTITION_LEADER_EPOCH,
        producer_id: NO_PRODUCER_ID,
        producer_epoch: NO_PRODUCER_EPOCH,
        timestamp_type: TimestampType::Creation,
        offset: 0,
        sequence: NO_SEQUENCE,
        timestamp: 1000,
        key: None,
        value: None,
        headers: IndexMap::new(),
    };
    let empty_record = Record {
        transactional: false,
        control: false,
        partition_leader_epoch: NO_PARTITION_LEADER_EPOCH,
        producer_id: NO_PRODUCER_ID,
        producer_epoch: NO_PRODUCER_EPOCH,
        timestamp_type: TimestampType::Creation,
        offset: 1,
        sequence: NO_SEQUENCE,
        timestamp: 1001,
        key: Some(Bytes::new()),
        value: Some(Bytes::new()),
        headers: IndexMap::new(),
    };
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(
        &mut buf,
        [&none_record, &empty_record].iter().copied(),
        &options,
    )
    .unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 2);

    // Record at offset 0: None key/value must decode as None
    assert!(decoded[0].key.is_none(), "None key must roundtrip as None, not empty");
    assert!(decoded[0].value.is_none(), "None value must roundtrip as None, not empty");

    // Record at offset 1: empty key/value must decode as Some(empty), not None
    assert_eq!(
        decoded[1].key.as_ref().map(|b| b.len()),
        Some(0),
        "empty key must roundtrip as Some(empty), not None"
    );
    assert_eq!(
        decoded[1].value.as_ref().map(|b| b.len()),
        Some(0),
        "empty value must roundtrip as Some(empty), not None"
    );
}

/// Nuance 5c: Kafka clients always send V2 batches with baseOffset=0.
/// The encoder must handle offset 0 correctly (no off-by-one, no special-casing).
#[test]
fn v2_base_offset_zero_roundtrip() {
    let records: Vec<Record> = (0..3)
        .map(|i| make_record(i, &format!("k{i}"), &format!("v{i}")))
        .collect();
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, records.iter(), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 3);
    assert_eq!(decoded[0].offset, 0);
    assert_eq!(decoded[1].offset, 1);
    assert_eq!(decoded[2].offset, 2);
}

/// Nuance 6a: Multiple records must be packed into a single batch,
/// not one batch per record. Verify by encoding N records and checking
/// the encoded output decodes to exactly N records in sequence.
#[test]
fn multiple_records_single_batch() {
    let records: Vec<Record> = (0..10)
        .map(|i| make_record(i, &format!("k{i}"), &format!("v{i}")))
        .collect();
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, records.iter(), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 10);

    // All records should have sequential offsets
    for (i, rec) in decoded.iter().enumerate() {
        assert_eq!(rec.offset, i as i64);
    }
}

/// Nuance 6a extended: Records with different batch properties (e.g. transactional
/// vs non-transactional) MUST be split into separate batches.
#[test]
fn different_properties_produce_separate_batches() {
    let rec_a = Record {
        transactional: false,
        control: false,
        partition_leader_epoch: NO_PARTITION_LEADER_EPOCH,
        producer_id: NO_PRODUCER_ID,
        producer_epoch: NO_PRODUCER_EPOCH,
        timestamp_type: TimestampType::Creation,
        offset: 0,
        sequence: NO_SEQUENCE,
        timestamp: 1000,
        key: Some(Bytes::from("a")),
        value: Some(Bytes::from("a")),
        headers: IndexMap::new(),
    };
    let rec_b = Record {
        transactional: true, // different batch property
        control: false,
        partition_leader_epoch: NO_PARTITION_LEADER_EPOCH,
        producer_id: 42,
        producer_epoch: 1,
        timestamp_type: TimestampType::Creation,
        offset: 1,
        sequence: 0,
        timestamp: 1001,
        key: Some(Bytes::from("b")),
        value: Some(Bytes::from("b")),
        headers: IndexMap::new(),
    };
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(
        &mut buf,
        [&rec_a, &rec_b].iter().copied(),
        &options,
    )
    .unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 2);
    assert!(!decoded[0].transactional);
    assert!(decoded[1].transactional);
}

/// Nuance 6b: Encoding must not mutate the input records.
#[test]
fn encode_does_not_mutate_input() {
    let records: Vec<Record> = (0..3)
        .map(|i| make_record(i, &format!("k{i}"), &format!("v{i}")))
        .collect();
    let originals = records.clone();
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, records.iter(), &options).unwrap();

    // Input records must be unchanged after encoding
    assert_eq!(records, originals, "encode must not mutate input records");
}

/// Edge case: Encoding an empty iterator produces zero bytes,
/// and decoding zero bytes produces an empty Vec.
#[test]
fn empty_iterator_roundtrip() {
    let options = RecordEncodeOptions {
        version: 2,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    let empty: Vec<&Record> = vec![];
    RecordBatchEncoder::encode(&mut buf, empty.into_iter(), &options).unwrap();
    assert!(buf.is_empty(), "encoding zero records should produce zero bytes");

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert!(decoded.is_empty(), "decoding zero bytes should produce zero records");
}

/// Nuance Summary #1 applied to legacy format: None vs empty on v0/v1.
#[test]
fn none_vs_empty_bytes_legacy_v1() {
    let none_record = Record {
        transactional: false,
        control: false,
        partition_leader_epoch: NO_PARTITION_LEADER_EPOCH,
        producer_id: NO_PRODUCER_ID,
        producer_epoch: NO_PRODUCER_EPOCH,
        timestamp_type: TimestampType::Creation,
        offset: 0,
        sequence: NO_SEQUENCE,
        timestamp: 1000,
        key: None,
        value: Some(Bytes::from("has-value")),
        headers: IndexMap::new(),
    };
    let options = RecordEncodeOptions {
        version: 1,
        compression: Compression::None,
    };

    let mut buf = BytesMut::new();
    RecordBatchEncoder::encode(&mut buf, std::iter::once(&none_record), &options).unwrap();

    let mut decode_buf = Bytes::from(buf);
    let decoded = RecordBatchDecoder::decode(&mut decode_buf).unwrap();
    assert_eq!(decoded.len(), 1);
    assert!(decoded[0].key.is_none(), "None key must stay None in legacy format");
    assert_eq!(decoded[0].value.as_ref().unwrap().as_ref(), b"has-value");
}

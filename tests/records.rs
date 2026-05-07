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

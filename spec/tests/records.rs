use proto_kafka_spec::error::{SpecError, Strict};
use proto_kafka_spec::records::{decode, Entry, Payload};
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
    let mut body = Writer::new();
    body.i8(1);
    body.i8(0);
    body.i64(1234);
    match key {
        None => body.i32(-1),
        Some(k) => {
            body.i32(k.len() as i32);
            body.bytes(k);
        }
    }
    body.i32(value.len() as i32);
    body.bytes(value);
    let body = body.into_bytes();
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

#[test]
fn unknown_magic_is_strict() {
    let b = v2_batch_with_magic(3, &[record(0, None, Some(b"v"), &[])], 0);
    assert_eq!(decode(&b), Err(SpecError::Strict(Strict::S6UnknownMagic)));
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

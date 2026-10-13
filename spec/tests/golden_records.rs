mod common;

use proto_kafka_spec::error::SpecError;
use proto_kafka_spec::records::{decode, decode_with, Entry, Mode, Payload, RecordSet};
use serde_json::{json, Value as J};
use std::collections::BTreeMap;

fn hex_or_null(b: &Option<Vec<u8>>) -> J {
    b.as_ref().map_or(J::Null, |b| {
        J::String(b.iter().map(|x| format!("{x:02x}")).collect())
    })
}

/// Header keys as Java's `h.key().getBytes(UTF_8)` sees them (lossy UTF-8 round trip).
fn lossy_key(k: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(k).into_owned().into_bytes()
}

/// True if the set holds a payload the spec keeps opaque (compressed v2 batch with
/// records, or a legacy wrapper with codec 1-3). Java decompresses these; the spec does
/// not validate their contents (documented non-goal, DIVERGENCES "Compressed payloads").
fn has_opaque_compressed(set: &RecordSet) -> bool {
    set.entries.iter().any(|e| match e {
        Entry::V2(b) => matches!(b.payload, Payload::Compressed(_)),
        Entry::Legacy(m) => m.attributes & 0x07 != 0,
    })
}

/// Maps the spec's record set to the summary Java emits (see Task 6 interface).
fn summarize(entries: &[Entry]) -> J {
    J::Array(
        entries
            .iter()
            .map(|e| match e {
                Entry::V2(b) => {
                    let log_append = b.attributes & 0x08 != 0;
                    let Payload::Records(recs) = &b.payload else { return json!("opaque compressed batch") };
                    json!({
                        "baseOffset": b.base_offset,
                        "lastOffset": b.base_offset + i64::from(b.last_offset_delta),
                        "partitionLeaderEpoch": b.partition_leader_epoch,
                        "magic": b.magic,
                        "crc": b.crc,
                        "producerId": b.producer_id,
                        "producerEpoch": b.producer_epoch,
                        "baseSequence": b.base_sequence,
                        "isTransactional": b.attributes & 0x10 != 0,
                        "isControl": b.attributes & 0x20 != 0,
                        "timestampType": if log_append { 1 } else { 0 },
                        "maxTimestamp": b.max_timestamp,
                        "records": recs.iter().map(|r| json!({
                            "offset": b.base_offset + i64::from(r.offset_delta),
                            "timestamp": if log_append { b.max_timestamp } else { b.base_timestamp + r.timestamp_delta },
                            "key": hex_or_null(&r.key),
                            "value": hex_or_null(&r.value),
                            "headers": r.headers.iter().map(|h| json!({
                                "key": hex_or_null(&Some(lossy_key(&h.key))),
                                "value": hex_or_null(&h.value),
                            })).collect::<Vec<_>>(),
                        })).collect::<Vec<_>>(),
                    })
                }
                Entry::Legacy(m) => {
                    let ts = m.timestamp.unwrap_or(-1);
                    let ts_type = match m.magic {
                        0 => -1,
                        _ if m.attributes & 0x08 != 0 => 1,
                        _ => 0,
                    };
                    json!({
                        "baseOffset": m.offset, "lastOffset": m.offset, "partitionLeaderEpoch": -1,
                        "magic": m.magic, "crc": m.crc, "producerId": -1, "producerEpoch": -1,
                        "baseSequence": -1, "isTransactional": false, "isControl": false,
                        "timestampType": ts_type, "maxTimestamp": ts,
                        "records": [{ "offset": m.offset, "timestamp": ts,
                                      "key": hex_or_null(&m.key), "value": hex_or_null(&m.value), "headers": [] }],
                    })
                }
            })
            .collect(),
    )
}

/// Strict rejections of Java-accepted vectors, by variant (asserted exactly).
const EXPECTED_STRICT: &[(&str, usize)] = &[("S1NegativeLength", 8), ("S2InvalidUtf8", 2)];
/// Java rejects, spec accepts because the payload is compressed and opaque (asserted exactly).
const EXPECTED_OPAQUE: usize = 1;

/// Same contract as `golden_messages::messages_conform_to_java`: Java-accepted vectors
/// decode in `JavaLenient` mode to Java's summary, and the strict decode equals the lenient
/// one or is `Err(Strict(_))`; Java-rejected vectors are rejected in both modes, except
/// opaque compressed payloads (counted separately).
#[test]
fn records_conform_to_java() {
    let mut failures = Vec::new();
    let mut strict: BTreeMap<String, usize> = BTreeMap::new();
    let mut opaque = 0usize;
    let mut origins: BTreeMap<String, usize> = BTreeMap::new();
    let lines = common::lines("records.jsonl");
    assert!(lines.len() > 100);
    for l in &lines {
        let bytes = common::hex(l["bytes"].as_str().unwrap());
        let origin = l["origin"].as_str().unwrap();
        let mutated = common::is_mutated(origin);
        *origins.entry(origin.to_string()).or_default() += 1;
        let ctx = format!("{origin} {}", l["bytes"]);
        let strict_res = decode(&bytes);
        let lenient_res = decode_with(&bytes, Mode::JavaLenient);
        if !l["ok"].as_bool().unwrap() {
            match (&strict_res, &lenient_res) {
                (Err(_), Err(_)) => {}
                (Ok(set), _) | (_, Ok(set)) if has_opaque_compressed(set) => opaque += 1,
                _ => failures.push(format!(
                    "{ctx}: Java rejects ({}), spec accepts (strict ok: {}, lenient ok: {})",
                    l["error"],
                    strict_res.is_ok(),
                    lenient_res.is_ok()
                )),
            }
            continue;
        }
        let lenient = match lenient_res {
            Ok(set) => set,
            Err(e) => {
                failures.push(format!("{ctx}: Java accepts, lenient rejects {e:?}"));
                continue;
            }
        };
        let got = summarize(&lenient.entries);
        if got != l["batches"] {
            failures.push(format!("{ctx}: lenient {got} != java {}", l["batches"]));
            continue;
        }
        match strict_res {
            Ok(set) if set == lenient => {}
            Ok(_) => failures.push(format!("{ctx}: strict decode differs from lenient")),
            Err(SpecError::Strict(s)) if mutated => {
                *strict.entry(format!("{s:?}")).or_default() += 1;
            }
            Err(e) => failures.push(format!("{ctx}: Java accepts, strict rejects {e:?}")),
        }
    }
    eprintln!(
        "checked {} record vectors {origins:?}; strict rejections where Java accepts: {strict:?}; \
         opaque compressed accepted where Java rejects: {opaque}",
        lines.len()
    );
    assert!(
        failures.is_empty(),
        "{} failures; first 10:\n{}",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    let expected: BTreeMap<String, usize> = EXPECTED_STRICT
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();
    assert_eq!(strict, expected, "strict histogram drifted");
    assert_eq!(opaque, EXPECTED_OPAQUE, "opaque-compressed count drifted");
}

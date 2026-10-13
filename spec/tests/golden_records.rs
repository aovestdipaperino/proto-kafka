mod common;

use proto_kafka_spec::error::SpecError;
use proto_kafka_spec::records::{decode, Entry, Payload};
use serde_json::{json, Value as J};

fn hex_or_null(b: &Option<Vec<u8>>) -> J {
    b.as_ref().map_or(J::Null, |b| {
        J::String(b.iter().map(|x| format!("{x:02x}")).collect())
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
                    let Payload::Records(recs) = &b.payload else { panic!("compressed batch in golden set") };
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
                                "key": hex_or_null(&Some(h.key.clone())),
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

#[test]
fn records_conform_to_java() {
    let mut failures = Vec::new();
    let mut strict: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let lines = common::lines("records.jsonl");
    assert!(lines.len() > 100);
    for l in &lines {
        let bytes = common::hex(l["bytes"].as_str().unwrap());
        let origin = l["origin"].as_str().unwrap();
        match (l["ok"].as_bool().unwrap(), decode(&bytes)) {
            (true, Ok(set)) => {
                let got = summarize(&set.entries);
                if got != l["batches"] {
                    failures.push(format!(
                        "{origin} {}: spec {got} != java {}",
                        l["bytes"], l["batches"]
                    ));
                }
            }
            (true, Err(SpecError::Strict(s))) if origin == "mutated" => {
                *strict.entry(format!("{s:?}")).or_default() += 1;
            }
            (true, Err(e)) => failures.push(format!(
                "{origin} {}: Java accepts, spec rejects {e:?}",
                l["bytes"]
            )),
            (false, Ok(_)) => failures.push(format!(
                "{origin} {}: Java rejects ({}), spec accepts",
                l["bytes"], l["error"]
            )),
            (false, Err(_)) => {}
        }
    }
    eprintln!(
        "checked {} record vectors; strict rejections where Java accepts: {strict:?}",
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
}

//! The crate's record batch decoder and encoder against Kafka 4.1.0's golden record vectors
//! (`verification/golden/vectors/records.jsonl`).
//!
//! Contract, per vector:
//! - Java rejected: the crate rejects.
//! - Java accepted, v2 only: the crate's batches and records equal Java's summary (batch
//!   header fields, delete horizon, and every record's offset, timestamp, sequence, key,
//!   value and headers), or the crate rejects and the rejection is an allowed strictness: the
//!   spec's strict mode rejects with `Strict(_)`, or the set ends in an incomplete batch, which
//!   Java silently drops and the crate refuses (D9, `verification/DIVERGENCES.md`).
//! - valid (Java wrote these bytes): the crate accepts, and re-encoding each decoded batch
//!   gives back Java's bytes exactly.
//!
//! Sets holding a legacy (v0/v1) entry are skipped and counted: the crate rejects legacy
//! message sets by design (`ProtoError::UnsupportedMessageSetVersion`).

use bytes::{Bytes, BytesMut};
use proto_kafka::records::{
    BatchDecodeInfo, Record, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions,
    RecordSet, TimestampType,
};
use proto_kafka_spec::error::SpecError;
use serde_json::{json, Value as J};
use std::collections::BTreeMap;
use std::io::BufRead;

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hex_or_null(b: Option<&[u8]>) -> J {
    b.map_or(J::Null, |b| J::String(to_hex(b)))
}

/// The crate's view of one batch in the shape `GoldenGen.emitRecordDecode` writes, minus the
/// fields the crate does not expose per batch (`lastOffset`, `crc`, `maxTimestamp`; the
/// re-encode check on valid vectors covers them).
fn summarize(info: &BatchDecodeInfo, set: &RecordSet) -> J {
    json!({
        "baseOffset": info.min_offset,
        "partitionLeaderEpoch": info.partition_leader_epoch,
        "magic": info.version,
        "producerId": info.producer_id,
        "producerEpoch": info.producer_epoch,
        "baseSequence": info.base_sequence,
        "isTransactional": info.transactional,
        "isControl": info.control,
        "timestampType": match info.timestamp_type {
            TimestampType::Creation => 0,
            TimestampType::LogAppend => 1,
        },
        "deleteHorizonMs": info.delete_horizon_ms,
        "records": set.records.iter().map(record_summary).collect::<Vec<_>>(),
    })
}

fn record_summary(r: &Record) -> J {
    json!({
        "offset": r.offset,
        "timestamp": r.timestamp,
        "sequence": r.sequence,
        "key": hex_or_null(r.key.as_deref()),
        "value": hex_or_null(r.value.as_deref()),
        "headers": r.headers.iter().map(|(k, v)| json!({
            "key": to_hex(k.as_bytes()),
            "value": hex_or_null(v.as_deref()),
        })).collect::<Vec<_>>(),
    })
}

fn java_summary(batch: &J) -> J {
    let mut b = batch.clone();
    let o = b.as_object_mut().unwrap();
    for k in ["lastOffset", "crc", "maxTimestamp"] {
        o.remove(k);
    }
    b
}

/// Decode with the crate: per-batch headers plus the decoded records.
fn crate_decode(bytes: &[u8]) -> Result<Vec<(BatchDecodeInfo, RecordSet)>, String> {
    let infos = RecordBatchDecoder::decode_batch_info(&mut Bytes::copy_from_slice(bytes))
        .map_err(|e| e.to_string())?;
    let sets = RecordBatchDecoder::decode_all(&mut Bytes::copy_from_slice(bytes))
        .map_err(|e| e.to_string())?;
    assert_eq!(
        infos.len(),
        sets.len(),
        "header and record decoders disagree"
    );
    Ok(infos.into_iter().zip(sets).collect())
}

fn crate_reencode(decoded: &[(BatchDecodeInfo, RecordSet)]) -> Result<Vec<u8>, String> {
    let mut out = BytesMut::new();
    for (_, set) in decoded {
        RecordBatchEncoder::encode(
            &mut out,
            &set.records,
            &RecordEncodeOptions {
                version: set.version,
                compression: set.compression,
            },
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(out.to_vec())
}

fn has_legacy_entry(bytes: &[u8], java_batches: &J) -> bool {
    let java_legacy = java_batches
        .as_array()
        .is_some_and(|bs| bs.iter().any(|b| b["magic"].as_i64() != Some(2)));
    java_legacy || bytes.get(16).is_some_and(|&m| m != 2)
}

/// Outcome counts, asserted exactly so that a drift in either decoder fails the test.
const EXPECTED: &[(&str, usize)] = &[
    ("crate_rejects_spec_strict_rejects", 2),
    ("crate_rejects_truncated_tail", 118),
    ("java_accepts_crate_matches", 56),
    ("java_rejects_crate_rejects", 233),
    ("skipped_legacy", 124),
    ("valid_reencoded_identically", 22),
];

#[test]
fn crate_records_conform_to_java_golden_vectors() {
    let path = format!(
        "{}/verification/golden/vectors/records.jsonl",
        env!("CARGO_MANIFEST_DIR")
    );
    let file = std::fs::File::open(&path).expect("records.jsonl");
    let mut failures = Vec::new();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for line in std::io::BufReader::new(file).lines() {
        let l: J = serde_json::from_str(&line.unwrap()).unwrap();
        let origin = l["origin"].as_str().unwrap();
        let bytes = hex(l["bytes"].as_str().unwrap());
        let ctx = format!("{origin} {}", l["bytes"]);
        let decoded = crate_decode(&bytes);

        if !l["ok"].as_bool().unwrap() {
            match decoded {
                Err(_) => *counts.entry("java_rejects_crate_rejects").or_default() += 1,
                Ok(_) => failures.push(format!(
                    "{ctx}: Java rejects ({}), crate accepts",
                    l["error"]
                )),
            }
            continue;
        }
        if has_legacy_entry(&bytes, &l["batches"]) {
            *counts.entry("skipped_legacy").or_default() += 1;
            continue;
        }
        let decoded = match decoded {
            Ok(d) => d,
            Err(e) if origin != "valid" => {
                let spec = proto_kafka_spec::records::decode(&bytes);
                match spec {
                    Err(SpecError::Strict(_)) => {
                        *counts
                            .entry("crate_rejects_spec_strict_rejects")
                            .or_default() += 1
                    }
                    Ok(set) if set.truncated_tail > 0 => {
                        *counts.entry("crate_rejects_truncated_tail").or_default() += 1
                    }
                    _ => failures.push(format!("{ctx}: Java accepts, crate rejects: {e}")),
                }
                continue;
            }
            Err(e) => {
                failures.push(format!("{ctx}: Java wrote it, crate rejects: {e}"));
                continue;
            }
        };
        let got = J::Array(decoded.iter().map(|(i, s)| summarize(i, s)).collect());
        let want = J::Array(
            l["batches"]
                .as_array()
                .unwrap()
                .iter()
                .map(java_summary)
                .collect(),
        );
        if got != want {
            failures.push(format!("{ctx}:\n  crate {got}\n  java  {want}"));
            continue;
        }
        *counts.entry("java_accepts_crate_matches").or_default() += 1;
        if origin == "valid" {
            match crate_reencode(&decoded) {
                Ok(out) if out == bytes => {
                    *counts.entry("valid_reencoded_identically").or_default() += 1
                }
                Ok(out) => failures.push(format!("{ctx}: re-encoded to {}", to_hex(&out))),
                Err(e) => failures.push(format!("{ctx}: re-encode failed: {e}")),
            }
        }
    }
    eprintln!("record vectors: {counts:?}");
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
    let expected: BTreeMap<&str, usize> = EXPECTED.iter().copied().collect();
    assert_eq!(counts, expected, "outcome histogram drifted");
}

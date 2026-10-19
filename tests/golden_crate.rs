//! The crate's real decoders and encoders against Kafka 4.1.0's golden vectors.
//!
//! Contract, per vector:
//! - default/random (Java wrote these bytes): the crate decodes, consumes every byte, and
//!   re-encodes to identical bytes.
//! - mutated, Java rejected: the crate rejects.
//! - mutated, Java accepted: the crate accepts and consumes what Java consumed, or rejects
//!   and the spec's strict mode rejects with `Strict(_)` (an allowed strictness).
//!
//! Vectors listed in `verification/crate_known_divergences.txt` (with a D-id) are skipped
//! and counted; an unlisted failure fails the test.
//!
//! Versions Kafka 4.1.0 marks `"latestVersionUnstable": true` are skipped and counted
//! separately: the crate's generator deliberately does not emit them
//! (`protocol_codegen/src/generate_messages/generate.rs`, `without_last()`), while the golden
//! generator includes them (`GoldenGen.java`, `highestSupportedVersion(true)`).
#![cfg(all(feature = "client", feature = "broker", feature = "messages_enums"))]

use bytes::{Bytes, BytesMut};
use proto_kafka::messages::{ApiKey, RequestHeader, RequestKind, ResponseHeader, ResponseKind};
use proto_kafka::protocol::{Decodable, Encodable};
use proto_kafka_spec::error::SpecError;
use proto_kafka_spec::schema::{find, find_by_name, Kind};
use std::collections::BTreeSet;
use std::io::BufRead;

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn root(p: &str) -> String {
    format!("{}/{}", env!("CARGO_MANIFEST_DIR"), p)
}

/// Decode with the crate; returns (consumed, re-encoded bytes or None if re-encoding failed).
fn crate_roundtrip(
    key: i16,
    kind: &str,
    v: i16,
    bytes: &[u8],
) -> Result<(usize, Option<Vec<u8>>), String> {
    let mut b = Bytes::copy_from_slice(bytes);
    let mut out = BytesMut::new();
    match kind {
        "request" => {
            let api = ApiKey::try_from(key).map_err(|e| format!("api key: {e:?}"))?;
            let m = RequestKind::decode(api, &mut b, v).map_err(|e| e.to_string())?;
            let consumed = bytes.len() - b.len();
            Ok((consumed, m.encode(&mut out, v).ok().map(|()| out.to_vec())))
        }
        "response" => {
            let api = ApiKey::try_from(key).map_err(|e| format!("api key: {e:?}"))?;
            let m = ResponseKind::decode(api, &mut b, v).map_err(|e| e.to_string())?;
            let consumed = bytes.len() - b.len();
            Ok((consumed, m.encode(&mut out, v).ok().map(|()| out.to_vec())))
        }
        "request_header" => {
            let m = RequestHeader::decode(&mut b, v).map_err(|e| e.to_string())?;
            let consumed = bytes.len() - b.len();
            Ok((consumed, m.encode(&mut out, v).ok().map(|()| out.to_vec())))
        }
        "response_header" => {
            let m = ResponseHeader::decode(&mut b, v).map_err(|e| e.to_string())?;
            let consumed = bytes.len() - b.len();
            Ok((consumed, m.encode(&mut out, v).ok().map(|()| out.to_vec())))
        }
        k => panic!("unknown kind {k}"),
    }
}

fn spec_is_strict_reject(key: i16, kind: &str, v: i16, bytes: &[u8]) -> bool {
    let schema = match kind {
        "request" => find(key, Kind::Request),
        "response" => find(key, Kind::Response),
        "request_header" => find_by_name("RequestHeader"),
        _ => find_by_name("ResponseHeader"),
    }
    .expect("schema");
    matches!(
        proto_kafka_spec::interp::decode(schema, v, bytes),
        Err(SpecError::Strict(_))
    )
}

/// `(api_key, version)` pairs that are the unstable latest version of a Kafka 4.1.0 request
/// schema (`clients/src/main/resources/common/message/*.json`, `"latestVersionUnstable": true`):
/// `InitProducerId` v6, `OffsetCommit` v10, `OffsetFetch` v10, `StreamsGroupHeartbeat` v0 and
/// `StreamsGroupDescribe` v0. The crate generates none of them (88 and 89 have no `ApiKey` at all).
const UNSTABLE_NOT_GENERATED: [(i16, i16); 5] = [(22, 6), (8, 10), (9, 10), (88, 0), (89, 0)];

fn known() -> BTreeSet<String> {
    let text = std::fs::read_to_string(root("verification/crate_known_divergences.txt"))
        .unwrap_or_default();
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| l.split_whitespace().take(4).collect::<Vec<_>>().join(" "))
        .collect()
}

#[test]
fn crate_conforms_to_java_golden_vectors() {
    let known = known();
    let f = std::fs::File::open(root("verification/golden/vectors/messages.jsonl")).unwrap();
    let (mut checked, mut skipped, mut unstable) = (0usize, 0usize, 0usize);
    let mut failures = Vec::new();
    for line in std::io::BufReader::new(f).lines() {
        let j: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let key = i16::try_from(j["api_key"].as_i64().unwrap()).unwrap();
        let kind = j["kind"].as_str().unwrap();
        let v = i16::try_from(j["version"].as_i64().unwrap()).unwrap();
        let origin = j["origin"].as_str().unwrap();
        let hexs = j["bytes"].as_str().unwrap();
        if matches!(kind, "request" | "response") && UNSTABLE_NOT_GENERATED.contains(&(key, v)) {
            unstable += 1;
            continue;
        }
        let id = format!("{key} {kind} {v} {hexs}");
        if known.contains(&id) {
            skipped += 1;
            continue;
        }
        checked += 1;
        let bytes = hex(hexs);
        let java_ok = j["ok"].as_bool().unwrap();
        let ours = crate_roundtrip(key, kind, v, &bytes);
        let fail = match (origin, java_ok, &ours) {
            ("mutated", false, Ok(_)) => {
                Some(format!("Java rejects ({}), crate accepts", j["error"]))
            }
            ("mutated", false, Err(_)) => None,
            ("mutated", true, Ok((n, _))) if *n as u64 == j["consumed"].as_u64().unwrap() => None,
            ("mutated", true, Ok((n, _))) => Some(format!("consumed {n}, Java {}", j["consumed"])),
            ("mutated", true, Err(_)) if spec_is_strict_reject(key, kind, v, &bytes) => None,
            ("mutated", true, Err(e)) => Some(format!("Java accepts, crate rejects: {e}")),
            (_, _, Ok((n, Some(re)))) if *n == bytes.len() && *re == bytes => None,
            (_, _, Ok((n, re))) => Some(format!(
                "consumed {n}/{}, re-encode equal: {}",
                bytes.len(),
                re.as_deref() == Some(&bytes[..])
            )),
            (_, _, Err(e)) => Some(format!("crate rejects Java-written bytes: {e}")),
        };
        if let Some(f) = fail {
            failures.push(format!("{id} [{origin}]: {f}"));
        }
    }
    eprintln!("checked {checked}, skipped (known divergences) {skipped}, skipped (unstable, not generated) {unstable}");
    assert!(
        failures.is_empty(),
        "{} failures; first 25:\n{}",
        failures.len(),
        failures
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

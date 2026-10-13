//! D2 regression: default-constructed headers encode exactly as Kafka 4.1.0's Java
//! defaults (golden vectors from verification/golden). Before regeneration the stale
//! generated code encoded nullable defaults as "" instead of null.

use bytes::BytesMut;
use proto_kafka::messages::{RequestHeader, ResponseHeader};
use proto_kafka::protocol::Encodable;
use std::io::BufRead;

fn java_default(kind: &str, version: i64) -> Vec<u8> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/verification/golden/vectors/messages.jsonl"
    );
    let f = std::fs::File::open(path).expect("golden vectors (Phase 1)");
    for line in std::io::BufReader::new(f).lines() {
        let j: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        if j["kind"] == kind && j["version"] == version && j["origin"] == "default" {
            let h = j["bytes"].as_str().unwrap();
            return (0..h.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
                .collect();
        }
    }
    panic!("no default vector for {kind} v{version}");
}

fn encode<T: Encodable>(m: &T, v: i16) -> Vec<u8> {
    let mut b = BytesMut::new();
    m.encode(&mut b, v).unwrap();
    b.to_vec()
}

#[test]
fn request_header_defaults_match_java() {
    for v in 1..=2 {
        assert_eq!(
            encode(&RequestHeader::default(), v),
            java_default("request_header", i64::from(v)),
            "v{v}"
        );
    }
}

#[test]
fn response_header_defaults_match_java() {
    for v in 0..=1 {
        assert_eq!(
            encode(&ResponseHeader::default(), v),
            java_default("response_header", i64::from(v)),
            "v{v}"
        );
    }
}

mod common;

use proto_kafka_spec::header::{request_header_version, response_header_version};
use proto_kafka_spec::schema::{find, Kind};

#[test]
fn header_versions_match_java() {
    let lines = common::lines("headers.jsonl");
    let expected: usize = proto_kafka_spec::generated_schemas::ALL
        .iter()
        .filter(|m| m.kind == Kind::Request)
        .map(|m| (m.valid.max - m.valid.min + 1) as usize)
        .sum();
    assert_eq!(
        lines.len(),
        expected,
        "one header line per request API version"
    );
    for l in lines {
        let key = l["api_key"].as_i64().unwrap() as i16;
        let v = l["version"].as_i64().unwrap() as i16;
        let req = find(key, Kind::Request).unwrap();
        let resp = find(key, Kind::Response).unwrap();
        assert_eq!(
            request_header_version(req, v) as i64,
            l["request_header"].as_i64().unwrap(),
            "{l}"
        );
        assert_eq!(
            response_header_version(resp, v) as i64,
            l["response_header"].as_i64().unwrap(),
            "{l}"
        );
    }
}

#[test]
fn json_helper_rejects_missing_untagged_field() {
    let m = find(10, Kind::Request).unwrap();
    let r = std::panic::catch_unwind(|| common::json_struct(m.root, 0, &serde_json::json!({})));
    assert!(r.is_err(), "absent untagged field must panic");
}
